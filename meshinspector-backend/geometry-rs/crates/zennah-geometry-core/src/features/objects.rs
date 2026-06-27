use super::cone_approx::cone_angle_from_radius_height;
use super::support::{cone_center_point, cone_dir, cone_positive_radius};
use super::{
    feature_to_primitive, FeatureObjectDescriptor, FeatureObjectProperty,
    FeatureObjectPropertyKind, FeaturePrimitive, Primitive,
};

const MESH_LIB_REFERENCE: &str = "MR::Features::primitiveToObject";
const FEATURE_CLASS_NAME: &str = "Feature";
const FEATURE_CLASS_NAME_PLURAL: &str = "Features";

pub fn feature_object_descriptors_impl(
    features: &[FeaturePrimitive],
    infinite_extent: f64,
) -> Result<Vec<FeatureObjectDescriptor>, String> {
    if features.is_empty() {
        return Err(
            "feature object descriptors require at least one feature primitive".to_string(),
        );
    }
    if !infinite_extent.is_finite() || infinite_extent <= 0.0 {
        return Err("feature object infinite extent must be finite and positive".to_string());
    }

    features
        .iter()
        .map(|feature| primitive_to_object_descriptor(feature, infinite_extent))
        .collect()
}

fn primitive_to_object_descriptor(
    feature: &FeaturePrimitive,
    infinite_extent: f64,
) -> Result<FeatureObjectDescriptor, String> {
    match feature_to_primitive(feature)? {
        Primitive::Sphere { center, radius } => {
            if radius == 0.0 {
                Ok(descriptor(
                    feature,
                    "PointObject",
                    vec![vector_property(
                        "Point",
                        FeatureObjectPropertyKind::Position,
                        center,
                    )],
                ))
            } else {
                Ok(descriptor(
                    feature,
                    "SphereObject",
                    vec![
                        scalar_property(
                            "Radius",
                            FeatureObjectPropertyKind::LinearDimension,
                            radius,
                        ),
                        vector_property("Center", FeatureObjectPropertyKind::Position, center),
                    ],
                ))
            }
        }
        Primitive::Plane { center, normal } => Ok(descriptor(
            feature,
            "PlaneObject",
            vec![
                vector_property("Center", FeatureObjectPropertyKind::Position, center),
                vector_property("Normal", FeatureObjectPropertyKind::Direction, normal),
                scalar_property(
                    "Size",
                    FeatureObjectPropertyKind::LinearDimension,
                    infinite_extent,
                ),
                scalar_property(
                    "SizeX",
                    FeatureObjectPropertyKind::LinearDimension,
                    infinite_extent,
                ),
                scalar_property(
                    "SizeY",
                    FeatureObjectPropertyKind::LinearDimension,
                    infinite_extent,
                ),
            ],
        )),
        cone @ Primitive::ConeSegment {
            reference_point,
            positive_side_radius,
            negative_side_radius,
            positive_length,
            negative_length,
            ..
        } => {
            if cone_is_circle(cone) {
                let center = cone_center_point(cone);
                if positive_side_radius == 0.0 && negative_side_radius == 0.0 {
                    return Ok(descriptor(
                        feature,
                        "PointObject",
                        vec![vector_property(
                            "Point",
                            FeatureObjectPropertyKind::Position,
                            center,
                        )],
                    ));
                }
                return Ok(descriptor(
                    feature,
                    "CircleObject",
                    vec![
                        scalar_property(
                            "Radius",
                            FeatureObjectPropertyKind::LinearDimension,
                            cone_positive_radius(cone),
                        ),
                        vector_property("Center", FeatureObjectPropertyKind::Position, center),
                        vector_property(
                            "Normal",
                            FeatureObjectPropertyKind::Direction,
                            cone_dir(cone),
                        ),
                    ],
                ));
            }

            if positive_side_radius == 0.0 && negative_side_radius == 0.0 {
                return Ok(descriptor(
                    feature,
                    "LineObject",
                    vec![
                        vector_property(
                            "Center",
                            FeatureObjectPropertyKind::Position,
                            cone_center_point(cone),
                        ),
                        vector_property(
                            "Direction",
                            FeatureObjectPropertyKind::Direction,
                            cone_dir(cone),
                        ),
                        scalar_property(
                            "Length",
                            FeatureObjectPropertyKind::LinearDimension,
                            cone_length(cone).unwrap_or(infinite_extent),
                        ),
                    ],
                ));
            }

            if positive_side_radius == negative_side_radius {
                return Ok(descriptor(
                    feature,
                    "CylinderObject",
                    vec![
                        scalar_property(
                            "Radius",
                            FeatureObjectPropertyKind::LinearDimension,
                            positive_side_radius,
                        ),
                        scalar_property(
                            "Length",
                            FeatureObjectPropertyKind::LinearDimension,
                            cone_length(cone).unwrap_or(infinite_extent),
                        ),
                        vector_property(
                            "Center",
                            FeatureObjectPropertyKind::Position,
                            cone_center_point(cone),
                        ),
                        vector_property(
                            "Main axis",
                            FeatureObjectPropertyKind::Direction,
                            cone_dir(cone),
                        ),
                    ],
                ));
            }

            if negative_side_radius == 0.0 && negative_length == 0.0 {
                let angle = cone_angle_from_radius_height(positive_side_radius, positive_length)?;
                return Ok(descriptor(
                    feature,
                    "ConeObject",
                    vec![
                        scalar_property("Angle", FeatureObjectPropertyKind::Angle, angle),
                        scalar_property(
                            "Height",
                            FeatureObjectPropertyKind::LinearDimension,
                            positive_length,
                        ),
                        vector_property(
                            "Center",
                            FeatureObjectPropertyKind::Position,
                            reference_point,
                        ),
                        vector_property(
                            "Main axis",
                            FeatureObjectPropertyKind::Direction,
                            cone_dir(cone),
                        ),
                    ],
                ));
            }

            Err(format!(
                "feature {} maps to a cone segment that is not represented by current FeatureObject descriptors",
                feature.feature_id
            ))
        }
    }
}

