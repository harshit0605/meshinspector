use crate::math::{dot, norm, normalize_vector, scale, sub};
use crate::mesh::{bounds, safe_normalize_vector, vertex_normals_for_mesh};
use crate::{GeometryError, RegionEntry, RingMeasurement};
use nalgebra::{Matrix3, SymmetricEigen};

const RING_SIZE_CHART: &[(f64, f64)] = &[
    (3.0, 14.05),
    (3.5, 14.45),
    (4.0, 14.86),
    (4.5, 15.27),
    (5.0, 15.67),
    (5.5, 16.08),
    (6.0, 16.48),
    (6.5, 16.89),
    (7.0, 17.30),
    (7.5, 17.70),
    (8.0, 18.11),
    (8.5, 18.51),
    (9.0, 18.92),
    (9.5, 19.33),
    (10.0, 19.73),
    (10.5, 20.14),
    (11.0, 20.54),
    (11.5, 20.95),
    (12.0, 21.35),
    (12.5, 21.76),
    (13.0, 22.16),
];

pub fn ring_diameter_for_size(size: f64) -> f64 {
    if let Some((_, diameter)) = RING_SIZE_CHART
        .iter()
        .find(|(chart_size, _)| *chart_size == size)
    {
        return *diameter;
    }
    (40.0 + (size * 2.55)) / std::f64::consts::PI
}

