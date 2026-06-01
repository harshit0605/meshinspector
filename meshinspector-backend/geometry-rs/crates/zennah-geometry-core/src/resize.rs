use crate::distance::nearest_distances_to_validated_indices;
use crate::math::{add, dot, scale, sub};
use crate::GeometryError;
use nalgebra::{Matrix3, SymmetricEigen};
use std::collections::BTreeSet;

pub fn radial_scale_vertices(
    vertices: &[[f64; 3]],
    scale_factor: f64,
    ring_axis: Option<[f64; 3]>,
    preserve_indices: &[i64],
) -> Result<Vec<[f64; 3]>, GeometryError> {
    if vertices.is_empty() {
        return Ok(Vec::new());
    }

    let center = centroid(vertices);
    let axis = match ring_axis {
        Some(axis) => crate::math::normalize_vector(axis)?,
        None => detected_ring_axis(vertices, center)?,
    };
    let preserve = validate_preserve_indices(preserve_indices, vertices.len())?;
    let local_scale = radial_local_scales(vertices, scale_factor, &preserve);

    let scaled = vertices
        .iter()
        .zip(local_scale.iter())
        .map(|(vertex, vertex_scale)| {
            let relative = sub(*vertex, center);
            let axial_distance = dot(relative, axis);
            let axial_component = scale(axis, axial_distance);
            let radial_component = sub(relative, axial_component);
            add(
                center,
                add(axial_component, scale(radial_component, *vertex_scale)),
            )
        })
        .collect();
    Ok(scaled)
}

pub fn resize_ring_vertices(
    vertices: &[[f64; 3]],
    current_size: f64,
    target_size: f64,
    ring_axis: Option<[f64; 3]>,
    preserve_indices: &[i64],
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let scale_factor = crate::jewelry::ring_diameter_for_size(target_size)
        / crate::jewelry::ring_diameter_for_size(current_size);
    radial_scale_vertices(vertices, scale_factor, ring_axis, preserve_indices)
}

fn radial_local_scales(vertices: &[[f64; 3]], scale_factor: f64, preserve: &[usize]) -> Vec<f64> {
    let mut local_scale = vec![scale_factor; vertices.len()];
    if preserve.is_empty() {
        return local_scale;
    }

    let distances = nearest_distances_to_validated_indices(vertices, preserve);
    let mut preserve_distances: Vec<f64> = preserve.iter().map(|index| distances[*index]).collect();
    let falloff_mm = (percentile(&mut preserve_distances, 95.0).max(2.5) * 2.2).max(2.5);

    for (vertex_index, distance) in distances.iter().enumerate() {
        let mut protection = (-0.5 * (distance / falloff_mm.max(1e-3)).powi(2)).exp();
        if *distance > falloff_mm * 2.5 {
            protection = 0.0;
        }
        local_scale[vertex_index] = 1.0 + (scale_factor - 1.0) * (1.0 - 0.88 * protection);
    }
    for index in preserve {
        local_scale[*index] = 1.0 + (scale_factor - 1.0) * 0.08;
    }
    local_scale
}

fn centroid(vertices: &[[f64; 3]]) -> [f64; 3] {
    let mut total = [0.0; 3];
    for vertex in vertices {
        for axis in 0..3 {
            total[axis] += vertex[axis];
        }
    }
    scale(total, 1.0 / vertices.len() as f64)
}

fn detected_ring_axis(vertices: &[[f64; 3]], center: [f64; 3]) -> Result<[f64; 3], GeometryError> {
    let mut covariance = Matrix3::<f64>::zeros();
    if vertices.len() > 1 {
        for vertex in vertices {
            let centered = sub(*vertex, center);
            for row in 0..3 {
                for column in 0..3 {
                    covariance[(row, column)] += centered[row] * centered[column];
                }
            }
        }
        covariance /= vertices.len() as f64 - 1.0;
    }

    let eigen = SymmetricEigen::new(covariance);
    let axis_index = (0..3)
        .min_by(|left, right| {
            eigen.eigenvalues[*left]
                .partial_cmp(&eigen.eigenvalues[*right])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0);
    crate::math::normalize_vector([
        eigen.eigenvectors[(0, axis_index)],
        eigen.eigenvectors[(1, axis_index)],
        eigen.eigenvectors[(2, axis_index)],
    ])
}

fn validate_preserve_indices(
    preserve_indices: &[i64],
    vertex_count: usize,
) -> Result<Vec<usize>, GeometryError> {
    let mut values = BTreeSet::new();
    for index in preserve_indices {
        if *index < 0 || *index as usize >= vertex_count {
            return Err(GeometryError::PreserveIndexOutOfBounds {
                index: *index,
                vertex_count,
            });
        }
        values.insert(*index as usize);
    }
    Ok(values.into_iter().collect())
}

fn percentile(values: &mut [f64], percentile: f64) -> f64 {
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    if values.is_empty() {
        return f64::NAN;
    }
    if values.len() == 1 {
        return values[0];
    }

    let rank = percentile / 100.0 * (values.len() as f64 - 1.0);
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        return values[lower];
    }
    values[lower] + (values[upper] - values[lower]) * (rank - lower as f64)
}
