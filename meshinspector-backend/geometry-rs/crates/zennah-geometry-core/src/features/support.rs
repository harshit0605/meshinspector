use super::{add, dot, normalize, scale, sub, Primitive};

pub(super) fn cone_center_point(cone: Primitive) -> [f64; 3] {
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
    add(
        reference_point,
        scale(dir, (positive_length - negative_length) / 2.0),
    )
}

pub(super) fn cone_base_point(cone: Primitive, positive: bool) -> [f64; 3] {
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
    add(
        reference_point,
        scale(
            dir,
            if positive {
                positive_length
            } else {
                -negative_length
            },
        ),
    )
}

pub(super) fn cone_dir(cone: Primitive) -> [f64; 3] {
    let Primitive::ConeSegment { dir, .. } = cone else {
        unreachable!("expected cone segment primitive");
    };
    dir
}

pub(super) fn cone_positive_radius(cone: Primitive) -> f64 {
    cone_radius(cone, true)
}

pub(super) fn cone_radius(cone: Primitive, positive: bool) -> f64 {
    let Primitive::ConeSegment {
        positive_side_radius,
        negative_side_radius,
        ..
    } = cone
    else {
        unreachable!("expected cone segment primitive");
    };
    if positive {
        positive_side_radius
    } else {
        negative_side_radius
    }
}

pub(super) fn cone_is_zero_radius(cone: Primitive) -> bool {
    matches!(
        cone,
        Primitive::ConeSegment {
            positive_side_radius: 0.0,
            negative_side_radius: 0.0,
            ..
        }
    )
}

pub(super) fn cone_has_equal_radii(cone: Primitive) -> bool {
    match cone {
        Primitive::ConeSegment {
            positive_side_radius,
            negative_side_radius,
            ..
        } => (positive_side_radius - negative_side_radius).abs() <= f64::EPSILON,
        _ => false,
    }
}

pub(super) fn cone_is_circle(cone: Primitive) -> bool {
    match cone {
        Primitive::ConeSegment {
            positive_length,
            negative_length,
            ..
        } => (positive_length + negative_length).abs() <= f64::EPSILON,
        _ => false,
    }
}

pub(super) fn cone_guess_angle_dir(cone: Primitive, angle_point: [f64; 3]) -> [f64; 3] {
    let dir = cone_dir(cone);
    let center = cone_center_point(cone);
    if dot(dir, sub(center, angle_point)) < 0.0 {
        scale(dir, -1.0)
    } else {
        dir
    }
}

pub(super) fn plane_line_intersection(
    plane_center: [f64; 3],
    plane_normal: [f64; 3],
    line_point: [f64; 3],
    line_dir: [f64; 3],
) -> Option<[f64; 3]> {
    let denom = dot(line_dir, plane_normal);
    if denom.abs() <= 1e-12 {
        return None;
    }
    Some(sub(
        line_point,
        scale(
            line_dir,
            dot(sub(line_point, plane_center), plane_normal) / denom,
        ),
    ))
}

pub(super) fn closest_point_on_segment(a: [f64; 3], b: [f64; 3], point: [f64; 3]) -> [f64; 3] {
    let ab = sub(b, a);
    let denom = dot(ab, ab);
    if denom <= f64::EPSILON {
        return a;
    }
    let t = (dot(sub(point, a), ab) / denom).clamp(0.0, 1.0);
    add(a, scale(ab, t))
}

pub(super) fn closest_points_on_segments(
    p1: [f64; 3],
    q1: [f64; 3],
    p2: [f64; 3],
    q2: [f64; 3],
) -> Option<([f64; 3], [f64; 3])> {
    let d1 = sub(q1, p1);
    let d2 = sub(q2, p2);
    let r = sub(p1, p2);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let f = dot(d2, r);
    let (s, t) = if a <= f64::EPSILON && e <= f64::EPSILON {
        (0.0, 0.0)
    } else if a <= f64::EPSILON {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= f64::EPSILON {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let s = if denom.abs() > f64::EPSILON {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let tnom = b * s + f;
            if tnom < 0.0 {
                ((-c / a).clamp(0.0, 1.0), 0.0)
            } else if tnom > e {
                (((b - c) / a).clamp(0.0, 1.0), 1.0)
            } else {
                (s, tnom / e)
            }
        }
    };
    let point_a = add(p1, scale(d1, s));
    let point_b = add(p2, scale(d2, t));
    if point_a
        .iter()
        .chain(point_b.iter())
        .all(|value| value.is_finite())
    {
        Some((point_a, point_b))
    } else {
        None
    }
}

pub(super) fn arbitrary_perpendicular(dir: [f64; 3]) -> [f64; 3] {
    normalize(cross(dir, furthest_basis_vector(dir))).unwrap_or([1.0, 0.0, 0.0])
}

fn furthest_basis_vector(vector: [f64; 3]) -> [f64; 3] {
    let abs = [vector[0].abs(), vector[1].abs(), vector[2].abs()];
    if abs[0] <= abs[1] && abs[0] <= abs[2] {
        [1.0, 0.0, 0.0]
    } else if abs[1] <= abs[2] {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    }
}

pub(super) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