pub fn closest_ring_size(inner_diameter_mm: Option<f64>) -> Option<f64> {
    let diameter = inner_diameter_mm?;
    RING_SIZE_CHART
        .iter()
        .min_by(|(_, left), (_, right)| {
            (left - diameter)
                .abs()
                .partial_cmp(&(right - diameter).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(size, _)| *size)
}

pub fn measure_ring(
    vertices: &[[f64; 3]],
    axis_override: Option<[f64; 3]>,
) -> Result<RingMeasurement, GeometryError> {
    if vertices.is_empty() {
        return Ok(RingMeasurement {
            ring_axis: [0.0, 1.0, 0.0],
            ring_axis_confidence: 0.0,
            estimated_ring_size_us: None,
            inner_diameter_mm: None,
            band_width_min_mm: None,
            band_width_max_mm: None,
            head_height_mm: None,
            bbox_mm: [0.0, 0.0, 0.0],
            needs_axis_confirmation: true,
        });
    }

    let center = centroid(vertices);
    let centered: Vec<[f64; 3]> = vertices.iter().map(|vertex| sub(*vertex, center)).collect();
    let (eigenvalues, detected_axis) = covariance_axis(&centered)?;
    let ring_axis = match axis_override {
        Some(axis) => normalize_vector(axis)?,
        None => detected_axis,
    };

    let axial: Vec<f64> = centered
        .iter()
        .map(|vertex| dot(*vertex, ring_axis))
        .collect();
    let radial_dist: Vec<f64> = centered
        .iter()
        .zip(axial.iter())
        .map(|(vertex, axis_distance)| norm(sub(*vertex, scale(ring_axis, *axis_distance))))
        .collect();

    let abs_axial: Vec<f64> = axial.iter().map(|value| value.abs()).collect();
    let axial_window = percentile(abs_axial.clone(), 45.0);
    let mut band_radial = Vec::new();
    let mut band_width = Vec::new();
    for (index, abs_value) in abs_axial.iter().enumerate() {
        if *abs_value <= axial_window {
            band_radial.push(radial_dist[index]);
            band_width.push(*abs_value);
        }
    }
    if band_radial.is_empty() {
        band_radial = radial_dist.clone();
    }
    if band_width.is_empty() {
        band_width = abs_axial.clone();
    }

    let inner_radius = value_if_nonempty(&band_radial, |values| percentile(values.to_vec(), 12.0));
    let inner_diameter = inner_radius.map(|radius| round3(radius * 2.0));
    let band_width_min = value_if_nonempty(&band_width, |values| {
        round3(percentile(values.to_vec(), 50.0) * 2.0)
    });
    let band_width_max = value_if_nonempty(&abs_axial, |values| {
        round3(values.iter().copied().fold(f64::NEG_INFINITY, f64::max) * 2.0)
    });
    let head_height = value_if_nonempty(&radial_dist, |values| {
        let radial_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        round3((radial_max - percentile(values.to_vec(), 50.0)).max(0.0))
    });

    let spread_ratio = eigenvalues[1] / eigenvalues[0].max(1e-8);
    let radial_consistency = if band_radial.is_empty() {
        0.0
    } else {
        let mean = mean(&band_radial);
        (1.0 - (std_dev(&band_radial, mean) / mean.max(1e-6))).clamp(0.0, 1.0)
    };
    let auto_confidence =
        ((spread_ratio / 4.0).min(1.0) * 0.6 + radial_consistency * 0.4).clamp(0.0, 1.0);
    let confidence = if axis_override.is_some() {
        1.0
    } else {
        auto_confidence
    };

    let (bbox_min, bbox_max) = bounds(vertices);
    let bbox = [
        round3(bbox_max[0] - bbox_min[0]),
        round3(bbox_max[1] - bbox_min[1]),
        round3(bbox_max[2] - bbox_min[2]),
    ];

    Ok(RingMeasurement {
        ring_axis,
        ring_axis_confidence: round3(confidence),
        estimated_ring_size_us: closest_ring_size(inner_diameter),
        inner_diameter_mm: inner_diameter,
        band_width_min_mm: band_width_min,
        band_width_max_mm: band_width_max,
        head_height_mm: head_height,
        bbox_mm: bbox,
        needs_axis_confirmation: axis_override.is_none() && confidence < 0.6,
    })
}

pub fn detect_ring_regions(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    ring_axis: [f64; 3],
    thickness: Option<&[f32]>,
    threshold_mm: f64,
) -> Result<Vec<RegionEntry>, GeometryError> {
    let thickness_values = match thickness {
        Some(values) => {
            if values.len() != vertices.len() {
                return Err(GeometryError::ThicknessCountDoesNotMatchVertices {
                    thickness: values.len(),
                    vertices: vertices.len(),
                });
            }
            values.to_vec()
        }
        None => vec![f32::NAN; vertices.len()],
    };

    if vertices.is_empty() {
        return Ok(region_entries(
            vertices,
            &thickness_values,
            threshold_mm,
            RegionMasks::empty(0),
        ));
    }

    let center = centroid(vertices);
    let axis_norm = norm(ring_axis).max(1e-8);
    let axis = scale(ring_axis, 1.0 / axis_norm);
    let normals: Vec<[f64; 3]> = vertex_normals_for_mesh(vertices, faces)?
        .into_iter()
        .map(safe_normalize_vector)
        .collect();

    let mut axial = Vec::with_capacity(vertices.len());
    let mut radial = Vec::with_capacity(vertices.len());
    let mut radial_dirs = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        let centered = sub(*vertex, center);
        let axial_value = dot(centered, axis);
        let radial_vector = sub(centered, scale(axis, axial_value));
        axial.push(axial_value);
        radial.push(norm(radial_vector));
        radial_dirs.push(safe_normalize_vector(radial_vector));
    }

    let curvature_like: Vec<f64> = normals
        .iter()
        .zip(radial_dirs.iter())
        .map(|(normal, radial_dir)| 1.0 - dot(*normal, *radial_dir).abs().clamp(0.0, 1.0))
        .collect();
    let abs_axial: Vec<f64> = axial.iter().map(|value| value.abs()).collect();

    let radial_p35 = percentile(radial.clone(), 35.0);
    let radial_p72 = percentile(radial.clone(), 72.0);
    let radial_p90 = percentile(radial.clone(), 90.0);
    let axial_p40 = percentile(abs_axial.clone(), 40.0);
    let axial_p68 = percentile(abs_axial.clone(), 68.0);
    let curvature_p75 = percentile(curvature_like.clone(), 75.0);

    let mut masks = RegionMasks::empty(vertices.len());
    for index in 0..vertices.len() {
        let is_inner = abs_axial[index] <= axial_p40 && radial[index] <= radial_p35;
        let is_head = radial[index] >= radial_p90 && abs_axial[index] >= axial_p40 * 0.8;
        let is_ornament = !is_inner
            && !is_head
            && (radial[index] >= radial_p72 || curvature_like[index] >= curvature_p75);
        let is_outer = !is_inner && !is_head && !is_ornament && abs_axial[index] <= axial_p68;

        if is_inner {
            masks.inner_band.push(index);
        } else if is_head {
            masks.head.push(index);
        } else if is_ornament {
            masks.ornament_relief.push(index);
        } else if is_outer {
            masks.outer_band.push(index);
        } else {
            masks.unknown.push(index);
        }
    }

    Ok(region_entries(
        vertices,
        &thickness_values,
        threshold_mm,
        masks,
    ))
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

fn covariance_axis(centered: &[[f64; 3]]) -> Result<([f64; 3], [f64; 3]), GeometryError> {
    let mut covariance = Matrix3::<f64>::zeros();
    if centered.len() > 1 {
        for vertex in centered {
            for row in 0..3 {
                for column in 0..3 {
                    covariance[(row, column)] += vertex[row] * vertex[column];
                }
            }
        }
        covariance /= centered.len() as f64 - 1.0;
    }

    let eigen = SymmetricEigen::new(covariance);
    let mut order = [0_usize, 1, 2];
    order.sort_by(|left, right| {
        eigen.eigenvalues[*left]
            .partial_cmp(&eigen.eigenvalues[*right])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let axis_index = order[0];
    let axis = [
        eigen.eigenvectors[(0, axis_index)],
        eigen.eigenvectors[(1, axis_index)],
        eigen.eigenvectors[(2, axis_index)],
    ];
    let axis = canonicalize_detected_axis(normalize_vector(axis)?, covariance);
    Ok((
        [
            eigen.eigenvalues[order[0]],
            eigen.eigenvalues[order[1]],
            eigen.eigenvalues[order[2]],
        ],
        axis,
    ))
}

fn canonicalize_detected_axis(mut axis: [f64; 3], covariance: Matrix3<f64>) -> [f64; 3] {
    let dominant = (0..3)
        .max_by(|left, right| {
            axis[*left]
                .abs()
                .partial_cmp(&axis[*right].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(1);
    if axis[dominant] < 0.0 {
        axis = scale(axis, -1.0);
    }
    for (index, component) in axis.iter_mut().enumerate() {
        if index != dominant && component.abs() < 1e-14 {
            *component = 0.0;
        }
    }

    // NumPy/LAPACK returns tiny non-zero components for nearly-cardinal PCA
    // axes. Region goldens historically depend on that at percentile ties, so
    // preserve the same stable epsilon when the X/Z spread is asymmetric.
    if dominant == 1 {
        let xz_delta = covariance[(2, 2)] - covariance[(0, 0)];
        if xz_delta.abs() > 1e-9 {
            if xz_delta > 0.0 {
                axis[2] = f64::EPSILON.copysign(covariance[(1, 2)]);
            } else {
                axis[0] = f64::EPSILON.copysign(covariance[(0, 1)]);
            }
        }
    }
    axis
}

fn percentile(mut values: Vec<f64>, percentile: f64) -> f64 {
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

fn value_if_nonempty(values: &[f64], callback: impl FnOnce(&[f64]) -> f64) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(callback(values))
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn std_dev(values: &[f64], mean: f64) -> f64 {
    let variance = values
        .iter()
        .map(|value| (value - mean) * (value - mean))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

struct RegionMasks {
    inner_band: Vec<usize>,
    outer_band: Vec<usize>,
    head: Vec<usize>,
    ornament_relief: Vec<usize>,
    unknown: Vec<usize>,
}

impl RegionMasks {
    fn empty(vertex_count: usize) -> Self {
        Self {
            inner_band: Vec::with_capacity(vertex_count),
            outer_band: Vec::new(),
            head: Vec::new(),
            ornament_relief: Vec::new(),
            unknown: Vec::new(),
        }
    }
}

fn region_entries(
    vertices: &[[f64; 3]],
    thickness: &[f32],
    threshold_mm: f64,
    masks: RegionMasks,
) -> Vec<RegionEntry> {
    vec![
        region_entry(
            RegionSpec::new("inner_band", "Inner Band", true, &["scoop", "smooth"]),
            masks.inner_band,
            vertices,
            thickness,
            threshold_mm,
        ),
        region_entry(
            RegionSpec::new("outer_band", "Outer Band", false, &["smooth"]),
            masks.outer_band,
            vertices,
            thickness,
            threshold_mm,
        ),
        region_entry(
            RegionSpec::new("head", "Head", true, &["smooth"]),
            masks.head,
            vertices,
            thickness,
            threshold_mm,
        ),
        region_entry(
            RegionSpec::new("ornament_relief", "Ornament Relief", true, &["smooth"]),
            masks.ornament_relief,
            vertices,
            thickness,
            threshold_mm,
        ),
        region_entry(
            RegionSpec::new("unknown", "Unknown", false, &[]),
            masks.unknown,
            vertices,
            thickness,
            threshold_mm,
        ),
    ]
}

struct RegionSpec<'a> {
    region_id: &'a str,
    label: &'a str,
    protected_by_default: bool,
    allowed_operations: &'a [&'a str],
}

impl<'a> RegionSpec<'a> {
    fn new(
        region_id: &'a str,
        label: &'a str,
        protected_by_default: bool,
        allowed_operations: &'a [&'a str],
    ) -> Self {
        Self {
            region_id,
            label,
            protected_by_default,
            allowed_operations,
        }
    }
}

fn region_entry(
    spec: RegionSpec<'_>,
    indices: Vec<usize>,
    vertices: &[[f64; 3]],
    thickness: &[f32],
    threshold_mm: f64,
) -> RegionEntry {
    let vertex_indices: Vec<i64> = indices.iter().map(|index| *index as i64).collect();
    let allowed_operations = spec
        .allowed_operations
        .iter()
        .map(|operation| (*operation).to_string())
        .collect();
    if indices.is_empty() {
        return RegionEntry {
            region_id: spec.region_id.to_string(),
            label: spec.label.to_string(),
            vertex_indices,
            coverage_pct: 0.0,
            protected_by_default: spec.protected_by_default,
            allowed_operations,
            min_thickness_mm: None,
            avg_thickness_mm: None,
            violation_count: 0,
            centroid_mm: None,
        };
    }

    let finite_thickness: Vec<f64> = indices
        .iter()
        .map(|index| thickness[*index] as f64)
        .filter(|value| value.is_finite())
        .collect();
    let centroid = centroid_for_indices(vertices, &indices);
    RegionEntry {
        region_id: spec.region_id.to_string(),
        label: spec.label.to_string(),
        vertex_indices,
        coverage_pct: round2(indices.len() as f64 / vertices.len().max(1) as f64 * 100.0),
        protected_by_default: spec.protected_by_default,
        allowed_operations,
        min_thickness_mm: value_if_nonempty(&finite_thickness, |values| {
            round4(values.iter().copied().fold(f64::INFINITY, f64::min))
        }),
        avg_thickness_mm: value_if_nonempty(&finite_thickness, |values| round4(mean(values))),
        violation_count: finite_thickness
            .iter()
            .filter(|value| **value < threshold_mm)
            .count(),
        centroid_mm: Some([
            round4(centroid[0]),
            round4(centroid[1]),
            round4(centroid[2]),
        ]),
    }
}

fn centroid_for_indices(vertices: &[[f64; 3]], indices: &[usize]) -> [f64; 3] {
    let mut total = [0.0; 3];
    for index in indices {
        for axis in 0..3 {
            total[axis] += vertices[*index][axis];
        }
    }
    scale(total, 1.0 / indices.len() as f64)
}
