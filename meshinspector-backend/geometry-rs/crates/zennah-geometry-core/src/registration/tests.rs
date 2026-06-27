use super::*;

fn reference_points_and_normals() -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    (
        vec![
            [1.0, 1.0, -5.0],
            [14.0, 1.0, 1.0],
            [1.0, 14.0, 2.0],
            [-11.0, 2.0, 3.0],
            [1.0, -11.0, 4.0],
            [1.0, 2.0, 8.0],
            [2.0, 1.0, -5.0],
            [15.0, 1.5, 1.0],
            [1.5, 15.0, 2.0],
            [-11.0, 2.5, 3.1],
        ],
        vec![
            [0.0, 0.0, -1.0],
            [1.0, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.0, 0.1, 1.0],
            [0.1, -1.1, 1.1],
            [0.1, 0.1, 1.0],
            [0.1, 0.0, -1.0],
            [1.1, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.1, 0.1, 1.1],
        ],
    )
}

fn transform_points(
    points: &[[f64; 3]],
    rotation: Matrix3<f64>,
    translation: Vector3<f64>,
) -> Vec<[f64; 3]> {
    points
        .iter()
        .map(|point| {
            let transformed = rotation * vector(*point) + translation;
            [transformed.x, transformed.y, transformed.z]
        })
        .collect()
}

fn matrix_from_rows(rows: [[f64; 3]; 3]) -> Matrix3<f64> {
    Matrix3::new(
        rows[0][0], rows[0][1], rows[0][2], rows[1][0], rows[1][1], rows[1][2], rows[2][0],
        rows[2][1], rows[2][2],
    )
}

#[test]
fn pairwise_point_to_plane_icp_recovers_meshlib_style_translation_only_transform() {
    let (reference, normals) = reference_points_and_normals();
    let translation = Vector3::new(0.02, -0.015, 0.01);
    let floating = transform_points(&reference, Matrix3::identity(), translation);

    let result = pairwise_point_to_plane_icp(
        &floating,
        &reference,
        &normals,
        12,
        1e-12,
        IcpMode::TranslationOnly,
    )
    .expect("point-to-plane ICP should recover translation-only transform");

    assert_eq!(result.active_pair_count, floating.len());
    assert!(result.mean_square_distance < 1e-18);
    assert!((vector(result.translation) + translation).norm() < 1e-10);
    assert!((matrix_from_rows(result.rotation) - Matrix3::identity()).norm() < 1e-12);
}

#[test]
fn pairwise_point_to_plane_icp_recovers_meshlib_style_small_rigid_transform() {
    let (reference, normals) = reference_points_and_normals();
    let angle: f64 = 0.005;
    let rotation = Matrix3::new(
        angle.cos(),
        -angle.sin(),
        0.0,
        angle.sin(),
        angle.cos(),
        0.0,
        0.0,
        0.0,
        1.0,
    );
    let translation = Vector3::new(0.003, -0.002, 0.0015);
    let floating = transform_points(&reference, rotation, translation);

    let result = pairwise_point_to_plane_icp(
        &floating,
        &reference,
        &normals,
        25,
        1e-18,
        IcpMode::AnyRigidXf,
    )
    .expect("point-to-plane ICP should recover small rigid transform");

    let expected_inverse = rotation.transpose();
    assert_eq!(result.active_pair_count, floating.len());
    assert!(
        result.mean_square_distance < 1e-16,
        "mean square plane distance was {} with translation {:?} and rotation {:?}",
        result.mean_square_distance,
        result.translation,
        result.rotation
    );
    assert!((matrix_from_rows(result.rotation) - expected_inverse).norm() < 1e-7);
    let transformed = transform_points(
        &floating,
        matrix_from_rows(result.rotation),
        vector(result.translation),
    );
    for (actual, expected) in transformed.iter().zip(reference.iter()) {
        assert!((vector(*actual) - vector(*expected)).norm() < 1e-7);
    }
}

#[test]
fn point_to_plane_icp_distance_filter_rejects_far_pairs_like_meshlib_dist_threshold() {
    let (reference, normals) = reference_points_and_normals();
    let translation = Vector3::new(0.02, -0.015, 0.01);
    let mut floating = transform_points(&reference, Matrix3::identity(), translation);
    floating.push([80.0, -70.0, 45.0]);

    let result = pairwise_point_to_plane_icp_with_filters(
        &floating,
        &reference,
        &normals,
        12,
        1e-12,
        IcpMode::TranslationOnly,
        IcpPairFilterOptions {
            max_pair_distance: Some(0.1),
            ..IcpPairFilterOptions::default()
        },
    )
    .expect("distance filter should keep in-threshold ICP pairs");

    assert_eq!(result.active_pair_count, reference.len());
    assert!((vector(result.translation) + translation).norm() < 1e-10);
}

