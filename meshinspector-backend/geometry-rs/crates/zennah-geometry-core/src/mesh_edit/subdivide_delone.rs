use super::topology::{ordered_edge, EdgeState, VertexFaces};
use super::{delone, flip_edge, opposite_edge, opposite_vertex, SubdivideMeshOptions};
use std::collections::BTreeSet;

pub(super) fn make_delone_origin_ring(
    vertices: &[[f64; 3]],
    faces: &mut [[usize; 3]],
    region: &mut [bool],
    edge_state: &mut EdgeState,
    vertex_faces: &mut VertexFaces,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    origin: usize,
    options: &SubdivideMeshOptions,
) {
    let max_flips = faces.len().saturating_mul(8).max(1);
    for _ in 0..max_flips {
        let Some((edge, face_pair)) = first_non_delone_origin_ring_edge(
            vertices,
            faces,
            region,
            vertex_faces,
            edge_state,
            not_flippable_edges,
            origin,
            options,
        ) else {
            break;
        };
        let before_first = faces[face_pair[0]];
        let before_second = faces[face_pair[1]];
        flip_edge(faces, region, edge_state, edge, face_pair);
        // Keep the adjacency in sync with the two faces the flip rewrote (a no-op
        // when flip_edge bailed out and left the faces unchanged).
        vertex_faces.replace_face(face_pair[0], before_first, faces[face_pair[0]]);
        vertex_faces.replace_face(face_pair[1], before_second, faces[face_pair[1]]);
    }
}

fn first_non_delone_origin_ring_edge(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    vertex_faces: &VertexFaces,
    edge_state: &EdgeState,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    origin: usize,
    options: &SubdivideMeshOptions,
) -> Option<([usize; 2], [usize; 2])> {
    let mut visited = BTreeSet::<[usize; 2]>::new();
    // The faces around `origin` (ascending index) are exactly the faces the old
    // full scan kept after filtering `face_contains_vertex(origin)`, in the same
    // order — so the first non-Delone edge chosen is unchanged.
    for face_index in vertex_faces.faces_around(origin) {
        let face = faces[*face_index];
        let Some(edge) = opposite_edge(face, origin) else {
            continue;
        };
        if !visited.insert(edge) || not_flippable_edges.contains(&edge) {
            continue;
        }
        let face_indices = vertex_faces.edge_faces(edge[0], edge[1]);
        if face_indices.len() != 2 {
            continue;
        }
        if !region.get(face_indices[0]).copied().unwrap_or(false)
            || !region.get(face_indices[1]).copied().unwrap_or(false)
        {
            continue;
        }
        let first = faces[face_indices[0]];
        let second = faces[face_indices[1]];
        let first_opposite = opposite_vertex(first, edge)?;
        let second_opposite = opposite_vertex(second, edge)?;
        if first_opposite == second_opposite {
            continue;
        }
        // Flipping would duplicate an existing edge — skip. EdgeState already
        // tracks the live edge set, so this is O(log E) instead of scanning faces.
        if edge_state.contains(ordered_edge(first_opposite, second_opposite)) {
            continue;
        }
        if !delone::delone_edge_satisfied_with_flip_limits(
            vertices,
            faces,
            edge,
            [face_indices[0], face_indices[1]],
            options.max_deviation_after_flip,
            options.max_angle_change_after_flip,
            options.critical_tri_aspect_ratio_flip,
        ) {
            return Some((edge, [face_indices[0], face_indices[1]]));
        }
    }
    None
}
