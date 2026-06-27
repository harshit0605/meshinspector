use super::{ordered_edge, ExactCutHoleFillPlan};
use crate::math::{cross, dot, norm, sub};
use crate::spatial::exact_cut_apply::ExactCutMeshResult;
use crate::spatial::exact_fill_plan::exact_planar_hole_fill_plan;

pub(super) fn open_cut_path_restoration_fill_plans(
    cut_mesh: &ExactCutMeshResult,
    epsilon: f64,
) -> Vec<ExactCutHoleFillPlan> {
    cut_mesh
        .cut_edge_paths
        .iter()
        .enumerate()
        .filter_map(|(path_index, path)| {
            let source_face = open_path_source_face(cut_mesh, path_index, path)?;
            let edge = *path.first()?;
            let boundary_loop =
                restoration_boundary_loop(cut_mesh, edge, source_face).or_else(|| {
                    orphan_restoration_boundary_loop(cut_mesh, edge, source_face, epsilon)
                })?;
            let fill_plan =
                exact_planar_hole_fill_plan(&cut_mesh.vertices, &boundary_loop, epsilon)?;
            Some(ExactCutHoleFillPlan {
                representative_edge: edge,
                boundary_loop,
                boundary_edges: path.to_vec(),
                source_face,
                source_face_for_faces: vec![source_face; fill_plan.num_tris],
                fill_plan,
            })
        })
        .collect()
}

fn open_path_source_face(
    cut_mesh: &ExactCutMeshResult,
    path_index: usize,
    path: &[[usize; 2]],
) -> Option<usize> {
    if path.len() != 1 || cut_mesh.cut_edge_path_closed.get(path_index).copied()? {
        return None;
    }
    let source_faces = cut_mesh.cut_edge_path_source_faces.get(path_index)?;
    if source_faces.len() != 1 {
        return None;
    }
    source_faces[0]
}

fn restoration_boundary_loop(
    cut_mesh: &ExactCutMeshResult,
    edge: [usize; 2],
    source_face: usize,
) -> Option<Vec<usize>> {
    let face = cut_mesh
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            (cut_mesh.source_face_for_faces.get(face_index).copied()? == source_face)
                .then_some(*face)
        })
        .find(|face| face_contains_ordered_edge(*face, edge))?;
    let face = [face[0] as usize, face[1] as usize, face[2] as usize];
    let third = face
        .iter()
        .copied()
        .find(|vertex| *vertex != edge[0] && *vertex != edge[1])?;
    let has_forward_edge = face_has_directed_edge(face, edge);
    if has_forward_edge {
        Some(vec![edge[1], edge[0], third])
    } else {
        Some(vec![edge[0], edge[1], third])
    }
}

fn orphan_restoration_boundary_loop(
    cut_mesh: &ExactCutMeshResult,
    edge: [usize; 2],
    source_face: usize,
    epsilon: f64,
) -> Option<Vec<usize>> {
    let mut best = None::<([usize; 3], f64)>;
    for face in
        cut_mesh
            .faces
            .iter()
            .enumerate()
            .filter_map(|(face_index, face)| {
                (cut_mesh.source_face_for_faces.get(face_index).copied()? == source_face)
                    .then_some([face[0] as usize, face[1] as usize, face[2] as usize])
            })
    {
        if face_contains_ordered_edge([face[0] as i64, face[1] as i64, face[2] as i64], edge) {
            continue;
        }
        if !face.contains(&edge[0]) && !face.contains(&edge[1]) {
            continue;
        }
        for third in face {
            if third == edge[0] || third == edge[1] {
                continue;
            }
            let Some(mut candidate) = oriented_restoration_triangle(cut_mesh, edge, third, face)
            else {
                continue;
            };
            let area = triangle_area(cut_mesh, candidate);
            if area <= epsilon * epsilon {
                continue;
            }
            if best.is_none_or(|(_, best_area)| area > best_area) {
                if !candidate.contains(&third) {
                    candidate[2] = third;
                }
                best = Some((candidate, area));
            }
        }
    }
    best.map(|(face, _)| face.to_vec())
}

