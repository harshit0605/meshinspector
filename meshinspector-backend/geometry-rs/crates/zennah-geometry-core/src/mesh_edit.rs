use crate::{GeometryError, MeshArrays};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

mod decimate;
mod delone;
mod input;
mod offset_verts;
mod priority;
mod projection;
mod smooth;
mod subdivide_delone;
mod topology;

pub use decimate::{decimate_mesh, DecimateMeshOptions, DecimateMeshResult, DecimateMeshStrategy};
pub use delone::{make_delone_edge_flips, MakeDeloneEdgeFlipsOptions, MakeDeloneEdgeFlipsResult};
pub use offset_verts::offset_verts_mesh;

use topology::{
    face_edges, face_has_oriented_edge, ordered_edge, oriented_edge_with_opposite,
    split_face_on_edge, EdgeRecord, EdgeState, VertexFaces,
};
pub(in crate::mesh_edit) use topology::{opposite_edge, opposite_vertex};

#[derive(Debug, Clone, PartialEq)]
pub struct SubdivideMeshOptions {
    pub max_edge_len: f64,
    pub curvature_priority: f64,
    pub max_edge_splits: usize,
    pub subdivide_border: bool,
    pub project_on_original_mesh: bool,
    pub project_new_vertices_to_unit_sphere: bool,
    pub smooth_mode: bool,
    pub min_sharp_dihedral_angle: f64,
    pub max_tri_aspect_ratio: f64,
    pub max_splittable_tri_aspect_ratio: f64,
    pub max_deviation_after_flip: Option<f64>,
    pub max_angle_change_after_flip: Option<f64>,
    pub critical_tri_aspect_ratio_flip: Option<f64>,
    pub region_faces: Option<Vec<usize>>,
    pub not_flippable_edges: Vec<[usize; 2]>,
}

