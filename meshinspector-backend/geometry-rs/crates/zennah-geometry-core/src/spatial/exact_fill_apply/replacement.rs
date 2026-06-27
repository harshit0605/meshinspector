use super::{ordered_edge, ordered_edge_set, ExactCutHoleFillPlan};
use crate::spatial::exact_cut_apply::ExactCutMeshResult;
use crate::spatial::exact_fill_plan::exact_planar_hole_fill_plan;
use std::collections::BTreeSet;

pub(super) fn closed_cut_path_fill_plans(
    cut_mesh: &ExactCutMeshResult,
    epsilon: f64,
    boundary_edge_sets: &BTreeSet<BTreeSet<[usize; 2]>>,
) -> Vec<ExactCutHoleFillPlan> {
    cut_mesh
        .cut_edge_paths
        .iter()
        .zip(&cut_mesh.cut_edge_path_closed)
        .enumerate()
        .filter_map(|(path_index, (path, closed))| {
            if !closed || path.len() < 3 {
                return None;
            }
            let edge_set = ordered_edge_set(path);
            if boundary_edge_sets.contains(&edge_set) {
                return None;
            }
            let boundary_loop = closed_path_boundary_loop(path)?;
            let fill_plan =
                exact_planar_hole_fill_plan(&cut_mesh.vertices, &boundary_loop, epsilon)?;
            let source_face = source_face_for_path(cut_mesh, path_index, path);
            Some(ExactCutHoleFillPlan {
                representative_edge: path[0],
                boundary_loop,
                boundary_edges: path.to_vec(),
                source_face,
                source_face_for_faces: vec![source_face; fill_plan.num_tris],
                fill_plan,
            })
        })
        .collect()
}

fn closed_path_boundary_loop(path: &[[usize; 2]]) -> Option<Vec<usize>> {
    let first = path.first()?;
    if path.last()?.get(1).copied() != Some(first[0]) {
        return None;
    }
    if !path.windows(2).all(|window| window[0][1] == window[1][0]) {
        return None;
    }
    Some(path.iter().map(|edge| edge[0]).collect())
}

fn source_face_for_path(
    cut_mesh: &ExactCutMeshResult,
    path_index: usize,
    path: &[[usize; 2]],
) -> usize {
    let fallback = cut_mesh
        .cut_edge_path_source_faces
        .get(path_index)
        .and_then(|source_faces| source_faces.iter().flatten().next().copied())
        .or_else(|| {
            path.iter()
                .find_map(|edge| source_face_for_edge(cut_mesh, *edge))
        })
        .unwrap_or(0);
    cut_mesh
        .cut_edge_path_source_faces
        .get(path_index)
        .and_then(|source_faces| missing_side_source_face(cut_mesh, path, source_faces))
        .unwrap_or(fallback)
}

fn missing_side_source_face(
    cut_mesh: &ExactCutMeshResult,
    path: &[[usize; 2]],
    path_source_faces: &[Option<usize>],
) -> Option<usize> {
    let left_sources = primary_side_source_faces(cut_mesh, path, DirectedEdgeSide::Left);
    let right_sources = primary_side_source_faces(cut_mesh, path, DirectedEdgeSide::Right);
    let left_mismatches = side_source_mismatches(path_source_faces, &left_sources);
    let right_mismatches = side_source_mismatches(path_source_faces, &right_sources);
    if left_mismatches == 0 && right_mismatches == 0 {
        return None;
    }
    if left_mismatches > right_mismatches {
        left_sources.into_iter().flatten().next()
    } else if right_mismatches > left_mismatches {
        right_sources.into_iter().flatten().next()
    } else {
        None
    }
}

fn primary_side_source_faces(
    cut_mesh: &ExactCutMeshResult,
    path: &[[usize; 2]],
    side: DirectedEdgeSide,
) -> Vec<Option<usize>> {
    path.iter()
        .map(|edge| {
            source_faces_for_edge_side(cut_mesh, *edge, side)
                .first()
                .copied()
        })
        .collect()
}

