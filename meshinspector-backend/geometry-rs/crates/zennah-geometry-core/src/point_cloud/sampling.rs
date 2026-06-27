use std::collections::BTreeSet;

use super::types::{GridSample, PointCloudSelectionObject};

pub fn point_cloud_extract_selected_points_as_object(
    points: &[[f64; 3]],
    selected_point_ids: &[usize],
) -> Result<PointCloudSelectionObject, String> {
    validate_point_cloud(points)?;
    if selected_point_ids.is_empty() {
        return Err("selected_point_ids must not be empty".to_string());
    }

    let mut source_point_indices = BTreeSet::new();
    for point_id in selected_point_ids {
        if *point_id >= points.len() {
            return Err(format!(
                "selected point {point_id} is outside point count {}",
                points.len()
            ));
        }
        source_point_indices.insert(*point_id);
    }

    let source_point_indices = source_point_indices.into_iter().collect::<Vec<_>>();
    let selected_points = source_point_indices
        .iter()
        .map(|index| points[*index])
        .collect::<Vec<_>>();
    Ok(PointCloudSelectionObject {
        points: selected_points,
        source_point_indices,
    })
}

pub fn point_cloud_grid_sample_indices(
    points: &[[f64; 3]],
    voxel_size: f64,
    max_voxels: usize,
) -> Result<Vec<usize>, String> {
    validate_point_cloud(points)?;
    if voxel_size <= 0.0 {
        return Ok((0..points.len()).collect());
    }
    if !voxel_size.is_finite() {
        return Err("voxel_size must be finite".to_string());
    }
    if max_voxels == 0 {
        return Err("max_voxels must be positive".to_string());
    }

    let (min, max) = bounding_box(points);
    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let mut effective_voxel_size = voxel_size;
    let estimated_voxels = size
        .iter()
        .map(|extent| extent / effective_voxel_size)
        .product::<f64>();
    if estimated_voxels > max_voxels as f64 {
        effective_voxel_size *= (estimated_voxels / max_voxels as f64).cbrt();
    }
    let dims = grid_dimensions(size, effective_voxel_size);
    let voxel_size_by_axis = [
        axis_voxel_size(size[0], dims[0]),
        axis_voxel_size(size[1], dims[1]),
        axis_voxel_size(size[2], dims[2]),
    ];
    let recip_voxel_size = [
        1.0 / voxel_size_by_axis[0],
        1.0 / voxel_size_by_axis[1],
        1.0 / voxel_size_by_axis[2],
    ];
    let voxel_count = dims[0]
        .checked_mul(dims[1])
        .and_then(|count| count.checked_mul(dims[2]))
        .ok_or_else(|| "point cloud grid dimensions overflowed".to_string())?;
    let mut samples = vec![None::<GridSample>; voxel_count];

    for (index, point) in points.iter().enumerate() {
        let position = point_grid_position(*point, min, recip_voxel_size, dims);
        let center = voxel_center(position, min, voxel_size_by_axis);
        let center_distance_sq = squared_distance(*point, center);
        let voxel_index = voxel_index(position, dims);
        let replace = samples[voxel_index]
            .map(|sample| center_distance_sq < sample.center_distance_sq)
            .unwrap_or(true);
        if replace {
            samples[voxel_index] = Some(GridSample {
                index,
                center_distance_sq,
            });
        }
    }

    let mut indices = samples
        .into_iter()
        .flatten()
        .map(|sample| sample.index)
        .collect::<Vec<_>>();
    indices.sort_unstable();
    Ok(indices)
}

pub fn point_cloud_uniform_sample_indices(
    points: &[[f64; 3]],
    distance: f64,
    min_normal_dot: f64,
    lexicographical_order: bool,
    normals: Option<&[[f64; 3]]>,
) -> Result<Vec<usize>, String> {
    validate_point_cloud(points)?;
    if !distance.is_finite() {
        return Err("distance must be finite".to_string());
    }
    if !min_normal_dot.is_finite() {
        return Err("min_normal_dot must be finite".to_string());
    }
    if let Some(normals) = normals {
        validate_normals(points, normals)?;
    }

    let mut visited = vec![false; points.len()];
    let mut sampled = vec![false; points.len()];
    let max_distance_sq = distance * distance;
    let order = point_visit_order(points, lexicographical_order);

    for index in order {
        if visited[index] {
            continue;
        }
        sampled[index] = true;
        let mut local_max_distance_sq = max_distance_sq;
        let mut near_points = Vec::<(usize, f64)>::new();

        for (neighbor_index, neighbor) in points.iter().enumerate() {
            let distance_sq = squared_distance(points[index], *neighbor);
            if distance_sq > max_distance_sq {
                continue;
            }
            if let Some(normals) = normals {
                if dot(normals[index], normals[neighbor_index]).abs() < min_normal_dot {
                    local_max_distance_sq = local_max_distance_sq.min(distance_sq);
                    continue;
                }
            }
            near_points.push((neighbor_index, distance_sq));
        }

        for (neighbor_index, distance_sq) in near_points {
            if distance_sq < local_max_distance_sq {
                visited[neighbor_index] = true;
            }
        }
    }

    Ok(sampled
        .into_iter()
        .enumerate()
        .filter_map(|(index, is_sampled)| is_sampled.then_some(index))
        .collect())
}