#[test]
fn point_to_plane_icp_cos_threshold_rejects_opposed_normals_like_meshlib() {
    let (reference, normals) = reference_points_and_normals();
    let translation = Vector3::new(0.02, -0.015, 0.01);
    let floating = transform_points(&reference, Matrix3::identity(), translation);
    let mut floating_normals = normals.clone();
    floating_normals[2] = [
        -floating_normals[2][0],
        -floating_normals[2][1],
        -floating_normals[2][2],
    ];

    let result = pairwise_point_to_plane_icp_with_filters(
        &floating,
        &reference,
        &normals,
        12,
        1e-12,
        IcpMode::TranslationOnly,
        IcpPairFilterOptions {
            floating_normals: Some(&floating_normals),
            cos_threshold: Some(0.7),
            ..IcpPairFilterOptions::default()
        },
    )
    .expect("cosine filter should keep consistent source/target normal pairs");

    assert_eq!(result.active_pair_count, reference.len() - 1);
    assert!((vector(result.translation) + translation).norm() < 1e-10);
}

#[test]
fn point_to_plane_icp_mutual_closest_rejects_non_reciprocal_pairs_like_meshlib() {
    let (reference, normals) = reference_points_and_normals();
    let translation = Vector3::new(0.02, -0.015, 0.01);
    let mut floating = transform_points(&reference, Matrix3::identity(), translation);
    let non_reciprocal = vector(reference[0]) + Vector3::new(0.05, 0.0, 0.0);
    floating.push([non_reciprocal.x, non_reciprocal.y, non_reciprocal.z]);

    let result = pairwise_point_to_plane_icp_with_filters(
        &floating,
        &reference,
        &normals,
        12,
        1e-12,
        IcpMode::TranslationOnly,
        IcpPairFilterOptions {
            mutual_closest: true,
            ..IcpPairFilterOptions::default()
        },
    )
    .expect("mutual closest filtering should keep reciprocal ICP pairs");

    assert_eq!(result.active_pair_count, reference.len());
    assert!((vector(result.translation) + translation).norm() < 1e-10);
}

#[test]
fn multiway_point_to_point_icp_independent_mode_fixes_last_like_meshlib() {
    let (reference, _) = reference_points_and_normals();
    let first_offset = Vector3::new(0.3, -0.06, 0.04);
    let second_offset = Vector3::new(0.1, -0.02, 0.01);
    let first = transform_points(&reference, Matrix3::identity(), first_offset);
    let second = transform_points(&reference, Matrix3::identity(), second_offset);
    let objects = vec![first, second, reference.clone()];

    let result = multiway_point_to_point_icp(&objects, 60, 1e-18, IcpMode::TranslationOnly, None)
        .expect(
            "multiway point-to-point ICP should align independently while fixing the last object",
        );

    assert_eq!(result.fixed_object_index, 2);
    assert_eq!(result.active_pair_count, reference.len() * 6);
    assert!(result.mean_square_distance < 1e-18);
    assert!((vector(result.transforms[0].translation) + first_offset).norm() < 1e-8);
    assert!((vector(result.transforms[1].translation) + second_offset).norm() < 1e-8);
    assert!(vector(result.transforms[2].translation).norm() < 1e-12);

    for (index, object) in objects.iter().enumerate() {
        let transform = &result.transforms[index];
        let transformed = transform_points(
            object,
            matrix_from_rows(transform.rotation),
            vector(transform.translation),
        );
        for (actual, expected) in transformed.iter().zip(reference.iter()) {
            assert!((vector(*actual) - vector(*expected)).norm() < 1e-8);
        }
    }
}