fn oriented_restoration_triangle(
    cut_mesh: &ExactCutMeshResult,
    edge: [usize; 2],
    third: usize,
    source_face: [usize; 3],
) -> Option<[usize; 3]> {
    let source_normal = triangle_normal(cut_mesh, source_face)?;
    let mut candidate = [edge[0], edge[1], third];
    let candidate_normal = triangle_normal(cut_mesh, candidate)?;
    if dot(candidate_normal, source_normal) < 0.0 {
        candidate.swap(0, 1);
    }
    Some(candidate)
}

fn triangle_normal(cut_mesh: &ExactCutMeshResult, face: [usize; 3]) -> Option<[f64; 3]> {
    let a = *cut_mesh.vertices.get(face[0])?;
    let b = *cut_mesh.vertices.get(face[1])?;
    let c = *cut_mesh.vertices.get(face[2])?;
    let normal = cross(sub(b, a), sub(c, a));
    (norm(normal) > f64::EPSILON).then_some(normal)
}

fn triangle_area(cut_mesh: &ExactCutMeshResult, face: [usize; 3]) -> f64 {
    let Some(normal) = triangle_normal(cut_mesh, face) else {
        return 0.0;
    };
    0.5 * norm(normal)
}

fn face_contains_ordered_edge(face: [i64; 3], edge: [usize; 2]) -> bool {
    let face = [face[0] as usize, face[1] as usize, face[2] as usize];
    (0..3).any(|index| ordered_edge([face[index], face[(index + 1) % 3]]) == ordered_edge(edge))
}

fn face_has_directed_edge(face: [usize; 3], edge: [usize; 2]) -> bool {
    (0..3).any(|index| [face[index], face[(index + 1) % 3]] == edge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_cut_path_restoration_adds_source_owned_opposite_side_face() {
        let cut_mesh = ExactCutMeshResult {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            cut_edges: vec![[0, 1]],
            cut_edge_paths: vec![vec![[0, 1]]],
            cut_edge_path_closed: vec![false],
            cut_edge_path_source_faces: vec![vec![Some(7)]],
            collapsed_cut_segment_paths: Vec::new(),
            collapsed_cut_segment_path_source_faces: Vec::new(),
            source_face_for_faces: vec![7],
            cut_face_source_events: Vec::new(),
            skipped_source_faces: Vec::new(),
        };

        let plans = open_cut_path_restoration_fill_plans(&cut_mesh, 1e-9);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].representative_edge, [0, 1]);
        assert_eq!(plans[0].boundary_loop, vec![1, 0, 2]);
        assert_eq!(plans[0].source_face, 7);
        assert_eq!(plans[0].source_face_for_faces, vec![7]);
        assert_eq!(plans[0].fill_plan.triangles, vec![[1, 0, 2]]);
    }

    #[test]
    fn open_cut_path_restoration_materializes_orphan_source_edge() {
        let cut_mesh = ExactCutMeshResult {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.5, 0.0, 0.0],
            ],
            faces: vec![[0, 1, 2]],
            cut_edges: vec![[0, 3]],
            cut_edge_paths: vec![vec![[3, 0]]],
            cut_edge_path_closed: vec![false],
            cut_edge_path_source_faces: vec![vec![Some(7)]],
            collapsed_cut_segment_paths: Vec::new(),
            collapsed_cut_segment_path_source_faces: Vec::new(),
            source_face_for_faces: vec![7],
            cut_face_source_events: Vec::new(),
            skipped_source_faces: Vec::new(),
        };

        let plans = open_cut_path_restoration_fill_plans(&cut_mesh, 1e-9);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].representative_edge, [3, 0]);
        assert_eq!(plans[0].source_face, 7);
        assert_eq!(plans[0].source_face_for_faces, vec![7]);
        assert_eq!(plans[0].fill_plan.num_tris, 1);
        assert!(plans[0].boundary_loop.contains(&3));
        assert!(plans[0].boundary_loop.contains(&0));
        assert!(plans[0].boundary_loop.contains(&2));
    }
}
