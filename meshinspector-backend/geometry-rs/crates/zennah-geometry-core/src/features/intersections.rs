use super::support::{
    arbitrary_perpendicular, cone_base_point, cone_center_point, cone_dir, cone_is_circle,
    cone_is_zero_radius, cross, plane_line_intersection,
};
use super::{add, dot, length, normalize, scale, sub, FeatureIntersectionPrimitive, Primitive};

pub(super) fn measure_intersections(
    a: Primitive,
    b: Primitive,
) -> Vec<FeatureIntersectionPrimitive> {
    match (a, b) {
        (
            Primitive::Sphere {
                center: center_a,
                radius: radius_a,
            },
            Primitive::Sphere {
                center: center_b,
                radius: radius_b,
            },
        ) => sphere_sphere_intersections(center_a, radius_a, center_b, radius_b),
        (Primitive::ConeSegment { .. }, Primitive::Sphere { center, radius }) => {
            cone_sphere_intersections(a, center, radius)
        }
        (Primitive::Sphere { center, radius }, Primitive::ConeSegment { .. }) => {
            cone_sphere_intersections(b, center, radius)
        }
        (
            Primitive::Plane { center, normal },
            Primitive::Sphere {
                center: sphere_center,
                radius,
            },
        ) => plane_sphere_intersections(center, normal, sphere_center, radius),
        (
            Primitive::Sphere {
                center: sphere_center,
                radius,
            },
            Primitive::Plane { center, normal },
        ) => plane_sphere_intersections(center, normal, sphere_center, radius),
        (Primitive::Plane { center, normal }, Primitive::ConeSegment { .. }) => {
            plane_cone_intersections(center, normal, b)
        }
        (Primitive::ConeSegment { .. }, Primitive::Plane { center, normal }) => {
            plane_cone_intersections(center, normal, a)
        }
        (
            Primitive::Plane {
                center: center_a,
                normal: normal_a,
            },
            Primitive::Plane {
                center: center_b,
                normal: normal_b,
            },
        ) => plane_plane_intersections(center_a, normal_a, center_b, normal_b),
        _ => Vec::new(),
    }
}

fn sphere_sphere_intersections(
    center_a: [f64; 3],
    radius_a: f64,
    center_b: [f64; 3],
    radius_b: f64,
) -> Vec<FeatureIntersectionPrimitive> {
    if radius_a <= 0.0 || radius_b <= 0.0 {
        return Vec::new();
    }
    let delta = sub(center_b, center_a);
    let center_distance = length(delta);
    if center_distance <= f64::EPSILON {
        return Vec::new();
    }
    let s = (center_distance + radius_a + radius_b) / 2.0;
    let heron = s * (s - center_distance) * (s - radius_a) * (s - radius_b);
    if heron < 0.0 || !heron.is_finite() {
        return Vec::new();
    }
    let intersection_radius = heron.sqrt() * 2.0 / center_distance;
    if !intersection_radius.is_finite() {
        return Vec::new();
    }
    let dir = scale(delta, 1.0 / center_distance);
    let forward = (radius_a * radius_a - intersection_radius * intersection_radius)
        .max(0.0)
        .sqrt();
    FeatureIntersectionPrimitive::circle(
        add(center_a, scale(dir, forward)),
        dir,
        intersection_radius,
    )
    .into_iter()
    .collect()
}

fn plane_sphere_intersections(
    plane_center: [f64; 3],
    plane_normal: [f64; 3],
    sphere_center: [f64; 3],
    sphere_radius: f64,
) -> Vec<FeatureIntersectionPrimitive> {
    if sphere_radius <= 0.0 {
        return Vec::new();
    }
    let signed = dot(plane_normal, sub(sphere_center, plane_center));
    let exact_distance = signed.abs() - sphere_radius;
    if exact_distance > 0.0 {
        return Vec::new();
    }
    let center = sub(sphere_center, scale(plane_normal, signed));
    let normal = if signed > 0.0 {
        plane_normal
    } else {
        scale(plane_normal, -1.0)
    };
    let radius = (sphere_radius * sphere_radius - signed * signed)
        .max(0.0)
        .sqrt();
    FeatureIntersectionPrimitive::circle(center, normal, radius)
        .into_iter()
        .collect()
}

