use crate::math::distance_sq;
use crate::types::GeometryError;

use super::geodesic_descent::{
    mesh_steepest_descent_edge_step, mesh_steepest_descent_triangle_step,
};
use super::geodesic_descent_vertex::mesh_steepest_descent_vertex_step;

const VERTEX_EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSteepestDescentPath {
    pub start_face_index: usize,
    pub start_barycentric: [f64; 3],
    pub start_point: [f64; 3],
    pub start_value: f64,
    pub edges: Vec<[usize; 2]>,
    pub positions: Vec<f64>,
    pub points: Vec<[f64; 3]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub reached_vertex: Option<usize>,
    pub stopped_reason: &'static str,
    pub steps: usize,
    pub meshlib_reference: &'static str,
}

pub fn mesh_steepest_descent_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    vertex_scalars: &[f64],
    face_index: usize,
    start_barycentric: [f64; 3],
    max_steps: usize,
) -> Result<MeshSteepestDescentPath, GeometryError> {
    if max_steps == 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_steps",
            value: "must_be_positive".to_string(),
        });
    }

    let first = mesh_steepest_descent_triangle_step(
        vertices,
        faces_i64,
        vertex_scalars,
        face_index,
        start_barycentric,
    )?;
    let start_point = first.start_point;
    let start_value = first.start_value;
    let mut edges = Vec::new();
    let mut positions = Vec::new();
    let mut points = Vec::new();
    let mut stopped_reason = "local_minimum";
    let mut reached_vertex = None;

    if let (Some(edge), Some(position), Some(point)) = (
        first.crossed_edge,
        first.edge_position,
        first.crossing_point,
    ) {
        edges.push(edge);
        positions.push(position);
        points.push(point);
    }

    while !edges.is_empty() && edges.len() < max_steps {
        let edge = *edges.last().expect("edges is non-empty");
        let position = *positions.last().expect("positions tracks edges");
        let next = if let Some(vertex) = edge_position_vertex(edge, position) {
            let step =
                mesh_steepest_descent_vertex_step(vertices, faces_i64, vertex_scalars, vertex)?;
            if step.crossed_edge.is_none() {
                reached_vertex = Some(vertex);
            }
            (step.crossed_edge, step.edge_position, step.crossing_point)
        } else {
            let step = mesh_steepest_descent_edge_step(
                vertices,
                faces_i64,
                vertex_scalars,
                [edge[0] as i64, edge[1] as i64],
                position,
            )?;
            (
                step.crossed_edge,
                step.crossing_edge_position,
                step.crossing_point,
            )
        };

        let (Some(next_edge), Some(next_position), Some(next_point)) = next else {
            break;
        };
        if edge == next_edge && (position - next_position).abs() <= VERTEX_EPSILON {
            stopped_reason = "cycle_guard";
            break;
        }
        edges.push(next_edge);
        positions.push(next_position);
        points.push(next_point);
    }

    if !edges.is_empty() && edges.len() == max_steps && reached_vertex.is_none() {
        stopped_reason = "max_steps";
    }

    let segment_lengths = descent_segment_lengths(start_point, &points);
    let steps = segment_lengths.len();
    let length_mm = segment_lengths.iter().sum();
    Ok(MeshSteepestDescentPath {
        start_face_index: face_index,
        start_barycentric,
        start_point,
        start_value,
        edges,
        positions,
        points,
        segment_lengths,
        length_mm,
        reached_vertex,
        stopped_reason,
        steps,
        meshlib_reference: "MR::computeSteepestDescentPath",
    })
}

fn edge_position_vertex(edge: [usize; 2], position: f64) -> Option<usize> {
    if position <= VERTEX_EPSILON {
        Some(edge[0])
    } else if position >= 1.0 - VERTEX_EPSILON {
        Some(edge[1])
    } else {
        None
    }
}

fn descent_segment_lengths(start_point: [f64; 3], points: &[[f64; 3]]) -> Vec<f64> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut previous = start_point;
    let mut lengths = Vec::with_capacity(points.len());
    for point in points {
        lengths.push(distance_sq(previous, *point).sqrt());
        previous = *point;
    }
    lengths
}
