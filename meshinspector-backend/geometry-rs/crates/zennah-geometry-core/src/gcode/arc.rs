use super::{WorkPlane, ACCURACY, ARC_STEP_RADIANS};

pub(super) struct ArcAction {
    pub(super) points: Vec<[f64; 3]>,
    pub(super) warning: Option<String>,
}

pub(super) fn arc_points_from_center(
    center: [f64; 3],
    begin: [f64; 3],
    end: [f64; 3],
    clockwise: bool,
    work_plane: WorkPlane,
) -> ArcAction {
    let center_work = work_plane.to_work(center);
    let begin_work = work_plane.to_work(begin);
    let end_work = work_plane.to_work(end);
    let begin_vec = [
        begin_work[0] - center_work[0],
        begin_work[1] - center_work[1],
    ];
    let end_vec = [end_work[0] - center_work[0], end_work[1] - center_work[1]];
    let mut action = arc_points_in_work(
        center_work,
        begin_work,
        end_work,
        begin_vec,
        end_vec,
        clockwise,
    );
    for point in &mut action.points {
        *point = work_plane.from_work(*point);
    }
    action
}

pub(super) fn arc_points_from_radius(
    begin: [f64; 3],
    end: [f64; 3],
    radius: f64,
    clockwise: bool,
    work_plane: WorkPlane,
) -> ArcAction {
    let begin_work = work_plane.to_work(begin);
    let end_work = work_plane.to_work(end);
    if radius < ACCURACY {
        return ArcAction {
            points: vec![begin, end],
            warning: Some("Wrong radius".to_string()),
        };
    }
    let middle = [
        (begin_work[0] + end_work[0]) * 0.5,
        (begin_work[1] + end_work[1]) * 0.5,
    ];
    let middle_vec = [middle[0] - begin_work[0], middle[1] - begin_work[1]];
    let middle_len_sq = middle_vec[0] * middle_vec[0] + middle_vec[1] * middle_vec[1];
    let normal_len_sq = radius * radius - middle_len_sq;
    if normal_len_sq < 0.0 || middle_len_sq <= 0.0 {
        return ArcAction {
            points: vec![begin, end],
            warning: Some("Wrong radius".to_string()),
        };
    }
    let middle_len = middle_len_sq.sqrt();
    let middle_normal = [middle_vec[1] / middle_len, -middle_vec[0] / middle_len];
    let side = if clockwise == (radius > 0.0) {
        1.0
    } else {
        -1.0
    };
    let center = [
        middle[0] + middle_normal[0] * normal_len_sq.sqrt() * side,
        middle[1] + middle_normal[1] * normal_len_sq.sqrt() * side,
        0.0,
    ];
    let mut action = arc_points_in_work(
        center,
        begin_work,
        end_work,
        [begin_work[0] - center[0], begin_work[1] - center[1]],
        [end_work[0] - center[0], end_work[1] - center[1]],
        clockwise,
    );
    for point in &mut action.points {
        *point = work_plane.from_work(*point);
    }
    action
}

fn arc_points_in_work(
    center: [f64; 3],
    begin: [f64; 3],
    end: [f64; 3],
    begin_vec: [f64; 2],
    end_vec: [f64; 2],
    clockwise: bool,
) -> ArcAction {
    let begin_len_sq = begin_vec[0] * begin_vec[0] + begin_vec[1] * begin_vec[1];
    let end_len_sq = end_vec[0] * end_vec[0] + end_vec[1] * end_vec[1];
    if begin_len_sq <= 0.0 || end_len_sq <= 0.0 {
        return ArcAction {
            points: vec![begin, end],
            warning: Some("Wrong radius".to_string()),
        };
    }

    let max_len_sq = begin_len_sq.max(end_len_sq);
    let delta_len_sq = (begin_len_sq - end_len_sq).abs();
    let warning = if delta_len_sq >= 2.5 * ACCURACY * max_len_sq {
        Some(format!(
            "Begin and end radius are different: diff = {:.6}",
            (delta_len_sq as f32).sqrt()
        ))
    } else {
        None
    };

    let begin_len = begin_len_sq.sqrt();
    let end_len = end_len_sq.sqrt();
    let mut begin_angle = (begin_vec[1] / begin_len).atan2(begin_vec[0] / begin_len);
    let mut end_angle = (end_vec[1] / end_len).atan2(end_vec[0] / end_len);
    if clockwise && begin_angle <= end_angle {
        begin_angle += std::f64::consts::TAU;
    } else if !clockwise && end_angle <= begin_angle {
        end_angle += std::f64::consts::TAU;
    }

    let step_count = ((end_angle - begin_angle).abs() / ARC_STEP_RADIANS)
        .ceil()
        .clamp(10.0, 60.0) as usize;
    let angle_step = (end_angle - begin_angle) / step_count as f64;
    let helical = (begin[2] - end[2]).abs() > ACCURACY;
    let z_step = if step_count > 0 {
        (end[2] - begin[2]) / step_count as f64
    } else {
        0.0
    };
    let mut points = Vec::with_capacity(step_count + 1);
    points.push(begin);
    for step in 1..=step_count {
        let angle = angle_step * step as f64;
        let cos = angle.cos();
        let sin = angle.sin();
        points.push([
            center[0] + cos * begin_vec[0] - sin * begin_vec[1],
            center[1] + sin * begin_vec[0] + cos * begin_vec[1],
            if helical {
                begin[2] + z_step * step as f64
            } else {
                begin[2]
            },
        ]);
    }
    ArcAction { points, warning }
}
