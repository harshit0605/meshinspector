use nalgebra::{Matrix3, SMatrix, SVector, Vector3};

use crate::spatial::{build_flat_bvh, nearest_point_in_cloud};
use rayon::prelude::*;

mod multiway;
pub use multiway::{
    multiway_aabb_cascade_combined_icp, multiway_aabb_cascade_point_to_plane_icp,
    multiway_aabb_cascade_point_to_point_icp, multiway_all_object_combined_icp,
    multiway_all_object_point_to_plane_icp, multiway_all_object_point_to_point_icp,
    multiway_combined_icp, multiway_point_to_plane_icp, multiway_point_to_point_icp,
    multiway_sequential_cascade_combined_icp, multiway_sequential_cascade_point_to_plane_icp,
    multiway_sequential_cascade_point_to_point_icp, MultiwayIcpObjectResult,
    MultiwayIcpRegistrationResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcpMode {
    AnyRigidXf,
    TranslationOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IcpRegistrationResult {
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub transform: [[f64; 4]; 4],
    pub iterations: usize,
    pub mean_square_distance: f64,
    pub active_pair_count: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IcpPairFilterOptions<'a> {
    pub floating_normals: Option<&'a [[f64; 3]]>,
    pub max_pair_distance: Option<f64>,
    pub cos_threshold: Option<f64>,
    pub far_dist_factor: Option<f64>,
    pub mutual_closest: bool,
}

pub(super) struct PointPlanePair {
    pub(super) floating: Vector3<f64>,
    pub(super) reference: Vector3<f64>,
    pub(super) reference_normal: Vector3<f64>,
    pub(super) distance_sq: f64,
}

pub fn pairwise_point_to_point_icp(
    floating: &[[f64; 3]],
    reference: &[[f64; 3]],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
) -> Result<IcpRegistrationResult, String> {
    validate_points("floating", floating)?;
    validate_points("reference", reference)?;
    let minimum_pairs = match mode {
        IcpMode::AnyRigidXf => 3,
        IcpMode::TranslationOnly => 1,
    };
    if floating.len() < minimum_pairs || reference.len() < minimum_pairs {
        return Err(format!(
            "ICP mode {mode:?} requires at least {minimum_pairs} points in each cloud"
        ));
    }

    let mut rotation = Matrix3::<f64>::identity();
    let mut translation = Vector3::<f64>::zeros();
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let mut best_rotation = rotation;
    let mut best_translation = translation;
    let mut best_distance =
        mean_square_distance(floating, reference, &rotation, &translation).unwrap_or(f64::MAX);
    let mut iterations = 0;
    let mut active_pair_count = 0;

    for iteration in 1..=iteration_limit {
        let pairs = nearest_point_pairs(floating, reference, &rotation, &translation)?;
        active_pair_count = pairs.len();
        let (delta_rotation, delta_translation) = point_to_point_amendment(&pairs, mode)?;
        rotation = delta_rotation * rotation;
        translation = delta_rotation * translation + delta_translation;
        iterations = iteration;

        let current_distance = mean_square_distance(floating, reference, &rotation, &translation)
            .ok_or_else(|| "ICP produced no active pairs".to_string())?;
        if current_distance + tolerance < best_distance {
            best_rotation = rotation;
            best_translation = translation;
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            best_rotation = rotation;
            best_translation = translation;
            best_distance = current_distance;
            break;
        } else {
            break;
        }

        if best_distance <= tolerance {
            break;
        }
    }

    Ok(result_from_parts(
        best_rotation,
        best_translation,
        iterations,
        best_distance,
        active_pair_count,
    ))
}

pub fn pairwise_point_to_plane_icp(
    floating: &[[f64; 3]],
    reference: &[[f64; 3]],
    reference_normals: &[[f64; 3]],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
) -> Result<IcpRegistrationResult, String> {
    pairwise_point_to_plane_icp_with_filters(
        floating,
        reference,
        reference_normals,
        max_iterations,
        tolerance,
        mode,
        IcpPairFilterOptions::default(),
    )
}

pub fn pairwise_point_to_plane_icp_with_filters(
    floating: &[[f64; 3]],
    reference: &[[f64; 3]],
    reference_normals: &[[f64; 3]],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    filter_options: IcpPairFilterOptions<'_>,
) -> Result<IcpRegistrationResult, String> {
    validate_points("floating", floating)?;
    validate_points("reference", reference)?;
    validate_normals(reference, reference_normals)?;
    validate_filter_options(floating, filter_options)?;
    let minimum_pairs = match mode {
        IcpMode::AnyRigidXf => 3,
        IcpMode::TranslationOnly => 1,
    };
    if floating.len() < minimum_pairs || reference.len() < minimum_pairs {
        return Err(format!(
            "point-to-plane ICP mode {mode:?} requires at least {minimum_pairs} points in each cloud"
        ));
    }

    let mut rotation = Matrix3::<f64>::identity();
    let mut translation = Vector3::<f64>::zeros();
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let mut best_rotation = rotation;
    let mut best_translation = translation;
    let mut best_distance = mean_square_plane_distance(
        floating,
        reference,
        reference_normals,
        &rotation,
        &translation,
        filter_options,
    )
    .unwrap_or(f64::MAX);
    let mut iterations = 0;
    let mut active_pair_count = 0;

    for iteration in 1..=iteration_limit {
        let pairs = nearest_point_plane_pairs(
            floating,
            reference,
            reference_normals,
            &rotation,
            &translation,
            filter_options,
        )?;
        active_pair_count = pairs.len();
        let center = pair_center(&pairs);
        let (delta_rotation, delta_translation) = point_to_plane_amendment(&pairs, mode)?;
        rotation = delta_rotation * rotation;
        translation =
            delta_rotation * translation + delta_translation - delta_rotation * center + center;
        iterations = iteration;

        let current_distance = mean_square_plane_distance(
            floating,
            reference,
            reference_normals,
            &rotation,
            &translation,
            filter_options,
        )
        .ok_or_else(|| "point-to-plane ICP produced no active pairs".to_string())?;
        if current_distance + tolerance < best_distance {
            best_rotation = rotation;
            best_translation = translation;
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            best_rotation = rotation;
            best_translation = translation;
            best_distance = current_distance;
            break;
        } else {
            break;
        }

        if best_distance <= tolerance {
            break;
        }
    }

    Ok(result_from_parts(
        best_rotation,
        best_translation,
        iterations,
        best_distance,
        active_pair_count,
    ))
}

pub(super) fn validate_points(name: &str, points: &[[f64; 3]]) -> Result<(), String> {
    if points.is_empty() {
        return Err(format!("{name} point cloud must not be empty"));
    }
    if points
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(format!("{name} point cloud coordinates must be finite"));
    }
    Ok(())
}

pub(super) fn validate_normals(reference: &[[f64; 3]], normals: &[[f64; 3]]) -> Result<(), String> {
    if normals.len() != reference.len() {
        return Err("reference_normals length must match reference point count".to_string());
    }
    if normals
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("reference normals must be finite".to_string());
    }
    if normals
        .iter()
        .any(|normal| vector(*normal).norm_squared() <= f64::EPSILON)
    {
        return Err("reference normals must be non-zero".to_string());
    }
    Ok(())
}

