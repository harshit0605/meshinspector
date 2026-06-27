//! In-place, heap-driven fast path for plain bulk decimation.
//!
//! The reference `decimate_mesh_serial_state` re-scans every edge and clones the
//! whole vertex+face arrays on every single collapse, which is O(V+F) per
//! collapse — fine for the tiny MeshLib-parity fixtures, catastrophic on a
//! 200k-face ring (measured ~5 collapses/s). This module reproduces the
//! reference's exact collapse *sequence* for the common "no flips / no twins /
//! no subset / no attributes / default boundary policy / unbounded boundary
//! shift" case using:
//!   * a persistent in-place mesh (no per-collapse array clone), and
//!   * a lazy min-heap of edge candidates re-keyed only around each collapse.
//!
//! It is gated (see `fast_eligible`) so every option combination the parity
//! tests exercise — and every mesh small enough to be a fixture — runs the
//! untouched reference path. Bit-exact equivalence with the reference on a
//! non-trivial mesh is asserted by tests in the parent module.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use crate::mesh_edit::topology;

use super::helpers::{
    distance_sq, has_duplicate_vertices, midpoint, points_almost_equal, triangle_area_sq,
    triangle_aspect_ratio,
};
use super::qem::{compute_vertex_forms, sum_forms_for_boundary_policy};
use super::types::{DecimateMeshOptions, DecimateMeshState, DecimateMeshStrategy, QuadraticForm};

/// True when the fast path reproduces the reference exactly and is worth the
/// setup cost. Anything outside this envelope falls back to the reference.
#[allow(clippy::too_many_arguments)]
pub(super) fn fast_eligible(
    options: &DecimateMeshOptions,
    candidate_region: &[bool],
    tracked_region: &[bool],
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
    twin_map_is_empty: bool,
    vertex_uvs: &Option<Vec<[f64; 2]>>,
    vertex_colors: &Option<Vec<[u8; 4]>>,
    face_count: usize,
) -> bool {
    // Below this size the reference is already instant and the parity fixtures
    // all live here, so keeping them on the reference path guarantees no drift.
    const FAST_PATH_MIN_FACES: usize = 2_000;
    let unbounded = f64::MAX.sqrt();
    face_count >= FAST_PATH_MIN_FACES
        && options.max_angle_change < 0.0 // no Delone flips
        && options.max_bd_shift >= unbounded // no boundary-shift guard
        && options.touch_near_bd_edges
        && options.touch_bd_verts // default boundary policy -> boundary set is inert
        && not_flippable_edges.is_empty()
        && edges_to_collapse.is_none()
        && twin_map_is_empty
        && vertex_uvs.is_none()
        && vertex_colors.is_none()
        && candidate_region == tracked_region
}

#[derive(Clone, Copy)]
struct HeapKey {
    cost: f64,
    edge: [usize; 2],
}

impl PartialEq for HeapKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for HeapKey {}
impl Ord for HeapKey {
    fn cmp(&self, other: &Self) -> Ordering {
        // Mirror compare_qem_candidates / compare_candidates: smallest cost then
        // smallest edge wins. BinaryHeap is a max-heap, so invert the order.
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.edge.cmp(&self.edge))
    }
}
impl PartialOrd for HeapKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn squared_limit(value: f64) -> f64 {
    if value >= f64::MAX.sqrt() {
        f64::INFINITY
    } else {
        value * value
    }
}

struct FastMesh {
    vertices: Vec<[f64; 3]>,
    faces: Vec<[usize; 3]>,
    alive_face: Vec<bool>,
    candidate_region: Vec<bool>,
    tracked_region: Vec<bool>,
    alive_vertex: Vec<bool>,
    vertex_faces: Vec<Vec<usize>>,
    forms: Option<Vec<QuadraticForm>>,
}

impl FastMesh {
    /// Faces incident to `vertex`, pruned of any that no longer contain it.
    fn incident_faces(&mut self, vertex: usize) -> Vec<usize> {
        let faces = &self.faces;
        let alive = &self.alive_face;
        self.vertex_faces[vertex]
            .retain(|face_index| alive[*face_index] && faces[*face_index].contains(&vertex));
        self.vertex_faces[vertex].clone()
    }

