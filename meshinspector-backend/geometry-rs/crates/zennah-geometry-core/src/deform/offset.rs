use crate::math::{add, cross, dot, scale, sub};
use crate::mesh::{safe_normalize_vector, validate_faces};
use crate::GeometryError;
use rayon::prelude::*;

pub(super) fn local_offset_vertices_with_weights(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    weights: &[f32],
    amount_mm: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    if weights.len() != vertices.len() {
        return Err(GeometryError::WeightCountDoesNotMatchVertices {
            weights: weights.len(),
            vertices: vertices.len(),
        });
    }
    let active_vertices: Vec<usize> = weights
        .iter()
        .enumerate()
        .filter_map(|(vertex_index, weight)| {
            if *weight != 0.0 {
                Some(vertex_index)
            } else {
                None
            }
        })
        .collect();
    if active_vertices.is_empty() {
        return Ok(vertices.to_vec());
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut active_mask = vec![false; vertices.len()];
    for vertex_index in &active_vertices {
        active_mask[*vertex_index] = true;
    }

    let mut normals = vec![[0.0_f64; 3]; vertices.len()];
    for face in &faces {
        let [a, b, c] = *face;
        if !active_mask[a] && !active_mask[b] && !active_mask[c] {
            continue;
        }
        let normal = cross(sub(vertices[b], vertices[a]), sub(vertices[c], vertices[a]));
        if active_mask[a] {
            normals[a] = add(normals[a], normal);
        }
        if active_mask[b] {
            normals[b] = add(normals[b], normal);
        }
        if active_mask[c] {
            normals[c] = add(normals[c], normal);
        }
    }

    let center = scale(
        vertices.iter().copied().fold([0.0_f64; 3], add),
        1.0 / vertices.len() as f64,
    );
    let updates: Vec<(usize, [f64; 3])> = active_vertices
        .par_iter()
        .map(|vertex_index| {
            let normal = safe_normalize_vector(normals[*vertex_index]);
            let toward_center = safe_normalize_vector(sub(center, vertices[*vertex_index]));
            let direction = if dot(normal, toward_center) >= 0.0 {
                scale(normal, -1.0)
            } else {
                normal
            };
            (
                *vertex_index,
                add(
                    vertices[*vertex_index],
                    scale(direction, amount_mm * weights[*vertex_index] as f64),
                ),
            )
        })
        .collect();
    let mut displaced = vertices.to_vec();
    for (vertex_index, vertex) in updates {
        displaced[vertex_index] = vertex;
    }
    Ok(displaced)
}
