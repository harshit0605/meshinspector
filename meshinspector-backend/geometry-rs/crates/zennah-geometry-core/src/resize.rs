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

/// Outcome of [`fit_ring_to_diameter_vertices`].
#[derive(Debug, Clone, PartialEq)]
pub struct RingFitResult {
    pub vertices: Vec<[f64; 3]>,
    /// True when the protected (head) region was dropped because the requested
    /// scale was too extreme to preserve it without folding the surface.
    pub applied_uniform_fallback: bool,
    pub scale_factor: f64,
}

/// Fit a ring to an absolute target inner diameter, scaling the measured bore to
/// the target.
///
/// Within the safe band `[1/max_preserve_scale_ratio, max_preserve_scale_ratio]`
/// the bore is grown by scaling only the radial component about the ring axis,
/// which keeps the band width constant and (optionally) anchors a head/ornament
/// region — the correct model for small finger-size adjustments to a real ring.
///
/// Outside that band the radial-only model breaks down: it stretches any
/// protruding ornament *anisotropically* (≈`scale_factor` across the ring plane
/// but ≈1× along the axis), visibly deforming a snake head / gem setting, and
/// anchoring the head rigidly while the band grows that far would fold the
/// surface. So an extreme ratio falls back to a true **uniform** (isotropic)
/// similarity scale about the centroid: every local shape — head included — is
/// preserved while the bore still lands exactly on the target. This is the
/// correct behaviour for fitting an unscaled miniature up to a real ring size,
/// where holding a 5 mm head fixed against a 20 mm band would be nonsensical.
pub fn fit_ring_to_diameter_vertices(
    vertices: &[[f64; 3]],
    measured_diameter_mm: f64,
    target_diameter_mm: f64,
    ring_axis: Option<[f64; 3]>,
    preserve_indices: &[i64],
    max_preserve_scale_ratio: f64,
) -> Result<RingFitResult, GeometryError> {
    if !(measured_diameter_mm > 0.0) || !(target_diameter_mm > 0.0) {
        return Err(GeometryError::InvalidRingFitDiameter {
            measured_diameter_mm,
            target_diameter_mm,
        });
    }
    let scale_factor = target_diameter_mm / measured_diameter_mm;
    let ratio = max_preserve_scale_ratio.max(1.0);
    let within_safe_band = scale_factor >= 1.0 / ratio && scale_factor <= ratio;
    // A ratio outside the safe band always uses the uniform similarity scale (it
    // never distorts the ornament). The reported fallback flag stays scoped to
    // the case where a preserve set was actually requested and had to be dropped,
    // since that is what the service surfaces to the user.
    let use_uniform_scale = !within_safe_band;
    let applied_uniform_fallback = use_uniform_scale && !preserve_indices.is_empty();
    let scaled = if use_uniform_scale {
        uniform_scale_vertices(vertices, scale_factor)
    } else {
        radial_scale_vertices(vertices, scale_factor, ring_axis, preserve_indices)?
    };
    Ok(RingFitResult {
        vertices: scaled,
        applied_uniform_fallback,
        scale_factor,
    })
}

/// Isotropic similarity scale about the centroid: scales all three components by
/// `scale_factor`, so every local shape is preserved and all pairwise distances
/// (including the bore) scale by exactly `scale_factor`.
fn uniform_scale_vertices(vertices: &[[f64; 3]], scale_factor: f64) -> Vec<[f64; 3]> {
    if vertices.is_empty() {
        return Vec::new();
    }
    let center = centroid(vertices);
    vertices
        .iter()
        .map(|vertex| add(center, scale(sub(*vertex, center), scale_factor)))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A ring of revolution in the z-plane plus one ornament vertex protruding
    /// along the ring axis (+z) — like a snake head sitting proud of the band.
    fn ring_with_ornament() -> Vec<[f64; 3]> {
        vec![
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.5, 0.5, 2.0], // ornament protruding along the ring axis
        ]
    }

    /// Regression for the snake-head deformation: an extreme fit ratio must scale
    /// the whole piece *isotropically* so the ornament keeps its shape, rather
    /// than the old radial-only scale that stretched it across the ring plane
    /// while leaving its axial extent unchanged.
    #[test]
    fn extreme_ratio_fit_is_isotropic_similarity_scale() {
        let vertices = ring_with_ornament();
        let preserve = vec![4i64];
        let result = fit_ring_to_diameter_vertices(
            &vertices,
            2.0, // measured bore (ring plane)
            8.0, // target bore -> 4x, well outside the [1/1.5, 1.5] safe band
            Some([0.0, 0.0, 1.0]),
            &preserve,
            1.5,
        )
        .unwrap();

        assert!(result.applied_uniform_fallback);
        let s = result.scale_factor;
        assert!((s - 4.0).abs() < 1e-9);

        let center = centroid(&vertices);
        for (orig, got) in vertices.iter().zip(result.vertices.iter()) {
            let expected = add(center, scale(sub(*orig, center), s));
            for k in 0..3 {
                assert!(
                    (got[k] - expected[k]).abs() < 1e-9,
                    "axis {k}: {got:?} is not the isotropic similarity image {expected:?}",
                );
            }
        }

        // The protruding ornament's axial offset must grow by ~s (the bug left it
        // ~unchanged, which is exactly the visible head distortion).
        let axial_before = (vertices[4][2] - center[2]).abs();
        let axial_after = (result.vertices[4][2] - center[2]).abs();
        assert!(
            (axial_after / axial_before - s).abs() < 1e-6,
            "ornament axial extent grew {}x, expected {}x",
            axial_after / axial_before,
            s,
        );
    }

    /// A small (in-band) resize keeps the radial-only model: the band grows in
    /// the ring plane while a protruding ornament's axial extent is unchanged.
    #[test]
    fn in_band_ratio_keeps_radial_only_behaviour() {
        let vertices = ring_with_ornament();
        let result = fit_ring_to_diameter_vertices(
            &vertices,
            2.0,
            2.2, // 1.1x — inside the safe band
            Some([0.0, 0.0, 1.0]),
            &[],
            1.5,
        )
        .unwrap();

        assert!(!result.applied_uniform_fallback);
        // Axial (z) coordinate of the protruding vertex is preserved by radial scaling.
        assert!((result.vertices[4][2] - vertices[4][2]).abs() < 1e-9);
        // Radial extent in the ring plane grew.
        assert!(result.vertices[0][0].abs() > vertices[0][0].abs());
    }
}
