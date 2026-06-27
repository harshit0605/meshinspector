fn offset_closed_clockwise_contour(
    contour: &[[f64; 3]],
    offset: f64,
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<Vec<[f64; 3]>, String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours Type::Offset requires closed contours".to_string());
    }
    if offset == 0.0 {
        return Ok(contour.to_vec());
    }

    let points = &contour[..contour.len() - 1];
    if signed_area_xy(points) >= 0.0 {
        return Err(
            "OffsetContours closed round-corner slice currently supports clockwise contours with positive offsets"
                .to_string(),
        );
    }
    if offset < 0.0 {
        let offsets = vec![offset; points.len()];
        return offset_closed_clockwise_signed_inward_contour(contour, &offsets);
    }

    let mut output = Vec::with_capacity(points.len() * 6 + 1);
    for index in 0..points.len() {
        let previous = if index == 0 {
            points.len() - 1
        } else {
            index - 1
        };
        let next = (index + 1) % points.len();
        let current = points[index];
        let previous_normal = contour_normal(points[previous], current)?;
        let next_normal = contour_normal(current, points[next])?;
        let previous_line_start = add2(points[previous], scale2(previous_normal, offset));
        let start = add2(current, scale2(previous_normal, offset));
        let end = add2(current, scale2(next_normal, offset));
        let next_line_end = add2(points[next], scale2(next_normal, offset));
        if output.last().is_none_or(|last| !same_xy(*last, start)) {
            output.push(start);
        }

        let angle = find_angle(start, current, end);
        if (angle * offset) < 0.0 {
            match corner_type {
                OffsetContoursCornerType::Round => {
                    let steps = (angle.abs() / min_angle_precision).floor() as usize;
                    for step in 0..steps {
                        let ratio = (step + 1) as f64 / (steps + 1) as f64;
                        output.push(rotate_around(start, current, angle * ratio));
                    }
                }
                OffsetContoursCornerType::Sharp => {
                    insert_sharp_corner(
                        &mut output,
                        &SharpCornerParams {
                            lp: previous_line_start,
                            lc: start,
                            rc: end,
                            rn: next_line_end,
                            org: current,
                            lr_ang: angle,
                        },
                        max_sharp_angle,
                    );
                }
            }
        } else if !same_xy(current, start) {
            output.push(current);
        }
        output.push(end);
    }
    if let Some(first) = output.first().copied() {
        if output.last().is_none_or(|last| !same_xy(*last, first)) {
            output.push(first);
        }
    }
    Ok(output)
}

fn offset_closed_clockwise_shell_contours(
    contour: &[[f64; 3]],
    offset: f64,
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    if offset < 0.0 {
        let _ = offset_closed_clockwise_contour(
            contour,
            offset,
            min_angle_precision,
            corner_type,
            max_sharp_angle,
        )?;
        return Ok(Vec::new());
    }
    if offset == 0.0 {
        return Err(
            "OffsetContours closed shell slice currently supports positive fixed offsets"
                .to_string(),
        );
    }
    let outer = offset_closed_clockwise_contour(
        contour,
        offset,
        min_angle_precision,
        corner_type,
        max_sharp_angle,
    )?;
    let inner = offset_closed_clockwise_inward_shell_contour(contour, offset)?;
    Ok(vec![outer, inner])
}

fn offset_closed_clockwise_inward_shell_contour(
    contour: &[[f64; 3]],
    offset: f64,
) -> Result<Vec<[f64; 3]>, String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours shell mode requires closed contours".to_string());
    }

    let points = &contour[..contour.len() - 1];
    if signed_area_xy(points) >= 0.0 {
        return Err(
            "OffsetContours closed shell slice currently supports clockwise contours".to_string(),
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
        let previous_line_start = add2(points[previous], scale2(previous_normal, -offset));
        let previous_line_end = add2(points[index], scale2(previous_normal, -offset));
        let next_line_start = add2(points[index], scale2(next_normal, -offset));
        let next_line_end = add2(points[next], scale2(next_normal, -offset));
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
    output.push(inward[start_index]);
    let mut index = start_index;
    loop {
        index = if index == 0 {
            inward.len() - 1
        } else {
            index - 1
        };
        output.push(inward[index]);
        if index == start_index {
            break;
        }
    }
    Ok(output)
}

