use nalgebra::{Matrix3, Vector3};

use crate::spatial::{build_flat_bvh, nearest_point_in_cloud, FlatBvh};

mod all_object;
mod cascade;
pub use all_object::{
    multiway_all_object_combined_icp, multiway_all_object_point_to_plane_icp,
    multiway_all_object_point_to_point_icp,
};
pub use cascade::{
    multiway_aabb_cascade_combined_icp, multiway_aabb_cascade_point_to_plane_icp,
    multiway_aabb_cascade_point_to_point_icp, multiway_sequential_cascade_combined_icp,
    multiway_sequential_cascade_point_to_plane_icp, multiway_sequential_cascade_point_to_point_icp,
};

use super::{
    matrix_to_array, pair_center, point_to_plane_amendment, point_to_point_amendment,
    validate_normals, validate_points, vector, IcpMode, PointPlanePair,
};

#[derive(Debug, Clone, PartialEq)]
pub struct MultiwayIcpObjectResult {
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub transform: [[f64; 4]; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiwayIcpRegistrationResult {
    pub transforms: Vec<MultiwayIcpObjectResult>,
    pub iterations: usize,
    pub mean_square_distance: f64,
    pub active_pair_count: usize,
    pub fixed_object_index: usize,
}

#[derive(Debug, Clone)]
struct ObjectTransform {
    rotation: Matrix3<f64>,
    translation: Vector3<f64>,
}

#[derive(Debug, Clone, Copy)]
struct DirectedPair {
    source_object: usize,
    target_object: usize,
    source: Vector3<f64>,
    target: Vector3<f64>,
    distance_sq: f64,
}

#[derive(Debug, Clone, Copy)]
struct DirectedPlanePair {
    source_object: usize,
    target_object: usize,
    source: Vector3<f64>,
    target: Vector3<f64>,
    source_normal: Vector3<f64>,
    target_normal: Vector3<f64>,
    distance_sq: f64,
}

pub fn multiway_point_to_point_icp(
    objects: &[Vec<[f64; 3]>],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_point_iterations(
            objects,
            initial_transforms,
            iteration_limit,
            tolerance,
            mode,
            fixed_index,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_point_to_plane_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            initial_transforms,
            iteration_limit,
            tolerance,
            mode,
            fixed_index,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_combined_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let iteration_limit = max_iterations.max(3);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (point_transforms, point_iterations, _, _) = run_point_to_point_iterations(
        objects,
        initial_transforms,
        2,
        tolerance,
        mode,
        fixed_index,
    )?;
    let (transforms, plane_iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            point_transforms,
            iteration_limit - 2,
            tolerance,
            mode,
            fixed_index,
        )?;

    Ok(result_from_transforms(
        transforms,
        point_iterations + plane_iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

fn run_point_to_plane_iterations(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    mut transforms: Vec<ObjectTransform>,
    iteration_limit: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_index: usize,
) -> Result<(Vec<ObjectTransform>, usize, f64, usize), String> {
    let mut best_transforms = transforms.clone();
    let initial_pairs = directed_nearest_plane_pairs(objects, normals, &transforms)?;
    let mut best_distance = mean_square_plane_distance(&initial_pairs)
        .ok_or_else(|| "multiway point-to-plane ICP produced no active pairs".to_string())?;
    let mut active_pair_count = initial_pairs.len();
    let mut iterations = 0;

    for iteration in 1..=iteration_limit {
        let pairs = directed_nearest_plane_pairs(objects, normals, &transforms)?;
        active_pair_count = pairs.len();
        let deltas = object_plane_amendments(objects.len(), &pairs, mode)?;
        let candidate = apply_independent_updates(&transforms, &deltas, fixed_index);
        let candidate_pairs = directed_nearest_plane_pairs(objects, normals, &candidate)?;
        let current_distance = mean_square_plane_distance(&candidate_pairs)
            .ok_or_else(|| "multiway point-to-plane ICP produced no active pairs".to_string())?;
        iterations = iteration;

        if current_distance + tolerance < best_distance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
            break;
        } else {
            break;
        }

        if best_distance <= tolerance {
            break;
        }
    }

    Ok((
        best_transforms,
        iterations,
        best_distance,
        active_pair_count,
    ))
}

fn run_point_to_point_iterations(
    objects: &[Vec<[f64; 3]>],
    mut transforms: Vec<ObjectTransform>,
    iteration_limit: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_index: usize,
) -> Result<(Vec<ObjectTransform>, usize, f64, usize), String> {
    let mut best_transforms = transforms.clone();
    let initial_pairs = directed_nearest_pairs(objects, &transforms)?;
    let mut best_distance = mean_square_pair_distance(&initial_pairs)
        .ok_or_else(|| "multiway ICP produced no active pairs".to_string())?;
    let mut active_pair_count = initial_pairs.len();
    let mut iterations = 0;

    for iteration in 1..=iteration_limit {
        let pairs = directed_nearest_pairs(objects, &transforms)?;
        active_pair_count = pairs.len();
        let deltas = object_amendments(objects.len(), &pairs, mode)?;
        let candidate = apply_independent_updates(&transforms, &deltas, fixed_index);
        let candidate_pairs = directed_nearest_pairs(objects, &candidate)?;
        let current_distance = mean_square_pair_distance(&candidate_pairs)
            .ok_or_else(|| "multiway ICP produced no active pairs".to_string())?;
        iterations = iteration;

        if current_distance + tolerance < best_distance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
            break;
        } else {
            break;
        }

        if best_distance <= tolerance {
            break;
        }
    }

    Ok((
        best_transforms,
        iterations,
        best_distance,
        active_pair_count,
    ))
}

fn result_from_transforms(
    transforms: Vec<ObjectTransform>,
    iterations: usize,
    mean_square_distance: f64,
    active_pair_count: usize,
    fixed_object_index: usize,
) -> MultiwayIcpRegistrationResult {
    MultiwayIcpRegistrationResult {
        transforms: transforms
            .into_iter()
            .map(MultiwayIcpObjectResult::from_transform)
            .collect(),
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_object_index,
    }
}

fn validate_multiway_inputs(
    objects: &[Vec<[f64; 3]>],
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<usize, String> {
    if objects.len() < 2 {
        return Err("multiway ICP requires at least two point clouds".to_string());
    }
    let fixed_index = fixed_object_index.unwrap_or(objects.len() - 1);
    if fixed_index >= objects.len() {
        return Err("fixed_object_index must refer to an input point cloud".to_string());
    }
    let minimum_points = match mode {
        IcpMode::AnyRigidXf => 3,
        IcpMode::TranslationOnly => 1,
    };
    for (index, points) in objects.iter().enumerate() {
        validate_points(&format!("objects[{index}]"), points)?;
        if points.len() < minimum_points {
            return Err(format!(
                "multiway ICP mode {mode:?} requires at least {minimum_points} points in each cloud"
            ));
        }
    }
    Ok(fixed_index)
}

fn validate_multiway_normals(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
) -> Result<(), String> {
    if normals.len() != objects.len() {
        return Err("multiway ICP normals must match point cloud count".to_string());
    }
    for (index, (points, object_normals)) in objects.iter().zip(normals.iter()).enumerate() {
        validate_normals(points, object_normals)
            .map_err(|error| format!("objects[{index}] {error}"))?;
    }
    Ok(())
}

fn directed_nearest_pairs(
    objects: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
) -> Result<Vec<DirectedPair>, String> {
    let transformed = transformed_objects(objects, transforms);
    let mut pairs = Vec::new();
    for (source_index, source_points) in transformed.iter().enumerate() {
        pairs.reserve(source_points.len() * (objects.len() - 1));
        for (target_index, target_points) in transformed.iter().enumerate() {
            if source_index == target_index {
                continue;
            }
            let (target_arrays, bvh) = build_target_index(target_points);
            for source in source_points {
                let query = [source.x, source.y, source.z];
                let target_point_index = nearest_point_in_cloud(query, &bvh, &target_arrays)
                    .ok_or_else(|| "target point cloud must not be empty".to_string())?;
                let target = target_points[target_point_index];
                let distance_sq = (source - target).norm_squared();
                pairs.push(DirectedPair {
                    source_object: source_index,
                    target_object: target_index,
                    source: *source,
                    target,
                    distance_sq,
                });
            }
        }
    }
    Ok(pairs)
}

fn directed_nearest_plane_pairs(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
) -> Result<Vec<DirectedPlanePair>, String> {
    let transformed = transformed_objects(objects, transforms);
    let transformed_normals = transformed_normals(normals, transforms);
    let mut pairs = Vec::new();
    for (source_index, source_points) in transformed.iter().enumerate() {
        pairs.reserve(source_points.len() * (objects.len() - 1));
        for (target_index, target_points) in transformed.iter().enumerate() {
            if source_index == target_index {
                continue;
            }
            let (target_arrays, bvh) = build_target_index(target_points);
            for (source_point_index, source) in source_points.iter().enumerate() {
                let query = [source.x, source.y, source.z];
                let target_point_index = nearest_point_in_cloud(query, &bvh, &target_arrays)
                    .ok_or_else(|| "target point cloud must not be empty".to_string())?;
                let target = target_points[target_point_index];
                let distance_sq = (source - target).norm_squared();
                pairs.push(DirectedPlanePair {
                    source_object: source_index,
                    target_object: target_index,
                    source: *source,
                    target,
                    source_normal: transformed_normals[source_index][source_point_index],
                    target_normal: transformed_normals[target_index][target_point_index],
                    distance_sq,
                });
            }
        }
    }
    Ok(pairs)
}

fn transformed_objects(
    objects: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
) -> Vec<Vec<Vector3<f64>>> {
    objects
        .iter()
        .zip(transforms.iter())
        .map(|(points, transform)| {
            points
                .iter()
                .map(|point| transform.rotation * vector(*point) + transform.translation)
                .collect()
        })
        .collect()
}

fn transformed_normals(
    normals: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
) -> Vec<Vec<Vector3<f64>>> {
    normals
        .iter()
        .zip(transforms.iter())
        .map(|(object_normals, transform)| {
            object_normals
                .iter()
                .map(|normal| (transform.rotation * vector(*normal)).normalize())
                .collect()
        })
        .collect()
}

/// Build a BVH point-index over a (transformed) target object's points once, so
/// every source point queries it in O(log m) instead of an O(m) brute scan. The
/// BVH face index addresses both the returned `[f64;3]` array and the caller's
/// parallel `Vector3` array. Lowest-index tie-break in `nearest_point_in_cloud`
/// matches the previous brute scan exactly.
fn build_target_index(targets: &[Vector3<f64>]) -> (Vec<[f64; 3]>, FlatBvh) {
    let arrays: Vec<[f64; 3]> = targets.iter().map(|v| [v.x, v.y, v.z]).collect();
    let triangles: Vec<[[f64; 3]; 3]> = arrays.iter().map(|p| [*p, *p, *p]).collect();
    let bvh = build_flat_bvh(&triangles, 16);
    (arrays, bvh)
}

fn object_amendments(
    object_count: usize,
    directed_pairs: &[DirectedPair],
    mode: IcpMode,
) -> Result<Vec<(Matrix3<f64>, Vector3<f64>)>, String> {
    let mut amendments = Vec::with_capacity(object_count);
    for object_index in 0..object_count {
        let pairs = object_midpoint_pairs(object_index, directed_pairs);
        amendments.push(point_to_point_amendment(&pairs, mode)?);
    }
    Ok(amendments)
}

fn object_plane_amendments(
    object_count: usize,
    directed_pairs: &[DirectedPlanePair],
    mode: IcpMode,
) -> Result<Vec<(Matrix3<f64>, Vector3<f64>)>, String> {
    let mut amendments = Vec::with_capacity(object_count);
    for object_index in 0..object_count {
        let pairs = object_midpoint_plane_pairs(object_index, directed_pairs);
        let center = pair_center(&pairs);
        let (rotation, translation) = point_to_plane_amendment(&pairs, mode)?;
        amendments.push((rotation, translation - rotation * center + center));
    }
    Ok(amendments)
}

fn object_midpoint_pairs(
    object_index: usize,
    directed_pairs: &[DirectedPair],
) -> Vec<(Vector3<f64>, Vector3<f64>)> {
    let mut pairs = Vec::new();
    for pair in directed_pairs {
        let midpoint = (pair.source + pair.target) * 0.5;
        if pair.source_object == object_index {
            pairs.push((pair.source, midpoint));
        }
        if pair.target_object == object_index {
            pairs.push((pair.target, midpoint));
        }
    }
    pairs
}

fn object_midpoint_plane_pairs(
    object_index: usize,
    directed_pairs: &[DirectedPlanePair],
) -> Vec<PointPlanePair> {
    let mut pairs = Vec::new();
    for pair in directed_pairs {
        let midpoint = (pair.source + pair.target) * 0.5;
        if pair.source_object == object_index {
            pairs.push(PointPlanePair {
                floating: pair.source,
                reference: midpoint,
                reference_normal: pair.target_normal,
                distance_sq: pair.distance_sq,
            });
        }
        if pair.target_object == object_index {
            pairs.push(PointPlanePair {
                floating: pair.target,
                reference: midpoint,
                reference_normal: pair.source_normal,
                distance_sq: pair.distance_sq,
            });
        }
    }
    pairs
}

fn apply_independent_updates(
    transforms: &[ObjectTransform],
    deltas: &[(Matrix3<f64>, Vector3<f64>)],
    fixed_index: usize,
) -> Vec<ObjectTransform> {
    let (fixed_rotation, fixed_translation) = deltas[fixed_index];
    let fixed_inverse_rotation = fixed_rotation.transpose();
    let fixed_inverse_translation = -(fixed_inverse_rotation * fixed_translation);
    transforms
        .iter()
        .zip(deltas.iter())
        .enumerate()
        .map(
            |(index, (transform, (delta_rotation, delta_translation)))| {
                if index == fixed_index {
                    return transform.clone();
                }
                let updated_rotation = delta_rotation * transform.rotation;
                let updated_translation =
                    delta_rotation * transform.translation + delta_translation;
                ObjectTransform {
                    rotation: fixed_inverse_rotation * updated_rotation,
                    translation: fixed_inverse_rotation * updated_translation
                        + fixed_inverse_translation,
                }
            },
        )
        .collect()
}

fn mean_square_pair_distance(pairs: &[DirectedPair]) -> Option<f64> {
    if pairs.is_empty() {
        return None;
    }
    Some(pairs.iter().map(|pair| pair.distance_sq).sum::<f64>() / pairs.len() as f64)
}

fn mean_square_plane_distance(pairs: &[DirectedPlanePair]) -> Option<f64> {
    if pairs.is_empty() {
        return None;
    }
    let residual_sum = pairs
        .iter()
        .map(|pair| {
            let plane_distance = (pair.target - pair.source).dot(&pair.target_normal);
            plane_distance * plane_distance
        })
        .sum::<f64>();
    Some(residual_sum / pairs.len() as f64)
}

impl ObjectTransform {
    fn identity() -> Self {
        Self {
            rotation: Matrix3::identity(),
            translation: Vector3::zeros(),
        }
    }
}

impl MultiwayIcpObjectResult {
    fn from_transform(transform: ObjectTransform) -> Self {
        let rotation = matrix_to_array(transform.rotation);
        let translation = [
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ];
        Self {
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
        }
    }
}
