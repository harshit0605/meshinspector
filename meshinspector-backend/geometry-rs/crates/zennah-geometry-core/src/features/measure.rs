use super::support::{
    arbitrary_perpendicular, closest_point_on_segment, closest_points_on_segments, cone_base_point,
    cone_center_point, cone_dir, cone_guess_angle_dir, cone_has_equal_radii, cone_is_circle,
    cone_is_zero_radius, cone_positive_radius, cone_radius, cross, plane_line_intersection,
};
use super::{
    add, dot, length, normalize, scale, sub, FeatureAnglePart, FeatureDistancePart,
    FeatureMeasureStatus, Primitive,
};

pub(super) fn measure_distance(a: Primitive, b: Primitive) -> FeatureDistancePart {
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
        ) => sphere_to_sphere_distance(center_a, radius_a, center_b, radius_b),
        (Primitive::ConeSegment { .. }, Primitive::Sphere { center, radius }) => {
            cone_to_sphere_distance(a, center, radius, false)
        }
        (Primitive::Sphere { center, radius }, Primitive::ConeSegment { .. }) => {
            cone_to_sphere_distance(b, center, radius, true)
        }
        (
            Primitive::Plane { center, normal },
            Primitive::Sphere {
                center: sphere_center,
                radius,
            },
        ) => plane_to_sphere_distance(center, normal, sphere_center, radius, false),
        (
            Primitive::Sphere {
                center: sphere_center,
                radius,
            },
            Primitive::Plane { center, normal },
        ) => plane_to_sphere_distance(center, normal, sphere_center, radius, true),
        (Primitive::ConeSegment { .. }, Primitive::ConeSegment { .. }) => {
            if cone_is_zero_radius(a) && cone_is_zero_radius(b) {
                zero_radius_cone_exact_distance(a, b)
            } else {
                FeatureDistancePart::status(FeatureMeasureStatus::NotImplemented)
            }
        }
        (Primitive::Plane { center, normal }, Primitive::ConeSegment { .. }) => {
            plane_to_cone_distance(center, normal, b, false)
        }
        (Primitive::ConeSegment { .. }, Primitive::Plane { center, normal }) => {
            plane_to_cone_distance(center, normal, a, true)
        }
        (Primitive::Plane { .. }, Primitive::Plane { .. }) => {
            FeatureDistancePart::status(FeatureMeasureStatus::BadFeaturePair)
        }
    }
}

pub(super) fn measure_center_distance(a: Primitive, b: Primitive) -> FeatureDistancePart {
    match (a, b) {
        (
            Primitive::Sphere {
                center: center_a, ..
            },
            Primitive::Sphere {
                center: center_b, ..
            },
        ) => point_to_point_distance(center_a, center_b),
        (Primitive::ConeSegment { .. }, Primitive::Sphere { center, .. }) => {
            cone_to_point_center_distance(a, center, false)
        }
        (Primitive::Sphere { center, .. }, Primitive::ConeSegment { .. }) => {
            cone_to_point_center_distance(b, center, true)
        }
        (
            Primitive::Plane { center, normal },
            Primitive::Sphere {
                center: sphere_center,
                ..
            },
        ) => plane_to_point_center_distance(center, normal, sphere_center, false),
        (
            Primitive::Sphere {
                center: sphere_center,
                ..
            },
            Primitive::Plane { center, normal },
        ) => plane_to_point_center_distance(center, normal, sphere_center, true),
        (Primitive::ConeSegment { .. }, Primitive::ConeSegment { .. }) => {
            cone_to_cone_center_distance(a, b)
        }
        (Primitive::Plane { center, normal }, Primitive::ConeSegment { .. }) => {
            plane_to_point_center_distance(center, normal, cone_center_point(b), false)
        }
        (Primitive::ConeSegment { .. }, Primitive::Plane { center, normal }) => {
            plane_to_point_center_distance(center, normal, cone_center_point(a), true)
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
        ) => plane_to_plane_center_distance(center_a, normal_a, center_b, normal_b),
    }
}

