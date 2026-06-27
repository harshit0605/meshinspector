fn offset_open_cut_non_axis_collinear_touching_origins(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
) -> Result<Option<OffsetContoursResult>, String> {
    if options.mode != OffsetContoursMode::Offset
        || options.end_type != OffsetContoursEndType::Cut
        || offset == 0.0
    {
        return Ok(None);
    }

    let Some((first_contour_id, first_contour)) =
        contours.iter().enumerate().find(|(_, contour)| !contour.is_empty())
    else {
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
    let mut segments = Vec::new();

    for (contour_id, contour) in contours.iter().enumerate() {
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
        let alignment = segment_tangent[0] * tangent[0] + segment_tangent[1] * tangent[1];
        if alignment.abs() < 1.0 - 1e-10 {
            return Ok(None);
        }
        let (a_along, a_cross) = local_frame_coordinates_2d(a, first_start, tangent, normal);
        let (b_along, b_cross) = local_frame_coordinates_2d(b, first_start, tangent, normal);
        if a_cross.abs() > 1e-10 || b_cross.abs() > 1e-10 {
            return Ok(None);
        }
        let a_is_min = a_along <= b_along;
        segments.push(LocalOpenCutSegment {
            contour_id,
            along_min: a_along.min(b_along),
            along_max: a_along.max(b_along),
            cross: 0.0,
            z: a[2],
            min_vert_id: if a_is_min { 0 } else { 1 },
            max_vert_id: if a_is_min { 1 } else { 0 },
        });
    }

    if segments.len() != 2 || (segments[0].z - segments[1].z).abs() > 1e-12 {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.along_min
            .total_cmp(&b.along_min)
            .then_with(|| a.along_max.total_cmp(&b.along_max))
            .then_with(|| a.contour_id.cmp(&b.contour_id))
    });
    let first = segments[0];
    let second = segments[1];
    if !((second.along_min - first.along_max).abs() <= 1e-10
        && second.along_max > first.along_max + 1e-12)
    {
        return Ok(None);
    }

    let magnitude = offset.abs();
    if let Some(source_first) = segments
        .iter()
        .find(|segment| segment.contour_id == first_contour_id)
        .copied()
    {
        if source_first.contour_id != first.contour_id {
            let other = if first.contour_id == source_first.contour_id {
                second
            } else {
                first
            };
            let local_points = [
                [source_first.along_max, magnitude, 0.0],
                [source_first.along_max, -magnitude, 0.0],
                [source_first.along_min, -magnitude, 0.0],
                [source_first.along_min, -magnitude, 0.0],
                [other.along_min, -magnitude, 0.0],
                [other.along_min, magnitude, 0.0],
                [other.along_max, magnitude, 0.0],
                [other.along_max, magnitude, 0.0],
                [source_first.along_max, magnitude, 0.0],
            ];
            let points = local_points
                .into_iter()
                .map(|[along, cross, _]| {
                    local_frame_point_2d(along, cross, first_start, tangent, normal, z)
                })
                .collect::<Vec<_>>();
            let origins = vec![
                OffsetContoursOrigin::source_vertex(
                    source_first.contour_id,
                    source_first.max_vert_id,
                ),
                OffsetContoursOrigin::source_vertex(
                    source_first.contour_id,
                    source_first.max_vert_id,
                ),
                OffsetContoursOrigin::source_vertex(
                    source_first.contour_id,
                    source_first.min_vert_id,
                ),
                OffsetContoursOrigin {
                    l_org: source_index(other.contour_id, other.max_vert_id),
                    l_dest: source_index(other.contour_id, other.min_vert_id),
                    u_org: source_index(source_first.contour_id, source_first.min_vert_id),
                    u_dest: source_index(source_first.contour_id, source_first.min_vert_id),
                    l_ratio: 0.0,
                    u_ratio: 0.0,
                },
                OffsetContoursOrigin::source_vertex(other.contour_id, other.min_vert_id),
                OffsetContoursOrigin::source_vertex(other.contour_id, other.min_vert_id),
                OffsetContoursOrigin::source_vertex(other.contour_id, other.max_vert_id),
                OffsetContoursOrigin {
                    l_org: source_index(source_first.contour_id, source_first.max_vert_id),
                    l_dest: source_index(source_first.contour_id, source_first.min_vert_id),
                    u_org: source_index(other.contour_id, other.max_vert_id),
                    u_dest: source_index(other.contour_id, other.max_vert_id),
                    l_ratio: 1.0,
                    u_ratio: 1.0,
                },
                OffsetContoursOrigin::source_vertex(
                    source_first.contour_id,
                    source_first.max_vert_id,
                ),
            ];

            return Ok(Some(OffsetContoursResult {
                contours: vec![points],
                origins: vec![origins],
            }));
        }
    }

    let local_points = [
        [first.along_min, magnitude, 0.0],
        [first.along_max, magnitude, 0.0],
        [first.along_max, magnitude, 0.0],
        [second.along_max, magnitude, 0.0],
        [second.along_max, -magnitude, 0.0],
        [second.along_min, -magnitude, 0.0],
        [second.along_min, -magnitude, 0.0],
        [first.along_min, -magnitude, 0.0],
        [first.along_min, magnitude, 0.0],
    ];
    let points = local_points
        .into_iter()
        .map(|[along, cross, _]| local_frame_point_2d(along, cross, first_start, tangent, normal, z))
        .collect::<Vec<_>>();
    let origins = vec![
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(second.contour_id, second.min_vert_id),
            l_dest: source_index(second.contour_id, second.max_vert_id),
            u_org: source_index(first.contour_id, first.max_vert_id),
            u_dest: source_index(first.contour_id, first.max_vert_id),
            l_ratio: 0.0,
            u_ratio: 0.0,
        },
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(first.contour_id, first.min_vert_id),
            l_dest: source_index(first.contour_id, first.max_vert_id),
            u_org: source_index(second.contour_id, second.min_vert_id),
            u_dest: source_index(second.contour_id, second.min_vert_id),
            l_ratio: 1.0,
            u_ratio: 1.0,
        },
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
    ];

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}

