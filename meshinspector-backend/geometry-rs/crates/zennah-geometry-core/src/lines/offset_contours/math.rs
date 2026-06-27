pub(super) struct RoundCornerParams {
    pub lp: [f64; 3],
    pub lc: [f64; 3],
    pub rc: [f64; 3],
    pub rn: [f64; 3],
    pub org: [f64; 3],
    pub lr_ang: f64,
}

pub(super) fn insert_round_corner(
    contour: &mut Vec<[f64; 3]>,
    params: &RoundCornerParams,
    min_angle_precision: f64,
) -> Result<(), String> {
    let steps = (params.lr_ang.abs() / min_angle_precision).floor() as usize;
    if steps == 0 {
        return Ok(());
    }

    let left_vector = sub2(params.lc, params.lp);
    let right_vector = sub2(params.rc, params.rn);
    let left_radial = sub2(params.lc, params.org);
    let right_radial = sub2(params.rc, params.org);
    let round = is_nearly_perpendicular(left_radial, left_vector)
        && is_nearly_perpendicular(right_radial, right_vector);

    if round {
        for step in 0..steps {
            let ratio = (step + 1) as f64 / (steps + 1) as f64;
            contour.push(rotate_around(params.lc, params.org, params.lr_ang * ratio));
        }
        return Ok(());
    }

    let distance = length2(sub2(params.org, params.lc));
    let width = params.lr_ang.abs() / std::f64::consts::PI * 1.5;
    let left_next = add2(
        params.lc,
        scale2(normalize2(left_vector)?, width * distance),
    );
    let right_previous = add2(
        params.rc,
        scale2(normalize2(right_vector)?, width * distance),
    );
    for step in 0..steps {
        let t = (step + 1) as f64 / (steps + 1) as f64;
        contour.push(cubic_bezier2(
            params.lc,
            left_next,
            right_previous,
            params.rc,
            t,
            params.org[2],
        ));
    }
    Ok(())
}

pub(super) fn contour_normal(a: [f64; 3], b: [f64; 3]) -> Result<[f64; 3], String> {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = (dx * dx + dy * dy).sqrt();
    if length <= 1e-12 {
        return Err("OffsetContours contour edges must have positive XY length".to_string());
    }
    Ok([-dy / length, dx / length, 0.0])
}

pub(super) fn find_angle(previous: [f64; 3], origin: [f64; 3], next: [f64; 3]) -> f64 {
    let a = [previous[0] - origin[0], previous[1] - origin[1]];
    let b = [next[0] - origin[0], next[1] - origin[1]];
    let cross = a[0] * b[1] - a[1] * b[0];
    let dot = a[0] * b[0] + a[1] * b[1];
    if cross == 0.0 {
        if dot >= 0.0 {
            0.0
        } else {
            std::f64::consts::PI
        }
    } else {
        cross.atan2(dot)
    }
}

pub(super) fn rotate_around(point: [f64; 3], origin: [f64; 3], angle: f64) -> [f64; 3] {
    let x = point[0] - origin[0];
    let y = point[1] - origin[1];
    let (sin, cos) = angle.sin_cos();
    [
        origin[0] + x * cos - y * sin,
        origin[1] + x * sin + y * cos,
        origin[2],
    ]
}

pub(super) fn signed_area_xy(points: &[[f64; 3]]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>()
        * 0.5
}

pub(super) fn add2(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2]]
}

pub(super) fn sub2(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], 0.0]
}

pub(super) fn scale2(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, 0.0]
}

pub(super) fn cross2(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

pub(super) fn line_intersection_xy(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
) -> Option<[f64; 3]> {
    let ab = sub2(b, a);
    let cd = sub2(d, c);
    let denominator = cross2(ab, cd);
    if denominator.abs() <= 1e-12 {
        return None;
    }
    let t = cross2(sub2(c, a), cd) / denominator;
    Some([a[0] + ab[0] * t, a[1] + ab[1] * t, a[2]])
}

pub(super) fn restore_adjacent_edge_average_z(
    points: &[[f64; 3]],
    index: usize,
    point: [f64; 3],
) -> f64 {
    if points.is_empty() {
        return point[2];
    }
    let previous = if index == 0 {
        points.len() - 1
    } else {
        index - 1
    };
    let next = (index + 1) % points.len();
    let lower_z = interpolate_segment_z(points[previous], points[index], point);
    let upper_z = interpolate_segment_z(points[index], points[next], point);
    (lower_z + upper_z) * 0.5
}

pub(super) fn is_closed_contour(contour: &[[f64; 3]]) -> bool {
    contour
        .first()
        .zip(contour.last())
        .is_some_and(|(first, last)| same_xy(*first, *last))
}

pub(super) fn same_xy(a: [f64; 3], b: [f64; 3]) -> bool {
    (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12
}

fn dot2(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn length2(a: [f64; 3]) -> f64 {
    dot2(a, a).sqrt()
}

fn normalize2(a: [f64; 3]) -> Result<[f64; 3], String> {
    let length = length2(a);
    if length <= 1e-12 {
        return Err(
            "OffsetContours round-corner control vectors must have positive XY length".to_string(),
        );
    }
    Ok([a[0] / length, a[1] / length, 0.0])
}

fn is_nearly_perpendicular(radial: [f64; 3], tangent: [f64; 3]) -> bool {
    let length_sq = dot2(radial, radial);
    length_sq > 1e-24 && (dot2(radial, tangent) / length_sq).abs() < f32::EPSILON as f64 * 10.0
}

fn interpolate_segment_z(a: [f64; 3], b: [f64; 3], point: [f64; 3]) -> f64 {
    let segment = sub2(b, a);
    let denominator = dot2(segment, segment);
    if denominator <= 1e-24 {
        return a[2];
    }
    let ratio = (dot2(sub2(point, a), segment) / denominator).clamp(0.0, 1.0);
    (1.0 - ratio) * a[2] + ratio * b[2]
}

fn cubic_bezier2(
    p1: [f64; 3],
    p2: [f64; 3],
    p3: [f64; 3],
    p4: [f64; 3],
    t: f64,
    z: f64,
) -> [f64; 3] {
    let inv_t = 1.0 - t;
    let t_sq = t * t;
    let inv_t_sq = inv_t * inv_t;
    let t_cb = t_sq * t;
    let inv_t_cb = inv_t_sq * inv_t;
    [
        p1[0] * inv_t_cb + 3.0 * p2[0] * inv_t_sq * t + 3.0 * p3[0] * inv_t * t_sq + p4[0] * t_cb,
        p1[1] * inv_t_cb + 3.0 * p2[1] * inv_t_sq * t + 3.0 * p3[1] * inv_t * t_sq + p4[1] * t_cb,
        z,
    ]
}