pub(super) fn measure_angle(a: Primitive, b: Primitive) -> FeatureAnglePart {
    match (a, b) {
        (Primitive::Sphere { .. }, Primitive::Sphere { .. }) => {
            FeatureAnglePart::status(FeatureMeasureStatus::BadFeaturePair)
        }
        (
            Primitive::Plane { normal, center },
            Primitive::Plane {
                normal: normal_b,
                center: center_b,
            },
        ) => FeatureAnglePart::ok(center, center_b, normal, normal_b, true, true),
        (Primitive::Plane { center, normal }, Primitive::ConeSegment { dir, .. }) => {
            FeatureAnglePart::ok(
                center,
                cone_center_point(b),
                normal,
                dir,
                true,
                cone_is_circle(b),
            )
        }
        (Primitive::ConeSegment { dir, .. }, Primitive::Plane { center, normal }) => {
            FeatureAnglePart::ok(
                cone_center_point(a),
                center,
                dir,
                normal,
                cone_is_circle(a),
                true,
            )
        }
        (Primitive::ConeSegment { .. }, Primitive::ConeSegment { .. }) => {
            if !cone_has_equal_radii(a) || !cone_has_equal_radii(b) {
                return FeatureAnglePart::status(FeatureMeasureStatus::BadFeaturePair);
            }
            let distance = measure_distance(a, b);
            let point_a = distance
                .closest_point_a
                .unwrap_or_else(|| cone_center_point(a));
            let point_b = distance
                .closest_point_b
                .unwrap_or_else(|| cone_center_point(b));
            FeatureAnglePart::ok(
                point_a,
                point_b,
                cone_guess_angle_dir(a, point_a),
                cone_guess_angle_dir(b, point_b),
                false,
                false,
            )
        }
        _ => FeatureAnglePart::status(FeatureMeasureStatus::BadFeaturePair),
    }
}

fn sphere_to_sphere_distance(
    center_a: [f64; 3],
    radius_a: f64,
    center_b: [f64; 3],
    radius_b: f64,
) -> FeatureDistancePart {
    let delta = sub(center_b, center_a);
    let center_distance = length(delta);
    let dir = if center_distance > 0.0 {
        scale(delta, 1.0 / center_distance)
    } else {
        [1.0, 0.0, 0.0]
    };
    FeatureDistancePart::ok(
        center_distance - radius_a - radius_b,
        add(center_a, scale(dir, radius_a)),
        sub(center_b, scale(dir, radius_b)),
    )
}

fn cone_to_sphere_distance(
    cone: Primitive,
    sphere_center: [f64; 3],
    sphere_radius: f64,
    swapped: bool,
) -> FeatureDistancePart {
    if cone_is_zero_radius(cone) {
        return line_segment_to_sphere_distance(cone, sphere_center, sphere_radius, swapped);
    }
    if cone_is_circle(cone) {
        return circle_to_sphere_distance(cone, sphere_center, sphere_radius, swapped);
    }
    if cone_has_equal_radii(cone) {
        return cylinder_to_sphere_distance(cone, sphere_center, sphere_radius, swapped);
    }
    FeatureDistancePart::status(FeatureMeasureStatus::NotImplemented)
}

fn line_segment_to_sphere_distance(
    cone: Primitive,
    sphere_center: [f64; 3],
    sphere_radius: f64,
    swapped: bool,
) -> FeatureDistancePart {
    let point_a = closest_point_on_segment(
        cone_base_point(cone, true),
        cone_base_point(cone, false),
        sphere_center,
    );
    let delta = sub(sphere_center, point_a);
    let center_distance = length(delta);
    let dir = if center_distance > 0.0 {
        scale(delta, 1.0 / center_distance)
    } else {
        arbitrary_perpendicular(cone_dir(cone))
    };
    distance_part_with_optional_swap(
        center_distance - sphere_radius,
        point_a,
        sub(sphere_center, scale(dir, sphere_radius)),
        swapped,
    )
}

fn circle_to_sphere_distance(
    cone: Primitive,
    sphere_center: [f64; 3],
    sphere_radius: f64,
    swapped: bool,
) -> FeatureDistancePart {
    let center = cone_center_point(cone);
    let dir = cone_dir(cone);
    let radial = sub(
        sphere_center,
        add(center, scale(dir, dot(sub(sphere_center, center), dir))),
    );
    let radial_dir = normalize(radial).unwrap_or_else(|| arbitrary_perpendicular(dir));
    let point_a = add(center, scale(radial_dir, cone_positive_radius(cone)));
    let delta = sub(sphere_center, point_a);
    let center_distance = length(delta);
    let sphere_dir = if center_distance > 0.0 {
        scale(delta, 1.0 / center_distance)
    } else {
        radial_dir
    };
    distance_part_with_optional_swap(
        center_distance - sphere_radius,
        point_a,
        sub(sphere_center, scale(sphere_dir, sphere_radius)),
        swapped,
    )
}

