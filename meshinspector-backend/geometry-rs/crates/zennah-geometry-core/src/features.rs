mod cone_approx;
mod cylinder_approx;
mod intersections;
mod measure;
mod objects;
mod refine;
mod support;

use intersections::measure_intersections;
use measure::{measure_angle, measure_center_distance, measure_distance};
pub use objects::feature_object_descriptors_impl as feature_object_descriptors;
pub use refine::refine_feature_primitives_impl as refine_feature_primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeaturePrimitiveKind {
    Point,
    Sphere,
    Line,
    Plane,
    Circle,
    Cylinder,
    Cone,
}

impl FeaturePrimitiveKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::Sphere => "sphere",
            Self::Line => "line",
            Self::Plane => "plane",
            Self::Circle => "circle",
            Self::Cylinder => "cylinder",
            Self::Cone => "cone",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeaturePrimitive {
    pub feature_id: String,
    pub kind: FeaturePrimitiveKind,
    pub center: [f64; 3],
    pub direction: Option<[f64; 3]>,
    pub radius: f64,
    pub length: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureMeasureStatus {
    Ok,
    BadFeaturePair,
    BadRelativeLocation,
    NotImplemented,
    NotFinite,
}

impl FeatureMeasureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::BadFeaturePair => "bad_feature_pair",
            Self::BadRelativeLocation => "bad_relative_location",
            Self::NotImplemented => "not_implemented",
            Self::NotFinite => "not_finite",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureDistancePart {
    pub status: FeatureMeasureStatus,
    pub distance_mm: Option<f64>,
    pub closest_point_a: Option<[f64; 3]>,
    pub closest_point_b: Option<[f64; 3]>,
}

impl FeatureDistancePart {
    fn ok(distance_mm: f64, closest_point_a: [f64; 3], closest_point_b: [f64; 3]) -> Self {
        if distance_mm.is_finite()
            && closest_point_a.iter().all(|value| value.is_finite())
            && closest_point_b.iter().all(|value| value.is_finite())
        {
            Self {
                status: FeatureMeasureStatus::Ok,
                distance_mm: Some(distance_mm),
                closest_point_a: Some(closest_point_a),
                closest_point_b: Some(closest_point_b),
            }
        } else {
            Self::status(FeatureMeasureStatus::NotFinite)
        }
    }

    fn status(status: FeatureMeasureStatus) -> Self {
        Self {
            status,
            distance_mm: None,
            closest_point_a: None,
            closest_point_b: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureAnglePart {
    pub status: FeatureMeasureStatus,
    pub angle_radians: Option<f64>,
    pub angle_degrees: Option<f64>,
    pub point_a: Option<[f64; 3]>,
    pub point_b: Option<[f64; 3]>,
    pub direction_a: Option<[f64; 3]>,
    pub direction_b: Option<[f64; 3]>,
    pub is_surface_normal_a: bool,
    pub is_surface_normal_b: bool,
}

impl FeatureAnglePart {
    fn ok(
        point_a: [f64; 3],
        point_b: [f64; 3],
        direction_a: [f64; 3],
        direction_b: [f64; 3],
        is_surface_normal_a: bool,
        is_surface_normal_b: bool,
    ) -> Self {
        let dot_value = dot(direction_a, direction_b).clamp(-1.0, 1.0);
        let mut angle = dot_value.acos();
        if is_surface_normal_a != is_surface_normal_b {
            angle = std::f64::consts::FRAC_PI_2 - angle;
        }
        if angle.is_finite()
            && point_a.iter().all(|value| value.is_finite())
            && point_b.iter().all(|value| value.is_finite())
            && direction_a.iter().all(|value| value.is_finite())
            && direction_b.iter().all(|value| value.is_finite())
        {
            Self {
                status: FeatureMeasureStatus::Ok,
                angle_radians: Some(angle),
                angle_degrees: Some(angle.to_degrees()),
                point_a: Some(point_a),
                point_b: Some(point_b),
                direction_a: Some(direction_a),
                direction_b: Some(direction_b),
                is_surface_normal_a,
                is_surface_normal_b,
            }
        } else {
            Self::status(FeatureMeasureStatus::NotFinite)
        }
    }

    fn status(status: FeatureMeasureStatus) -> Self {
        Self {
            status,
            angle_radians: None,
            angle_degrees: None,
            point_a: None,
            point_b: None,
            direction_a: None,
            direction_b: None,
            is_surface_normal_a: false,
            is_surface_normal_b: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureIntersectionPrimitive {
    pub kind: FeaturePrimitiveKind,
    pub center: [f64; 3],
    pub direction: Option<[f64; 3]>,
    pub radius_mm: Option<f64>,
    pub length_mm: Option<f64>,
    pub start_point: Option<[f64; 3]>,
    pub end_point: Option<[f64; 3]>,
    pub meshlib_primitive: &'static str,
}

impl FeatureIntersectionPrimitive {
    fn point(center: [f64; 3]) -> Option<Self> {
        finite_point(center).then_some(Self {
            kind: FeaturePrimitiveKind::Point,
            center,
            direction: None,
            radius_mm: Some(0.0),
            length_mm: Some(0.0),
            start_point: None,
            end_point: None,
            meshlib_primitive: "MR::Features::Primitives::Sphere(point)",
        })
    }

    fn circle(center: [f64; 3], normal: [f64; 3], radius_mm: f64) -> Option<Self> {
        (finite_point(center) && finite_point(normal) && radius_mm.is_finite() && radius_mm >= 0.0)
            .then_some(Self {
                kind: FeaturePrimitiveKind::Circle,
                center,
                direction: Some(normal),
                radius_mm: Some(radius_mm),
                length_mm: Some(0.0),
                start_point: None,
                end_point: None,
                meshlib_primitive: "MR::Features::primitiveCircle",
            })
    }

    fn line(center: [f64; 3], direction: [f64; 3]) -> Option<Self> {
        (finite_point(center) && finite_point(direction)).then_some(Self {
            kind: FeaturePrimitiveKind::Line,
            center,
            direction: Some(direction),
            radius_mm: Some(0.0),
            length_mm: None,
            start_point: None,
            end_point: None,
            meshlib_primitive: "MR::Features::Primitives::ConeSegment(line)",
        })
    }

    fn line_segment(start_point: [f64; 3], end_point: [f64; 3]) -> Option<Self> {
        let delta = sub(end_point, start_point);
        let length_mm = length(delta);
        let direction = normalize(delta)?;
        (finite_point(start_point) && finite_point(end_point) && length_mm.is_finite()).then_some(
            Self {
                kind: FeaturePrimitiveKind::Line,
                center: add(start_point, scale(delta, 0.5)),
                direction: Some(direction),
                radius_mm: Some(0.0),
                length_mm: Some(length_mm),
                start_point: Some(start_point),
                end_point: Some(end_point),
                meshlib_primitive: "MR::toPrimitive(LineSegm3f)",
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeaturePairMeasurement {
    pub first_index: usize,
    pub second_index: usize,
    pub first_feature_id: String,
    pub second_feature_id: String,
    pub first_kind: FeaturePrimitiveKind,
    pub second_kind: FeaturePrimitiveKind,
    pub distance: FeatureDistancePart,
    pub center_distance: FeatureDistancePart,
    pub angle: FeatureAnglePart,
    pub intersections: Vec<FeatureIntersectionPrimitive>,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeatureRefineOptions {
    pub distance_limit: f64,
    pub normal_tolerance_degrees: f64,
    pub max_iterations: usize,
}

impl Default for FeatureRefineOptions {
    fn default() -> Self {
        Self {
            distance_limit: 0.1,
            normal_tolerance_degrees: 30.0,
            max_iterations: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureRefinement {
    pub feature_id: String,
    pub kind: FeaturePrimitiveKind,
    pub primitive: FeaturePrimitive,
    pub selected_vertex_indices: Vec<usize>,
    pub selected_count: usize,
    pub iterations: usize,
    pub converged: bool,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureObjectPropertyKind {
    Position,
    LinearDimension,
    Direction,
    Angle,
    Other,
}

impl FeatureObjectPropertyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Position => "position",
            Self::LinearDimension => "linear_dimension",
            Self::Direction => "direction",
            Self::Angle => "angle",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureObjectProperty {
    pub name: &'static str,
    pub kind: FeatureObjectPropertyKind,
    pub scalar_value: Option<f64>,
    pub vector_value: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureObjectDescriptor {
    pub feature_id: String,
    pub source_kind: FeaturePrimitiveKind,
    pub object_type: &'static str,
    pub class_name: &'static str,
    pub class_name_plural: &'static str,
    pub shared_properties: Vec<FeatureObjectProperty>,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, Copy)]
enum Primitive {
    Sphere {
        center: [f64; 3],
        radius: f64,
    },
    ConeSegment {
        reference_point: [f64; 3],
        dir: [f64; 3],
        positive_side_radius: f64,
        negative_side_radius: f64,
        positive_length: f64,
        negative_length: f64,
    },
    Plane {
        center: [f64; 3],
        normal: [f64; 3],
    },
}

pub fn feature_pair_measurements(
    features: &[FeaturePrimitive],
    pairs: &[[usize; 2]],
) -> Result<Vec<FeaturePairMeasurement>, String> {
    let primitives = features
        .iter()
        .map(feature_to_primitive)
        .collect::<Result<Vec<_>, _>>()?;
    pairs
        .iter()
        .map(|pair| {
            let first = *pair
                .first()
                .ok_or_else(|| "feature pair must contain first index".to_string())?;
            let second = *pair
                .get(1)
                .ok_or_else(|| "feature pair must contain second index".to_string())?;
            let Some(first_feature) = features.get(first) else {
                return Err(format!("feature pair first index {first} is out of range"));
            };
            let Some(second_feature) = features.get(second) else {
                return Err(format!(
                    "feature pair second index {second} is out of range"
                ));
            };
            Ok(FeaturePairMeasurement {
                first_index: first,
                second_index: second,
                first_feature_id: first_feature.feature_id.clone(),
                second_feature_id: second_feature.feature_id.clone(),
                first_kind: first_feature.kind,
                second_kind: second_feature.kind,
                distance: measure_distance(primitives[first], primitives[second]),
                center_distance: measure_center_distance(primitives[first], primitives[second]),
                angle: measure_angle(primitives[first], primitives[second]),
                intersections: measure_intersections(primitives[first], primitives[second]),
                meshlib_reference: "MR::Features::MeasureResult",
            })
        })
        .collect()
}

fn feature_to_primitive(feature: &FeaturePrimitive) -> Result<Primitive, String> {
    if !feature.center.iter().all(|value| value.is_finite()) {
        return Err(format!(
            "feature {} center must be finite",
            feature.feature_id
        ));
    }
    if feature.radius < 0.0 || !feature.radius.is_finite() {
        return Err(format!(
            "feature {} radius must be finite and non-negative",
            feature.feature_id
        ));
    }
    if feature.length < 0.0 || !feature.length.is_finite() {
        return Err(format!(
            "feature {} length must be finite and non-negative",
            feature.feature_id
        ));
    }
    match feature.kind {
        FeaturePrimitiveKind::Point => Ok(Primitive::Sphere {
            center: feature.center,
            radius: 0.0,
        }),
        FeaturePrimitiveKind::Sphere => Ok(Primitive::Sphere {
            center: feature.center,
            radius: feature.radius,
        }),
        FeaturePrimitiveKind::Line => {
            let dir = normalize_required(feature, "direction")?;
            let half = feature.length / 2.0;
            Ok(Primitive::ConeSegment {
                reference_point: sub(feature.center, scale(dir, half)),
                dir,
                positive_side_radius: 0.0,
                negative_side_radius: 0.0,
                positive_length: feature.length,
                negative_length: 0.0,
            })
        }
        FeaturePrimitiveKind::Plane => Ok(Primitive::Plane {
            center: feature.center,
            normal: normalize_required(feature, "normal")?,
        }),
        FeaturePrimitiveKind::Circle => Ok(Primitive::ConeSegment {
            reference_point: feature.center,
            dir: normalize_required(feature, "normal")?,
            positive_side_radius: feature.radius,
            negative_side_radius: feature.radius,
            positive_length: 0.0,
            negative_length: 0.0,
        }),
        FeaturePrimitiveKind::Cylinder => {
            let dir = normalize_required(feature, "direction")?;
            Ok(Primitive::ConeSegment {
                reference_point: feature.center,
                dir,
                positive_side_radius: feature.radius,
                negative_side_radius: feature.radius,
                positive_length: feature.length / 2.0,
                negative_length: feature.length / 2.0,
            })
        }
        FeaturePrimitiveKind::Cone => {
            let dir = normalize_required(feature, "direction")?;
            Ok(Primitive::ConeSegment {
                reference_point: feature.center,
                dir,
                positive_side_radius: feature.radius,
                negative_side_radius: 0.0,
                positive_length: feature.length,
                negative_length: 0.0,
            })
        }
    }
}

fn normalize_required(feature: &FeaturePrimitive, label: &str) -> Result<[f64; 3], String> {
    let Some(direction) = feature.direction else {
        return Err(format!("feature {} requires {label}", feature.feature_id));
    };
    normalize(direction)
        .ok_or_else(|| format!("feature {} {label} must be non-zero", feature.feature_id))
}

pub(super) fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let magnitude = length(vector);
    if magnitude <= f64::EPSILON || !magnitude.is_finite() {
        None
    } else {
        Some(scale(vector, 1.0 / magnitude))
    }
}

pub(super) fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(super) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn finite_point(point: [f64; 3]) -> bool {
    point.iter().all(|value| value.is_finite())
}

#[cfg(test)]
mod tests;