fn validate_filter_options(
    floating: &[[f64; 3]],
    options: IcpPairFilterOptions<'_>,
) -> Result<(), String> {
    if let Some(normals) = options.floating_normals {
        if normals.len() != floating.len() {
            return Err("floating_normals length must match floating point count".to_string());
        }
        validate_normals(floating, normals)?;
    } else if options.cos_threshold.is_some() {
        return Err("floating_normals are required when cos_threshold is set".to_string());
    }
    if let Some(distance) = options.max_pair_distance {
        if !distance.is_finite() || distance <= 0.0 {
            return Err("max_pair_distance must be a positive finite distance".to_string());
        }
    }
    if let Some(threshold) = options.cos_threshold {
        if !threshold.is_finite() || !(-1.0..=1.0).contains(&threshold) {
            return Err("cos_threshold must be finite and in [-1, 1]".to_string());
        }
    }
    if let Some(factor) = options.far_dist_factor {
        if !factor.is_finite() || factor <= 0.0 {
            return Err("far_dist_factor must be a positive finite factor".to_string());
        }
    }
    Ok(())
}

fn nearest_point_pairs(
    floating: &[[f64; 3]],
    reference: &[[f64; 3]],
    rotation: &Matrix3<f64>,
    translation: &Vector3<f64>,
) -> Result<Vec<(Vector3<f64>, Vector3<f64>)>, String> {
    if reference.is_empty() {
        return Err("reference point cloud must not be empty".to_string());
    }
    // BVH over the (static) reference cloud — built once, queried per floating
    // point in O(log m) instead of the O(n*m) brute scan. par_iter preserves
    // order and the lowest-index tie-break matches the brute scan exactly.
    let reference_tris: Vec<[[f64; 3]; 3]> = reference.iter().map(|p| [*p, *p, *p]).collect();
    let bvh = build_flat_bvh(&reference_tris, 16);
    floating
        .par_iter()
        .map(|point| {
            let transformed = rotation * vector(*point) + translation;
            let query = [transformed.x, transformed.y, transformed.z];
            let index = nearest_point_in_cloud(query, &bvh, reference)
                .ok_or_else(|| "reference point cloud must not be empty".to_string())?;
            Ok((transformed, vector(reference[index])))
        })
        .collect()
}