fn cylinder_to_sphere_distance(
    cone: Primitive,
    sphere_center: [f64; 3],
    sphere_radius: f64,
    swapped: bool,
) -> FeatureDistancePart {
    let Primitive::ConeSegment {
        reference_point,
        dir,
        positive_side_radius,
        positive_length,
        negative_length,
        ..
    } = cone
    else {
        unreachable!("expected cone segment primitive");
    };
    let delta = sub(sphere_center, reference_point);
    let axis_pos = dot(delta, dir);
    let axis_point = add(reference_point, scale(dir, axis_pos));
    let radial = sub(sphere_center, axis_point);
    let radial_dist = length(radial);
    let radial_dir = normalize(radial).unwrap_or_else(|| arbitrary_perpendicular(dir));
    let side_signed = radial_dist - positive_side_radius;
    let positive_cap_signed = axis_pos - positive_length;
    let negative_cap_signed = -negative_length - axis_pos;
    let outside_positive = positive_cap_signed > 0.0;
    let outside_negative = negative_cap_signed > 0.0;

    let (signed_distance, point_a, normal) =
        if side_signed <= 0.0 && !outside_positive && !outside_negative {
            let cap_signed = positive_cap_signed.max(negative_cap_signed);
            if side_signed > cap_signed {
                (
                    side_signed,
                    add(axis_point, scale(radial_dir, positive_side_radius)),
                    radial_dir,
                )
            } else {
                let positive = positive_cap_signed >= negative_cap_signed;
                let cap = if positive {
                    positive_length
                } else {
                    -negative_length
                };
                let normal = if positive { dir } else { scale(dir, -1.0) };
                (
                    cap_signed,
                    add(add(reference_point, scale(dir, cap)), radial),
                    normal,
                )
            }
        } else if side_signed > 0.0 && !outside_positive && !outside_negative {
            (
                side_signed,
                add(axis_point, scale(radial_dir, positive_side_radius)),
                radial_dir,
            )
        } else if side_signed <= 0.0 {
            let positive = outside_positive;
            let cap = if positive {
                positive_length
            } else {
                -negative_length
            };
            let normal = if positive { dir } else { scale(dir, -1.0) };
            (
                if positive {
                    positive_cap_signed
                } else {
                    negative_cap_signed
                },
                add(add(reference_point, scale(dir, cap)), radial),
                normal,
            )
        } else {
            let positive = positive_cap_signed > negative_cap_signed;
            let cap = if positive {
                positive_length
            } else {
                -negative_length
            };
            let along = if positive {
                positive_cap_signed
            } else {
                negative_cap_signed
            };
            let normal = normalize(add(
                scale(radial_dir, side_signed),
                scale(if positive { dir } else { scale(dir, -1.0) }, along),
            ))
            .unwrap_or(radial_dir);
            (
                (side_signed * side_signed + along * along).sqrt(),
                add(
                    add(reference_point, scale(dir, cap)),
                    scale(radial_dir, positive_side_radius),
                ),
                normal,
            )
        };

    distance_part_with_optional_swap(
        signed_distance - sphere_radius,
        point_a,
        sub(sphere_center, scale(normal, sphere_radius)),
        swapped,
    )
}

fn plane_to_sphere_distance(
    plane_center: [f64; 3],
    plane_normal: [f64; 3],
    sphere_center: [f64; 3],
    sphere_radius: f64,
    swapped: bool,
) -> FeatureDistancePart {
    let signed_center_distance = dot(plane_normal, sub(sphere_center, plane_center));
    let sphere_normal = if signed_center_distance >= 0.0 {
        plane_normal
    } else {
        scale(plane_normal, -1.0)
    };
    distance_part_with_optional_swap(
        signed_center_distance.abs() - sphere_radius,
        sub(sphere_center, scale(plane_normal, signed_center_distance)),
        sub(sphere_center, scale(sphere_normal, sphere_radius)),
        swapped,
    )
}

