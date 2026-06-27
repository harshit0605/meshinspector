use super::*;

#[test]
fn feature_center_distance_matches_meshlib_plane_sphere_contract() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "plane".to_string(),
            kind: FeaturePrimitiveKind::Plane,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 2.0]),
            radius: 0.0,
            length: 0.0,
        },
        FeaturePrimitive {
            feature_id: "sphere".to_string(),
            kind: FeaturePrimitiveKind::Sphere,
            center: [1.0, 2.0, 5.0],
            direction: None,
            radius: 2.0,
            length: 0.0,
        },
    ];
    let result = feature_pair_measurements(&features, &[[0, 1]]).unwrap();
    let exact = &result[0].distance;
    assert_eq!(exact.status, FeatureMeasureStatus::Ok);
    assert_eq!(exact.distance_mm, Some(3.0));
    assert_eq!(exact.closest_point_a, Some([1.0, 2.0, 0.0]));
    assert_eq!(exact.closest_point_b, Some([1.0, 2.0, 3.0]));
    let distance = &result[0].center_distance;
    assert_eq!(distance.status, FeatureMeasureStatus::Ok);
    assert_eq!(distance.distance_mm, Some(5.0));
    assert_eq!(distance.closest_point_a, Some([1.0, 2.0, 0.0]));
    assert_eq!(distance.closest_point_b, Some([1.0, 2.0, 5.0]));
    assert_eq!(result[0].angle.status, FeatureMeasureStatus::BadFeaturePair);
}

#[test]
fn feature_exact_distance_matches_meshlib_line_sphere_surface_contract() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "line".to_string(),
            kind: FeaturePrimitiveKind::Line,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 0.0,
            length: 4.0,
        },
        FeaturePrimitive {
            feature_id: "sphere".to_string(),
            kind: FeaturePrimitiveKind::Sphere,
            center: [3.0, 0.0, 0.0],
            direction: None,
            radius: 1.0,
            length: 0.0,
        },
    ];
    let result = feature_pair_measurements(&features, &[[0, 1]]).unwrap();
    let exact = &result[0].distance;
    assert_eq!(exact.status, FeatureMeasureStatus::Ok);
    assert_eq!(exact.distance_mm, Some(2.0));
    assert_eq!(exact.closest_point_a, Some([0.0, 0.0, 0.0]));
    assert_eq!(exact.closest_point_b, Some([2.0, 0.0, 0.0]));
    assert_eq!(result[0].center_distance.distance_mm, Some(3.0));
}

#[test]
fn feature_axis_angle_uses_meshlib_surface_normal_rule() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "plane".to_string(),
            kind: FeaturePrimitiveKind::Plane,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 0.0,
            length: 0.0,
        },
        FeaturePrimitive {
            feature_id: "line".to_string(),
            kind: FeaturePrimitiveKind::Line,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 0.0,
            length: 4.0,
        },
    ];
    let result = feature_pair_measurements(&features, &[[0, 1]]).unwrap();
    let angle = &result[0].angle;
    assert_eq!(angle.status, FeatureMeasureStatus::Ok);
    assert_eq!(angle.angle_radians, Some(std::f64::consts::FRAC_PI_2));
    assert_eq!(angle.angle_degrees, Some(90.0));
    assert_eq!(angle.is_surface_normal_a, true);
    assert_eq!(angle.is_surface_normal_b, false);
    assert_eq!(result[0].intersections.len(), 1);
    assert_eq!(result[0].intersections[0].kind, FeaturePrimitiveKind::Point);
    assert_eq!(result[0].intersections[0].center, [0.0, 0.0, 0.0]);
}

#[test]
fn feature_center_distance_handles_skew_line_segments() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "a".to_string(),
            kind: FeaturePrimitiveKind::Line,
            center: [0.0, 0.0, 0.0],
            direction: Some([1.0, 0.0, 0.0]),
            radius: 0.0,
            length: 2.0,
        },
        FeaturePrimitive {
            feature_id: "b".to_string(),
            kind: FeaturePrimitiveKind::Line,
            center: [0.0, 1.0, 1.0],
            direction: Some([0.0, 1.0, 0.0]),
            radius: 0.0,
            length: 2.0,
        },
    ];
    let result = feature_pair_measurements(&features, &[[0, 1]]).unwrap();
    let distance = &result[0].center_distance;
    assert_eq!(distance.status, FeatureMeasureStatus::Ok);
    assert_eq!(distance.distance_mm, Some(1.0));
    assert_eq!(distance.closest_point_a, Some([0.0, 0.0, 0.0]));
    assert_eq!(distance.closest_point_b, Some([0.0, 0.0, 1.0]));
}