fn nearest_point_plane_pairs(
    floating: &[[f64; 3]],
    reference: &[[f64; 3]],
    reference_normals: &[[f64; 3]],
    rotation: &Matrix3<f64>,
    translation: &Vector3<f64>,
    filter_options: IcpPairFilterOptions<'_>,
) -> Result<Vec<PointPlanePair>, String> {
    let mut pairs = Vec::with_capacity(floating.len());
    let max_pair_distance_sq = filter_options
        .max_pair_distance
        .map(|distance| distance * distance);
    if reference.is_empty() {
        return Err("reference point cloud must not be empty".to_string());
    }
    let reference_tris: Vec<[[f64; 3]; 3]> = reference.iter().map(|p| [*p, *p, *p]).collect();
    let bvh = build_flat_bvh(&reference_tris, 16);
    for (point_index, point) in floating.iter().enumerate() {
        let transformed = rotation * vector(*point) + translation;
        let query = [transformed.x, transformed.y, transformed.z];
        let index = nearest_point_in_cloud(query, &bvh, reference)
            .ok_or_else(|| "reference point cloud must not be empty".to_string())?;
        let best_distance = (transformed - vector(reference[index])).norm_squared();
        if let Some(max_distance_sq) = max_pair_distance_sq {
            if best_distance > max_distance_sq {
                continue;
            }
        }
        let reference_normal = vector(reference_normals[index]).normalize();
        if let Some(cos_threshold) = filter_options.cos_threshold {
            let floating_normals = filter_options.floating_normals.ok_or_else(|| {
                "floating_normals are required when cos_threshold is set".to_string()
            })?;
            let floating_normal = (rotation * vector(floating_normals[point_index])).normalize();
            if floating_normal.dot(&reference_normal) < cos_threshold {
                continue;
            }
        }
        let reference_point = vector(reference[index]);
        if filter_options.mutual_closest
            && !is_mutual_closest_source(
                point_index,
                reference_point,
                floating,
                rotation,
                translation,
            )
        {
            continue;
        }
        pairs.push(PointPlanePair {
            floating: transformed,
            reference: reference_point,
            reference_normal,
            distance_sq: best_distance,
        });
    }
    apply_far_distance_filter(&mut pairs, filter_options.far_dist_factor);
    Ok(pairs)
}

fn is_mutual_closest_source(
    point_index: usize,
    reference_point: Vector3<f64>,
    floating: &[[f64; 3]],
    rotation: &Matrix3<f64>,
    translation: &Vector3<f64>,
) -> bool {
    let mut best_index = None;
    let mut best_distance = f64::MAX;
    for (candidate_index, point) in floating.iter().enumerate() {
        let transformed = rotation * vector(*point) + translation;
        let distance = (transformed - reference_point).norm_squared();
        if distance < best_distance {
            best_distance = distance;
            best_index = Some(candidate_index);
        }
    }
    best_index == Some(point_index)
}

fn apply_far_distance_filter(pairs: &mut Vec<PointPlanePair>, far_dist_factor: Option<f64>) {
    let Some(factor) = far_dist_factor else {
        return;
    };
    for _ in 0..3 {
        if pairs.is_empty() {
            return;
        }
        let rms =
            (pairs.iter().map(|pair| pair.distance_sq).sum::<f64>() / pairs.len() as f64).sqrt();
        let max_distance_sq = (factor * rms) * (factor * rms);
        let before = pairs.len();
        pairs.retain(|pair| pair.distance_sq <= max_distance_sq);
        if pairs.len() == before {
            return;
        }
    }
}

