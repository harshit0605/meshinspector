use crate::mesh_edit::topology;
use crate::{GeometryError, MeshArrays};
use std::collections::{BTreeMap, BTreeSet};

use super::geometry::distance_sq_to_segment;
pub(super) use super::geometry::{
    add, distance_sq, dot, face_normal, invert_3x3, mat_vec, midpoint, points_almost_equal, sub,
    triangle_area_sq, triangle_aspect_ratio,
};
use super::qem::boundary_edges_in_region;
use super::types::{CollapsePlan, DecimateMeshOptions};

pub(super) fn normalize_not_flippable_edges(
    edges: &[[usize; 2]],
    vertex_count: usize,
) -> Result<BTreeSet<[usize; 2]>, GeometryError> {
    let mut normalized = BTreeSet::new();
    for edge in edges {
        if edge[0] == edge[1] || edge[0] >= vertex_count || edge[1] >= vertex_count {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "not_flippable_edges",
                value: format!("{edge:?}"),
            });
        }
        normalized.insert(topology::ordered_edge(edge[0], edge[1]));
    }
    Ok(normalized)
}

pub(super) fn normalize_edges_to_collapse(
    edges: Option<&[[usize; 2]]>,
    vertex_count: usize,
) -> Result<Option<BTreeSet<[usize; 2]>>, GeometryError> {
    let Some(edges) = edges else {
        return Ok(None);
    };
    let mut normalized = BTreeSet::new();
    for edge in edges {
        if edge[0] == edge[1] || edge[0] >= vertex_count || edge[1] >= vertex_count {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "edges_to_collapse",
                value: format!("{edge:?}"),
            });
        }
        normalized.insert(topology::ordered_edge(edge[0], edge[1]));
    }
    Ok(Some(normalized))
}

pub(super) fn normalize_twin_map(
    entries: &[[[usize; 2]; 2]],
    vertex_count: usize,
) -> Result<BTreeMap<[usize; 2], [usize; 2]>, GeometryError> {
    let mut normalized = BTreeMap::new();
    for entry in entries {
        let key = normalize_twin_edge(entry[0], vertex_count, entry)?;
        let value = normalize_twin_edge(entry[1], vertex_count, entry)?;
        if key == value {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "twin_map",
                value: format!("{entry:?}"),
            });
        }
        if let Some(existing) = normalized.insert(key, value) {
            if existing != value {
                return Err(GeometryError::InvalidSelectionParameter {
                    field: "twin_map",
                    value: format!("{entry:?}"),
                });
            }
        }
    }
    for (key, value) in &normalized {
        if normalized.get(value) != Some(key) {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "twin_map",
                value: format!("{key:?}->{value:?} missing reciprocal entry"),
            });
        }
    }
    Ok(normalized)
}

fn normalize_twin_edge(
    edge: [usize; 2],
    vertex_count: usize,
    entry: &[[usize; 2]; 2],
) -> Result<[usize; 2], GeometryError> {
    if edge[0] == edge[1] || edge[0] >= vertex_count || edge[1] >= vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "twin_map",
            value: format!("{entry:?}"),
        });
    }
    Ok(topology::ordered_edge(edge[0], edge[1]))
}

pub(super) fn edge_blocked_by_not_flippable(
    edge: [usize; 2],
    not_flippable_edges: &BTreeSet<[usize; 2]>,
) -> bool {
    if not_flippable_edges.is_empty() || not_flippable_edges.contains(&edge) {
        return false;
    }
    not_flippable_edges.iter().any(|protected| {
        protected[0] == edge[0]
            || protected[0] == edge[1]
            || protected[1] == edge[0]
            || protected[1] == edge[1]
    })
}

pub(super) fn edge_allowed_by_edges_to_collapse(
    edge: [usize; 2],
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
) -> bool {
    edges_to_collapse
        .as_ref()
        .map(|edges| edges.contains(&edge))
        .unwrap_or(true)
}

pub(super) fn remap_not_flippable_edges_after_collapse(
    not_flippable_edges: &mut BTreeSet<[usize; 2]>,
    kept_vertex: usize,
    dropped_vertex: usize,
) {
    if not_flippable_edges.is_empty() {
        return;
    }
    *not_flippable_edges = not_flippable_edges
        .iter()
        .filter_map(|edge| {
            let first = if edge[0] == dropped_vertex {
                kept_vertex
            } else {
                edge[0]
            };
            let second = if edge[1] == dropped_vertex {
                kept_vertex
            } else {
                edge[1]
            };
            (first != second).then_some(topology::ordered_edge(first, second))
        })
        .collect();
}

