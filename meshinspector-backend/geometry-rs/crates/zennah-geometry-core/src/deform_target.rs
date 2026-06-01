use crate::deform::{falloff_weights, outward_directions};
use crate::math::{add, scale};
use crate::GeometryError;
use rayon::prelude::*;

pub fn local_thicken_to_minimum_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    seed_indices: &[i64],
    thickness_values: &[f32],
    min_target_thickness_mm: f64,
    falloff_mm: f64,
    deficit_scale: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    if thickness_values.len() != vertices.len() {
        return Err(GeometryError::ThicknessCountDoesNotMatchVertices {
            thickness: thickness_values.len(),
            vertices: vertices.len(),
        });
    }
    if !min_target_thickness_mm.is_finite() || min_target_thickness_mm <= 0.0 {
        return Err(GeometryError::InvalidAdaptiveHollowInput {
            field: "min_target_thickness_mm",
            value: min_target_thickness_mm,
        });
    }
    if !deficit_scale.is_finite() || deficit_scale < 0.0 {
        return Err(GeometryError::InvalidAdaptiveHollowInput {
            field: "deficit_scale",
            value: deficit_scale,
        });
    }

    let directions = outward_directions(vertices, faces_i64)?;
    let weights = falloff_weights(vertices, seed_indices, falloff_mm, 3.0)?;
    let displaced = vertices
        .par_iter()
        .enumerate()
        .map(|(vertex_index, vertex)| {
            let thickness = thickness_values[vertex_index];
            let safe_thickness = if thickness.is_finite() {
                thickness.max(0.0) as f64
            } else {
                0.0
            };
            let deficit =
                (min_target_thickness_mm - safe_thickness).clamp(0.0, min_target_thickness_mm);
            add(
                *vertex,
                scale(
                    directions[vertex_index],
                    deficit * weights[vertex_index] as f64 * deficit_scale,
                ),
            )
        })
        .collect();
    Ok(displaced)
}
