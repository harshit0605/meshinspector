use super::super::exact_cut::ExactCutPrimitive;
use crate::math::{dot, sub};

pub(super) fn set_shared_interior(interior_vertex: &mut Option<usize>, candidate: usize) -> bool {
    match interior_vertex {
        Some(existing) => *existing == candidate,
        None => {
            *interior_vertex = Some(candidate);
            true
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FacePointPosition {
    Boundary(f64),
    Interior,
}

pub(super) fn face_point_position(
    face: [usize; 3],
    face_index: usize,
    primitive: ExactCutPrimitive,
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
) -> Option<FacePointPosition> {
    if let Some(position) = boundary_position(face, primitive, coordinate, vertices) {
        return Some(FacePointPosition::Boundary(position));
    }
    match primitive {
        ExactCutPrimitive::Face(source_face) if source_face == face_index => {
            Some(FacePointPosition::Interior)
        }
        _ => None,
    }
}

fn boundary_position(
    face: [usize; 3],
    primitive: ExactCutPrimitive,
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
) -> Option<f64> {
    match primitive {
        ExactCutPrimitive::Vertex(vertex) => face
            .iter()
            .position(|candidate| *candidate == vertex)
            .map(|index| index as f64),
        ExactCutPrimitive::Edge(edge) => {
            for edge_index in 0..3 {
                let start = face[edge_index];
                let end = face[(edge_index + 1) % 3];
                if ordered_edge(edge) != ordered_edge([start, end]) {
                    continue;
                }
                let parameter = edge_parameter(vertices[start], vertices[end], coordinate);
                return Some(edge_index as f64 + parameter);
            }
            None
        }
        ExactCutPrimitive::Face(_) => None,
    }
}

fn edge_parameter(start: [f64; 3], end: [f64; 3], point: [f64; 3]) -> f64 {
    let edge = sub(end, start);
    let length_sq = dot(edge, edge);
    if length_sq <= f64::EPSILON {
        return 0.0;
    }
    (dot(sub(point, start), edge) / length_sq).clamp(0.0, 1.0)
}

pub(super) fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

pub(super) fn nearly_equal(left: f64, right: f64, epsilon: f64) -> bool {
    (left - right).abs() <= epsilon
}

pub(super) fn effective_epsilon(epsilon: f64) -> f64 {
    if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    }
}