    fn edge_has_region_face(&mut self, edge: [usize; 2]) -> bool {
        let incident = self.incident_faces(edge[0]);
        incident.iter().any(|face_index| {
            self.candidate_region[*face_index] && self.faces[*face_index].contains(&edge[1])
        })
    }
}

/// Candidate cost + collapse position (+ summed QEM form for the minimize-error
/// strategy) for an edge under the current geometry. Mirrors
/// `qem_candidate_edge` / `shortest_candidate_edge` for the eligible
/// (boundary-None) case. `None` means "not a valid candidate".
fn edge_candidate(
    mesh: &FastMesh,
    edge: [usize; 2],
    options: &DecimateMeshOptions,
    max_error_sq: f64,
) -> Option<(f64, [f64; 3], Option<QuadraticForm>)> {
    match options.strategy {
        DecimateMeshStrategy::ShortestEdgeFirst => {
            let length_sq = distance_sq(mesh.vertices[edge[0]], mesh.vertices[edge[1]]);
            if !length_sq.is_finite() || length_sq > max_error_sq {
                return None;
            }
            let collapse_pos = if options.optimize_vertex_pos {
                midpoint(mesh.vertices[edge[0]], mesh.vertices[edge[1]])
            } else {
                mesh.vertices[edge[0]]
            };
            Some((length_sq, collapse_pos, None))
        }
        DecimateMeshStrategy::MinimizeError => {
            let forms = mesh.forms.as_ref()?;
            let (collapse_form, collapse_pos) = sum_forms_for_boundary_policy(
                forms[edge[0]],
                mesh.vertices[edge[0]],
                forms[edge[1]],
                mesh.vertices[edge[1]],
                edge,
                None,
                options,
            );
            if !collapse_form.c.is_finite() || collapse_form.c > max_error_sq {
                return None;
            }
            Some((collapse_form.c, collapse_pos, Some(collapse_form)))
        }
    }
}

struct CollapseOutcome {
    kept: usize,
    dropped: usize,
    faces_deleted: usize,
    collapse_pos: [f64; 3],
}

