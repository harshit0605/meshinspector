use super::{
    simplify_self_overlapping_outline_with_origins, OffsetContoursOrigin, OutlineOriginMode,
};
use super::super::OffsetContoursCornerType;
use super::super::math::{
    add2, contour_normal, find_angle, insert_round_corner, same_xy, scale2, RoundCornerParams,
};
use super::super::sharp::{insert_sharp_corner, SharpCornerParams};
use crate::lines::validate_contour;

pub(super) fn offset_open_contour_with_origins(
    contour: &[[f64; 3]],
    contour_id: usize,
    magnitudes: &[f64],
    min_angle_precision: f64,
    end_type: super::super::OffsetContoursEndType,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>), String> {
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
    let (mut output, mut origins) = offset_one_direction_open_contour_with_origins(
        contour,
        magnitudes,
        contour_id,
        min_angle_precision,
        corner_type,
        max_sharp_angle,
    )?;
    let (mut backward, mut backward_origins) = offset_one_direction_open_contour_with_origins(
        contour,
        &backward_offsets,
        contour_id,
        min_angle_precision,
        corner_type,
        max_sharp_angle,
    )?;
    backward.reverse();
    backward_origins.reverse();

    if output.len() < 2 || backward.len() < 2 {
        return Err(
            "OffsetContours open round-end slice requires at least one valid edge".to_string(),
        );
    }

    let end_origin = *contour.last().expect("contour length is checked above");
    let end = *output
        .last()
        .expect("offset output length is checked above");
    if end_type == super::super::OffsetContoursEndType::Round && !same_xy(end, end_origin) {
        let params = RoundCornerParams {
            lp: output[output.len() - 2],
            lc: end,
            org: end_origin,
            rc: backward[0],
            rn: backward[1],
            lr_ang: -std::f64::consts::PI,
        };
        append_round_corner_with_origins(
            &mut output,
            &mut origins,
            &params,
            min_angle_precision,
            OffsetContoursOrigin::source_vertex(contour_id, contour.len() - 1),
        )?;
    }

    output.extend(backward);
    origins.extend(backward_origins);

    let start_origin = contour[0];
    let start = *output.last().expect("offset output contains backward side");
    if end_type == super::super::OffsetContoursEndType::Round && !same_xy(start, start_origin) {
        let params = RoundCornerParams {
            lp: output[output.len() - 2],
            lc: start,
            org: start_origin,
            rc: output[0],
            rn: output[1],
            lr_ang: -std::f64::consts::PI,
        };
        append_round_corner_with_origins(
            &mut output,
            &mut origins,
            &params,
            min_angle_precision,
            OffsetContoursOrigin::source_vertex(contour_id, 0),
        )?;
    }

    output.push(output[0]);
    origins.push(
        origins
            .first()
            .copied()
            .unwrap_or_else(|| OffsetContoursOrigin::source_vertex(contour_id, 0)),
    );
    let has_variable_magnitudes = magnitudes
        .windows(2)
        .any(|pair| (pair[0] - pair[1]).abs() > 1e-12);
    let final_segment_decreases = magnitudes
        .windows(2)
        .last()
        .is_some_and(|pair| pair[0] > pair[1] + 1e-12);
    let origin_mode = if contour.len() == 3 && has_variable_magnitudes && final_segment_decreases {
        OutlineOriginMode::OpenPositiveVariableSourceEdges
    } else {
        OutlineOriginMode::OpenCanonicalSourceEdges
    };

    Ok(simplify_self_overlapping_outline_with_origins(
        contour,
        contour_id,
        output,
        origins,
        origin_mode,
    ))
}

fn offset_one_direction_open_contour_with_origins(
    contour: &[[f64; 3]],
    offsets: &[f64],
    contour_id: usize,
    min_angle_precision: f64,
    corner_type: OffsetContoursCornerType,
    max_sharp_angle: f64,
) -> Result<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>), String> {
    if contour.len() != offsets.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }
    let first_normal = contour_normal(contour[0], contour[1])?;
    let mut output = Vec::with_capacity(contour.len() * 3);
    let mut origins = Vec::with_capacity(contour.len() * 3);
    output.push(add2(contour[0], scale2(first_normal, offsets[0])));
    origins.push(OffsetContoursOrigin::source_vertex(contour_id, 0));

    let mut right_current = *output.first().expect("first offset point was just pushed");
    let mut right_next = right_current;
    for index in 0..contour.len() - 1 {
        let normal = contour_normal(contour[index], contour[index + 1])?;
        let offset = offsets[index];
        let next_offset = offsets[index + 1];
        let origin = contour[index];
        let source_origin = OffsetContoursOrigin::source_vertex(contour_id, index);
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
                    OffsetContoursCornerType::Round => append_round_corner_with_origins(
                        &mut output,
                        &mut origins,
                        &params,
                        min_angle_precision,
                        source_origin,
                    )?,
                    OffsetContoursCornerType::Sharp => {
                        let before = output.len();
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
                        origins.extend((before..output.len()).map(|_| source_origin));
                    }
                }
            } else {
                output.push(origin);
                origins.push(source_origin);
            }
            output.push(params.rc);
            origins.push(source_origin);
        }
        output.push(params.rn);
        origins.push(OffsetContoursOrigin::source_vertex(contour_id, index + 1));
    }
    Ok((output, origins))
}

fn append_round_corner_with_origins(
    output: &mut Vec<[f64; 3]>,
    origins: &mut Vec<OffsetContoursOrigin>,
    params: &RoundCornerParams,
    min_angle_precision: f64,
    origin: OffsetContoursOrigin,
) -> Result<(), String> {
    let before = output.len();
    insert_round_corner(output, params, min_angle_precision)?;
    origins.extend((before..output.len()).map(|_| origin));
    Ok(())
}