fn plane_to_cone_distance(
    plane_center: [f64; 3],
    plane_normal: [f64; 3],
    cone: Primitive,
    swapped: bool,
) -> FeatureDistancePart {
    let dir = cone_dir(cone);
    let mut side_dir = normalize(cross(cross(plane_normal, dir), dir))
        .unwrap_or_else(|| arbitrary_perpendicular(dir));
    if !side_dir.iter().all(|value| value.is_finite()) {
        side_dir = arbitrary_perpendicular(dir);
    }

    let mut have_positive = false;
    let mut have_negative = false;
    let mut first = true;
    let mut max_dist = 0.0;
    let mut min_dist = 0.0;
    let mut max_dist_point = [0.0, 0.0, 0.0];
    let mut min_dist_point = [0.0, 0.0, 0.0];

    for positive_side in [true, false] {
        let cap_center = cone_base_point(cone, positive_side);
        let side_radius = cone_radius(cone, positive_side);
        for point in [
            add(cap_center, scale(side_dir, side_radius)),
            sub(cap_center, scale(side_dir, side_radius)),
        ] {
            let dist = dot(plane_normal, sub(point, plane_center));
            if dist < 0.0 {
                have_negative = true;
            } else {
                have_positive = true;
            }
            if first || dist < min_dist {
                min_dist = dist;
                min_dist_point = point;
            }
            if first || dist > max_dist {
                max_dist = dist;
                max_dist_point = point;
            }
            first = false;
        }
    }

    if !have_positive && !have_negative {
        return FeatureDistancePart::status(FeatureMeasureStatus::BadRelativeLocation);
    }

    let (mut distance, point_b) = if !have_positive || (have_negative && max_dist < -min_dist) {
        (max_dist, max_dist_point)
    } else {
        (min_dist, min_dist_point)
    };
    distance = distance.abs();
    if have_positive && have_negative {
        distance = -distance;
    }
    let point_a = sub(
        point_b,
        scale(plane_normal, dot(plane_normal, sub(point_b, plane_center))),
    );
    distance_part_with_optional_swap(distance, point_a, point_b, swapped)
}

fn cone_to_point_center_distance(
    cone: Primitive,
    point: [f64; 3],
    swapped: bool,
) -> FeatureDistancePart {
    if cone_is_zero_radius(cone) {
        let closest = closest_point_on_segment(
            cone_base_point(cone, true),
            cone_base_point(cone, false),
            point,
        );
        return distance_with_optional_swap(closest, point, swapped);
    }
    distance_with_optional_swap(cone_center_point(cone), point, swapped)
}

fn cone_to_cone_center_distance(a: Primitive, b: Primitive) -> FeatureDistancePart {
    if cone_is_circle(a) && cone_is_circle(b) {
        return point_to_point_distance(cone_center_point(a), cone_center_point(b));
    }
    if cone_is_circle(a) {
        return cone_to_point_center_distance(b, cone_center_point(a), true);
    }
    if cone_is_circle(b) {
        return cone_to_point_center_distance(a, cone_center_point(b), false);
    }
    if !cone_is_zero_radius(a) || !cone_is_zero_radius(b) {
        return zero_radius_cone_center_distance(
            cone_with_zero_radius(a),
            cone_with_zero_radius(b),
        );
    }
    zero_radius_cone_center_distance(a, b)
}

fn zero_radius_cone_exact_distance(a: Primitive, b: Primitive) -> FeatureDistancePart {
    let Some((point_a, point_b)) = closest_points_on_segments(
        cone_base_point(a, true),
        cone_base_point(a, false),
        cone_base_point(b, true),
        cone_base_point(b, false),
    ) else {
        return FeatureDistancePart::status(FeatureMeasureStatus::BadRelativeLocation);
    };
    FeatureDistancePart::ok(length(sub(point_b, point_a)), point_a, point_b)
}