/// Validates `collapse_plan`'s guards (eligible case) on the local neighbourhood
/// without committing or cloning. The kept vertex is moved temporarily to
/// evaluate post-collapse geometry, then restored, so a rejected or
/// limit-deferred collapse leaves the mesh untouched.
fn validate_collapse(
    mesh: &mut FastMesh,
    edge: [usize; 2],
    collapse_pos: [f64; 3],
    options: &DecimateMeshOptions,
) -> Option<CollapseOutcome> {
    let mut keep = edge[0];
    let mut drop = edge[1];
    if points_almost_equal(collapse_pos, mesh.vertices[drop])
        && !points_almost_equal(collapse_pos, mesh.vertices[keep])
    {
        keep = edge[1];
        drop = edge[0];
    }

    // Metrics taken from the original endpoint positions (pre-move).
    let edge_len_sq = distance_sq(mesh.vertices[edge[0]], mesh.vertices[edge[1]]);
    let endpoint_collapse = points_almost_equal(collapse_pos, mesh.vertices[edge[0]])
        || points_almost_equal(collapse_pos, mesh.vertices[edge[1]]);

    let mut touched = mesh.incident_faces(keep);
    for face_index in mesh.incident_faces(drop) {
        if !touched.contains(&face_index) {
            touched.push(face_index);
        }
    }

    // MeshLib-faithful manifold guard (local form): a fused edge (m,w) must not
    // exceed two incident faces. `touched` already holds every face incident to
    // edge[0] or edge[1], which is exactly where (u,w)/(v,w) edges live.
    if creates_nonmanifold_edge_local(mesh, edge, &touched) {
        return None;
    }

    let mut max_old_aspect_ratio = options.max_triangle_aspect_ratio;
    for face_index in &touched {
        let face = mesh.faces[*face_index];
        if face.contains(&keep) || face.contains(&drop) {
            max_old_aspect_ratio =
                max_old_aspect_ratio.max(triangle_aspect_ratio(&mesh.vertices, face));
        }
    }

    let original_keep_pos = mesh.vertices[keep];
    mesh.vertices[keep] = collapse_pos;

    let mut max_new_aspect_ratio: f64 = 0.0;
    let mut faces_deleted = 0usize;
    let mut surviving: Vec<[usize; 3]> = Vec::with_capacity(touched.len());
    for face_index in &touched {
        let face = mesh.faces[*face_index];
        let face_touched = face.contains(&keep) || face.contains(&drop);
        let mut mapped = face;
        for vertex in &mut mapped {
            if *vertex == drop {
                *vertex = keep;
            }
        }
        if has_duplicate_vertices(mapped) || triangle_area_sq(&mesh.vertices, mapped) <= 1e-24 {
            faces_deleted += 1;
            continue;
        }
        if face_touched {
            max_new_aspect_ratio =
                max_new_aspect_ratio.max(triangle_aspect_ratio(&mesh.vertices, mapped));
        }
        surviving.push(mapped);
    }
    let violates_edge_len =
        local_violates_max_edge_len(&surviving, keep, options.max_edge_len, &mesh.vertices);
    mesh.vertices[keep] = original_keep_pos;

    let tiny_edge = options.tiny_edge_length >= 0.0
        && endpoint_collapse
        && edge_len_sq <= options.tiny_edge_length * options.tiny_edge_length;
    let violates_aspect_ratio = !tiny_edge
        && max_new_aspect_ratio > max_old_aspect_ratio
        && max_old_aspect_ratio <= options.critical_tri_aspect_ratio;

    if faces_deleted == 0
        || violates_edge_len
        || violates_aspect_ratio
        || local_has_duplicate_faces(&surviving)
    {
        return None;
    }

    Some(CollapseOutcome {
        kept: keep,
        dropped: drop,
        faces_deleted,
        collapse_pos,
    })
}

/// Local form of `helpers::creates_nonmanifold_edge` over the faces incident to
/// either endpoint (`touched`). Equivalent to the whole-face-list form on a
/// manifold mesh, since every face holding (u,w) or (v,w) contains u or v.
fn creates_nonmanifold_edge_local(mesh: &FastMesh, edge: [usize; 2], touched: &[usize]) -> bool {
    let [u, v] = edge;
    let mut merged = BTreeMap::<usize, usize>::new();
    for &face_index in touched {
        let face = mesh.faces[face_index];
        let has_u = face.contains(&u);
        let has_v = face.contains(&v);
        if has_u && has_v {
            continue;
        }
        if has_u {
            for &w in &face {
                if w != u {
                    *merged.entry(w).or_insert(0) += 1;
                }
            }
        } else if has_v {
            for &w in &face {
                if w != v {
                    *merged.entry(w).or_insert(0) += 1;
                }
            }
        }
    }
    merged.values().any(|incident_faces| *incident_faces > 2)
}

fn local_violates_max_edge_len(
    surviving: &[[usize; 3]],
    moved_vertex: usize,
    max_edge_len: f64,
    vertices: &[[f64; 3]],
) -> bool {
    if max_edge_len >= f64::MAX.sqrt() {
        return false;
    }
    let max_edge_len_sq = max_edge_len * max_edge_len;
    surviving
        .iter()
        .copied()
        .filter(|face| face.contains(&moved_vertex))
        .flat_map(topology::oriented_face_edges)
        .any(|edge| {
            (edge[0] == moved_vertex || edge[1] == moved_vertex)
                && distance_sq(vertices[edge[0]], vertices[edge[1]]) > max_edge_len_sq
        })
}

fn local_has_duplicate_faces(surviving: &[[usize; 3]]) -> bool {
    let mut seen = BTreeSet::<[usize; 3]>::new();
    for face in surviving {
        let mut sorted = *face;
        sorted.sort_unstable();
        if !seen.insert(sorted) {
            return true;
        }
    }
    false
}

