pub fn offset_contours_with_variable_offsets_and_origins(
    contours: &[Vec<[f64; 3]>],
    offsets: &[Vec<f64>],
    options: OffsetContoursOptions,
) -> Result<OffsetContoursResult, String> {
    offset_contours_with_variable_offsets_and_origins_and_z_options(
        contours,
        offsets,
        options,
        OffsetContoursZOptions::default(),
    )
}

pub fn offset_contours_with_variable_offsets_and_origins_and_z_options(
    contours: &[Vec<[f64; 3]>],
    offsets: &[Vec<f64>],
    options: OffsetContoursOptions,
    z_options: OffsetContoursZOptions,
) -> Result<OffsetContoursResult, String> {
    if contours.len() != offsets.len() {
        return Err("OffsetContours variable offsets must match contour count".to_string());
    }
    if !options.min_angle_precision.is_finite() || options.min_angle_precision <= 0.0 {
        return Err("OffsetContours min_angle_precision must be finite and positive".to_string());
    }
    if !options.max_sharp_angle.is_finite() {
        return Err("OffsetContours max_sharp_angle must be finite".to_string());
    }
    let mut output = OffsetContoursResult {
        contours: Vec::new(),
        origins: Vec::new(),
    };
    for (contour_id, (contour, contour_offsets)) in contours.iter().zip(offsets).enumerate() {
        if contour.is_empty() {
            if !contour_offsets.is_empty() {
                return Err("OffsetContours empty contours must have empty offset rows".to_string());
            }
            continue;
        }
        if contour.len() != contour_offsets.len() {
            return Err(
                "OffsetContours variable offsets must match contour point counts".to_string(),
            );
        }
        if contour_offsets.iter().any(|offset| !offset.is_finite()) {
            return Err("OffsetContours variable offsets must be finite".to_string());
        }
        let contour_results = if is_closed_contour(contour) {
            match options.mode {
                OffsetContoursMode::Offset => {
                    vec![offset_closed_clockwise_variable_contour_with_origins(
                        contour,
                        contour_offsets,
                        contour_id,
                        options.min_angle_precision,
                        options.corner_type,
                        options.max_sharp_angle,
                    )?]
                }
                OffsetContoursMode::Shell => offset_closed_clockwise_variable_shell_contours_with_origins(
                    contour,
                    contour_offsets,
                    contour_id,
                    options.min_angle_precision,
                    options.corner_type,
                    options.max_sharp_angle,
                )?,
            }
        } else {
            let magnitudes = contour_offsets
                .iter()
                .map(|offset| offset.abs())
                .collect::<Vec<_>>();
            vec![offset_open_contour_with_origins(
                contour,
                contour_id,
                &magnitudes,
                options.min_angle_precision,
                options.end_type,
                options.corner_type,
                options.max_sharp_angle,
            )?]
        };
        for (contour_points, contour_origins) in contour_results {
            output.contours.push(contour_points);
            output.origins.push(contour_origins);
        }
    }
    apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
    Ok(output)
}

fn offset_closed_clockwise_contour_with_origins(
    contour: &[[f64; 3]],
    contour_id: usize,
    offset: f64,
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>), String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours Type::Offset requires closed contours".to_string());
    }
    if offset <= 0.0 {
        return Err(
            "OffsetContours origins are currently supported for positive closed Type::Offset contours"
                .to_string(),
        );
    }

    let points = &contour[..contour.len() - 1];
    if signed_area_xy(points) >= 0.0 {
        return Err(
            "OffsetContours closed round-corner slice currently supports clockwise contours with positive offsets"
                .to_string(),
        );
    }

    let mut output = Vec::with_capacity(points.len() * 6 + 1);
    let mut origins = Vec::with_capacity(points.len() * 6 + 1);
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
        let origin = OffsetContoursOrigin::source_vertex(contour_id, index);
        if output.last().is_none_or(|last| !same_xy(*last, start)) {
            output.push(start);
            origins.push(origin);
        }

        let angle = find_angle(start, current, end);
        if (angle * offset) < 0.0 {
            match corner_type {
                OffsetContoursCornerType::Round => {
                    let steps = (angle.abs() / min_angle_precision).floor() as usize;
                    for step in 0..steps {
                        let ratio = (step + 1) as f64 / (steps + 1) as f64;
                        output.push(rotate_around(start, current, angle * ratio));
                        origins.push(origin);
                    }
                }
                OffsetContoursCornerType::Sharp => {
                    let before = output.len();
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
                    origins.extend((before..output.len()).map(|_| origin));
                }
            }
        } else if !same_xy(current, start) {
            output.push(current);
            origins.push(origin);
        }
        output.push(end);
        origins.push(origin);
    }
    if let Some(first) = output.first().copied() {
        if output.last().is_none_or(|last| !same_xy(*last, first)) {
            output.push(first);
            let first_origin = origins
                .first()
                .copied()
                .unwrap_or_else(|| OffsetContoursOrigin::source_vertex(contour_id, 0));
            origins.push(first_origin);
        }
    }
    Ok(simplify_self_overlapping_outline_with_origins(
        points,
        contour_id,
        output,
        origins,
        OutlineOriginMode::CanonicalSourceEdges,
    ))
}

