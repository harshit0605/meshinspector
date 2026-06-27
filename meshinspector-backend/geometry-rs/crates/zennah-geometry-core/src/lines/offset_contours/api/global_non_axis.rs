fn offset_open_cut_non_axis_three_collinear_overlap_global_outline(
    segments: &[LocalTouchingSegment],
    common_cross: f64,
    magnitude: f64,
    frame_origin: [f64; 3],
    tangent: [f64; 2],
    normal: [f64; 2],
    z: f64,
) -> Option<Vec<[f64; 3]>> {
    if segments.len() != 3 {
        return None;
    }
    let first = segments[0];
    let second = segments[1];
    let third = segments[2];
    let first_len = first.along_max - first.along_min;
    let second_len = second.along_max - second.along_min;
    let third_len = third.along_max - third.along_min;
    if first_len <= 1e-12
        || (second_len - first_len).abs() > 1e-9
        || (third_len - first_len).abs() > 1e-9
    {
        return None;
    }
    if (second.along_min - (first.along_min + 0.5 * first_len)).abs() > 1e-9
        || (third.along_min - first.along_max).abs() > 1e-9
        || (second.along_max - (first.along_max + 0.5 * first_len)).abs() > 1e-9
        || (third.along_max - (first.along_max + first_len)).abs() > 1e-9
    {
        return None;
    }

    let forward_cross = common_cross + magnitude;
    let reverse_cross = common_cross - magnitude;
    let middle_intersection = second.along_min + 0.6 * second_len;
    let local_outline = [
        [first.along_min, reverse_cross, 0.0],
        [first.along_min, forward_cross, 0.0],
        [second.along_min, forward_cross, 0.0],
        [second.along_min, forward_cross, 0.0],
        [middle_intersection, forward_cross, 0.0],
        [third.along_max, forward_cross, 0.0],
        [third.along_max, reverse_cross, 0.0],
        [middle_intersection, reverse_cross, 0.0],
        [second.along_min, reverse_cross, 0.0],
        [second.along_min, reverse_cross, 0.0],
        [first.along_min, reverse_cross, 0.0],
    ];
    Some(
        local_outline
            .into_iter()
            .map(|[along, cross, _]| local_frame_point(along, cross, frame_origin, tangent, normal, z))
            .collect(),
    )
}

fn offset_open_cut_collinear_touching_global_outlines(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
) -> Result<Option<Vec<Vec<[f64; 3]>>>, String> {
    if options.mode != OffsetContoursMode::Offset
        || options.end_type != OffsetContoursEndType::Cut
        || offset == 0.0
    {
        return Ok(None);
    }

    let Some(first_contour) = contours.iter().find(|contour| !contour.is_empty()) else {
        return Ok(None);
    };
    if first_contour.len() != 2 || is_closed_contour(first_contour) {
        return Ok(None);
    }
    validate_contour(first_contour)?;
    let first_start = first_contour[0];
    let first_end = first_contour[1];
    if (first_start[0] - first_end[0]).abs() <= 1e-12
        || (first_start[1] - first_end[1]).abs() <= 1e-12
    {
        return Ok(None);
    }

    let direction = [first_end[0] - first_start[0], first_end[1] - first_start[1]];
    let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
    if length <= 1e-12 {
        return Ok(None);
    }
    let tangent = [direction[0] / length, direction[1] / length];
    let normal = [-tangent[1], tangent[0]];
    let common_cross = local_frame_coordinates(first_start, first_start, tangent, normal).1;
    let z = first_start[2];
    let mut segments = Vec::new();

    for contour in contours {
        if contour.is_empty() {
            continue;
        }
        if contour.len() != 2 || is_closed_contour(contour) {
            return Ok(None);
        }
        validate_contour(contour)?;
        let a = contour[0];
        let b = contour[1];
        if (a[2] - b[2]).abs() > 1e-12 || (a[2] - z).abs() > 1e-12 {
            return Ok(None);
        }
        let segment_direction = [b[0] - a[0], b[1] - a[1]];
        let segment_length =
            (segment_direction[0] * segment_direction[0] + segment_direction[1] * segment_direction[1]).sqrt();
        if segment_length <= 1e-12 {
            return Ok(None);
        }
        let segment_tangent = [
            segment_direction[0] / segment_length,
            segment_direction[1] / segment_length,
        ];
        if segment_tangent[0] * tangent[0] + segment_tangent[1] * tangent[1] < 1.0 - 1e-10 {
            return Ok(None);
        }

        let (a_along, a_cross) = local_frame_coordinates(a, first_start, tangent, normal);
        let (b_along, b_cross) = local_frame_coordinates(b, first_start, tangent, normal);
        if (a_cross - common_cross).abs() > 1e-10 || (b_cross - common_cross).abs() > 1e-10 {
            return Ok(None);
        }
        segments.push(LocalTouchingSegment {
            along_min: a_along.min(b_along),
            along_max: a_along.max(b_along),
        });
    }

    if segments.len() <= 1 {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.along_min
            .total_cmp(&b.along_min)
            .then_with(|| a.along_max.total_cmp(&b.along_max))
    });
    if !local_segments_form_touching_expanding_chain(&segments) {
        return Ok(None);
    }

    let magnitude = offset.abs();
    let outline = local_touching_chain_outline(
        &segments,
        common_cross + magnitude,
        common_cross - magnitude,
        first_start,
        tangent,
        normal,
        z,
    );
    Ok(Some(vec![outline]))
}

