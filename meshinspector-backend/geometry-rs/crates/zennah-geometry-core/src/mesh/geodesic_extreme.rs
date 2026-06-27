use std::collections::BTreeMap;

use crate::types::GeometryError;

use super::base::validate_faces;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshExtremeEdgeType {
    Ridge,
    Gorge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeodesicExtremeEdges {
    pub extreme_type: MeshExtremeEdgeType,
    pub edge_indices: Vec<[usize; 2]>,
    pub meshlib_reference: &'static str,
}

pub fn mesh_geodesic_extreme_edges(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    scalars: &[f64],
    extreme_type: MeshExtremeEdgeType,
) -> Result<MeshGeodesicExtremeEdges, GeometryError> {
    if scalars.len() != vertices.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "surface_field",
            value: format!("expected_{}_got_{}", vertices.len(), scalars.len()),
        });
    }
    if scalars.iter().any(|value| value.is_nan()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "surface_field",
            value: "nan".to_string(),
        });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut edge_faces = BTreeMap::<[usize; 2], Vec<usize>>::new();
    for (face_index, face) in faces.iter().enumerate() {
        for edge in [
            sorted_edge(face[0], face[1]),
            sorted_edge(face[1], face[2]),
            sorted_edge(face[2], face[0]),
        ] {
            edge_faces.entry(edge).or_default().push(face_index);
        }
    }

    let mut edge_indices = Vec::new();
    for (edge, incident_faces) in edge_faces {
        if incident_faces.len() != 2 {
            continue;
        }
        let mut gradient_enters_any_side = false;
        let mut edge_has_unreachable_field = false;
        for face_index in incident_faces {
            let face = faces[face_index];
            let Some(third) = third_vertex(face, edge) else {
                continue;
            };
            if !scalars[edge[0]].is_finite()
                || !scalars[edge[1]].is_finite()
                || !scalars[third].is_finite()
            {
                edge_has_unreachable_field = true;
                break;
            }
            let mut gradient = triangle_scalar_gradient(
                vertices[edge[0]],
                vertices[edge[1]],
                vertices[third],
                scalars[edge[0]],
                scalars[edge[1]],
                scalars[third],
            );
            if extreme_type == MeshExtremeEdgeType::Gorge {
                gradient = scale(gradient, -1.0);
            }
            if direction_enters_edge_triangle(
                vertices[edge[0]],
                vertices[edge[1]],
                vertices[third],
                gradient,
            ) {
                gradient_enters_any_side = true;
                break;
            }
        }
        if !edge_has_unreachable_field && !gradient_enters_any_side {
            edge_indices.push(edge);
        }
    }

    Ok(MeshGeodesicExtremeEdges {
        extreme_type,
        edge_indices,
        meshlib_reference: "MR::findExtremeEdges",
    })
}

fn third_vertex(face: [usize; 3], edge: [usize; 2]) -> Option<usize> {
    face.into_iter()
        .find(|vertex| *vertex != edge[0] && *vertex != edge[1])
}

fn sorted_edge(a: usize, b: usize) -> [usize; 2] {
    if a <= b {
        [a, b]
    } else {
        [b, a]
    }
}

fn triangle_scalar_gradient(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    fa: f64,
    fb: f64,
    fc: f64,
) -> [f64; 3] {
    let u = sub(b, a);
    let v = sub(c, a);
    let uu = dot(u, u);
    let uv = dot(u, v);
    let vv = dot(v, v);
    let det = uu * vv - uv * uv;
    if det.abs() <= f64::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    let du = fb - fa;
    let dv = fc - fa;
    let alpha = (du * vv - dv * uv) / det;
    let beta = (dv * uu - du * uv) / det;
    add(scale(u, alpha), scale(v, beta))
}

fn direction_enters_edge_triangle(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    direction: [f64; 3],
) -> bool {
    let edge = sub(b, a);
    let edge_len_sq = dot(edge, edge);
    if edge_len_sq <= f64::EPSILON {
        return false;
    }
    let projection = scale(edge, dot(direction, edge) / edge_len_sq);
    let orthogonal = sub(direction, projection);
    dot(orthogonal, sub(c, a)) > 1e-12
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f64; 3], scalar: f64) -> [f64; 3] {
    [a[0] * scalar, a[1] * scalar, a[2] * scalar]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