pub(super) fn remap_edges_to_collapse_after_collapse(
    edges_to_collapse: &mut Option<BTreeSet<[usize; 2]>>,
    kept_vertex: usize,
    dropped_vertex: usize,
) {
    if let Some(edges_to_collapse) = edges_to_collapse.as_mut() {
        remap_not_flippable_edges_after_collapse(edges_to_collapse, kept_vertex, dropped_vertex);
    }
}

pub(super) fn remap_twin_map_after_collapse(
    twin_map: &mut BTreeMap<[usize; 2], [usize; 2]>,
    kept_vertex: usize,
    dropped_vertex: usize,
) {
    if twin_map.is_empty() {
        return;
    }
    let previous = std::mem::take(twin_map);
    let mut remapped = BTreeMap::new();
    for (key, value) in previous {
        if key > value {
            continue;
        }
        let Some(next_key) = remap_twin_edge_after_collapse(key, kept_vertex, dropped_vertex)
        else {
            continue;
        };
        let Some(next_value) = remap_twin_edge_after_collapse(value, kept_vertex, dropped_vertex)
        else {
            continue;
        };
        if next_key == next_value {
            continue;
        }
        remapped.insert(next_key, next_value);
        remapped.insert(next_value, next_key);
    }
    *twin_map = remapped;
}

fn remap_twin_edge_after_collapse(
    edge: [usize; 2],
    kept_vertex: usize,
    dropped_vertex: usize,
) -> Option<[usize; 2]> {
    let first = if edge[0] == dropped_vertex {
        kept_vertex
    } else {
        edge[0]
    };
    let second = if edge[1] == dropped_vertex {
        kept_vertex
    } else {
        edge[1]
    };
    (first != second).then_some(topology::ordered_edge(first, second))
}

pub(super) fn output_edges_to_collapse(
    faces: &[[usize; 3]],
    edges_to_collapse: Option<&BTreeSet<[usize; 2]>>,
    pack_mesh: bool,
    vertex_count: usize,
) -> Vec<[usize; 2]> {
    edges_to_collapse
        .map(|edges| output_not_flippable_edges(faces, edges, pack_mesh, vertex_count))
        .unwrap_or_default()
}

pub(super) fn output_not_flippable_edges(
    faces: &[[usize; 3]],
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    pack_mesh: bool,
    vertex_count: usize,
) -> Vec<[usize; 2]> {
    if !pack_mesh {
        return not_flippable_edges.iter().copied().collect();
    }
    let used: BTreeSet<usize> = faces.iter().flat_map(|face| face.iter().copied()).collect();
    let mut mapping = vec![usize::MAX; vertex_count];
    let mut next_index = 0_usize;
    for old_index in 0..vertex_count {
        if used.contains(&old_index) {
            mapping[old_index] = next_index;
            next_index += 1;
        }
    }
    not_flippable_edges
        .iter()
        .filter_map(|edge| {
            let first = mapping.get(edge[0]).copied().unwrap_or(usize::MAX);
            let second = mapping.get(edge[1]).copied().unwrap_or(usize::MAX);
            (first != usize::MAX && second != usize::MAX && first != second)
                .then_some(topology::ordered_edge(first, second))
        })
        .collect()
}

pub(super) fn output_twin_map(
    faces: &[[usize; 3]],
    twin_map: &BTreeMap<[usize; 2], [usize; 2]>,
    pack_mesh: bool,
    vertex_count: usize,
) -> Vec<[[usize; 2]; 2]> {
    if !pack_mesh {
        return twin_map.iter().map(|(key, value)| [*key, *value]).collect();
    }
    let used: BTreeSet<usize> = faces.iter().flat_map(|face| face.iter().copied()).collect();
    let mut mapping = vec![usize::MAX; vertex_count];
    let mut next_index = 0_usize;
    for old_index in 0..vertex_count {
        if used.contains(&old_index) {
            mapping[old_index] = next_index;
            next_index += 1;
        }
    }
    let mut packed = BTreeMap::new();
    for (key, value) in twin_map {
        if key > value {
            continue;
        }
        let Some(packed_key) = pack_twin_edge(*key, &mapping) else {
            continue;
        };
        let Some(packed_value) = pack_twin_edge(*value, &mapping) else {
            continue;
        };
        if packed_key == packed_value {
            continue;
        }
        packed.insert(packed_key, packed_value);
        packed.insert(packed_value, packed_key);
    }
    packed.iter().map(|(key, value)| [*key, *value]).collect()
}

fn pack_twin_edge(edge: [usize; 2], mapping: &[usize]) -> Option<[usize; 2]> {
    let first = mapping.get(edge[0]).copied().unwrap_or(usize::MAX);
    let second = mapping.get(edge[1]).copied().unwrap_or(usize::MAX);
    (first != usize::MAX && second != usize::MAX && first != second)
        .then_some(topology::ordered_edge(first, second))
}

