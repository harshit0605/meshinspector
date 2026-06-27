use super::super::math::{
    add2, contour_normal, is_closed_contour, line_intersection_xy, restore_adjacent_edge_average_z,
    scale2, signed_area_xy,
};
use super::super::OffsetContoursCornerType;
use super::{
    negative_variable_intersection_origin, offset_closed_clockwise_contour_with_origins,
    offset_closed_clockwise_variable_contour_with_origins, source_edge_ratio, source_index,
    OffsetContoursOrigin, SourceEdge,
};
use crate::lines::validate_contour;

pub(super) fn offset_closed_clockwise_shell_contours_with_origins(
    contour: &[[f64; 3]],
    contour_id: usize,
    offset: f64,
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<Vec<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>)>, String> {
    if offset < 0.0 {
        validate_contour(contour)?;
        return Ok(Vec::new());
    }
    if offset == 0.0 {
        return Err(
            "OffsetContours closed shell slice currently supports positive fixed offsets"
                .to_string(),
        );
    }
    let outer = offset_closed_clockwise_contour_with_origins(
        contour,
        contour_id,
        offset,
        min_angle_precision,
        corner_type,
        max_sharp_angle,
    )?;
    let points = &contour[..contour.len() - 1];
    let point_offsets = vec![offset; points.len()];
    let inner = offset_closed_clockwise_inward_shell_contour_with_origins(
        contour,
        contour_id,
        &point_offsets,
        ShellOriginMapMode::Fixed,
    )?;
    Ok(vec![outer, inner])
}

pub(super) fn offset_closed_clockwise_variable_shell_contours_with_origins(
    contour: &[[f64; 3]],
    offsets: &[f64],
    contour_id: usize,
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<Vec<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>)>, String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours shell mode requires closed contours".to_string());
    }
    if contour.len() != offsets.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }

    let points = &contour[..contour.len() - 1];
    let point_offsets = &offsets[..offsets.len() - 1];
    if signed_area_xy(points) >= 0.0 {
        return Err(
            "OffsetContours closed variable-offset shell slice currently supports clockwise contours"
                .to_string(),
        );
    }
    if (offsets[0] - offsets[offsets.len() - 1]).abs() > 1e-12 {
        return Err(
            "OffsetContours closed variable-offset slice requires matching first and closing offsets"
                .to_string(),
        );
    }
    let has_positive = point_offsets.iter().any(|offset| *offset > 0.0);
    let has_negative = point_offsets.iter().any(|offset| *offset < 0.0);
    if has_positive && has_negative {
        return Err(
            "OffsetContours closed variable-offset shell slice currently supports one signed offset direction"
                .to_string(),
        );
    }
    if has_negative {
        return Ok(Vec::new());
    }

    let outer = offset_closed_clockwise_variable_contour_with_origins(
        contour,
        offsets,
        contour_id,
        min_angle_precision,
        corner_type,
        max_sharp_angle,
    )?;
    let inner = offset_closed_clockwise_inward_shell_contour_with_origins(
        contour,
        contour_id,
        point_offsets,
        ShellOriginMapMode::Variable,
    )?;
    Ok(vec![outer, inner])
}

#[derive(Clone, Copy)]
enum ShellOriginMapMode {
    Fixed,
    Variable,
}

fn offset_closed_clockwise_inward_shell_contour_with_origins(
    contour: &[[f64; 3]],
    contour_id: usize,
    point_offsets: &[f64],
    map_mode: ShellOriginMapMode,
) -> Result<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>), String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours shell mode requires closed contours".to_string());
    }

    let points = &contour[..contour.len() - 1];
    if points.len() != point_offsets.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }
    if signed_area_xy(points) >= 0.0 || point_offsets.iter().any(|offset| *offset < 0.0) {
        return Err(
            "OffsetContours closed shell slice currently supports clockwise contours with positive offsets"
                .to_string(),
        );
    }

    let mut inward = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let previous = if index == 0 {
            points.len() - 1
        } else {
            index - 1
        };
        let next = (index + 1) % points.len();
        let previous_normal = contour_normal(points[previous], points[index])?;
        let next_normal = contour_normal(points[index], points[next])?;
        let previous_line_start = add2(
            points[previous],
            scale2(previous_normal, -point_offsets[previous]),
        );
        let previous_line_end = add2(
            points[index],
            scale2(previous_normal, -point_offsets[index]),
        );
        let next_line_start = add2(points[index], scale2(next_normal, -point_offsets[index]));
        let next_line_end = add2(points[next], scale2(next_normal, -point_offsets[next]));
        let mut point = line_intersection_xy(
            previous_line_start,
            previous_line_end,
            next_line_start,
            next_line_end,
        )
        .unwrap_or(next_line_start);
        point[2] = restore_adjacent_edge_average_z(points, index, point);
        inward.push(point);
    }

    let start_index = usize::from(inward.len() > 1);
    let mut output = Vec::with_capacity(inward.len() + 1);
    let mut origins = Vec::with_capacity(inward.len() + 1);
    output.push(inward[start_index]);
    origins.push(shell_intersection_origin(
        contour_id,
        start_index,
        points,
        inward[start_index],
        map_mode,
    ));
    let mut index = start_index;
    loop {
        index = if index == 0 {
            inward.len() - 1
        } else {
            index - 1
        };
        output.push(inward[index]);
        origins.push(shell_intersection_origin(
            contour_id,
            index,
            points,
            inward[index],
            map_mode,
        ));
        if index == start_index {
            break;
        }
    }
    Ok((output, origins))
}

fn shell_intersection_origin(
    contour_id: usize,
    index: usize,
    points: &[[f64; 3]],
    intersection: [f64; 3],
    map_mode: ShellOriginMapMode,
) -> OffsetContoursOrigin {
    match map_mode {
        ShellOriginMapMode::Fixed => {
            fixed_shell_intersection_origin(contour_id, index, points, intersection)
        }
        ShellOriginMapMode::Variable => {
            negative_variable_intersection_origin(contour_id, index, points, intersection)
        }
    }
}

fn fixed_shell_intersection_origin(
    contour_id: usize,
    index: usize,
    points: &[[f64; 3]],
    intersection: [f64; 3],
) -> OffsetContoursOrigin {
    let last = points.len() - 1;
    let (lower, upper) = if index == 0 {
        (
            SourceEdge { org: 0, dest: 1 },
            SourceEdge { org: 0, dest: last },
        )
    } else if index == last {
        (
            SourceEdge { org: 0, dest: last },
            SourceEdge {
                org: last - 1,
                dest: last,
            },
        )
    } else {
        (
            SourceEdge {
                org: index - 1,
                dest: index,
            },
            SourceEdge {
                org: index,
                dest: index + 1,
            },
        )
    };
    OffsetContoursOrigin {
        l_org: source_index(contour_id, lower.org),
        l_dest: source_index(contour_id, lower.dest),
        u_org: source_index(contour_id, upper.org),
        u_dest: source_index(contour_id, upper.dest),
        l_ratio: source_edge_ratio(lower, points, intersection),
        u_ratio: source_edge_ratio(upper, points, intersection),
    }
}