#[test]
fn feature_center_distance_matches_meshlib_parallel_cylinder_fallback() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "a".to_string(),
            kind: FeaturePrimitiveKind::Cylinder,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 0.5,
            length: 2.0,
        },
        FeaturePrimitive {
            feature_id: "b".to_string(),
            kind: FeaturePrimitiveKind::Cylinder,
            center: [1.0, 0.0, 4.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 0.5,
            length: 2.0,
        },
    ];
    let result = feature_pair_measurements(&features, &[[0, 1]]).unwrap();
    assert_eq!(
        result[0].distance.status,
        FeatureMeasureStatus::NotImplemented
    );
    let center = &result[0].center_distance;
    assert_eq!(center.status, FeatureMeasureStatus::Ok);
    assert_eq!(center.distance_mm, Some(17.0_f64.sqrt()));
    assert_eq!(center.closest_point_a, Some([0.0, 0.0, 4.0]));
    assert_eq!(center.closest_point_b, Some([1.0, 0.0, 8.0]));
}

#[test]
fn feature_intersections_match_meshlib_line_sphere_segment_contract() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "line".to_string(),
            kind: FeaturePrimitiveKind::Line,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 0.0,
            length: 6.0,
        },
        FeaturePrimitive {
            feature_id: "sphere".to_string(),
            kind: FeaturePrimitiveKind::Sphere,
            center: [0.0, 0.0, 0.0],
            direction: None,
            radius: 2.0,
            length: 0.0,
        },
    ];
    let result = feature_pair_measurements(&features, &[[0, 1]]).unwrap();
    assert_eq!(result[0].intersections.len(), 1);
    let segment = &result[0].intersections[0];
    assert_eq!(segment.kind, FeaturePrimitiveKind::Line);
    assert_eq!(segment.start_point, Some([0.0, 0.0, 2.0]));
    assert_eq!(segment.end_point, Some([0.0, 0.0, -2.0]));
    assert_eq!(segment.length_mm, Some(4.0));
    assert_eq!(segment.meshlib_primitive, "MR::toPrimitive(LineSegm3f)");
}

#[test]
fn feature_intersections_match_meshlib_sphere_sphere_circle_contract() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "a".to_string(),
            kind: FeaturePrimitiveKind::Sphere,
            center: [0.0, 0.0, 0.0],
            direction: None,
            radius: 2.0,
            length: 0.0,
        },
        FeaturePrimitive {
            feature_id: "b".to_string(),
            kind: FeaturePrimitiveKind::Sphere,
            center: [3.0, 0.0, 0.0],
            direction: None,
            radius: 2.0,
            length: 0.0,
        },
    ];
    let result = feature_pair_measurements(&features, &[[0, 1]]).unwrap();
    assert_eq!(result[0].intersections.len(), 1);
    let circle = &result[0].intersections[0];
    assert_eq!(circle.kind, FeaturePrimitiveKind::Circle);
    assert_eq!(circle.center, [1.5, 0.0, 0.0]);
    assert_eq!(circle.direction, Some([1.0, 0.0, 0.0]));
    assert!((circle.radius_mm.unwrap() - 1.3228756555322954).abs() < 1e-12);
    assert_eq!(circle.meshlib_primitive, "MR::Features::primitiveCircle");
}