pub(super) fn boundary_vertices_in_region(
    faces: &[[usize; 3]],
    region: &[bool],
) -> BTreeSet<usize> {
    boundary_edges_in_region(faces, region)
        .into_iter()
        .flat_map(|edge| [edge[0], edge[1]])
        .collect()
}

pub(super) fn subdivide_part_region(
    region: &[bool],
    subdivide_parts: usize,
    part_index: usize,
) -> Vec<bool> {
    let selected_indices: Vec<usize> = region
        .iter()
        .enumerate()
        .filter_map(|(index, selected)| selected.then_some(index))
        .collect();
    let start = selected_indices.len() * part_index / subdivide_parts;
    let end = selected_indices.len() * (part_index + 1) / subdivide_parts;
    let mut part = vec![false; region.len()];
    for face_index in &selected_indices[start..end] {
        part[*face_index] = true;
    }
    part
}

pub(super) fn proportional_quota(
    limit: usize,
    part_face_count: usize,
    total_face_count: usize,
) -> usize {
    if limit == usize::MAX {
        return usize::MAX;
    }
    ((limit as u128 * part_face_count as u128) / total_face_count.max(1) as u128) as usize
}

pub(super) fn collapse_plan(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    candidate_region: &[bool],
    tracked_region: &[bool],
    edge: [usize; 2],
    collapse_pos: [f64; 3],
    options: &DecimateMeshOptions,
) -> Option<CollapsePlan> {
    // MeshLib-faithful manifold guard: collapsing edge (u,v) fuses, for every
    // common neighbour w, the edges (u,w) and (v,w) into a single edge (m,w).
    // That edge must end up with at most two incident faces or the surface
    // becomes non-manifold. (This is the *necessary* condition MeshLib enforces,
    // not the stricter link condition — a common neighbour reached only through
    // boundary edges is fine.)
    if creates_nonmanifold_edge(faces, edge) {
        return None;
    }

    let mut keep = edge[0];
    let mut drop = edge[1];
    if points_almost_equal(collapse_pos, vertices[drop])
        && !points_almost_equal(collapse_pos, vertices[keep])
    {
        keep = edge[1];
        drop = edge[0];
    }
    let mut next_vertices = vertices.to_vec();
    next_vertices[keep] = collapse_pos;

    let mut next_faces = Vec::with_capacity(faces.len());
    let mut next_candidate_region = Vec::with_capacity(candidate_region.len());
    let mut next_tracked_region = Vec::with_capacity(tracked_region.len());
    let mut faces_deleted = 0;
    if violates_max_boundary_shift(
        vertices,
        faces,
        candidate_region,
        edge,
        collapse_pos,
        options.max_bd_shift,
    ) {
        return None;
    }

    let mut max_old_aspect_ratio = options.max_triangle_aspect_ratio;
    let mut max_new_aspect_ratio: f64 = 0.0;
    for (face_index, face) in faces.iter().copied().enumerate() {
        let face_touched = face.contains(&keep) || face.contains(&drop);
        if face_touched {
            max_old_aspect_ratio = max_old_aspect_ratio.max(triangle_aspect_ratio(vertices, face));
        }
        let mut mapped = face;
        for vertex in &mut mapped {
            if *vertex == drop {
                *vertex = keep;
            }
        }
        if has_duplicate_vertices(mapped) || triangle_area_sq(&next_vertices, mapped) <= 1e-24 {
            faces_deleted += 1;
            continue;
        }
        if face_touched {
            max_new_aspect_ratio =
                max_new_aspect_ratio.max(triangle_aspect_ratio(&next_vertices, mapped));
        }
        next_faces.push(mapped);
        next_candidate_region.push(candidate_region[face_index]);
        next_tracked_region.push(tracked_region[face_index]);
    }

    let edge_len_sq = distance_sq(vertices[edge[0]], vertices[edge[1]]);
    let endpoint_collapse = points_almost_equal(collapse_pos, vertices[edge[0]])
        || points_almost_equal(collapse_pos, vertices[edge[1]]);
    let tiny_edge = options.tiny_edge_length >= 0.0
        && endpoint_collapse
        && edge_len_sq <= options.tiny_edge_length * options.tiny_edge_length;
    let violates_aspect_ratio = !tiny_edge
        && max_new_aspect_ratio > max_old_aspect_ratio
        && max_old_aspect_ratio <= options.critical_tri_aspect_ratio;
    if faces_deleted == 0
        || violates_max_edge_len(&next_vertices, &next_faces, keep, options.max_edge_len)
        || violates_aspect_ratio
        || has_duplicate_faces(&next_faces)
    {
        return None;
    }

    Some(CollapsePlan {
        vertices: next_vertices,
        faces: next_faces,
        candidate_region: next_candidate_region,
        tracked_region: next_tracked_region,
        faces_deleted,
        kept_vertex: keep,
        dropped_vertex: drop,
    })
}

