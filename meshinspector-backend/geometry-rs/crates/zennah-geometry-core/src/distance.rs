use crate::math::distance_sq;
use crate::GeometryError;
use rayon::prelude::*;
use std::collections::BTreeSet;

pub fn nearest_distances_to_indices(
    vertices: &[[f64; 3]],
    target_indices: &[i64],
) -> Result<Vec<f64>, GeometryError> {
    let targets = validate_target_indices(target_indices, vertices.len())?;
    Ok(nearest_distances_to_validated_indices(vertices, &targets))
}

pub(crate) fn nearest_distances_to_validated_indices(
    vertices: &[[f64; 3]],
    targets: &[usize],
) -> Vec<f64> {
    let distances = vertices
        .par_iter()
        .map(|vertex| {
            targets
                .iter()
                .map(|target| distance_sq(*vertex, vertices[*target]))
                .fold(f64::INFINITY, f64::min)
                .sqrt()
        })
        .collect();
    distances
}

fn validate_target_indices(
    target_indices: &[i64],
    vertex_count: usize,
) -> Result<Vec<usize>, GeometryError> {
    if target_indices.is_empty() {
        return Err(GeometryError::EmptySeedIndices);
    }
    let mut unique = BTreeSet::new();
    for target in target_indices {
        if *target < 0 {
            return Err(GeometryError::NegativeSeedIndex { seed: *target });
        }
        let index = *target as usize;
        if index >= vertex_count {
            return Err(GeometryError::SeedIndexOutOfBounds {
                seed: *target,
                vertex_count,
            });
        }
        unique.insert(index);
    }
    Ok(unique.into_iter().collect())
}