fn descriptor(
    feature: &FeaturePrimitive,
    object_type: &'static str,
    shared_properties: Vec<FeatureObjectProperty>,
) -> FeatureObjectDescriptor {
    FeatureObjectDescriptor {
        feature_id: feature.feature_id.clone(),
        source_kind: feature.kind,
        object_type,
        class_name: FEATURE_CLASS_NAME,
        class_name_plural: FEATURE_CLASS_NAME_PLURAL,
        shared_properties,
        meshlib_reference: MESH_LIB_REFERENCE,
    }
}

fn scalar_property(
    name: &'static str,
    kind: FeatureObjectPropertyKind,
    scalar_value: f64,
) -> FeatureObjectProperty {
    FeatureObjectProperty {
        name,
        kind,
        scalar_value: Some(scalar_value),
        vector_value: None,
    }
}

fn vector_property(
    name: &'static str,
    kind: FeatureObjectPropertyKind,
    vector_value: [f64; 3],
) -> FeatureObjectProperty {
    FeatureObjectProperty {
        name,
        kind,
        scalar_value: None,
        vector_value: Some(vector_value),
    }
}

fn cone_is_circle(cone: Primitive) -> bool {
    match cone {
        Primitive::ConeSegment {
            positive_length,
            negative_length,
            ..
        } => (positive_length + negative_length).abs() <= f64::EPSILON,
        _ => false,
    }
}

fn cone_length(cone: Primitive) -> Option<f64> {
    match cone {
        Primitive::ConeSegment {
            positive_length,
            negative_length,
            ..
        } => {
            let length = positive_length + negative_length;
            length.is_finite().then_some(length)
        }
        _ => None,
    }
}