fn offset_closed_clockwise_variable_contour_with_origins(
    contour: &[[f64; 3]],
    offsets: &[f64],
    contour_id: usize,
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>), String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours Type::Offset requires closed contours".to_string());
    }
    if contour.len() != offsets.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }

    let points = &contour[..contour.len() - 1];
    let point_offsets = &offsets[..offsets.len() - 1];
    if signed_area_xy(points) >= 0.0 {
        return Err(
            "OffsetContours closed variable-offset slice currently supports clockwise contours with positive offsets"
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
    if has_negative && !has_positive {
        return offset_closed_clockwise_negative_variable_contour_with_origins(
            contour, contour_id, offsets,
        );
    }
    if !has_positive {
        return Ok(identity_contour_with_origins(contour, contour_id));
    }

    let mut output = Vec::with_capacity(points.len() * 6 + 1);
    let mut origins = Vec::with_capacity(points.len() * 6 + 1);
    for index in 0..points.len() {
        let previous = if index == 0 {
            points.len() - 1
        } else {
            index - 1
        };
        let next = (index + 1) % points.len();
        let current = points[index];
        let current_offset = point_offsets[index];
        let previous_offset = point_offsets[previous];
        let next_offset = point_offsets[next];
        let previous_normal = contour_normal(points[previous], current)?;
        let next_normal = contour_normal(current, points[next])?;
        let previous_line_start = add2(points[previous], scale2(previous_normal, previous_offset));
        let start = add2(current, scale2(previous_normal, current_offset));
        let end = add2(current, scale2(next_normal, current_offset));
        let next_line_end = add2(points[next], scale2(next_normal, next_offset));
        let origin = OffsetContoursOrigin::source_vertex(contour_id, index);
        if output.last().is_none_or(|last| !same_xy(*last, start)) {
            output.push(start);
            origins.push(origin);
        }

        let angle = find_angle(start, current, end);
        if (angle * current_offset) < 0.0 {
            let before = output.len();
            match corner_type {
                OffsetContoursCornerType::Round => insert_round_corner(
                    &mut output,
                    &RoundCornerParams {
                        lp: previous_line_start,
                        lc: start,
                        org: current,
                        rc: end,
                        rn: next_line_end,
                        lr_ang: angle,
                    },
                    min_angle_precision,
                )?,
                OffsetContoursCornerType::Sharp => insert_sharp_corner(
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
                ),
            }
            origins.extend((before..output.len()).map(|_| origin));
        } else if !same_xy(current, start) {
            output.push(current);
            origins.push(origin);
        }
        output.push(end);
        origins.push(origin);
    }
    if let Some(first) = output.first().copied() {
        if output.last().is_none_or(|last| !same_xy(*last, first)) {
            output.push(first);
            let first_origin = origins
                .first()
                .copied()
                .unwrap_or_else(|| OffsetContoursOrigin::source_vertex(contour_id, 0));
            origins.push(first_origin);
        }
    }
    if has_positive {
        let uniform_positive = !has_negative
            && point_offsets
                .iter()
                .all(|offset| (*offset - point_offsets[0]).abs() <= 1e-12);
        return Ok(simplify_self_overlapping_outline_with_origins(
            points,
            contour_id,
            output,
            origins,
            if uniform_positive || has_negative {
                OutlineOriginMode::CanonicalSourceEdges
            } else {
                OutlineOriginMode::PositiveVariableSourceEdges
            },
        ));
    }
    Ok((output, origins))
}