#[test]
fn multiway_point_to_plane_icp_independent_mode_fixes_last_like_meshlib() {
    let (reference, normals) = reference_points_and_normals();
    let first_offset = Vector3::new(0.03, -0.02, 0.01);
    let second_offset = Vector3::new(0.01, -0.005, 0.003);
    let first = transform_points(&reference, Matrix3::identity(), first_offset);
    let second = transform_points(&reference, Matrix3::identity(), second_offset);
    let objects = vec![first, second, reference.clone()];
    let object_normals = vec![normals.clone(), normals.clone(), normals];

    let result = multiway_point_to_plane_icp(
        &objects,
        &object_normals,
        60,
        1e-18,
        IcpMode::TranslationOnly,
        None,
    )
    .expect("multiway point-to-plane ICP should align independently while fixing the last object");

    assert_eq!(result.fixed_object_index, 2);
    assert_eq!(result.active_pair_count, reference.len() * 6);
    assert!(result.mean_square_distance < 1e-18);
    assert!((vector(result.transforms[0].translation) + first_offset).norm() < 1e-8);
    assert!((vector(result.transforms[1].translation) + second_offset).norm() < 1e-8);
    assert!(vector(result.transforms[2].translation).norm() < 1e-12);

    for (index, object) in objects.iter().enumerate() {
        let transform = &result.transforms[index];
        let transformed = transform_points(
            object,
            matrix_from_rows(transform.rotation),
            vector(transform.translation),
        );
        for (actual, expected) in transformed.iter().zip(reference.iter()) {
            assert!((vector(*actual) - vector(*expected)).norm() < 1e-8);
        }
    }
}

#[test]
fn multiway_combined_icp_runs_meshlib_point_then_plane_schedule() {
    let (reference, normals) = reference_points_and_normals();
    let first_offset = Vector3::new(0.03, -0.02, 0.01);
    let second_offset = Vector3::new(0.01, -0.005, 0.003);
    let first = transform_points(&reference, Matrix3::identity(), first_offset);
    let second = transform_points(&reference, Matrix3::identity(), second_offset);
    let objects = vec![first, second, reference.clone()];
    let object_normals = vec![normals.clone(), normals.clone(), normals];

    let result = multiway_combined_icp(
        &objects,
        &object_normals,
        60,
        1e-18,
        IcpMode::TranslationOnly,
        None,
    )
    .expect(
        "multiway combined ICP should use MeshLib's point-to-point then point-to-plane schedule",
    );

    assert_eq!(result.fixed_object_index, 2);
    assert_eq!(result.active_pair_count, reference.len() * 6);
    assert!(result.iterations >= 2);
    assert!(result.mean_square_distance < 1e-18);
    assert!((vector(result.transforms[0].translation) + first_offset).norm() < 1e-8);
    assert!((vector(result.transforms[1].translation) + second_offset).norm() < 1e-8);
    assert!(vector(result.transforms[2].translation).norm() < 1e-12);

    for (index, object) in objects.iter().enumerate() {
        let transform = &result.transforms[index];
        let transformed = transform_points(
            object,
            matrix_from_rows(transform.rotation),
            vector(transform.translation),
        );
        for (actual, expected) in transformed.iter().zip(reference.iter()) {
            assert!((vector(*actual) - vector(*expected)).norm() < 1e-8);
        }
    }
}

#[test]
fn multiway_all_object_point_to_point_icp_solves_meshlib_global_system_fixing_last() {
    let (reference, _) = reference_points_and_normals();
    let first_angle: f64 = 0.004;
    let first_rotation = Matrix3::new(
        first_angle.cos(),
        -first_angle.sin(),
        0.0,
        first_angle.sin(),
        first_angle.cos(),
        0.0,
        0.0,
        0.0,
        1.0,
    );
    let second_angle: f64 = -0.003;
    let second_rotation = Matrix3::new(
        second_angle.cos(),
        -second_angle.sin(),
        0.0,
        second_angle.sin(),
        second_angle.cos(),
        0.0,
        0.0,
        0.0,
        1.0,
    );
    let first_translation = Vector3::new(0.03, -0.02, 0.01);
    let second_translation = Vector3::new(0.01, -0.005, 0.003);
    let first = transform_points(&reference, first_rotation, first_translation);
    let second = transform_points(&reference, second_rotation, second_translation);
    let objects = vec![first, second, reference.clone()];

    let result =
        multiway_all_object_point_to_point_icp(&objects, 60, 1e-18, IcpMode::AnyRigidXf, None)
            .expect("all-object point-to-point ICP should solve a single MeshLib-style system");

    assert_eq!(result.fixed_object_index, 2);
    assert_eq!(result.active_pair_count, reference.len() * 6);
    assert!(result.mean_square_distance < 1e-16);
    assert!(
        (matrix_from_rows(result.transforms[0].rotation) - first_rotation.transpose()).norm()
            < 1e-6
    );
    assert!(
        (matrix_from_rows(result.transforms[1].rotation) - second_rotation.transpose()).norm()
            < 1e-6
    );
    assert!(vector(result.transforms[2].translation).norm() < 1e-12);

    for (index, object) in objects.iter().enumerate() {
        let transform = &result.transforms[index];
        let transformed = transform_points(
            object,
            matrix_from_rows(transform.rotation),
            vector(transform.translation),
        );
        for (actual, expected) in transformed.iter().zip(reference.iter()) {
            assert!((vector(*actual) - vector(*expected)).norm() < 1e-6);
        }
    }
}