pub(super) fn point_to_point_amendment(
    pairs: &[(Vector3<f64>, Vector3<f64>)],
    mode: IcpMode,
) -> Result<(Matrix3<f64>, Vector3<f64>), String> {
    if pairs.is_empty() {
        return Err("ICP requires at least one active pair".to_string());
    }
    let (floating_center, reference_center) = pair_centroids(pairs);
    if mode == IcpMode::TranslationOnly {
        return Ok((Matrix3::identity(), reference_center - floating_center));
    }

    let mut covariance = Matrix3::<f64>::zeros();
    for (floating, reference) in pairs {
        covariance += (floating - floating_center) * (reference - reference_center).transpose();
    }
    let svd = covariance.svd(true, true);
    let u = svd
        .u
        .ok_or_else(|| "ICP rigid SVD did not return U".to_string())?;
    let v_t = svd
        .v_t
        .ok_or_else(|| "ICP rigid SVD did not return Vt".to_string())?;
    let v = v_t.transpose();
    let mut handedness = Matrix3::<f64>::identity();
    if (v * u.transpose()).determinant() < 0.0 {
        handedness[(2, 2)] = -1.0;
    }
    let rotation = v * handedness * u.transpose();
    let translation = reference_center - rotation * floating_center;
    Ok((rotation, translation))
}

pub(super) fn point_to_plane_amendment(
    pairs: &[PointPlanePair],
    mode: IcpMode,
) -> Result<(Matrix3<f64>, Vector3<f64>), String> {
    if pairs.is_empty() {
        return Err("point-to-plane ICP requires at least one active pair".to_string());
    }
    let center = pair_center(pairs);
    if mode == IcpMode::TranslationOnly {
        let mut matrix = Matrix3::<f64>::zeros();
        let mut rhs = Vector3::<f64>::zeros();
        for pair in pairs {
            let normal = pair.reference_normal;
            let residual = (pair.reference - pair.floating).dot(&normal);
            matrix += normal * normal.transpose();
            rhs += normal * residual;
        }
        return Ok((
            Matrix3::identity(),
            solve_3x3(matrix, rhs, "point-to-plane translation solve")?,
        ));
    }

    let mut matrix = SMatrix::<f64, 6, 6>::zeros();
    let mut rhs = SVector::<f64, 6>::zeros();
    for pair in pairs {
        let source = pair.floating - center;
        let target = pair.reference - center;
        let normal = pair.reference_normal;
        let rotation_terms = source.cross(&normal);
        let coefficients = SVector::<f64, 6>::from_row_slice(&[
            rotation_terms.x,
            rotation_terms.y,
            rotation_terms.z,
            normal.x,
            normal.y,
            normal.z,
        ]);
        let residual = target.dot(&normal) - source.dot(&normal);
        matrix += coefficients * coefficients.transpose();
        rhs += coefficients * residual;
    }

    let solution = solve_6x6(matrix, rhs, "point-to-plane rigid solve")?;
    let mut angles = Vector3::new(solution[0], solution[1], solution[2]);
    let mut translation = Vector3::new(solution[3], solution[4], solution[5]);
    const P2PL_ANGLE_LIMIT: f64 = std::f64::consts::PI / 6.0;
    let angle = angles.norm();
    if angle > P2PL_ANGLE_LIMIT {
        angles *= P2PL_ANGLE_LIMIT / angle;
        translation = recompute_point_to_plane_translation(pairs, center, angles)?;
    }
    Ok((rotation_from_axis_vector(angles), translation))
}

pub(super) fn pair_center(pairs: &[PointPlanePair]) -> Vector3<f64> {
    let mut center = Vector3::<f64>::zeros();
    for pair in pairs {
        center += pair.floating;
        center += pair.reference;
    }
    center / (pairs.len() as f64 * 2.0)
}

fn recompute_point_to_plane_translation(
    pairs: &[PointPlanePair],
    center: Vector3<f64>,
    angles: Vector3<f64>,
) -> Result<Vector3<f64>, String> {
    let mut matrix = Matrix3::<f64>::zeros();
    let mut rhs = Vector3::<f64>::zeros();
    for pair in pairs {
        let source = pair.floating - center;
        let target = pair.reference - center;
        let normal = pair.reference_normal;
        let rotation_terms = source.cross(&normal);
        let residual = target.dot(&normal) - source.dot(&normal) - rotation_terms.dot(&angles);
        matrix += normal * normal.transpose();
        rhs += normal * residual;
    }
    solve_3x3(
        matrix,
        rhs,
        "point-to-plane limited-angle translation solve",
    )
}

fn solve_3x3(
    matrix: Matrix3<f64>,
    rhs: Vector3<f64>,
    context: &str,
) -> Result<Vector3<f64>, String> {
    matrix
        .svd(true, true)
        .solve(&rhs, 1e-12)
        .map_err(|_| format!("{context} failed"))
}