pub(super) fn validate_point_cloud(points: &[[f64; 3]]) -> Result<(), String> {
    validate_point_rows("point cloud", points, false)
}

pub(super) fn validate_point_rows(
    name: &str,
    points: &[[f64; 3]],
    allow_empty: bool,
) -> Result<(), String> {
    if points.is_empty() {
        if allow_empty {
            return Ok(());
        }
        return Err(format!("{name} must not be empty"));
    }
    if points
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(format!("{name} coordinates must be finite"));
    }
    Ok(())
}

pub(super) fn validate_distance_limit(name: &str, value: f64) -> Result<(), String> {
    if value.is_nan() || value < 0.0 {
        return Err(format!("{name} must be non-negative"));
    }
    Ok(())
}

pub(super) fn validate_point_indices(
    name: &str,
    indices: &[usize],
    point_count: usize,
) -> Result<(), String> {
    if indices.iter().any(|index| *index >= point_count) {
        return Err(format!("{name} must reference existing points"));
    }
    Ok(())
}

pub(super) fn validate_normals(points: &[[f64; 3]], normals: &[[f64; 3]]) -> Result<(), String> {
    if normals.len() != points.len() {
        return Err("normals must have the same length as points".to_string());
    }
    if normals
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("normals must be finite".to_string());
    }
    Ok(())
}

fn point_visit_order(points: &[[f64; 3]], lexicographical_order: bool) -> Vec<usize> {
    let mut order = (0..points.len()).collect::<Vec<_>>();
    if lexicographical_order {
        order.sort_by(|left, right| {
            points[*left][0]
                .total_cmp(&points[*right][0])
                .then_with(|| points[*left][1].total_cmp(&points[*right][1]))
                .then_with(|| points[*left][2].total_cmp(&points[*right][2]))
        });
    }
    order
}

fn bounding_box(points: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let mut min = points[0];
    let mut max = points[0];
    for point in points.iter().skip(1) {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    (min, max)
}

fn grid_dimensions(size: [f64; 3], voxel_size: f64) -> [usize; 3] {
    const MAX_VOXELS_IN_ONE_DIM: usize = 1 << 10;
    size.map(|extent| ((extent / voxel_size).ceil() as usize).clamp(1, MAX_VOXELS_IN_ONE_DIM))
}

fn axis_voxel_size(extent: f64, dimension: usize) -> f64 {
    if extent <= 0.0 {
        1.0
    } else {
        extent / dimension as f64
    }
}

fn point_grid_position(
    point: [f64; 3],
    min: [f64; 3],
    recip_voxel_size: [f64; 3],
    dims: [usize; 3],
) -> [usize; 3] {
    [
        axis_grid_position(point[0], min[0], recip_voxel_size[0], dims[0]),
        axis_grid_position(point[1], min[1], recip_voxel_size[1], dims[1]),
        axis_grid_position(point[2], min[2], recip_voxel_size[2], dims[2]),
    ]
}

fn axis_grid_position(point: f64, min: f64, recip_voxel_size: f64, dimension: usize) -> usize {
    (((point - min) * recip_voxel_size) as usize).clamp(0, dimension - 1)
}

fn voxel_center(position: [usize; 3], min: [f64; 3], voxel_size: [f64; 3]) -> [f64; 3] {
    [
        min[0] + (position[0] as f64 + 0.5) * voxel_size[0],
        min[1] + (position[1] as f64 + 0.5) * voxel_size[1],
        min[2] + (position[2] as f64 + 0.5) * voxel_size[2],
    ]
}

fn voxel_index(position: [usize; 3], dims: [usize; 3]) -> usize {
    position[0] + dims[0] * (position[1] + dims[1] * position[2])
}

pub(super) fn squared_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    dx * dx + dy * dy + dz * dz
}

pub(super) fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}
