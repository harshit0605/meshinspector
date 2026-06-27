use nalgebra::Vector3;

mod indexer;

use indexer::{build_cascade_indexer, CascadeGrouping, CascadeIndexer};

use super::{
    all_object, mean_square_pair_distance, mean_square_plane_distance, result_from_transforms,
    validate_multiway_inputs, validate_multiway_normals, DirectedPair, DirectedPlanePair,
    MultiwayIcpRegistrationResult, ObjectTransform,
};
use crate::registration::IcpMode;

pub fn multiway_sequential_cascade_point_to_point_icp(
    objects: &[Vec<[f64; 3]>],
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    let max_group_size = validate_max_group_size(max_group_size)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_point_iterations(
            objects,
            initial_transforms,
            max_group_size,
            iteration_limit,
            tolerance,
            fixed_index,
            CascadeGrouping::Sequential,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_sequential_cascade_point_to_plane_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let max_group_size = validate_max_group_size(max_group_size)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            initial_transforms,
            max_group_size,
            iteration_limit,
            tolerance,
            fixed_index,
            CascadeGrouping::Sequential,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_sequential_cascade_combined_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let max_group_size = validate_max_group_size(max_group_size)?;
    let iteration_limit = max_iterations.max(3);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (point_transforms, point_iterations, _, _) = run_point_to_point_iterations(
        objects,
        initial_transforms,
        max_group_size,
        2,
        tolerance,
        fixed_index,
        CascadeGrouping::Sequential,
    )?;
    let (transforms, plane_iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            point_transforms,
            max_group_size,
            iteration_limit - 2,
            tolerance,
            fixed_index,
            CascadeGrouping::Sequential,
        )?;

    Ok(result_from_transforms(
        transforms,
        point_iterations + plane_iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_aabb_cascade_point_to_point_icp(
    objects: &[Vec<[f64; 3]>],
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    let max_group_size = validate_max_group_size(max_group_size)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_point_iterations(
            objects,
            initial_transforms,
            max_group_size,
            iteration_limit,
            tolerance,
            fixed_index,
            CascadeGrouping::AabbTreeBased,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_aabb_cascade_point_to_plane_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let max_group_size = validate_max_group_size(max_group_size)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            initial_transforms,
            max_group_size,
            iteration_limit,
            tolerance,
            fixed_index,
            CascadeGrouping::AabbTreeBased,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_aabb_cascade_combined_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let max_group_size = validate_max_group_size(max_group_size)?;
    let iteration_limit = max_iterations.max(3);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (point_transforms, point_iterations, _, _) = run_point_to_point_iterations(
        objects,
        initial_transforms,
        max_group_size,
        2,
        tolerance,
        fixed_index,
        CascadeGrouping::AabbTreeBased,
    )?;
    let (transforms, plane_iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            point_transforms,
            max_group_size,
            iteration_limit - 2,
            tolerance,
            fixed_index,
            CascadeGrouping::AabbTreeBased,
        )?;

    Ok(result_from_transforms(
        transforms,
        point_iterations + plane_iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

fn validate_max_group_size(max_group_size: usize) -> Result<usize, String> {
    if max_group_size < 2 {
        return Err("cascade ICP requires max_group_size greater than one".to_string());
    }
    Ok(max_group_size)
}

fn run_point_to_point_iterations(
    objects: &[Vec<[f64; 3]>],
    mut transforms: Vec<ObjectTransform>,
    max_group_size: usize,
    iteration_limit: usize,
    tolerance: f64,
    fixed_index: usize,
    grouping: CascadeGrouping,
) -> Result<(Vec<ObjectTransform>, usize, f64, usize), String> {
    let mut best_transforms = normalize_to_fixed(transforms.clone(), fixed_index);
    let initial_pairs = cascade_pairs(objects, &best_transforms, max_group_size, grouping)?;
    let mut best_distance = mean_square_pair_distance(&initial_pairs)
        .ok_or_else(|| "multiway sequential cascade ICP produced no active pairs".to_string())?;
    let mut active_pair_count = initial_pairs.len();
    let mut iterations = 0;

    for iteration in 1..=iteration_limit {
        let candidate =
            apply_point_to_point_cascade_pass(objects, &transforms, max_group_size, grouping)?;
        let candidate = normalize_to_fixed(candidate, fixed_index);
        let candidate_pairs = cascade_pairs(objects, &candidate, max_group_size, grouping)?;
        let current_distance = mean_square_pair_distance(&candidate_pairs).ok_or_else(|| {
            "multiway sequential cascade ICP produced no active pairs".to_string()
        })?;
        iterations = iteration;
        active_pair_count = candidate_pairs.len();

        if current_distance + tolerance < best_distance {
            transforms = candidate.clone();
            best_transforms = candidate;
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            best_transforms = candidate;
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

fn run_point_to_plane_iterations(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    mut transforms: Vec<ObjectTransform>,
    max_group_size: usize,
    iteration_limit: usize,
    tolerance: f64,
    fixed_index: usize,
    grouping: CascadeGrouping,
) -> Result<(Vec<ObjectTransform>, usize, f64, usize), String> {
    let mut best_transforms = normalize_to_fixed(transforms.clone(), fixed_index);
    let initial_pairs =
        cascade_plane_pairs(objects, normals, &best_transforms, max_group_size, grouping)?;
    let mut best_distance = mean_square_plane_distance(&initial_pairs).ok_or_else(|| {
        "multiway sequential cascade point-to-plane ICP produced no active pairs".to_string()
    })?;
    let mut active_pair_count = initial_pairs.len();
    let mut iterations = 0;

    for iteration in 1..=iteration_limit {
        let candidate = apply_point_to_plane_cascade_pass(
            objects,
            normals,
            &transforms,
            max_group_size,
            grouping,
        )?;
        let candidate = normalize_to_fixed(candidate, fixed_index);
        let candidate_pairs =
            cascade_plane_pairs(objects, normals, &candidate, max_group_size, grouping)?;
        let current_distance = mean_square_plane_distance(&candidate_pairs).ok_or_else(|| {
            "multiway sequential cascade point-to-plane ICP produced no active pairs".to_string()
        })?;
        iterations = iteration;
        active_pair_count = candidate_pairs.len();

        if current_distance + tolerance < best_distance {
            transforms = candidate.clone();
            best_transforms = candidate;
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            best_transforms = candidate;
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

fn apply_point_to_point_cascade_pass(
    objects: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    max_group_size: usize,
    grouping: CascadeGrouping,
) -> Result<Vec<ObjectTransform>, String> {
    let mut updated = transforms.to_vec();
    let indexer = build_cascade_indexer(grouping, objects, max_group_size)?;
    for layer in 0..indexer.num_layers() {
        for hypergroup in 0..indexer.num_elements(layer + 1) {
            let nodes = indexer.element_nodes(layer + 1, hypergroup);
            if nodes.len() <= 1 {
                continue;
            }
            let mut local_pairs = Vec::new();
            for (local_source, source_node) in nodes.iter().copied().enumerate() {
                for (local_target, target_node) in nodes.iter().copied().enumerate() {
                    if source_node == target_node {
                        continue;
                    }
                    append_node_pairs(
                        objects,
                        &updated,
                        &indexer.element_leaves(layer, source_node),
                        &indexer.element_leaves(layer, target_node),
                        local_source,
                        local_target,
                        &mut local_pairs,
                    )?;
                }
            }
            apply_node_deltas(
                &mut updated,
                indexer.as_ref(),
                layer,
                &nodes,
                local_pairs,
                None,
            )?;
        }
    }
    Ok(updated)
}

fn apply_point_to_plane_cascade_pass(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    max_group_size: usize,
    grouping: CascadeGrouping,
) -> Result<Vec<ObjectTransform>, String> {
    let mut updated = transforms.to_vec();
    let indexer = build_cascade_indexer(grouping, objects, max_group_size)?;
    for layer in 0..indexer.num_layers() {
        for hypergroup in 0..indexer.num_elements(layer + 1) {
            let nodes = indexer.element_nodes(layer + 1, hypergroup);
            if nodes.len() <= 1 {
                continue;
            }
            let mut local_pairs = Vec::new();
            for (local_source, source_node) in nodes.iter().copied().enumerate() {
                for (local_target, target_node) in nodes.iter().copied().enumerate() {
                    if source_node == target_node {
                        continue;
                    }
                    append_node_plane_pairs(
                        objects,
                        normals,
                        &updated,
                        &indexer.element_leaves(layer, source_node),
                        &indexer.element_leaves(layer, target_node),
                        local_source,
                        local_target,
                        &mut local_pairs,
                    )?;
                }
            }
            apply_node_deltas(
                &mut updated,
                indexer.as_ref(),
                layer,
                &nodes,
                Vec::new(),
                Some(local_pairs),
            )?;
        }
    }
    Ok(updated)
}

fn apply_node_deltas(
    transforms: &mut [ObjectTransform],
    indexer: &dyn CascadeIndexer,
    layer: usize,
    nodes: &[usize],
    point_pairs: Vec<DirectedPair>,
    plane_pairs: Option<Vec<DirectedPlanePair>>,
) -> Result<(), String> {
    let local_fixed_index = nodes.len() - 1;
    let deltas = if let Some(plane_pairs) = plane_pairs {
        if plane_pairs.is_empty() {
            vec![ObjectTransform::identity(); nodes.len()]
        } else {
            all_object::solve_point_to_plane_updates(
                nodes.len(),
                local_fixed_index,
                &plane_pairs,
                1e-2,
            )?
        }
    } else if point_pairs.is_empty() {
        vec![ObjectTransform::identity(); nodes.len()]
    } else {
        all_object::solve_point_to_point_updates(
            nodes.len(),
            local_fixed_index,
            &point_pairs,
            1e-2,
        )?
    };

    for (local_index, node) in nodes.iter().copied().enumerate() {
        for leaf in indexer.element_leaves(layer, node) {
            let transform = &transforms[leaf];
            let delta = &deltas[local_index];
            transforms[leaf] = ObjectTransform {
                rotation: delta.rotation * transform.rotation,
                translation: delta.rotation * transform.translation + delta.translation,
            };
        }
    }
    Ok(())
}

fn cascade_pairs(
    objects: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    max_group_size: usize,
    grouping: CascadeGrouping,
) -> Result<Vec<DirectedPair>, String> {
    let indexer = build_cascade_indexer(grouping, objects, max_group_size)?;
    let mut pairs = Vec::new();
    for layer in 0..indexer.num_layers() {
        for hypergroup in 0..indexer.num_elements(layer + 1) {
            let nodes = indexer.element_nodes(layer + 1, hypergroup);
            for (local_source, source_node) in nodes.iter().copied().enumerate() {
                for (local_target, target_node) in nodes.iter().copied().enumerate() {
                    if source_node == target_node {
                        continue;
                    }
                    append_node_pairs(
                        objects,
                        transforms,
                        &indexer.element_leaves(layer, source_node),
                        &indexer.element_leaves(layer, target_node),
                        local_source,
                        local_target,
                        &mut pairs,
                    )?;
                }
            }
        }
    }
    Ok(pairs)
}

fn cascade_plane_pairs(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    max_group_size: usize,
    grouping: CascadeGrouping,
) -> Result<Vec<DirectedPlanePair>, String> {
    let indexer = build_cascade_indexer(grouping, objects, max_group_size)?;
    let mut pairs = Vec::new();
    for layer in 0..indexer.num_layers() {
        for hypergroup in 0..indexer.num_elements(layer + 1) {
            let nodes = indexer.element_nodes(layer + 1, hypergroup);
            for (local_source, source_node) in nodes.iter().copied().enumerate() {
                for (local_target, target_node) in nodes.iter().copied().enumerate() {
                    if source_node == target_node {
                        continue;
                    }
                    append_node_plane_pairs(
                        objects,
                        normals,
                        transforms,
                        &indexer.element_leaves(layer, source_node),
                        &indexer.element_leaves(layer, target_node),
                        local_source,
                        local_target,
                        &mut pairs,
                    )?;
                }
            }
        }
    }
    Ok(pairs)
}

fn append_node_pairs(
    objects: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    source_leaves: &[usize],
    target_leaves: &[usize],
    source_node: usize,
    target_node: usize,
    pairs: &mut Vec<DirectedPair>,
) -> Result<(), String> {
    for source_leaf in source_leaves {
        for source_point in &objects[*source_leaf] {
            let source = transform_point(*source_point, &transforms[*source_leaf]);
            let (target, distance_sq) =
                nearest_transformed_point(source, objects, transforms, target_leaves)?;
            pairs.push(DirectedPair {
                source_object: source_node,
                target_object: target_node,
                source,
                target,
                distance_sq,
            });
        }
    }
    Ok(())
}

fn append_node_plane_pairs(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    source_leaves: &[usize],
    target_leaves: &[usize],
    source_node: usize,
    target_node: usize,
    pairs: &mut Vec<DirectedPlanePair>,
) -> Result<(), String> {
    for source_leaf in source_leaves {
        for (source_point_index, source_point) in objects[*source_leaf].iter().enumerate() {
            let source = transform_point(*source_point, &transforms[*source_leaf]);
            let source_normal = transform_normal(
                normals[*source_leaf][source_point_index],
                &transforms[*source_leaf],
            );
            let (target_leaf, target_point_index, target, distance_sq) =
                nearest_transformed_point_index(source, objects, transforms, target_leaves)?;
            let target_normal = transform_normal(
                normals[target_leaf][target_point_index],
                &transforms[target_leaf],
            );
            pairs.push(DirectedPlanePair {
                source_object: source_node,
                target_object: target_node,
                source,
                target,
                source_normal,
                target_normal,
                distance_sq,
            });
        }
    }
    Ok(())
}

fn nearest_transformed_point(
    source: Vector3<f64>,
    objects: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    target_leaves: &[usize],
) -> Result<(Vector3<f64>, f64), String> {
    nearest_transformed_point_index(source, objects, transforms, target_leaves)
        .map(|(_, _, point, distance_sq)| (point, distance_sq))
}

fn nearest_transformed_point_index(
    source: Vector3<f64>,
    objects: &[Vec<[f64; 3]>],
    transforms: &[ObjectTransform],
    target_leaves: &[usize],
) -> Result<(usize, usize, Vector3<f64>, f64), String> {
    let mut best = None;
    let mut best_distance = f64::MAX;
    for target_leaf in target_leaves {
        for (target_point_index, target_point) in objects[*target_leaf].iter().enumerate() {
            let target = transform_point(*target_point, &transforms[*target_leaf]);
            let distance_sq = (source - target).norm_squared();
            if distance_sq < best_distance {
                best = Some((*target_leaf, target_point_index, target));
                best_distance = distance_sq;
            }
        }
    }
    best.map(|(target_leaf, target_point_index, target)| {
        (target_leaf, target_point_index, target, best_distance)
    })
    .ok_or_else(|| "target cascade node must not be empty".to_string())
}

fn transform_point(point: [f64; 3], transform: &ObjectTransform) -> Vector3<f64> {
    transform.rotation * Vector3::new(point[0], point[1], point[2]) + transform.translation
}

fn transform_normal(normal: [f64; 3], transform: &ObjectTransform) -> Vector3<f64> {
    (transform.rotation * Vector3::new(normal[0], normal[1], normal[2])).normalize()
}

fn normalize_to_fixed(
    transforms: Vec<ObjectTransform>,
    fixed_index: usize,
) -> Vec<ObjectTransform> {
    if fixed_index + 1 == transforms.len() {
        return transforms;
    }
    let fixed_inverse = inverse_transform(&transforms[fixed_index]);
    transforms
        .iter()
        .map(|transform| compose_transforms(&fixed_inverse, transform))
        .collect()
}

fn inverse_transform(transform: &ObjectTransform) -> ObjectTransform {
    let rotation = transform.rotation.transpose();
    ObjectTransform {
        rotation,
        translation: -(rotation * transform.translation),
    }
}

fn compose_transforms(first: &ObjectTransform, second: &ObjectTransform) -> ObjectTransform {
    ObjectTransform {
        rotation: first.rotation * second.rotation,
        translation: first.rotation * second.translation + first.translation,
    }
}