#[test]
fn multiway_all_object_point_to_plane_icp_solves_meshlib_global_system_fixing_last() {
    let (reference, normals) = reference_points_and_normals();
    let first_offset = Vector3::new(0.03, -0.02, 0.01);
    let second_offset = Vector3::new(0.01, -0.005, 0.003);
    let first = transform_points(&reference, Matrix3::identity(), first_offset);
    let second = transform_points(&reference, Matrix3::identity(), second_offset);
    let objects = vec![first, second, reference.clone()];
    let object_normals = vec![normals.clone(), normals.clone(), normals];

    let result = multiway_all_object_point_to_plane_icp(
        &objects,
        &object_normals,
        60,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect("all-object point-to-plane ICP should solve a single MeshLib-style system");

    assert_eq!(result.fixed_object_index, 2);
    assert_eq!(result.active_pair_count, reference.len() * 6);
    assert!(result.mean_square_distance < 1e-18);
    assert!((vector(result.transforms[0].translation) + first_offset).norm() < 1e-8);
    assert!((vector(result.transforms[1].translation) + second_offset).norm() < 1e-8);
    assert!(vector(result.transforms[2].translation).norm() < 1e-12);
}

#[test]
fn multiway_all_object_combined_icp_runs_meshlib_point_then_plane_schedule() {
    let (reference, normals) = reference_points_and_normals();
    let first_offset = Vector3::new(0.03, -0.02, 0.01);
    let second_offset = Vector3::new(0.01, -0.005, 0.003);
    let first = transform_points(&reference, Matrix3::identity(), first_offset);
    let second = transform_points(&reference, Matrix3::identity(), second_offset);
    let objects = vec![first, second, reference.clone()];
    let object_normals = vec![normals.clone(), normals.clone(), normals];

    let result = multiway_all_object_combined_icp(
        &objects,
        &object_normals,
        60,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect(
        "all-object combined ICP should use MeshLib's point-to-point then point-to-plane schedule",
    );

    assert_eq!(result.fixed_object_index, 2);
    assert_eq!(result.active_pair_count, reference.len() * 6);
    assert!(result.iterations >= 2);
    assert!(result.mean_square_distance < 1e-18);
    assert!((vector(result.transforms[0].translation) + first_offset).norm() < 1e-8);
    assert!((vector(result.transforms[1].translation) + second_offset).norm() < 1e-8);
    assert!(vector(result.transforms[2].translation).norm() < 1e-12);
}

#[test]
fn multiway_sequential_cascade_icp_matches_meshlib_max_group_size_two_layers() {
    let (reference, normals) = reference_points_and_normals();
    let offsets = [
        Vector3::new(0.03, -0.02, 0.01),
        Vector3::new(0.01, -0.005, 0.003),
        Vector3::new(-0.02, 0.015, -0.004),
        Vector3::zeros(),
    ];
    let objects = offsets
        .iter()
        .map(|offset| transform_points(&reference, Matrix3::identity(), *offset))
        .collect::<Vec<_>>();
    let object_normals = vec![normals.clone(), normals.clone(), normals.clone(), normals];

    let point_result = multiway_sequential_cascade_point_to_point_icp(
        &objects,
        2,
        60,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect("sequential cascade point-to-point ICP should match MeshLib maxGroupSize=2");
    let plane_result = multiway_sequential_cascade_point_to_plane_icp(
        &objects,
        &object_normals,
        2,
        60,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect("sequential cascade point-to-plane ICP should match MeshLib maxGroupSize=2");
    let combined_result = multiway_sequential_cascade_combined_icp(
        &objects,
        &object_normals,
        2,
        60,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect("sequential cascade combined ICP should match MeshLib's point then plane schedule");

    for result in [&point_result, &plane_result, &combined_result] {
        assert_eq!(result.fixed_object_index, 3);
        assert!(result.active_pair_count >= reference.len() * 8);
        assert!(result.mean_square_distance < 1e-16);
        for (index, offset) in offsets.iter().enumerate() {
            assert!(
                (vector(result.transforms[index].translation) + offset).norm() < 1e-7,
                "object {index} translation was {:?}",
                result.transforms[index].translation
            );
            let transformed = transform_points(
                &objects[index],
                matrix_from_rows(result.transforms[index].rotation),
                vector(result.transforms[index].translation),
            );
            for (actual, expected) in transformed.iter().zip(reference.iter()) {
                assert!((vector(*actual) - vector(*expected)).norm() < 1e-7);
            }
        }
    }

    assert!(combined_result.iterations >= 2);
}

#[test]
fn multiway_aabb_cascade_icp_groups_spatially_interleaved_objects_like_meshlib() {
    let spatial_reference = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.2],
        [0.2, 0.9, -0.2],
        [-0.1, 0.2, 1.1],
        [0.7, 0.4, 0.8],
    ];
    let spatial_offsets = [
        Vector3::new(-5.1, 0.0, 0.0),
        Vector3::new(0.2, 0.0, 0.0),
        Vector3::new(-5.0, 0.0, 0.0),
        Vector3::zeros(),
    ];
    let spatial_objects = spatial_offsets
        .iter()
        .map(|offset| transform_points(&spatial_reference, Matrix3::identity(), *offset))
        .collect::<Vec<_>>();
    let point_result = multiway_aabb_cascade_point_to_point_icp(
        &spatial_objects,
        2,
        80,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect("AABB cascade point-to-point ICP should mirror MeshLib AABBTreeBased grouping");

    let (reference, normals) = reference_points_and_normals();
    let offsets = [
        Vector3::new(0.03, -0.02, 0.01),
        Vector3::new(0.01, -0.005, 0.003),
        Vector3::new(-0.02, 0.015, -0.004),
        Vector3::zeros(),
    ];
    let objects = offsets
        .iter()
        .map(|offset| transform_points(&reference, Matrix3::identity(), *offset))
        .collect::<Vec<_>>();
    let object_normals = vec![normals.clone(), normals.clone(), normals.clone(), normals];

    let plane_result = multiway_aabb_cascade_point_to_plane_icp(
        &objects,
        &object_normals,
        2,
        80,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect("AABB cascade point-to-plane ICP should mirror MeshLib AABBTreeBased grouping");
    let combined_result = multiway_aabb_cascade_combined_icp(
        &objects,
        &object_normals,
        2,
        80,
        1e-18,
        IcpMode::AnyRigidXf,
        None,
    )
    .expect("AABB cascade combined ICP should run MeshLib's point then plane schedule");

    assert_eq!(point_result.fixed_object_index, 3);
    assert!(point_result.active_pair_count >= spatial_reference.len() * 8);
    assert!(
        point_result.mean_square_distance < 1e-16,
        "mean square distance was {} with translations {:?}",
        point_result.mean_square_distance,
        point_result
            .transforms
            .iter()
            .map(|transform| transform.translation)
            .collect::<Vec<_>>()
    );
    for (index, _offset) in spatial_offsets.iter().enumerate() {
        let transformed = transform_points(
            &spatial_objects[index],
            matrix_from_rows(point_result.transforms[index].rotation),
            vector(point_result.transforms[index].translation),
        );
        for (actual, expected) in transformed.iter().zip(spatial_reference.iter()) {
            assert!((vector(*actual) - vector(*expected)).norm() < 1e-7);
        }
    }

    for result in [&plane_result, &combined_result] {
        assert_eq!(result.fixed_object_index, 3);
        assert!(result.active_pair_count >= reference.len() * 8);
        assert!(
            result.mean_square_distance < 1e-16,
            "mean square distance was {} with translations {:?}",
            result.mean_square_distance,
            result
                .transforms
                .iter()
                .map(|transform| transform.translation)
                .collect::<Vec<_>>()
        );
    }

    assert!(combined_result.iterations >= 2);
}

#[test]
fn multiway_aabb_cascade_icp_handles_small_object_sets_without_panic() {
    let reference = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let offset = Vector3::new(0.25, -0.1, 0.05);
    let objects = vec![
        reference.clone(),
        transform_points(&reference, Matrix3::identity(), offset),
    ];

    let result = multiway_aabb_cascade_point_to_point_icp(
        &objects,
        64,
        10,
        1e-12,
        IcpMode::TranslationOnly,
        Some(0),
    )
    .expect("AABB cascade ICP should accept object counts below max_group_size");

    assert_eq!(result.fixed_object_index, 0);
    assert_eq!(result.active_pair_count, reference.len() * objects.len());
    assert!(result.mean_square_distance < 1e-18);
    assert!((vector(result.transforms[0].translation)).norm() < 1e-12);
    assert!((vector(result.transforms[1].translation) + offset).norm() < 1e-9);
}