fn side_source_mismatches(
    path_source_faces: &[Option<usize>],
    side_source_faces: &[Option<usize>],
) -> usize {
    path_source_faces
        .iter()
        .zip(side_source_faces)
        .filter(|(path_source, side_source)| side_source.is_some() && side_source != path_source)
        .count()
}

#[derive(Clone, Copy)]
enum DirectedEdgeSide {
    Left,
    Right,
}

fn source_face_for_edge(cut_mesh: &ExactCutMeshResult, edge: [usize; 2]) -> Option<usize> {
    source_faces_for_edge(cut_mesh, edge).first().copied()
}

fn source_faces_for_edge(cut_mesh: &ExactCutMeshResult, edge: [usize; 2]) -> Vec<usize> {
    source_faces_for_edge_side(cut_mesh, edge, DirectedEdgeSide::Left)
        .into_iter()
        .chain(source_faces_for_edge_side(
            cut_mesh,
            edge,
            DirectedEdgeSide::Right,
        ))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn source_faces_for_edge_side(
    cut_mesh: &ExactCutMeshResult,
    edge: [usize; 2],
    side: DirectedEdgeSide,
) -> Vec<usize> {
    let directed_edge = match side {
        DirectedEdgeSide::Left => edge,
        DirectedEdgeSide::Right => [edge[1], edge[0]],
    };
    let edge = ordered_edge(edge);
    let mut source_faces = cut_mesh
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            (0..3)
                .any(|edge_index| {
                    let candidate = [
                        face[edge_index] as usize,
                        face[(edge_index + 1) % 3] as usize,
                    ];
                    ordered_edge(candidate) == edge && candidate == directed_edge
                })
                .then(|| cut_mesh.source_face_for_faces.get(face_index).copied())
                .flatten()
        })
        .collect::<Vec<_>>();
    source_faces.sort_unstable();
    source_faces.dedup();
    source_faces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_cut_path_fill_plans_use_one_removed_face_source_per_plan() {
        let cut_mesh = ExactCutMeshResult {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            faces: Vec::new(),
            cut_edges: Vec::new(),
            cut_edge_paths: vec![vec![[0, 1], [1, 2], [2, 3], [3, 0]]],
            cut_edge_path_closed: vec![true],
            cut_edge_path_source_faces: vec![vec![Some(10), Some(20), Some(30), Some(40)]],
            collapsed_cut_segment_paths: Vec::new(),
            collapsed_cut_segment_path_source_faces: Vec::new(),
            source_face_for_faces: Vec::new(),
            cut_face_source_events: Vec::new(),
            skipped_source_faces: Vec::new(),
        };

        let plans = closed_cut_path_fill_plans(&cut_mesh, 1e-9, &BTreeSet::new());

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].source_face, 10);
        assert_eq!(plans[0].fill_plan.triangles.len(), 2);
        assert_eq!(plans[0].source_face_for_faces, vec![10, 10]);
    }

    #[test]
    fn closed_cut_path_fill_plans_prefer_missing_side_source_owner() {
        let cut_mesh = ExactCutMeshResult {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.5, 0.5, 1.0],
            ],
            faces: vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
            cut_edges: Vec::new(),
            cut_edge_paths: vec![vec![[0, 1], [1, 2], [2, 3], [3, 0]]],
            cut_edge_path_closed: vec![true],
            cut_edge_path_source_faces: vec![vec![Some(10), Some(20), Some(30), Some(40)]],
            collapsed_cut_segment_paths: Vec::new(),
            collapsed_cut_segment_path_source_faces: Vec::new(),
            source_face_for_faces: vec![90, 91, 92, 93],
            cut_face_source_events: Vec::new(),
            skipped_source_faces: Vec::new(),
        };

        let plans = closed_cut_path_fill_plans(&cut_mesh, 1e-9, &BTreeSet::new());

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].source_face, 90);
        assert_eq!(plans[0].fill_plan.triangles.len(), 2);
        assert_eq!(plans[0].source_face_for_faces, vec![90, 90]);
    }
}