impl Default for SubdivideMeshOptions {
    fn default() -> Self {
        Self {
            max_edge_len: 0.0,
            curvature_priority: 0.0,
            max_edge_splits: 1000,
            subdivide_border: true,
            project_on_original_mesh: false,
            project_new_vertices_to_unit_sphere: false,
            smooth_mode: false,
            min_sharp_dihedral_angle: std::f64::consts::PI / 6.0,
            max_tri_aspect_ratio: 0.0,
            max_splittable_tri_aspect_ratio: f64::MAX,
            max_deviation_after_flip: Some(1.0),
            max_angle_change_after_flip: None,
            critical_tri_aspect_ratio_flip: Some(1000.0),
            region_faces: None,
            not_flippable_edges: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubdivideMeshResult {
    pub mesh: MeshArrays,
    pub splits_done: usize,
    pub region_faces: Vec<usize>,
    pub region_face_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CandidateEdge {
    vertices: [usize; 2],
    length_sq: f64,
    rank: usize,
}

pub fn subdivide_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    options: SubdivideMeshOptions,
) -> Result<SubdivideMeshResult, GeometryError> {
    input::validate_subdivide_options(&options)?;
    let mut vertices = vertices.to_vec();
    let mut faces = input::validate_faces(faces_i64, vertices.len())?;
    let original_mesh = options
        .project_on_original_mesh
        .then(|| (vertices.clone(), faces.clone()));
    let mut edge_state = EdgeState::from_faces(&faces);
    // Vertex→faces adjacency, maintained incrementally so each split looks up the
    // faces on an edge / around a vertex in O(degree) rather than rescanning the
    // whole mesh — the original quadratic cost.
    let mut vertex_faces = VertexFaces::from_faces(&faces, vertices.len());
    let mut region = input::initial_region(faces.len(), options.region_faces.as_deref())?;
    let mut not_flippable_edges =
        normalize_not_flippable_edges(&options.not_flippable_edges, vertices.len())?;
    let max_edge_len_sq = options.max_edge_len * options.max_edge_len;
    let mut vertex_pseudo_normals = (options.curvature_priority > 0.0)
        .then(|| priority::compute_per_vertex_pseudo_normals(&vertices, &faces));
    let mut splits_done = 0;
    let mut vertices_to_project = Vec::<usize>::new();
    let mut vertices_to_smooth = Vec::<usize>::new();
    let mut queued_splittable_edges = BTreeSet::<[usize; 2]>::new();
    let mut above_splittable_faces = BTreeSet::<usize>::new();
    if options.max_splittable_tri_aspect_ratio < f64::MAX {
        refresh_above_splittable_faces(
            &vertices,
            &faces,
            &region,
            options.max_splittable_tri_aspect_ratio,
            &mut above_splittable_faces,
        );
        cache_splittable_candidate_edges(
            &vertices,
            &faces,
            &region,
            max_edge_len_sq,
            options.subdivide_border,
            options.max_splittable_tri_aspect_ratio,
            options.curvature_priority,
            vertex_pseudo_normals.as_deref(),
            None,
            &mut queued_splittable_edges,
        );
    }

    // When the whole mesh is in the subdivide region with border splitting and
    // no curvature/aspect gating, the next edge can be picked straight from the
    // incremental EdgeState instead of rebuilding the edge→faces map every
    // iteration (which made subdivision quadratic). Output is identical.
    let full_region_fast = options.curvature_priority == 0.0
        && options.max_splittable_tri_aspect_ratio == f64::MAX
        && options.max_tri_aspect_ratio < 1.0
        && options.subdivide_border
        && region.iter().all(|&selected| selected);

    while splits_done < options.max_edge_splits {
        if options.max_tri_aspect_ratio >= 1.0
            && !region_has_tri_above_aspect(
                &vertices,
                &faces,
                &region,
                options.max_tri_aspect_ratio,
            )
        {
            break;
        }
        let candidate = if full_region_fast {
            longest_candidate_edge_full_region(&vertices, &edge_state, max_edge_len_sq)
        } else {
            longest_candidate_edge(
                &vertices,
                &faces,
                &region,
                max_edge_len_sq,
                options.subdivide_border,
                options.max_splittable_tri_aspect_ratio,
                options.curvature_priority,
                vertex_pseudo_normals.as_deref(),
                &edge_state,
                &queued_splittable_edges,
            )
        };
        let Some(candidate) = candidate else {
            break;
        };

        let is_inner_split_edge = (options.project_on_original_mesh || options.smooth_mode)
            && projection::is_inner_edge(&faces, candidate.vertices);
        let new_vertex = split_edge(
            &mut vertices,
            &mut faces,
            &mut region,
            &mut edge_state,
            &mut vertex_faces,
            &mut not_flippable_edges,
            candidate.vertices,
            &options,
        );
        splits_done += 1;
        if options.project_on_original_mesh && is_inner_split_edge {
            vertices_to_project.push(new_vertex);
        }
        if options.smooth_mode && is_inner_split_edge {
            vertices_to_smooth.push(new_vertex);
        }
        if let Some(normals) = &mut vertex_pseudo_normals {
            normals.push(priority::interpolated_split_normal(
                normals,
                candidate.vertices,
            ));
        }
        if options.max_splittable_tri_aspect_ratio < f64::MAX {
            cache_recovered_splittable_face_edges(
                &vertices,
                &faces,
                &region,
                max_edge_len_sq,
                options.subdivide_border,
                options.max_splittable_tri_aspect_ratio,
                options.curvature_priority,
                vertex_pseudo_normals.as_deref(),
                &mut above_splittable_faces,
                &mut queued_splittable_edges,
            );
            cache_splittable_candidate_edges(
                &vertices,
                &faces,
                &region,
                max_edge_len_sq,
                options.subdivide_border,
                options.max_splittable_tri_aspect_ratio,
                options.curvature_priority,
                vertex_pseudo_normals.as_deref(),
                Some(new_vertex),
                &mut queued_splittable_edges,
            );
        }
    }

    if let Some((original_vertices, original_faces)) = original_mesh {
        projection::project_vertices_to_original_mesh(
            &mut vertices,
            &vertices_to_project,
            &original_vertices,
            &original_faces,
        );
    }
    if options.smooth_mode {
        smooth::smooth_vertices_cotan(
            &mut vertices,
            &faces,
            &vertices_to_smooth,
            options.min_sharp_dihedral_angle,
        );
    }

    let region_faces: Vec<usize> = region
        .iter()
        .enumerate()
        .filter_map(|(index, selected)| selected.then_some(index))
        .collect();
    Ok(SubdivideMeshResult {
        mesh: MeshArrays {
            vertices,
            faces: faces
                .into_iter()
                .map(|face| [face[0] as i64, face[1] as i64, face[2] as i64])
                .collect(),
        },
        splits_done,
        region_face_count: region_faces.len(),
        region_faces,
    })
}

fn longest_candidate_edge(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    max_edge_len_sq: f64,
    subdivide_border: bool,
    max_splittable_tri_aspect_ratio: f64,
    curvature_priority: f64,
    vertex_pseudo_normals: Option<&[[f64; 3]]>,
    edge_state: &EdgeState,
    queued_splittable_edges: &BTreeSet<[usize; 2]>,
) -> Option<CandidateEdge> {
    let incident = edge_incident_faces(faces);
    incident
        .into_iter()
        .filter_map(|(edge, face_indices)| {
            if !edge_is_in_subdivide_region(&face_indices, region, subdivide_border) {
                return None;
            }
            let length_sq =
                priority::edge_len_sq(vertices, vertex_pseudo_normals, edge, curvature_priority);
            if length_sq < max_edge_len_sq {
                return None;
            }
            if max_splittable_tri_aspect_ratio < f64::MAX
                && !queued_splittable_edges.contains(&edge)
            {
                return None;
            }
            Some(CandidateEdge {
                vertices: edge,
                length_sq,
                rank: edge_state.record(edge).rank,
            })
        })
        .max_by(compare_candidate_edges)
}

/// Fast equivalent of `longest_candidate_edge` for the common full-region case
/// (whole mesh selected, `subdivide_border`, no curvature priority, no
/// splittable-aspect gating). It scans the incrementally-maintained `EdgeState`
/// edge set instead of rebuilding the edge→faces map from every face on every
/// iteration — the rebuild made subdivision quadratic in mesh size. Because the
/// edge set, the `(length_sq, rank, vertices)` key, and `max_by` ordering are
/// identical to the linear scan (every edge qualifies once the region/border
/// filter is a no-op, and `edge_len_sq` reduces to the plain squared length when
/// `curvature_priority == 0`), the chosen split edge — and therefore the whole
/// output — is bit-for-bit identical to the original path.
fn longest_candidate_edge_full_region(
    vertices: &[[f64; 3]],
    edge_state: &EdgeState,
    max_edge_len_sq: f64,
) -> Option<CandidateEdge> {
    edge_state
        .iter_edges()
        .filter_map(|(edge, rank)| {
            let length_sq = priority::edge_len_sq(vertices, None, edge, 0.0);
            if length_sq < max_edge_len_sq {
                return None;
            }
            Some(CandidateEdge {
                vertices: edge,
                length_sq,
                rank,
            })
        })
        .max_by(compare_candidate_edges)
}

fn region_has_tri_above_aspect(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    max_tri_aspect_ratio: f64,
) -> bool {
    faces.iter().enumerate().any(|(face_index, face)| {
        region.get(face_index).copied().unwrap_or(false)
            && priority::triangle_aspect_ratio(vertices, *face) > max_tri_aspect_ratio
    })
}

fn edge_touches_too_narrow_splittable_face(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    face_indices: &[usize],
    max_splittable_tri_aspect_ratio: f64,
) -> bool {
    face_indices.iter().any(|face_index| {
        region.get(*face_index).copied().unwrap_or(false)
            && priority::triangle_aspect_ratio(vertices, faces[*face_index])
                > max_splittable_tri_aspect_ratio
    })
}

pub(in crate::mesh_edit) fn edge_incident_faces(
    faces: &[[usize; 3]],
) -> BTreeMap<[usize; 2], Vec<usize>> {
    let mut incident = BTreeMap::<[usize; 2], Vec<usize>>::new();
    for (face_index, face) in faces.iter().enumerate() {
        for edge in face_edges(*face) {
            incident.entry(edge).or_default().push(face_index);
        }
    }
    incident
}

fn cache_splittable_candidate_edges(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    max_edge_len_sq: f64,
    subdivide_border: bool,
    max_splittable_tri_aspect_ratio: f64,
    curvature_priority: f64,
    vertex_pseudo_normals: Option<&[[f64; 3]]>,
    incident_vertex: Option<usize>,
    queued_splittable_edges: &mut BTreeSet<[usize; 2]>,
) {
    for (edge, face_indices) in edge_incident_faces(faces) {
        if let Some(vertex) = incident_vertex {
            if edge[0] != vertex && edge[1] != vertex {
                continue;
            }
        }
        if !edge_is_in_subdivide_region(&face_indices, region, subdivide_border) {
            continue;
        }
        if priority::edge_len_sq(vertices, vertex_pseudo_normals, edge, curvature_priority)
            < max_edge_len_sq
        {
            continue;
        }
        if edge_touches_too_narrow_splittable_face(
            vertices,
            faces,
            region,
            &face_indices,
            max_splittable_tri_aspect_ratio,
        ) {
            continue;
        }
        queued_splittable_edges.insert(edge);
    }
}

fn refresh_above_splittable_faces(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    max_splittable_tri_aspect_ratio: f64,
    above_splittable_faces: &mut BTreeSet<usize>,
) {
    above_splittable_faces.clear();
    for (face_index, face) in faces.iter().enumerate() {
        if region.get(face_index).copied().unwrap_or(false)
            && priority::triangle_aspect_ratio(vertices, *face) > max_splittable_tri_aspect_ratio
        {
            above_splittable_faces.insert(face_index);
        }
    }
}

fn cache_recovered_splittable_face_edges(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    max_edge_len_sq: f64,
    subdivide_border: bool,
    max_splittable_tri_aspect_ratio: f64,
    curvature_priority: f64,
    vertex_pseudo_normals: Option<&[[f64; 3]]>,
    above_splittable_faces: &mut BTreeSet<usize>,
    queued_splittable_edges: &mut BTreeSet<[usize; 2]>,
) {
    let incident = edge_incident_faces(faces);
    for (face_index, face) in faces.iter().enumerate() {
        if !region.get(face_index).copied().unwrap_or(false) {
            above_splittable_faces.remove(&face_index);
            continue;
        }
        let above =
            priority::triangle_aspect_ratio(vertices, *face) > max_splittable_tri_aspect_ratio;
        if above {
            above_splittable_faces.insert(face_index);
            continue;
        }
        if !above_splittable_faces.remove(&face_index) {
            continue;
        }
        for edge in face_edges(*face) {
            let Some(face_indices) = incident.get(&edge) else {
                continue;
            };
            if !edge_is_in_subdivide_region(face_indices, region, subdivide_border) {
                continue;
            }
            if priority::edge_len_sq(vertices, vertex_pseudo_normals, edge, curvature_priority)
                < max_edge_len_sq
            {
                continue;
            }
            if edge_touches_too_narrow_splittable_face(
                vertices,
                faces,
                region,
                face_indices,
                max_splittable_tri_aspect_ratio,
            ) {
                continue;
            }
            queued_splittable_edges.insert(edge);
        }
    }
}

fn edge_is_in_subdivide_region(
    face_indices: &[usize],
    region: &[bool],
    subdivide_border: bool,
) -> bool {
    let selected = face_indices
        .iter()
        .filter(|face_index| region.get(**face_index).copied().unwrap_or(false))
        .count();
    if subdivide_border {
        selected > 0
    } else {
        selected > 0 && selected == face_indices.len()
    }
}

fn normalize_not_flippable_edges(
    edges: &[[usize; 2]],
    vertex_count: usize,
) -> Result<BTreeSet<[usize; 2]>, GeometryError> {
    let mut normalized = BTreeSet::new();
    for edge in edges {
        if edge[0] >= vertex_count || edge[1] >= vertex_count || edge[0] == edge[1] {
            return Err(GeometryError::InvalidMeshEditInput {
                field: "not_flippable_edges",
                value: edge[0].max(edge[1]) as f64,
            });
        }
        normalized.insert(ordered_edge(edge[0], edge[1]));
    }
    Ok(normalized)
}

fn compare_candidate_edges(left: &CandidateEdge, right: &CandidateEdge) -> Ordering {
    left.length_sq
        .total_cmp(&right.length_sq)
        .then_with(|| left.rank.cmp(&right.rank))
        .then_with(|| left.vertices.cmp(&right.vertices))
}

/// Faces incident to `edge`, ordered (org→dest first, dest→org next, anything
/// else last) — identical to the original `incident_faces_in_split_order` full
/// scan, but the candidate faces come from the `VertexFaces` adjacency
/// (ascending index, matching the old scan order) instead of every face.
fn incident_faces_in_split_order(
    faces: &[[usize; 3]],
    vertex_faces: &VertexFaces,
    edge: [usize; 2],
    record: EdgeRecord,
) -> Vec<usize> {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut other = Vec::new();
    for face_index in vertex_faces.edge_faces(edge[0], edge[1]) {
        let face = faces[face_index];
        if face_has_oriented_edge(face, record.org, record.dest) {
            left.push(face_index);
        } else if face_has_oriented_edge(face, record.dest, record.org) {
            right.push(face_index);
        } else {
            other.push(face_index);
        }
    }
    left.into_iter().chain(right).chain(other).collect()
}

fn split_edge(
    vertices: &mut Vec<[f64; 3]>,
    faces: &mut Vec<[usize; 3]>,
    region: &mut Vec<bool>,
    edge_state: &mut EdgeState,
    vertex_faces: &mut VertexFaces,
    not_flippable_edges: &mut BTreeSet<[usize; 2]>,
    edge: [usize; 2],
    options: &SubdivideMeshOptions,
) -> usize {
    let new_vertex = [
        (vertices[edge[0]][0] + vertices[edge[1]][0]) * 0.5,
        (vertices[edge[0]][1] + vertices[edge[1]][1]) * 0.5,
        (vertices[edge[0]][2] + vertices[edge[1]][2]) * 0.5,
    ];
    let new_vertex_index = vertices.len();
    vertices.push(new_vertex);
    vertex_faces.add_vertex();

    let edge_record = edge_state.record(edge);
    let face_indices = incident_faces_in_split_order(faces, vertex_faces, edge, edge_record);
    let connector_vertices: Vec<usize> = face_indices
        .iter()
        .filter_map(|face_index| opposite_vertex(faces[*face_index], edge))
        .collect();
    edge_state.split_edge(edge, new_vertex_index, &connector_vertices);
    if not_flippable_edges.remove(&edge) {
        not_flippable_edges.insert(ordered_edge(edge[0], new_vertex_index));
        not_flippable_edges.insert(ordered_edge(new_vertex_index, edge[1]));
    }

    for face_index in face_indices {
        let old_tri = faces[face_index];
        let selected = region[face_index];
        let (first, second) = split_face_on_edge(old_tri, edge_record, new_vertex_index);
        let new_face_index = faces.len();
        faces[face_index] = first;
        faces.push(second);
        region[face_index] = selected;
        region.push(selected);
        vertex_faces.replace_face(face_index, old_tri, first);
        vertex_faces.add_face(new_face_index, second);
    }
    if options.project_new_vertices_to_unit_sphere {
        vertices[new_vertex_index] = project_to_unit_sphere(vertices[new_vertex_index]);
    }
    subdivide_delone::make_delone_origin_ring(
        vertices,
        faces,
        region,
        edge_state,
        vertex_faces,
        not_flippable_edges,
        new_vertex_index,
        options,
    );
    #[cfg(debug_assertions)]
    debug_assert!(
        vertex_faces.matches_faces(faces),
        "VertexFaces desynced from faces after split"
    );
    new_vertex_index
}

fn project_to_unit_sphere(point: [f64; 3]) -> [f64; 3] {
    let len_sq = point[0] * point[0] + point[1] * point[1] + point[2] * point[2];
    if len_sq <= f64::EPSILON {
        return point;
    }
    let inv_len = 1.0 / len_sq.sqrt();
    [point[0] * inv_len, point[1] * inv_len, point[2] * inv_len]
}

pub(in crate::mesh_edit) fn delone_edge_satisfied(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    edge: [usize; 2],
    face_pair: [usize; 2],
) -> bool {
    let Some((a_index, c_index, d_index)) = oriented_edge_with_opposite(faces[face_pair[0]], edge)
    else {
        return true;
    };
    let Some(b_index) = opposite_vertex(faces[face_pair[1]], edge) else {
        return true;
    };
    let a = vertices[a_index];
    let b = vertices[b_index];
    let c = vertices[c_index];
    let d = vertices[d_index];
    delone::quadrangle_satisfied(a, b, c, d)
}

pub(in crate::mesh_edit) fn flip_edge(
    faces: &mut [[usize; 3]],
    region: &mut [bool],
    edge_state: &mut EdgeState,
    edge: [usize; 2],
    face_pair: [usize; 2],
) {
    let first = faces[face_pair[0]];
    let second = faces[face_pair[1]];
    let Some((a, b, c)) = oriented_edge_with_opposite(first, edge) else {
        return;
    };
    let Some((_, _, d)) = oriented_edge_with_opposite(second, edge) else {
        return;
    };
    if c == d {
        return;
    }
    edge_state.flip_edge(edge, c, d);
    faces[face_pair[0]] = [c, d, b];
    faces[face_pair[1]] = [d, c, a];
    let first_region = region[face_pair[0]];
    let second_region = region[face_pair[1]];
    region[face_pair[0]] = first_region;
    region[face_pair[1]] = second_region;
}
