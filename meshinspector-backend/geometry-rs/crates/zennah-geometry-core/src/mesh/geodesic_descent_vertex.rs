use crate::math::{add, distance_sq, dot, scale, sub};
use crate::types::GeometryError;

use super::base::validate_faces;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSteepestDescentVertexStep {
    pub start_vertex: usize,
    pub start_point: [f64; 3],
    pub start_value: f64,
    pub crossed_edge: Option<[usize; 2]>,
    pub edge_position: Option<f64>,
    pub crossing_point: Option<[f64; 3]>,
    pub gradient_norm: Option<f64>,
    pub kind: &'static str,
    pub source: &'static str,
    pub meshlib_reference: &'static str,
}

pub fn mesh_steepest_descent_vertex_step(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    vertex_scalars: &[f64],
    vertex_index: usize,
) -> Result<MeshSteepestDescentVertexStep, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if vertex_index >= vertices.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_index",
            value: format!("{vertex_index} for {} vertices", vertices.len()),
        });
    }
    if vertex_scalars.len() != vertices.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_scalars",
            value: format!(
                "scalar count {} does not match vertex count {}",
                vertex_scalars.len(),
                vertices.len()
            ),
        });
    }
    if vertex_scalars.iter().any(|value| !value.is_finite()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_scalars",
            value: "scalars_must_be_finite".to_string(),
        });
    }

    let start_point = vertices[vertex_index];
    let start_value = vertex_scalars[vertex_index];
    let mut best: Option<VertexCandidate> = None;
    let mut max_grad_sq = 0.0;

    for neighbor in vertex_neighbors_from_faces(&faces, vertex_index) {
        let delta = sub(vertices[neighbor], start_point);
        let dist_sq = distance_sq(delta, [0.0, 0.0, 0.0]);
        let value_delta = vertex_scalars[neighbor] - start_value;
        if value_delta < 0.0 {
            if dist_sq == 0.0 && max_grad_sq == 0.0 && best.is_none() {
                best = Some(VertexCandidate::direct_edge([neighbor, vertex_index], 0.0));
            } else if dist_sq > 0.0 {
                let edge_grad_sq = value_delta.powi(2) / dist_sq;
                if edge_grad_sq > max_grad_sq {
                    max_grad_sq = edge_grad_sq;
                    best = Some(VertexCandidate::direct_edge([neighbor, vertex_index], 0.0));
                }
            }
        }
    }

    for face in &faces {
        for i in 0..3 {
            if face[i] != vertex_index {
                continue;
            }
            let d = face[(i + 1) % 3];
            let x = face[(i + 2) % 3];
            let pd = sub(vertices[d], start_point);
            let px = sub(vertices[x], start_point);
            let vd = vertex_scalars[d] - start_value;
            let vx = vertex_scalars[x] - start_value;
            let gradient = triangle_gradient(start_point, vertices[d], vertices[x], vd, vx);
            let grad_sq = distance_sq(gradient, [0.0, 0.0, 0.0]);
            if grad_sq > max_grad_sq {
                if let Some(position) = find_tri_exit_pos(pd, px, gradient) {
                    max_grad_sq = grad_sq;
                    best = Some(VertexCandidate::face_exit([d, x], position, grad_sq.sqrt()));
                }
            }
        }
    }

    let Some(candidate) = best else {
        return Ok(MeshSteepestDescentVertexStep {
            start_vertex: vertex_index,
            start_point,
            start_value,
            crossed_edge: None,
            edge_position: None,
            crossing_point: None,
            gradient_norm: None,
            kind: "flat",
            source: "none",
            meshlib_reference: "MR::findSteepestDescentPoint(VertId)",
        });
    };
    let crossing_point = edge_point(
        vertices[candidate.edge[0]],
        vertices[candidate.edge[1]],
        candidate.position,
    );
    Ok(MeshSteepestDescentVertexStep {
        start_vertex: vertex_index,
        start_point,
        start_value,
        crossed_edge: Some(candidate.edge),
        edge_position: Some(candidate.position),
        crossing_point: Some(crossing_point),
        gradient_norm: candidate.gradient_norm,
        kind: candidate.kind,
        source: candidate.source,
        meshlib_reference: "MR::findSteepestDescentPoint(VertId)",
    })
}

#[derive(Debug, Clone, Copy)]
struct VertexCandidate {
    edge: [usize; 2],
    position: f64,
    gradient_norm: Option<f64>,
    kind: &'static str,
    source: &'static str,
}

impl VertexCandidate {
    fn direct_edge(edge: [usize; 2], position: f64) -> Self {
        Self {
            edge,
            position,
            gradient_norm: None,
            kind: "vertex",
            source: "edge",
        }
    }

    fn face_exit(edge: [usize; 2], position: f64, gradient_norm: f64) -> Self {
        let kind = if position <= 1e-12 || position >= 1.0 - 1e-12 {
            "vertex"
        } else {
            "edge"
        };
        Self {
            edge,
            position,
            gradient_norm: Some(gradient_norm),
            kind,
            source: "face",
        }
    }
}

fn find_tri_exit_pos(b: [f64; 3], c: [f64; 3], gradient: [f64; 3]) -> Option<f64> {
    let grad_sq = distance_sq(gradient, [0.0, 0.0, 0.0]);
    if grad_sq <= 0.0 {
        return None;
    }
    let d = sub(c, b);
    let gort = sub(d, scale(gradient, dot(d, gradient) / grad_sq));
    let god = dot(gort, d);
    if god <= 0.0 {
        return None;
    }
    let gob = -dot(gort, b);
    if gob <= 0.0 || gob >= god {
        return None;
    }
    let position = gob / god;
    let intersection = add(scale(c, position), scale(b, 1.0 - position));
    if dot(gradient, intersection) >= 0.0 {
        return None;
    }
    Some(position)
}

fn vertex_neighbors_from_faces(faces: &[[usize; 3]], vertex: usize) -> Vec<usize> {
    let mut neighbors = Vec::new();
    for face in faces {
        for i in 0..3 {
            if face[i] == vertex {
                let prev = face[(i + 2) % 3];
                let next = face[(i + 1) % 3];
                if !neighbors.contains(&prev) {
                    neighbors.push(prev);
                }
                if !neighbors.contains(&next) {
                    neighbors.push(next);
                }
            }
        }
    }
    neighbors
}

fn triangle_gradient(a: [f64; 3], b: [f64; 3], c: [f64; 3], vb: f64, vc: f64) -> [f64; 3] {
    let e0 = sub(b, a);
    let e1 = sub(c, a);
    let d00 = dot(e0, e0);
    let d01 = dot(e0, e1);
    let d11 = dot(e1, e1);
    let denominator = d00 * d11 - d01 * d01;
    if denominator.abs() <= 1e-18 {
        return [0.0, 0.0, 0.0];
    }
    let alpha = (vb * d11 - vc * d01) / denominator;
    let beta = (vc * d00 - vb * d01) / denominator;
    add(scale(e0, alpha), scale(e1, beta))
}

fn edge_point(a: [f64; 3], b: [f64; 3], position: f64) -> [f64; 3] {
    add(scale(a, 1.0 - position), scale(b, position))
}
