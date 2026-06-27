mod fill;
mod optimizer;
mod repetitions;
mod topology;

pub use fill::*;
use optimizer::optimize_fan;
pub use repetitions::*;
pub use topology::*;

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudLocalFan {
    pub neighbors: Vec<i64>,
    pub boundary_neighbor: i64,
    pub actual_radius: f64,
    pub removed_count: usize,
}

pub fn point_cloud_local_neighbor_fan(
    points: &[[f64; 3]],
    center_index: usize,
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<PointCloudLocalFan, String> {
    validate_points(points)?;
    if center_index >= points.len() {
        return Err("center_index must reference a point".to_string());
    }
    if !radius.is_finite() || radius < 0.0 {
        return Err("radius must be finite and non-negative".to_string());
    }
    if !boundary_angle.is_finite() || boundary_angle < 0.0 {
        return Err("boundary_angle must be finite and non-negative".to_string());
    }
    if !crit_angle.is_finite() || crit_angle < 0.0 {
        return Err("crit_angle must be finite and non-negative".to_string());
    }
    if (radius > 0.0) == (num_neighbors > 0) {
        return Err("exactly one of radius or num_neighbors must be positive".to_string());
    }
    if let Some(normals) = normals {
        validate_normals(points, normals)?;
    }
    for &index in untrusted_indices {
        if index >= points.len() {
            return Err("untrusted_indices must reference existing points".to_string());
        }
    }

    let (mut neighbors, actual_radius) = if radius > 0.0 {
        neighbors_in_radius(points, center_index, radius)
    } else {
        nearest_neighbors(points, center_index, num_neighbors)
    };
    if let Some(normals) = normals {
        filter_crossing_normals(normals, untrusted_indices, center_index, &mut neighbors);
    }
    if neighbors.is_empty() {
        return Ok(PointCloudLocalFan {
            neighbors: Vec::new(),
            boundary_neighbor: -1,
            actual_radius,
            removed_count: 0,
        });
    }

    let sort_data = sort_fan_neighbors(
        points,
        center_index,
        normals,
        untrusted_indices,
        &mut neighbors,
    );
    let (boundary_neighbor, removed_count) = optimize_fan(
        points,
        center_index,
        normals,
        untrusted_indices,
        &mut neighbors,
        sort_data,
        boundary_angle,
        max_removes,
        crit_angle,
    );

    Ok(PointCloudLocalFan {
        neighbors: neighbors.into_iter().map(|index| index as i64).collect(),
        boundary_neighbor: boundary_neighbor.map(|index| index as i64).unwrap_or(-1),
        actual_radius,
        removed_count,
    })
}

#[derive(Debug, Clone)]
pub(super) struct FanSortData {
    pub(super) angles: Vec<f64>,
    pub(super) normal: [f64; 3],
    pub(super) normalizer_sq: f64,
}

fn neighbors_in_radius(points: &[[f64; 3]], center_index: usize, radius: f64) -> (Vec<usize>, f64) {
    let radius_sq = radius * radius;
    let center = points[center_index];
    let neighbors = points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            (index != center_index && squared_distance(center, *point) <= radius_sq)
                .then_some(index)
        })
        .collect();
    (neighbors, radius)
}

fn nearest_neighbors(
    points: &[[f64; 3]],
    center_index: usize,
    num_neighbors: usize,
) -> (Vec<usize>, f64) {
    let center = points[center_index];
    let mut candidates = points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            (index != center_index).then_some((index, squared_distance(center, *point)))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    candidates.truncate(num_neighbors);
    let actual_radius = candidates
        .last()
        .map(|(_, distance_sq)| distance_sq.sqrt())
        .unwrap_or(0.0);
    (
        candidates.into_iter().map(|(index, _)| index).collect(),
        actual_radius,
    )
}

fn filter_crossing_normals(
    normals: &[[f64; 3]],
    untrusted_indices: &[usize],
    center_index: usize,
    neighbors: &mut Vec<usize>,
) {
    if untrusted_indices.contains(&center_index) {
        return;
    }
    let center_normal = normals[center_index];
    neighbors.retain(|neighbor| {
        untrusted_indices.contains(neighbor) || dot(center_normal, normals[*neighbor]) >= -0.3
    });
}

fn sort_fan_neighbors(
    points: &[[f64; 3]],
    center_index: usize,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
    neighbors: &mut [usize],
) -> FanSortData {
    let center = points[center_index];
    let normal = local_plane_normal(points, center_index, normals, untrusted_indices, neighbors);
    let first_projected = project_to_plane(points[neighbors[0]], center, normal);
    let mut base = sub(first_projected, center);
    let mut normalizer_sq = length_sq(base);
    if normalizer_sq > 0.0 {
        base = scale(base, 1.0 / normalizer_sq.sqrt());
    } else {
        base = [0.0, 0.0, 0.0];
        for neighbor in neighbors.iter().skip(1) {
            let projected = project_to_plane(points[*neighbor], center, normal);
            normalizer_sq = length_sq(sub(projected, center));
            if normalizer_sq > 0.0 {
                break;
            }
        }
        if normalizer_sq <= 0.0 {
            normalizer_sq = 1.0;
        }
    }

    let mut ordered = neighbors
        .iter()
        .copied()
        .enumerate()
        .map(|(position, index)| {
            let projected = project_to_plane(points[index], center, normal);
            let direction = normalized(sub(projected, center));
            let cross_prod = cross(direction, base);
            let sign = if dot(cross_prod, normal) < 0.0 {
                -1.0
            } else {
                1.0
            };
            let angle = (sign * length(cross_prod)).atan2(dot(direction, base));
            (angle, position, index)
        })
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });

    let mut angles = Vec::with_capacity(ordered.len());
    for (slot, (angle, _, index)) in neighbors.iter_mut().zip(ordered) {
        *slot = index;
        angles.push(angle);
    }
    FanSortData {
        angles,
        normal,
        normalizer_sq,
    }
}