#[test]
fn feature_object_descriptors_match_meshlib_primitive_to_object_contract() {
    let features = vec![
        FeaturePrimitive {
            feature_id: "point_from_sphere".to_string(),
            kind: FeaturePrimitiveKind::Sphere,
            center: [1.0, 2.0, 3.0],
            direction: None,
            radius: 0.0,
            length: 0.0,
        },
        FeaturePrimitive {
            feature_id: "plane_xy".to_string(),
            kind: FeaturePrimitiveKind::Plane,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 0.0,
            length: 0.0,
        },
        FeaturePrimitive {
            feature_id: "cylinder_z".to_string(),
            kind: FeaturePrimitiveKind::Cylinder,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 2.0,
            length: 5.0,
        },
        FeaturePrimitive {
            feature_id: "cone_z".to_string(),
            kind: FeaturePrimitiveKind::Cone,
            center: [0.0, 0.0, 0.0],
            direction: Some([0.0, 0.0, 1.0]),
            radius: 2.0,
            length: 10.0,
        },
    ];

    let descriptors = feature_object_descriptors(&features, 25.0).unwrap();

    assert_eq!(descriptors[0].object_type, "PointObject");
    assert_eq!(descriptors[0].source_kind, FeaturePrimitiveKind::Sphere);
    assert_eq!(descriptors[0].shared_properties.len(), 1);
    assert_eq!(descriptors[0].shared_properties[0].name, "Point");
    assert_eq!(
        descriptors[0].shared_properties[0].kind,
        FeatureObjectPropertyKind::Position
    );
    assert_eq!(
        descriptors[0].shared_properties[0].vector_value,
        Some([1.0, 2.0, 3.0])
    );

    assert_eq!(descriptors[1].object_type, "PlaneObject");
    let plane_properties = descriptors[1]
        .shared_properties
        .iter()
        .map(|property| {
            (
                property.name,
                property.kind,
                property.scalar_value,
                property.vector_value,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        plane_properties,
        vec![
            (
                "Center",
                FeatureObjectPropertyKind::Position,
                None,
                Some([0.0, 0.0, 0.0])
            ),
            (
                "Normal",
                FeatureObjectPropertyKind::Direction,
                None,
                Some([0.0, 0.0, 1.0])
            ),
            (
                "Size",
                FeatureObjectPropertyKind::LinearDimension,
                Some(25.0),
                None
            ),
            (
                "SizeX",
                FeatureObjectPropertyKind::LinearDimension,
                Some(25.0),
                None
            ),
            (
                "SizeY",
                FeatureObjectPropertyKind::LinearDimension,
                Some(25.0),
                None
            ),
        ]
    );

    assert_eq!(descriptors[2].object_type, "CylinderObject");
    assert_eq!(
        descriptors[2]
            .shared_properties
            .iter()
            .map(|property| property.name)
            .collect::<Vec<_>>(),
        vec!["Radius", "Length", "Center", "Main axis"]
    );
    assert_eq!(
        descriptors[2].meshlib_reference,
        "MR::Features::primitiveToObject"
    );
    assert_eq!(descriptors[3].object_type, "ConeObject");
    assert_eq!(
        descriptors[3]
            .shared_properties
            .iter()
            .map(|property| property.name)
            .collect::<Vec<_>>(),
        vec!["Angle", "Height", "Center", "Main axis"]
    );
    assert_eq!(
        descriptors[3].shared_properties[0].scalar_value,
        Some((2.0_f64 / 10.0).atan())
    );
    assert_eq!(descriptors[3].shared_properties[1].scalar_value, Some(10.0));
    assert_eq!(
        descriptors[3].shared_properties[2].vector_value,
        Some([0.0, 0.0, 0.0])
    );
}

#[test]
fn refine_feature_primitives_matches_meshlib_plane_refine_contract() {
    let vertices = vec![
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    let faces = vec![[0, 1, 2], [2, 1, 3]];
    let features = vec![FeaturePrimitive {
        feature_id: "plane_xy".to_string(),
        kind: FeaturePrimitiveKind::Plane,
        center: [0.25, 0.25, 0.9],
        direction: Some([0.0, 0.0, 1.0]),
        radius: 0.0,
        length: 0.0,
    }];

    let refinements = refine_feature_primitives(
        &vertices,
        &faces,
        &features,
        FeatureRefineOptions {
            distance_limit: 0.2,
            normal_tolerance_degrees: 30.0,
            max_iterations: 4,
        },
    )
    .unwrap();

    assert_eq!(refinements.len(), 1);
    let refinement = &refinements[0];
    assert_eq!(refinement.meshlib_reference, "MR::refineFeatureObject");
    assert_eq!(refinement.selected_vertex_indices, vec![0, 1, 2, 3]);
    assert_eq!(refinement.selected_count, 4);
    assert!(refinement.converged);
    assert_eq!(refinement.iterations, 2);
    assert_eq!(refinement.primitive.kind, FeaturePrimitiveKind::Plane);
    let direction = refinement.primitive.direction.unwrap();
    assert!(direction[0].abs() < 1e-12);
    assert!(direction[1].abs() < 1e-12);
    assert!((direction[2] - 1.0).abs() < 1e-12);
    assert!((refinement.primitive.center[2] - 1.0).abs() < 1e-12);
    assert_eq!(refinement.primitive.length, (2.0_f64).sqrt());
}
