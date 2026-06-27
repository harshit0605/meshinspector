use super::sampling::{
    dot, squared_distance, validate_distance_limit, validate_normals, validate_point_indices,
    validate_point_rows,
};
use super::types::{PointCloudClosestPair, PointCloudProjectionResult};

pub fn point_cloud_nearest_projections(
    query_points: &[[f64; 3]],
    reference_points: &[[f64; 3]],
    up_dist_limit_sq: f64,
    lo_dist_limit_sq: f64,
    skip_same_index: bool,
) -> Result<PointCloudProjectionResult, String> {
    validate_point_rows("query points", query_points, true)?;
    validate_point_rows("reference points", reference_points, true)?;
    validate_distance_limit("up_dist_limit_sq", up_dist_limit_sq)?;
    validate_distance_limit("lo_dist_limit_sq", lo_dist_limit_sq)?;

    let mut points = Vec::with_capacity(query_points.len());
    let mut squared_distances = Vec::with_capacity(query_points.len());
    let mut vertex_indices = Vec::with_capacity(query_points.len());

    for (query_index, query) in query_points.iter().enumerate() {
        let mut best_index = None::<usize>;
        let mut best_distance_sq = up_dist_limit_sq;
        for (reference_index, reference) in reference_points.iter().enumerate() {
            if skip_same_index && query_index == reference_index {
                continue;
            }
            let distance_sq = squared_distance(*query, *reference);
            if distance_sq < best_distance_sq {
                best_index = Some(reference_index);
                best_distance_sq = distance_sq;
                if distance_sq <= lo_dist_limit_sq {
                    break;
                }
            }
        }
        if let Some(best_index) = best_index {
            points.push(reference_points[best_index]);
            squared_distances.push(best_distance_sq);
            vertex_indices.push(best_index as i64);
        } else {
            points.push([0.0, 0.0, 0.0]);
            squared_distances.push(up_dist_limit_sq);
            vertex_indices.push(-1);
        }
    }

    Ok(PointCloudProjectionResult {
        points,
        squared_distances,
        vertex_indices,
    })
}

pub fn point_cloud_n_closest_neighbors(
    points: &[[f64; 3]],
    num_neighbors: usize,
    up_dist_limit_sq: f64,
) -> Result<Vec<Vec<i64>>, String> {
    validate_point_rows("point cloud", points, true)?;
    if num_neighbors == 0 {
        return Err("num_neighbors must be positive".to_string());
    }
    validate_distance_limit("up_dist_limit_sq", up_dist_limit_sq)?;

    let mut rows = Vec::with_capacity(points.len());
    for (point_index, point) in points.iter().enumerate() {
        let mut candidates = points
            .iter()
            .enumerate()
            .filter_map(|(candidate_index, candidate)| {
                if candidate_index == point_index {
                    return None;
                }
                let distance_sq = squared_distance(*point, *candidate);
                (distance_sq < up_dist_limit_sq).then_some((candidate_index, distance_sq))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        });

        let mut row = candidates
            .into_iter()
            .take(num_neighbors)
            .map(|(candidate_index, _)| candidate_index as i64)
            .collect::<Vec<_>>();
        row.resize(num_neighbors, -1);
        rows.push(row);
    }
    Ok(rows)
}

pub fn point_cloud_two_closest_points(
    points: &[[f64; 3]],
) -> Result<PointCloudClosestPair, String> {
    validate_point_rows("point cloud", points, true)?;
    let mut best_pair = [-1_i64, -1_i64];
    let mut best_distance_sq = f64::INFINITY;

    for left in 0..points.len() {
        for right in (left + 1)..points.len() {
            let distance_sq = squared_distance(points[left], points[right]);
            let candidate_pair = [left as i64, right as i64];
            if distance_sq < best_distance_sq
                || (distance_sq == best_distance_sq && candidate_pair < best_pair)
            {
                best_distance_sq = distance_sq;
                best_pair = candidate_pair;
            }
        }
    }

    Ok(PointCloudClosestPair {
        vertex_indices: best_pair,
        squared_distance: best_distance_sq,
    })
}

pub fn point_cloud_neighbors_in_radius(
    points: &[[f64; 3]],
    center_index: usize,
    radius: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<Vec<i64>, String> {
    validate_point_rows("point cloud", points, false)?;
    if center_index >= points.len() {
        return Err("center_index must reference a point".to_string());
    }
    if !radius.is_finite() || radius < 0.0 {
        return Err("radius must be finite and non-negative".to_string());
    }
    if let Some(normals) = normals {
        validate_normals(points, normals)?;
    }
    validate_point_indices("untrusted_indices", untrusted_indices, points.len())?;

    let radius_sq = radius * radius;
    let center = points[center_index];
    let mut neighbors = points
        .iter()
        .enumerate()
        .filter_map(|(candidate_index, candidate)| {
            if candidate_index == center_index {
                return None;
            }
            (squared_distance(center, *candidate) <= radius_sq).then_some(candidate_index)
        })
        .collect::<Vec<_>>();

    if let Some(normals) = normals {
        if !untrusted_indices.contains(&center_index) {
            neighbors.retain(|neighbor_index| {
                untrusted_indices.contains(neighbor_index)
                    || dot(normals[center_index], normals[*neighbor_index]) >= -0.3
            });
        }
    }

    Ok(neighbors.into_iter().map(|index| index as i64).collect())
}