fn apply_collapse(mesh: &mut FastMesh, outcome: &CollapseOutcome) {
    let keep = outcome.kept;
    let drop = outcome.dropped;
    mesh.vertices[keep] = outcome.collapse_pos;

    let mut affected = mesh.incident_faces(keep);
    for face_index in mesh.incident_faces(drop) {
        if !affected.contains(&face_index) {
            affected.push(face_index);
        }
    }

    for face_index in affected {
        if !mesh.alive_face[face_index] {
            continue;
        }
        let mut face = mesh.faces[face_index];
        for vertex in &mut face {
            if *vertex == drop {
                *vertex = keep;
            }
        }
        if has_duplicate_vertices(face) || triangle_area_sq(&mesh.vertices, face) <= 1e-24 {
            mesh.alive_face[face_index] = false;
            for vertex in mesh.faces[face_index] {
                mesh.vertex_faces[vertex].retain(|f| *f != face_index);
            }
            continue;
        }
        mesh.faces[face_index] = face;
        if !mesh.vertex_faces[keep].contains(&face_index) {
            mesh.vertex_faces[keep].push(face_index);
        }
    }

    mesh.alive_vertex[drop] = false;
    mesh.vertex_faces[drop].clear();
    // Prune dead faces from the kept vertex's incidence list. Borrow the two
    // fields disjointly instead of `mesh.alive_face.clone()` — the clone copied
    // the whole face-count-length Vec on *every* collapse (Θ(F) per collapse →
    // Θ(F²) total), which was the entire source of the superlinear scaling.
    let FastMesh {
        alive_face,
        vertex_faces,
        ..
    } = mesh;
    vertex_faces[keep].retain(|f| alive_face[*f]);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn decimate_mesh_serial_state_fast(
    vertices: Vec<[f64; 3]>,
    faces: Vec<[usize; 3]>,
    candidate_region: Vec<bool>,
    tracked_region: Vec<bool>,
    options: &DecimateMeshOptions,
    not_flippable_edges: BTreeSet<[usize; 2]>,
    edges_to_collapse: Option<BTreeSet<[usize; 2]>>,
    twin_map: BTreeMap<[usize; 2], [usize; 2]>,
    vertex_uvs: Option<Vec<[f64; 2]>>,
    vertex_colors: Option<Vec<[u8; 4]>>,
) -> DecimateMeshState {
    let vertex_count = vertices.len();
    let face_count = faces.len();
    let forms = (options.strategy == DecimateMeshStrategy::MinimizeError).then(|| {
        compute_vertex_forms(
            &vertices,
            &faces,
            &candidate_region,
            &not_flippable_edges,
            options.angle_weighted_dist_to_plane,
            options.stabilizer,
        )
    });

    let mut alive_vertex = vec![false; vertex_count];
    let mut vertex_faces: Vec<Vec<usize>> = vec![Vec::new(); vertex_count];
    for (face_index, face) in faces.iter().enumerate() {
        for vertex in face {
            vertex_faces[*vertex].push(face_index);
            alive_vertex[*vertex] = true;
        }
    }

    let mut mesh = FastMesh {
        vertices,
        faces,
        alive_face: vec![true; face_count],
        candidate_region,
        tracked_region,
        alive_vertex,
        vertex_faces,
        forms,
    };

    let max_error_sq = squared_limit(options.max_error);

    let mut heap: BinaryHeap<HeapKey> = BinaryHeap::new();
    let mut seeded = BTreeSet::<[usize; 2]>::new();
    for face_index in 0..mesh.faces.len() {
        if !mesh.candidate_region[face_index] {
            continue;
        }
        for raw in topology::oriented_face_edges(mesh.faces[face_index]) {
            let edge = topology::ordered_edge(raw[0], raw[1]);
            if seeded.insert(edge) {
                if let Some((cost, _, _)) = edge_candidate(&mesh, edge, options, max_error_sq) {
                    heap.push(HeapKey { cost, edge });
                }
            }
        }
    }

    let mut verts_deleted = 0usize;
    let mut faces_deleted = 0usize;
    let mut alive_faces = face_count;
    let mut error_introduced = options.max_error;

    while let Some(HeapKey { cost, edge }) = heap.pop() {
        let [a, b] = edge;
        if !mesh.alive_vertex[a] || !mesh.alive_vertex[b] {
            continue;
        }
        // Discard stale entries: when an edge's cost last changed, a fresh entry
        // was pushed, so a popped entry whose recomputed cost differs is stale.
        let Some((current_cost, collapse_pos, collapse_form)) =
            edge_candidate(&mesh, edge, options, max_error_sq)
        else {
            continue;
        };
        if current_cost != cost {
            continue;
        }
        if !mesh.edge_has_region_face(edge) {
            continue;
        }

        let Some(outcome) = validate_collapse(&mut mesh, edge, collapse_pos, options) else {
            continue;
        };

        if verts_deleted + 1 > options.max_deleted_vertices
            || faces_deleted + outcome.faces_deleted > options.max_deleted_faces
        {
            error_introduced = cost.sqrt();
            break;
        }

        apply_collapse(&mut mesh, &outcome);
        // Reference stores the *selection-time* summed form (computed from the
        // pre-collapse endpoints) at forms[edge[0]]; replicate exactly so the
        // next iteration's candidate costs match bit-for-bit.
        if let (Some(forms), Some(form)) = (mesh.forms.as_mut(), collapse_form) {
            forms[edge[0]] = form;
        }
        verts_deleted += 1;
        faces_deleted += outcome.faces_deleted;
        alive_faces -= outcome.faces_deleted;

        // Re-key every edge of every in-region alive face now incident to the
        // kept vertex: covers cost changes at `kept` and validity changes of the
        // surrounding 1-ring.
        let incident = mesh.incident_faces(outcome.kept);
        let mut touched_edges = BTreeSet::<[usize; 2]>::new();
        for face_index in incident {
            if !mesh.candidate_region[face_index] {
                continue;
            }
            for raw in topology::oriented_face_edges(mesh.faces[face_index]) {
                touched_edges.insert(topology::ordered_edge(raw[0], raw[1]));
            }
        }
        for e in touched_edges {
            if let Some((cost, _, _)) = edge_candidate(&mesh, e, options, max_error_sq) {
                heap.push(HeapKey { cost, edge: e });
            }
        }

        if alive_faces == 0 {
            error_introduced = options.max_error;
            break;
        }
    }

    finalize(
        mesh,
        verts_deleted,
        faces_deleted,
        error_introduced,
        not_flippable_edges,
        edges_to_collapse,
        twin_map,
        vertex_uvs,
        vertex_colors,
    )
}

#[allow(clippy::too_many_arguments)]
fn finalize(
    mesh: FastMesh,
    verts_deleted: usize,
    faces_deleted: usize,
    error_introduced: f64,
    not_flippable_edges: BTreeSet<[usize; 2]>,
    edges_to_collapse: Option<BTreeSet<[usize; 2]>>,
    twin_map: BTreeMap<[usize; 2], [usize; 2]>,
    vertex_uvs: Option<Vec<[f64; 2]>>,
    vertex_colors: Option<Vec<[u8; 4]>>,
) -> DecimateMeshState {
    let FastMesh {
        vertices,
        faces,
        alive_face,
        tracked_region,
        ..
    } = mesh;

    let mut out_faces = Vec::with_capacity(faces.len());
    let mut out_region = Vec::with_capacity(faces.len());
    for (face_index, face) in faces.into_iter().enumerate() {
        if alive_face[face_index] {
            out_faces.push(face);
            out_region.push(tracked_region[face_index]);
        }
    }

    DecimateMeshState {
        vertices,
        faces: out_faces,
        region: out_region,
        verts_deleted,
        faces_deleted,
        error_introduced,
        not_flippable_edges,
        edges_to_collapse,
        twin_map,
        vertex_uvs,
        vertex_colors,
    }
}