fn zero_radius_cone_center_distance(a: Primitive, b: Primitive) -> FeatureDistancePart {
    let dir_dot = dot(cone_dir(a), cone_dir(b));
    if dir_dot.abs() < 0.7071067811865475 {
        return zero_radius_cone_exact_distance(a, b);
    }

    let a_length = cone_length(a);
    let b_length = cone_length(b);
    if a_length.is_finite() != b_length.is_finite() {
        return if a_length.is_finite() {
            cone_to_point_center_distance(b, cone_center_point(a), true)
        } else {
            cone_to_point_center_distance(a, cone_center_point(b), false)
        };
    }

    let mut b_dir_fixed = cone_dir(b);
    if dir_dot < 0.0 {
        b_dir_fixed = scale(b_dir_fixed, -1.0);
    }
    let Some(average_dir) = normalize(add(cone_dir(a), b_dir_fixed)) else {
        return FeatureDistancePart::status(FeatureMeasureStatus::BadRelativeLocation);
    };
    let denom = dot(average_dir, cone_dir(a));
    if denom.abs() <= f64::EPSILON {
        return FeatureDistancePart::status(FeatureMeasureStatus::BadRelativeLocation);
    }

    let a_center = cone_center_point(a);
    let b_center = cone_center_point(b);
    let offset = dot(sub(b_center, a_center), average_dir) / denom;
    let point_a = add(a_center, scale(cone_dir(a), offset));
    let point_b = add(b_center, scale(b_dir_fixed, offset));
    FeatureDistancePart::ok(length(sub(point_b, point_a)), point_a, point_b)
}

fn cone_length(cone: Primitive) -> f64 {
    let Primitive::ConeSegment {
        positive_length,
        negative_length,
        ..
    } = cone
    else {
        unreachable!("expected cone segment primitive");
    };
    positive_length + negative_length
}

fn cone_with_zero_radius(cone: Primitive) -> Primitive {
    let Primitive::ConeSegment {
        reference_point,
        dir,
        positive_length,
        negative_length,
        ..
    } = cone
    else {
        unreachable!("expected cone segment primitive");
    };
    Primitive::ConeSegment {
        reference_point,
        dir,
        positive_side_radius: 0.0,
        negative_side_radius: 0.0,
        positive_length,
        negative_length,
    }
}

fn plane_to_point_center_distance(
    plane_center: [f64; 3],
    plane_normal: [f64; 3],
    point: [f64; 3],
    swapped: bool,
) -> FeatureDistancePart {
    let signed = dot(plane_normal, sub(point, plane_center));
    distance_with_optional_swap(sub(point, scale(plane_normal, signed)), point, swapped)
}

fn plane_to_plane_center_distance(
    center_a: [f64; 3],
    normal_a: [f64; 3],
    center_b: [f64; 3],
    normal_b: [f64; 3],
) -> FeatureDistancePart {
    let mut normal_b_fixed = normal_b;
    if dot(normal_a, normal_b) < 0.0 {
        normal_b_fixed = scale(normal_b_fixed, -1.0);
    }
    let Some(average_normal) = normalize(add(normal_a, normal_b_fixed)) else {
        return FeatureDistancePart::status(FeatureMeasureStatus::BadRelativeLocation);
    };
    let center_b_projected = sub(
        center_b,
        scale(average_normal, dot(average_normal, sub(center_b, center_a))),
    );
    let average_center = add(center_a, scale(sub(center_b_projected, center_a), 0.5));
    let Some(point_a) = plane_line_intersection(center_a, normal_a, average_center, average_normal)
    else {
        return FeatureDistancePart::status(FeatureMeasureStatus::BadRelativeLocation);
    };
    let Some(point_b) = plane_line_intersection(center_b, normal_b, average_center, average_normal)
    else {
        return FeatureDistancePart::status(FeatureMeasureStatus::BadRelativeLocation);
    };
    FeatureDistancePart::ok(length(sub(point_b, point_a)), point_a, point_b)
}

fn distance_part_with_optional_swap(
    distance: f64,
    point_a: [f64; 3],
    point_b: [f64; 3],
    swapped: bool,
) -> FeatureDistancePart {
    if swapped {
        FeatureDistancePart::ok(distance, point_b, point_a)
    } else {
        FeatureDistancePart::ok(distance, point_a, point_b)
    }
}

fn distance_with_optional_swap(
    point_a: [f64; 3],
    point_b: [f64; 3],
    swapped: bool,
) -> FeatureDistancePart {
    distance_part_with_optional_swap(length(sub(point_b, point_a)), point_a, point_b, swapped)
}

fn point_to_point_distance(point_a: [f64; 3], point_b: [f64; 3]) -> FeatureDistancePart {
    FeatureDistancePart::ok(length(sub(point_b, point_a)), point_a, point_b)
}