fn violates_max_boundary_shift(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    edge: [usize; 2],
    collapse_pos: [f64; 3],
    max_bd_shift: f64,
) -> bool {
    if max_bd_shift >= f64::MAX.sqrt() {
        return false;
    }
    let max_shift_sq = max_bd_shift * max_bd_shift;
    let boundary_edges = boundary_edges_in_region(faces, region);
    for vertex in edge {
        if points_almost_equal(vertices[vertex], collapse_pos) {
            continue;
        }
        let mut has_boundary_edge = false;
        let mut close_to_existing_boundary = false;
        for boundary_edge in boundary_edges.iter().copied() {
            if boundary_edge[0] != vertex && boundary_edge[1] != vertex {
                continue;
            }
            has_boundary_edge = true;
            let distance_sq = distance_sq_to_segment(
                collapse_pos,
                vertices[boundary_edge[0]],
                vertices[boundary_edge[1]],
            );
            if distance_sq <= max_shift_sq {
                close_to_existing_boundary = true;
                break;
            }
        }
        if has_boundary_edge && !close_to_existing_boundary {
            return true;
        }
    }
    false
}

fn violates_max_edge_len(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    moved_vertex: usize,
    max_edge_len: f64,
) -> bool {
    if max_edge_len >= f64::MAX.sqrt() {
        return false;
    }
    let max_edge_len_sq = max_edge_len * max_edge_len;
    faces
        .iter()
        .copied()
        .filter(|face| face.contains(&moved_vertex))
        .flat_map(topology::oriented_face_edges)
        .any(|edge| {
            (edge[0] == moved_vertex || edge[1] == moved_vertex)
                && distance_sq(vertices[edge[0]], vertices[edge[1]]) > max_edge_len_sq
        })
}

/// Whether collapsing edge `(u, v)` would push a fused edge `(m, w)` above two
/// incident faces — the necessary-and-sufficient non-manifold condition for a
/// triangle-mesh edge collapse. `merged[w]` counts the faces that will be
/// incident to `(m, w)` after the collapse: faces holding `(u, w)` without `v`
/// plus faces holding `(v, w)` without `u` (the apex triangles `(u, v, w)` are
/// deleted, so they are skipped). Reference (whole-face-list) form.
pub(super) fn creates_nonmanifold_edge(faces: &[[usize; 3]], edge: [usize; 2]) -> bool {
    let [u, v] = edge;
    let mut merged = BTreeMap::<usize, usize>::new();
    for face in faces {
        let has_u = face.contains(&u);
        let has_v = face.contains(&v);
        if has_u && has_v {
            continue; // apex triangle (u,v,w) is deleted by the collapse
        }
        if has_u {
            for &w in face {
                if w != u {
                    *merged.entry(w).or_insert(0) += 1;
                }
            }
        } else if has_v {
            for &w in face {
                if w != v {
                    *merged.entry(w).or_insert(0) += 1;
                }
            }
        }
    }
    merged.values().any(|incident_faces| *incident_faces > 2)
}

fn has_duplicate_faces(faces: &[[usize; 3]]) -> bool {
    let mut seen = BTreeSet::<[usize; 3]>::new();
    for face in faces {
        let mut sorted = *face;
        sorted.sort_unstable();
        if !seen.insert(sorted) {
            return true;
        }
    }
    false
}

pub(super) fn finish_mesh(
    vertices: Vec<[f64; 3]>,
    faces: Vec<[usize; 3]>,
    pack_mesh: bool,
) -> MeshArrays {
    if pack_mesh {
        pack_mesh_arrays(&vertices, &faces)
    } else {
        MeshArrays {
            vertices,
            faces: faces
                .into_iter()
                .map(|face| [face[0] as i64, face[1] as i64, face[2] as i64])
                .collect(),
        }
    }
}

fn pack_mesh_arrays(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> MeshArrays {
    let used: BTreeSet<usize> = faces.iter().flat_map(|face| face.iter().copied()).collect();
    let mut mapping = vec![usize::MAX; vertices.len()];
    let mut packed_vertices = Vec::with_capacity(used.len());
    for (old_index, vertex) in vertices.iter().copied().enumerate() {
        if used.contains(&old_index) {
            mapping[old_index] = packed_vertices.len();
            packed_vertices.push(vertex);
        }
    }
    let packed_faces = faces
        .iter()
        .map(|face| {
            [
                mapping[face[0]] as i64,
                mapping[face[1]] as i64,
                mapping[face[2]] as i64,
            ]
        })
        .collect();
    MeshArrays {
        vertices: packed_vertices,
        faces: packed_faces,
    }
}

pub(super) fn has_duplicate_vertices(face: [usize; 3]) -> bool {
    face[0] == face[1] || face[1] == face[2] || face[2] == face[0]
}
