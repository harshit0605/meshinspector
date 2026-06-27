use super::math::{add2, cross2, find_angle, line_intersection_xy, rotate_around, sub2};

pub(super) struct SharpCornerParams {
    pub lp: [f64; 3],
    pub lc: [f64; 3],
    pub rc: [f64; 3],
    pub rn: [f64; 3],
    pub org: [f64; 3],
    pub lr_ang: f64,
}

pub(super) fn insert_sharp_corner(
    contour: &mut Vec<[f64; 3]>,
    params: &SharpCornerParams,
    max_sharp_angle: f64,
) {
    if max_sharp_angle <= 0.0 {
        return;
    }
    let open_angle = cross2(sub2(params.rc, params.lc), sub2(params.rn, params.lc)) * params.lr_ang
        < 0.0
        || cross2(sub2(params.lp, params.rc), sub2(params.lc, params.rc)) * params.lr_ang < 0.0;
    if open_angle {
        return;
    }

    let mut real_angle = find_angle(
        params.rn,
        params.rc,
        add2(params.rc, sub2(params.lp, params.lc)),
    );
    if params.lr_ang < 0.0 {
        real_angle = -std::f64::consts::PI - real_angle;
    } else {
        real_angle = -std::f64::consts::PI + real_angle;
    }
    if cross2(sub2(params.rc, params.rn), sub2(params.lc, params.lp)) * params.lr_ang < 0.0 {
        return;
    }

    let intersection = line_intersection_xy(params.lp, params.lc, params.rn, params.rc);
    if let Some(mut point) = intersection.filter(|_| real_angle.abs() <= max_sharp_angle) {
        point[2] = params.org[2];
        contour.push(point);
        return;
    }
    if real_angle.abs() <= 1e-12 {
        return;
    }

    let mut left_angle_ratio = params.lr_ang * 0.5;
    if let Some(point) = intersection {
        left_angle_ratio = find_angle(params.lc, params.org, point);
    }
    let excess = real_angle.abs() - max_sharp_angle;
    let left_angle =
        left_angle_ratio - real_angle.signum() * excess * left_angle_ratio / real_angle;
    let left_rotated = rotate_around(params.lc, params.org, left_angle);
    if let Some(mut point) = line_intersection_xy(params.lp, params.lc, params.org, left_rotated) {
        point[2] = params.org[2];
        contour.push(point);
    }

    let right_angle_ratio = params.lr_ang - left_angle_ratio;
    let right_angle =
        right_angle_ratio - real_angle.signum() * excess * right_angle_ratio / real_angle;
    let right_rotated = rotate_around(params.rc, params.org, -right_angle);
    if let Some(mut point) = line_intersection_xy(params.rn, params.rc, params.org, right_rotated) {
        point[2] = params.org[2];
        contour.push(point);
    }
}