fn solve_6x6(
    matrix: SMatrix<f64, 6, 6>,
    rhs: SVector<f64, 6>,
    context: &str,
) -> Result<SVector<f64, 6>, String> {
    matrix
        .svd(true, true)
        .solve(&rhs, 1e-12)
        .map_err(|_| format!("{context} failed"))
}

fn rotation_from_axis_vector(angles: Vector3<f64>) -> Matrix3<f64> {
    let angle = angles.norm();
    let skew = skew_symmetric(angles);
    if angle <= 1e-12 {
        Matrix3::identity() + skew
    } else {
        let axis_skew = skew / angle;
        Matrix3::identity()
            + axis_skew * angle.sin()
            + (axis_skew * axis_skew) * (1.0 - angle.cos())
    }
}

fn skew_symmetric(vector: Vector3<f64>) -> Matrix3<f64> {
    Matrix3::new(
        0.0, -vector.z, vector.y, vector.z, 0.0, -vector.x, -vector.y, vector.x, 0.0,
    )
}

fn pair_centroids(pairs: &[(Vector3<f64>, Vector3<f64>)]) -> (Vector3<f64>, Vector3<f64>) {
    let mut floating_sum = Vector3::<f64>::zeros();
    let mut reference_sum = Vector3::<f64>::zeros();
    for (floating, reference) in pairs {
        floating_sum += floating;
        reference_sum += reference;
    }
    let weight = pairs.len() as f64;
    (floating_sum / weight, reference_sum / weight)
}

fn mean_square_distance(
    floating: &[[f64; 3]],
    reference: &[[f64; 3]],
    rotation: &Matrix3<f64>,
    translation: &Vector3<f64>,
) -> Option<f64> {
    let pairs = nearest_point_pairs(floating, reference, rotation, translation).ok()?;
    if pairs.is_empty() {
        return None;
    }
    let distance_sum = pairs
        .iter()
        .map(|(floating, reference)| (floating - reference).norm_squared())
        .sum::<f64>();
    Some(distance_sum / pairs.len() as f64)
}

fn mean_square_plane_distance(
    floating: &[[f64; 3]],
    reference: &[[f64; 3]],
    reference_normals: &[[f64; 3]],
    rotation: &Matrix3<f64>,
    translation: &Vector3<f64>,
    filter_options: IcpPairFilterOptions<'_>,
) -> Option<f64> {
    let pairs = nearest_point_plane_pairs(
        floating,
        reference,
        reference_normals,
        rotation,
        translation,
        filter_options,
    )
    .ok()?;
    if pairs.is_empty() {
        return None;
    }
    let distance_sum = pairs
        .iter()
        .map(|pair| {
            let plane_distance = (pair.floating - pair.reference).dot(&pair.reference_normal);
            plane_distance * plane_distance
        })
        .sum::<f64>();
    Some(distance_sum / pairs.len() as f64)
}

fn result_from_parts(
    rotation: Matrix3<f64>,
    translation: Vector3<f64>,
    iterations: usize,
    mean_square_distance: f64,
    active_pair_count: usize,
) -> IcpRegistrationResult {
    let rotation = matrix_to_array(rotation);
    let translation = [translation.x, translation.y, translation.z];
    IcpRegistrationResult {
        rotation,
        translation,
        transform: [
            [
                rotation[0][0],
                rotation[0][1],
                rotation[0][2],
                translation[0],
            ],
            [
                rotation[1][0],
                rotation[1][1],
                rotation[1][2],
                translation[1],
            ],
            [
                rotation[2][0],
                rotation[2][1],
                rotation[2][2],
                translation[2],
            ],
            [0.0, 0.0, 0.0, 1.0],
        ],
        iterations,
        mean_square_distance,
        active_pair_count,
    }
}

pub(super) fn matrix_to_array(matrix: Matrix3<f64>) -> [[f64; 3]; 3] {
    [
        [matrix[(0, 0)], matrix[(0, 1)], matrix[(0, 2)]],
        [matrix[(1, 0)], matrix[(1, 1)], matrix[(1, 2)]],
        [matrix[(2, 0)], matrix[(2, 1)], matrix[(2, 2)]],
    ]
}

pub(super) fn vector(point: [f64; 3]) -> Vector3<f64> {
    Vector3::new(point[0], point[1], point[2])
}

#[cfg(test)]
mod tests;
