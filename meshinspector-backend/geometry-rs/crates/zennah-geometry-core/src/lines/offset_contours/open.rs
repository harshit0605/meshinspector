fn offset_open_round_contour(
    contour: &[[f64; 3]],
    offset: f64,
    min_angle_precision: f64,
    end_type: OffsetContoursEndType,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<Vec<[f64; 3]>, String> {
    validate_contour(contour)?;
    if contour.len() < 2 {
        return Err(
            "OffsetContours open round-end slice requires at least two contour points".to_string(),
        );
    }
    if offset == 0.0 {
        return Ok(contour.to_vec());
    }

    let magnitudes = vec![offset.abs(); contour.len()];
    offset_open_contour_with_magnitudes(
        contour,
        &magnitudes,
        min_angle_precision,
        end_type,
        corner_type,
        max_sharp_angle,
    )
}

fn offset_open_contour_with_magnitudes(
    contour: &[[f64; 3]],
    magnitudes: &[f64],
    min_angle_precision: f64,
    end_type: OffsetContoursEndType,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<Vec<[f64; 3]>, String> {
    validate_contour(contour)?;
    if contour.len() < 2 {
        return Err(
            "OffsetContours open round-end slice requires at least two contour points".to_string(),
        );
    }
    if contour.len() != magnitudes.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }

    let backward_offsets = magnitudes.iter().map(|offset| -*offset).collect::<Vec<_>>();
    let mut output = offset_one_direction_open_contour(
        contour,
        magnitudes,
        min_angle_precision,
        corner_type,
        max_sharp_angle,
    )?;
    let mut backward = offset_one_direction_open_contour(
        contour,
        &backward_offsets,
        min_angle_precision,
        corner_type,
        max_sharp_angle,
    )?;
    backward.reverse();

    if output.len() < 2 || backward.len() < 2 {
        return Err(
            "OffsetContours open round-end slice requires at least one valid edge".to_string(),
        );
    }

    let end_origin = *contour.last().expect("contour length is checked above");
    let end = *output
        .last()
        .expect("offset output length is checked above");
    if end_type == OffsetContoursEndType::Round && !same_xy(end, end_origin) {
        let params = RoundCornerParams {
            lp: output[output.len() - 2],
            lc: end,
            org: end_origin,
            rc: backward[0],
            rn: backward[1],
            lr_ang: -std::f64::consts::PI,
        };
        insert_round_corner(&mut output, &params, min_angle_precision)?;
    }

    output.extend(backward);

    let start_origin = contour[0];
    let start = *output.last().expect("offset output contains backward side");
    if end_type == OffsetContoursEndType::Round && !same_xy(start, start_origin) {
        let params = RoundCornerParams {
            lp: output[output.len() - 2],
            lc: start,
            org: start_origin,
            rc: output[0],
            rn: output[1],
            lr_ang: -std::f64::consts::PI,
        };
        insert_round_corner(&mut output, &params, min_angle_precision)?;
    }

    output.push(output[0]);
    Ok(output)
}

fn offset_one_direction_open_contour(
    contour: &[[f64; 3]],
    offsets: &[f64],
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<Vec<[f64; 3]>, String> {
    if contour.len() != offsets.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }
    let first_normal = contour_normal(contour[0], contour[1])?;
    let mut output = Vec::with_capacity(contour.len() * 3);
    output.push(add2(contour[0], scale2(first_normal, offsets[0])));

    let mut right_current = *output.first().expect("first offset point was just pushed");
    let mut right_next = right_current;
    for index in 0..contour.len() - 1 {
        let normal = contour_normal(contour[index], contour[index + 1])?;
        let offset = offsets[index];
        let next_offset = offsets[index + 1];
        let origin = contour[index];
        let params = RoundCornerParams {
            lp: right_current,
            lc: right_next,
            rc: add2(origin, scale2(normal, offset)),
            rn: add2(contour[index + 1], scale2(normal, next_offset)),
            org: origin,
            lr_ang: find_angle(right_next, origin, add2(origin, scale2(normal, offset))),
        };
        right_current = params.rc;
        right_next = params.rn;

        if params.lr_ang.abs() >= std::f64::consts::PI / 360.0 {
            if params.lr_ang * offset < 0.0 {
                match corner_type {
                    OffsetContoursCornerType::Round => {
                        insert_round_corner(&mut output, &params, min_angle_precision)?;
                    }
                    OffsetContoursCornerType::Sharp => {
                        insert_sharp_corner(
                            &mut output,
                            &SharpCornerParams {
                                lp: params.lp,
                                lc: params.lc,
                                rc: params.rc,
                                rn: params.rn,
                                org: params.org,
                                lr_ang: params.lr_ang,
                            },
                            max_sharp_angle,
                        );
                    }
                }
            } else {
                output.push(origin);
            }
            output.push(params.rc);
        }
        output.push(params.rn);
    }
    Ok(output)
}