pub(super) fn fan_boundary_neighbor(
    neighbors: &[usize],
    angles: &[f64],
    boundary_angle: f64,
) -> Option<usize> {
    if neighbors.is_empty() {
        return None;
    }
    for index in 0..angles.len() {
        let next = if index + 1 < angles.len() {
            index + 1
        } else {
            0
        };
        let mut diff = angles[next] - angles[index];
        if next == 0 {
            diff += std::f64::consts::TAU;
        }
        if diff > boundary_angle {
            return Some(neighbors[index]);
        }
    }
    None
}

fn local_plane_normal(
    points: &[[f64; 3]],
    center_index: usize,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
    neighbors: &[usize],
) -> [f64; 3] {
    if let Some(normals) = normals {
        if !untrusted_indices.contains(&center_index) {
            return normalized(normals[center_index]);
        }
    }
    best_fit_normal(points, center_index, neighbors)
}

fn best_fit_normal(points: &[[f64; 3]], center_index: usize, neighbors: &[usize]) -> [f64; 3] {
    let center = points[center_index];
    for left in 0..neighbors.len() {
        let a = sub(points[neighbors[left]], center);
        for right in (left + 1)..neighbors.len() {
            let candidate = cross(a, sub(points[neighbors[right]], center));
            if length_sq(candidate) > 1e-24 {
                return normalized(candidate);
            }
        }
    }
    [0.0, 0.0, 1.0]
}

fn validate_points(points: &[[f64; 3]]) -> Result<(), String> {
    if points.is_empty() {
        return Err("point cloud must not be empty".to_string());
    }
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err("point cloud coordinates must be finite".to_string());
    }
    Ok(())
}

fn validate_normals(points: &[[f64; 3]], normals: &[[f64; 3]]) -> Result<(), String> {
    if normals.len() != points.len() {
        return Err("normals must match point cloud length".to_string());
    }
    if normals.iter().flatten().any(|value| !value.is_finite()) {
        return Err("normals must be finite".to_string());
    }
    Ok(())
}

fn project_to_plane(point: [f64; 3], plane_point: [f64; 3], normal: [f64; 3]) -> [f64; 3] {
    sub(point, scale(normal, dot(sub(point, plane_point), normal)))
}

pub(super) fn squared_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    length_sq(sub(left, right))
}

pub(super) fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

pub(super) fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

pub(super) fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

pub(super) fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

pub(super) fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

pub(super) fn length(vector: [f64; 3]) -> f64 {
    length_sq(vector).sqrt()
}

pub(super) fn length_sq(vector: [f64; 3]) -> f64 {
    dot(vector, vector)
}

pub(super) fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let len = length(vector);
    if len > 0.0 {
        scale(vector, 1.0 / len)
    } else {
        [0.0, 0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_cloud_local_neighbor_fan_orders_projected_neighbors_like_meshlib() {
        let points = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let fan = point_cloud_local_neighbor_fan(
            &points,
            0,
            1.1,
            0,
            3.2,
            0,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("local fan should build");

        assert_eq!(fan.neighbors, vec![2, 1, 4, 3]);
        assert_eq!(fan.boundary_neighbor, -1);
        assert_eq!(fan.actual_radius, 1.1);
        assert_eq!(fan.removed_count, 0);
    }

    #[test]
    fn point_cloud_local_neighbor_fan_marks_neighbor_before_large_boundary_gap() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let fan = point_cloud_local_neighbor_fan(
            &points,
            0,
            1.1,
            0,
            3.0,
            0,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("local fan should build");

        assert_eq!(fan.neighbors, vec![2, 1]);
        assert_eq!(fan.boundary_neighbor, 1);
    }

    #[test]
    fn point_cloud_local_neighbor_fan_supports_num_neighbor_search_radius() {
        let points = vec![
            [0.0, 0.0, 0.0],
            [0.5, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
        ];

        let fan = point_cloud_local_neighbor_fan(
            &points,
            0,
            0.0,
            2,
            std::f64::consts::TAU,
            0,
            std::f64::consts::TAU,
            None,
            &[],
        )
        .expect("local fan should build");

        assert_eq!(fan.neighbors, vec![1, 2]);
        assert_eq!(fan.actual_radius, 1.0);
    }

    #[test]
    fn point_cloud_local_neighbor_fan_optimizer_removes_center_coincident_neighbor_like_meshlib() {
        let points = vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ];

        let fan = point_cloud_local_neighbor_fan(
            &points,
            0,
            1.1,
            0,
            std::f64::consts::TAU,
            1,
            std::f64::consts::TAU,
            None,
            &[],
        )
        .expect("local fan should build");

        assert_eq!(fan.neighbors, vec![2, 3]);
        assert_eq!(fan.removed_count, 0);
    }
}