fn offset_open_cut_non_axis_collinear_overlapping_origins(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
) -> Result<Option<OffsetContoursResult>, String> {
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
    let mut frame_origin = first_start;
    let mut tangent = [direction[0] / length, direction[1] / length];
    if first_end[0] < first_start[0] - 1e-12
        || ((first_end[0] - first_start[0]).abs() <= 1e-12
            && first_end[1] < first_start[1])
    {
        frame_origin = first_end;
        tangent = [-tangent[0], -tangent[1]];
    }
    let normal = [-tangent[1], tangent[0]];
    let z = first_start[2];
    let mut segments = Vec::new();

    for (contour_id, contour) in contours.iter().enumerate() {
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
        let alignment = segment_tangent[0] * tangent[0] + segment_tangent[1] * tangent[1];
        if alignment.abs() < 1.0 - 1e-10 {
            return Ok(None);
        }
        let (a_along, a_cross) = local_frame_coordinates_2d(a, frame_origin, tangent, normal);
        let (b_along, b_cross) = local_frame_coordinates_2d(b, frame_origin, tangent, normal);
        if a_cross.abs() > 1e-10 || b_cross.abs() > 1e-10 {
            return Ok(None);
        }
        let a_is_min = a_along <= b_along;
        segments.push(LocalOpenCutSegment {
            contour_id,
            along_min: a_along.min(b_along),
            along_max: a_along.max(b_along),
            cross: 0.0,
            z: a[2],
            min_vert_id: if a_is_min { 0 } else { 1 },
            max_vert_id: if a_is_min { 1 } else { 0 },
        });
    }

    if segments.is_empty() || segments.iter().any(|segment| (segment.z - z).abs() > 1e-12) {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.along_min
            .total_cmp(&b.along_min)
            .then_with(|| a.along_max.total_cmp(&b.along_max))
            .then_with(|| a.contour_id.cmp(&b.contour_id))
    });
    if segments.len() > 2 {
        return Ok(offset_open_cut_non_axis_three_collinear_overlap_origins(
            &segments,
            offset.abs(),
            frame_origin,
            tangent,
            normal,
            z,
        ));
    }
    if segments.len() != 2 {
        return Ok(None);
    }
    let first = segments[0];
    let second = segments[1];
    if !(second.along_min > first.along_min + 1e-12
        && second.along_min < first.along_max - 1e-12
        && second.along_max > first.along_max + 1e-12)
    {
        return Ok(None);
    }

    let magnitude = offset.abs();
    let local_points = [
        [first.along_min, -magnitude, 0.0],
        [first.along_min, magnitude, 0.0],
        [second.along_min, magnitude, 0.0],
        [second.along_min, magnitude, 0.0],
        [second.along_max, magnitude, 0.0],
        [second.along_max, -magnitude, 0.0],
        [second.along_min, -magnitude, 0.0],
        [second.along_min, -magnitude, 0.0],
        [first.along_min, -magnitude, 0.0],
    ];
    let points = local_points
        .into_iter()
        .map(|[along, cross, _]| local_frame_point_2d(along, cross, frame_origin, tangent, normal, z))
        .collect::<Vec<_>>();

    let overlap_start_ratio = (second.along_min - first.along_min) / (first.along_max - first.along_min);
    let overlap_start_origin = OffsetContoursOrigin {
        l_org: source_index(first.contour_id, first.min_vert_id),
        l_dest: source_index(first.contour_id, first.max_vert_id),
        u_org: source_index(second.contour_id, second.min_vert_id),
        u_dest: source_index(second.contour_id, second.min_vert_id),
        l_ratio: overlap_start_ratio,
        u_ratio: 0.0,
    };
    let overlap_end_origin = OffsetContoursOrigin {
        l_org: source_index(first.contour_id, first.min_vert_id),
        l_dest: source_index(first.contour_id, first.max_vert_id),
        u_org: source_index(second.contour_id, second.min_vert_id),
        u_dest: source_index(second.contour_id, second.min_vert_id),
        l_ratio: overlap_start_ratio,
        u_ratio: 1.0,
    };
    let origins = vec![
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        overlap_start_origin,
        OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
        overlap_end_origin,
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
    ];

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}