fn offset_open_cut_parallel_global_outlines(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
) -> Result<Option<Vec<Vec<[f64; 3]>>>, String> {
    if options.mode != OffsetContoursMode::Offset
        || options.end_type != OffsetContoursEndType::Cut
        || offset == 0.0
    {
        return Ok(None);
    }

    let Some(first_contour) = contours.iter().find(|contour| !contour.is_empty()) else {
        return Ok(None);
    };
    if first_contour.len() != 2 || is_closed_contour(first_contour) {
        return Ok(None);
    }
    validate_contour(first_contour)?;
    let first_start = first_contour[0];
    let first_end = first_contour[1];
    if (first_start[0] - first_end[0]).abs() <= 1e-12
        || (first_start[1] - first_end[1]).abs() <= 1e-12
    {
        return Ok(None);
    }

    let direction = [first_end[0] - first_start[0], first_end[1] - first_start[1]];
    let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
    if length <= 1e-12 {
        return Ok(None);
    }
    let tangent = [direction[0] / length, direction[1] / length];
    let normal = [-tangent[1], tangent[0]];
    let z = first_start[2];
    let magnitude = offset.abs();
    let mut rects = Vec::new();

    for contour in contours {
        if contour.is_empty() {
            continue;
        }
        if contour.len() != 2 || is_closed_contour(contour) {
            return Ok(None);
        }
        validate_contour(contour)?;
        let a = contour[0];
        let b = contour[1];
        if (a[2] - b[2]).abs() > 1e-12 || (a[2] - z).abs() > 1e-12 {
            return Ok(None);
        }
        let segment_direction = [b[0] - a[0], b[1] - a[1]];
        let segment_length =
            (segment_direction[0] * segment_direction[0] + segment_direction[1] * segment_direction[1]).sqrt();
        if segment_length <= 1e-12 {
            return Ok(None);
        }
        let segment_tangent = [
            segment_direction[0] / segment_length,
            segment_direction[1] / segment_length,
        ];
        if segment_tangent[0] * tangent[0] + segment_tangent[1] * tangent[1] < 1.0 - 1e-10 {
            return Ok(None);
        }

        let (a_along, a_cross) = local_frame_coordinates(a, first_start, tangent, normal);
        let (b_along, b_cross) = local_frame_coordinates(b, first_start, tangent, normal);
        if (a_cross - b_cross).abs() > 1e-10 {
            return Ok(None);
        }
        let cross = (a_cross + b_cross) * 0.5;
        rects.push(LocalOffsetRect {
            along_min: a_along.min(b_along),
            along_max: a_along.max(b_along),
            cross_min: cross - magnitude,
            cross_max: cross + magnitude,
        });
    }

    if rects.len() <= 1 {
        return Ok(None);
    }
    if rects
        .iter()
        .all(|rect| (rect.cross_min - rects[0].cross_min).abs() <= 1e-10)
    {
        return Ok(None);
    }

    let mut local_outlines = local_rect_union_outlines(&rects)?;
    if local_outlines.len() >= rects.len() {
        return Ok(None);
    }
    for local_outline in &mut local_outlines {
        rotate_local_parallel_outline_to_meshlib_start(local_outline, &rects);
    }
    let outlines = local_outlines
        .into_iter()
        .map(|outline| {
            outline
                .into_iter()
                .map(|[along, cross, _]| local_frame_point(along, cross, first_start, tangent, normal, z))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    Ok(Some(outlines))
}