fn cone_sphere_intersections(
    cone: Primitive,
    sphere_center: [f64; 3],
    sphere_radius: f64,
) -> Vec<FeatureIntersectionPrimitive> {
    if !cone_is_zero_radius(cone) || sphere_radius <= 0.0 {
        return Vec::new();
    }
    let dir = cone_dir(cone);
    let reference = cone_center_point(cone);
    let center_delta = sub(sphere_center, reference);
    let signed_axis_distance = dot(center_delta, dir);
    let axis_to_center = sub(center_delta, scale(dir, signed_axis_distance));
    let axis_distance = length(axis_to_center);
    if axis_distance >= sphere_radius {
        return Vec::new();
    }
    let positive = cone_base_point(cone, true);
    let negative = cone_base_point(cone, false);
    if point_inside_sphere(positive, sphere_center, sphere_radius)
        && point_inside_sphere(negative, sphere_center, sphere_radius)
    {
        return Vec::new();
    }
    let midpoint = sub(sphere_center, axis_to_center);
    let half_len = (sphere_radius * sphere_radius - axis_distance * axis_distance)
        .max(0.0)
        .sqrt();
    let backward = dot(dir, sub(cone_center_point(cone), sphere_center)) < 0.0;
    let first = add(
        midpoint,
        scale(dir, half_len * if backward { -1.0 } else { 1.0 }),
    );
    let second = if point_inside_sphere(positive, sphere_center, sphere_radius) {
        positive
    } else if point_inside_sphere(negative, sphere_center, sphere_radius) {
        negative
    } else {
        add(
            midpoint,
            scale(dir, half_len * if backward { 1.0 } else { -1.0 }),
        )
    };
    FeatureIntersectionPrimitive::line_segment(first, second)
        .into_iter()
        .collect()
}

fn plane_cone_intersections(
    plane_center: [f64; 3],
    plane_normal: [f64; 3],
    cone: Primitive,
) -> Vec<FeatureIntersectionPrimitive> {
    let dir = cone_dir(cone);
    if cone_is_circle(cone) {
        if length(cross(plane_normal, dir)).powi(2) < 0.008_f64.powi(2) {
            return Vec::new();
        }
        return plane_plane_intersections(plane_center, plane_normal, cone_center_point(cone), dir);
    }
    if !cone_is_zero_radius(cone) {
        return Vec::new();
    }
    if dot(plane_normal, dir).abs() < 0.008 {
        return Vec::new();
    }
    plane_line_intersection(plane_center, plane_normal, cone_center_point(cone), dir)
        .and_then(FeatureIntersectionPrimitive::point)
        .into_iter()
        .collect()
}

fn plane_plane_intersections(
    center_a: [f64; 3],
    normal_a: [f64; 3],
    center_b: [f64; 3],
    normal_b: [f64; 3],
) -> Vec<FeatureIntersectionPrimitive> {
    if dot(normal_a, normal_b).abs() >= 0.99995 {
        return Vec::new();
    }
    let Some((point, dir)) = plane_plane_intersection(center_a, normal_a, center_b, normal_b)
    else {
        return Vec::new();
    };
    FeatureIntersectionPrimitive::line(point, dir)
        .into_iter()
        .collect()
}

fn plane_plane_intersection(
    center_a: [f64; 3],
    normal_a: [f64; 3],
    center_b: [f64; 3],
    normal_b: [f64; 3],
) -> Option<([f64; 3], [f64; 3])> {
    let dir = normalize(cross(normal_a, normal_b))?;
    let denom = dot(cross(normal_a, normal_b), cross(normal_a, normal_b));
    if denom <= f64::EPSILON {
        return None;
    }
    let d_a = dot(normal_a, center_a);
    let d_b = dot(normal_b, center_b);
    let point = scale(
        add(
            scale(cross(normal_b, cross(normal_a, normal_b)), d_a),
            scale(cross(cross(normal_a, normal_b), normal_a), d_b),
        ),
        1.0 / denom,
    );
    Some((point, dir))
}

fn point_inside_sphere(point: [f64; 3], sphere_center: [f64; 3], sphere_radius: f64) -> bool {
    dot(sub(point, sphere_center), sub(point, sphere_center)) < sphere_radius * sphere_radius
}

#[allow(dead_code)]
fn arbitrary_intersection_direction(dir: [f64; 3]) -> [f64; 3] {
    normalize(dir).unwrap_or_else(|| arbitrary_perpendicular([0.0, 0.0, 1.0]))
}