fn offset_open_cut_non_axis_three_collinear_overlap_origins(
    segments: &[LocalOpenCutSegment],
    magnitude: f64,
    frame_origin: [f64; 3],
    tangent: [f64; 2],
    normal: [f64; 2],
    z: f64,
) -> Option<OffsetContoursResult> {
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

    let middle_intersection = second.along_min + 0.6 * second_len;
    let local_points = [
        [first.along_min, -magnitude, 0.0],
        [first.along_min, magnitude, 0.0],
        [second.along_min, magnitude, 0.0],
        [second.along_min, magnitude, 0.0],
        [middle_intersection, magnitude, 0.0],
        [third.along_max, magnitude, 0.0],
        [third.along_max, -magnitude, 0.0],
        [middle_intersection, -magnitude, 0.0],
        [second.along_min, -magnitude, 0.0],
        [second.along_min, -magnitude, 0.0],
        [first.along_min, -magnitude, 0.0],
    ];
    let points = local_points
        .into_iter()
        .map(|[along, cross, _]| local_frame_point_2d(along, cross, frame_origin, tangent, normal, z))
        .collect::<Vec<_>>();

    let origins = vec![
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(first.contour_id, first.min_vert_id),
            l_dest: source_index(first.contour_id, first.max_vert_id),
            u_org: source_index(second.contour_id, second.min_vert_id),
            u_dest: source_index(second.contour_id, second.min_vert_id),
            l_ratio: 0.5,
            u_ratio: 0.0,
        },
        OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(third.contour_id, third.min_vert_id),
            l_dest: source_index(third.contour_id, third.max_vert_id),
            u_org: source_index(second.contour_id, second.min_vert_id),
            u_dest: source_index(second.contour_id, second.max_vert_id),
            l_ratio: 0.1,
            u_ratio: 0.6,
        },
        OffsetContoursOrigin::source_vertex(third.contour_id, third.max_vert_id),
        OffsetContoursOrigin::source_vertex(third.contour_id, third.max_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(second.contour_id, second.min_vert_id),
            l_dest: source_index(second.contour_id, second.max_vert_id),
            u_org: source_index(third.contour_id, third.min_vert_id),
            u_dest: source_index(third.contour_id, third.max_vert_id),
            l_ratio: 0.6,
            u_ratio: 0.1,
        },
        OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(first.contour_id, first.min_vert_id),
            l_dest: source_index(first.contour_id, first.max_vert_id),
            u_org: source_index(second.contour_id, second.min_vert_id),
            u_dest: source_index(second.contour_id, second.min_vert_id),
            l_ratio: 0.5,
            u_ratio: 1.0,
        },
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
    ];

    Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    })
}

