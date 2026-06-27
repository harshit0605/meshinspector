fn offset_open_cut_horizontal_collinear_chain_origins(
    segments: &[HorizontalOpenCutSegment],
    magnitude: f64,
) -> Option<OffsetContoursResult> {
    if segments.iter().any(|segment| segment.min_vert_id != 0) {
        return None;
    }
    for pair in segments.windows(2) {
        let first = pair[0];
        let second = pair[1];
        if !(second.x_min > first.x_min + 1e-12
            && second.x_min <= first.x_max + 1e-12
            && second.x_max > first.x_max + 1e-12)
        {
            return None;
        }
    }

    let first = segments[0];
    let last = *segments.last()?;
    let y = first.y;
    let z = first.z;
    let mut points = Vec::with_capacity(segments.len() * 4 + 1);
    let mut origins = Vec::with_capacity(segments.len() * 4 + 1);

    points.push([first.x_min, y + magnitude, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        first.contour_id,
        first.min_vert_id,
    ));
    for (index, segment) in segments.iter().enumerate() {
        points.push([segment.x_max, y + magnitude, z]);
        origins.push(OffsetContoursOrigin::source_vertex(
            segment.contour_id,
            segment.max_vert_id,
        ));
        if let Some(next) = segments.get(index + 1) {
            let next_ratio = (segment.x_max - next.x_min) / (next.x_max - next.x_min);
            points.push([segment.x_max, y + magnitude, z]);
            origins.push(OffsetContoursOrigin {
                l_org: source_index(segment.contour_id, segment.max_vert_id),
                l_dest: source_index(segment.contour_id, segment.max_vert_id),
                u_org: source_index(next.contour_id, next.min_vert_id),
                u_dest: source_index(next.contour_id, next.max_vert_id),
                l_ratio: 1.0,
                u_ratio: next_ratio,
            });
        }
    }

    points.push([last.x_max, y - magnitude, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        last.contour_id,
        last.max_vert_id,
    ));
    for index in (0..segments.len()).rev() {
        let segment = segments[index];
        points.push([segment.x_min, y - magnitude, z]);
        origins.push(OffsetContoursOrigin::source_vertex(
            segment.contour_id,
            segment.min_vert_id,
        ));
        if index > 0 {
            let previous = segments[index - 1];
            let previous_ratio = (segment.x_min - previous.x_min) / (previous.x_max - previous.x_min);
            points.push([segment.x_min, y - magnitude, z]);
            origins.push(OffsetContoursOrigin {
                l_org: source_index(segment.contour_id, segment.min_vert_id),
                l_dest: source_index(segment.contour_id, segment.min_vert_id),
                u_org: source_index(previous.contour_id, previous.min_vert_id),
                u_dest: source_index(previous.contour_id, previous.max_vert_id),
                l_ratio: 0.0,
                u_ratio: previous_ratio,
            });
        }
    }
    points.push([first.x_min, y + magnitude, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        first.contour_id,
        first.min_vert_id,
    ));

    Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    })
}

fn offset_open_cut_vertical_collinear_overlapping_origins(
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

    let magnitude = offset.abs();
    let mut segments = Vec::new();
    for (contour_id, contour) in contours.iter().enumerate() {
        if contour.is_empty() {
            continue;
        }
        if contour.len() != 2 || is_closed_contour(contour) {
            return Ok(None);
        }
        validate_contour(contour)?;
        let start = contour[0];
        let end = contour[1];
        if (start[0] - end[0]).abs() > 1e-12
            || (start[2] - end[2]).abs() > 1e-12
            || (start[1] - end[1]).abs() <= 1e-12
        {
            return Ok(None);
        }
        let start_is_min = start[1] <= end[1];
        segments.push(VerticalOpenCutSegment {
            contour_id,
            x: start[0],
            y_min: start[1].min(end[1]),
            y_max: start[1].max(end[1]),
            z: start[2],
            min_vert_id: if start_is_min { 0 } else { 1 },
            max_vert_id: if start_is_min { 1 } else { 0 },
        });
    }
    if segments.len() < 2 {
        return Ok(None);
    }
    if segments
        .iter()
        .any(|segment| (segment.z - segments[0].z).abs() > 1e-12 || (segment.x - segments[0].x).abs() > 1e-12)
    {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.y_min
            .total_cmp(&b.y_min)
            .then_with(|| a.y_max.total_cmp(&b.y_max))
            .then_with(|| a.contour_id.cmp(&b.contour_id))
    });
    if segments.len() > 2 {
        return Ok(offset_open_cut_vertical_collinear_chain_origins(
            &segments, magnitude,
        ));
    }
    let first = segments[0];
    let second = segments[1];
    if !(second.y_min > first.y_min + 1e-12
        && second.y_min < first.y_max - 1e-12
        && second.y_max > first.y_max + 1e-12)
    {
        return Ok(None);
    }

    let x = first.x;
    let z = first.z;
    let overlap_start_ratio = (second.y_min - first.y_min) / (first.y_max - first.y_min);
    let overlap_end_ratio = (first.y_max - second.y_min) / (second.y_max - second.y_min);
    let overlap_start_origin = OffsetContoursOrigin {
        l_org: source_index(second.contour_id, second.min_vert_id),
        l_dest: source_index(second.contour_id, second.min_vert_id),
        u_org: source_index(first.contour_id, first.max_vert_id),
        u_dest: source_index(first.contour_id, first.min_vert_id),
        l_ratio: 0.0,
        u_ratio: overlap_start_ratio,
    };
    let overlap_end_origin = OffsetContoursOrigin {
        l_org: source_index(second.contour_id, second.min_vert_id),
        l_dest: source_index(second.contour_id, second.max_vert_id),
        u_org: source_index(first.contour_id, first.max_vert_id),
        u_dest: source_index(first.contour_id, first.max_vert_id),
        l_ratio: overlap_end_ratio,
        u_ratio: 1.0,
    };

    let (points, origins) = if first.min_vert_id != 0 {
        (
            vec![
                [x + magnitude, first.y_max, z],
                [x + magnitude, first.y_min, z],
                [x - magnitude, first.y_min, z],
                [x - magnitude, second.y_min, z],
                [x - magnitude, second.y_min, z],
                [x - magnitude, second.y_max, z],
                [x + magnitude, second.y_max, z],
                [x + magnitude, first.y_max, z],
                [x + magnitude, first.y_max, z],
            ],
            vec![
                OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
                OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
                OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
                overlap_start_origin,
                OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
                OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
                OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
                overlap_end_origin,
                OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
            ],
        )
    } else {
        (
            vec![
                [x - magnitude, first.y_min, z],
                [x - magnitude, second.y_min, z],
                [x - magnitude, second.y_min, z],
                [x - magnitude, second.y_max, z],
                [x + magnitude, second.y_max, z],
                [x + magnitude, first.y_max, z],
                [x + magnitude, first.y_max, z],
                [x + magnitude, first.y_min, z],
                [x - magnitude, first.y_min, z],
            ],
            vec![
                OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
                overlap_start_origin,
                OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
                OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
                OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
                overlap_end_origin,
                OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
                OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
                OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
            ],
        )
    };

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}

fn offset_open_cut_vertical_collinear_chain_origins(
    segments: &[VerticalOpenCutSegment],
    magnitude: f64,
) -> Option<OffsetContoursResult> {
    if segments.iter().any(|segment| segment.min_vert_id != 0) {
        return None;
    }
    for pair in segments.windows(2) {
        let first = pair[0];
        let second = pair[1];
        if !(second.y_min > first.y_min + 1e-12
            && second.y_min <= first.y_max + 1e-12
            && second.y_max > first.y_max + 1e-12)
        {
            return None;
        }
    }

    let first = segments[0];
    let last = *segments.last()?;
    let x = first.x;
    let z = first.z;
    let mut points = Vec::with_capacity(segments.len() * 4 + 1);
    let mut origins = Vec::with_capacity(segments.len() * 4 + 1);

    points.push([x - magnitude, first.y_min, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        first.contour_id,
        first.min_vert_id,
    ));
    for index in 1..segments.len() {
        let previous = segments[index - 1];
        let segment = segments[index];
        let previous_ratio = (segment.y_min - previous.y_min) / (previous.y_max - previous.y_min);
        points.push([x - magnitude, segment.y_min, z]);
        origins.push(OffsetContoursOrigin {
            l_org: source_index(segment.contour_id, segment.min_vert_id),
            l_dest: source_index(segment.contour_id, segment.min_vert_id),
            u_org: source_index(previous.contour_id, previous.max_vert_id),
            u_dest: source_index(previous.contour_id, previous.min_vert_id),
            l_ratio: 0.0,
            u_ratio: previous_ratio,
        });
        points.push([x - magnitude, segment.y_min, z]);
        origins.push(OffsetContoursOrigin::source_vertex(
            segment.contour_id,
            segment.min_vert_id,
        ));
    }
    points.push([x - magnitude, last.y_max, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        last.contour_id,
        last.max_vert_id,
    ));

    points.push([x + magnitude, last.y_max, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        last.contour_id,
        last.max_vert_id,
    ));
    for index in (1..segments.len()).rev() {
        let previous = segments[index - 1];
        let segment = segments[index];
        let segment_ratio = (previous.y_max - segment.y_min) / (segment.y_max - segment.y_min);
        points.push([x + magnitude, previous.y_max, z]);
        origins.push(OffsetContoursOrigin {
            l_org: source_index(segment.contour_id, segment.min_vert_id),
            l_dest: source_index(segment.contour_id, segment.max_vert_id),
            u_org: source_index(previous.contour_id, previous.max_vert_id),
            u_dest: source_index(previous.contour_id, previous.max_vert_id),
            l_ratio: segment_ratio,
            u_ratio: 1.0,
        });
        points.push([x + magnitude, previous.y_max, z]);
        origins.push(OffsetContoursOrigin::source_vertex(
            previous.contour_id,
            previous.max_vert_id,
        ));
    }
    points.push([x + magnitude, first.y_min, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        first.contour_id,
        first.min_vert_id,
    ));
    points.push([x - magnitude, first.y_min, z]);
    origins.push(OffsetContoursOrigin::source_vertex(
        first.contour_id,
        first.min_vert_id,
    ));

    Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    })
}

fn offset_open_cut_collinear_touching_origins(
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

    if let Some(output) =
        offset_open_cut_vertical_collinear_touching_origins(contours, offset, options)?
    {
        return Ok(Some(output));
    }
    offset_open_cut_non_axis_collinear_touching_origins(contours, offset, options)
}

fn offset_open_cut_vertical_collinear_touching_origins(
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

    let magnitude = offset.abs();
    let mut segments = Vec::new();
    for (contour_id, contour) in contours.iter().enumerate() {
        if contour.is_empty() {
            continue;
        }
        if contour.len() != 2 || is_closed_contour(contour) {
            return Ok(None);
        }
        validate_contour(contour)?;
        let start = contour[0];
        let end = contour[1];
        if (start[0] - end[0]).abs() > 1e-12
            || (start[2] - end[2]).abs() > 1e-12
            || (start[1] - end[1]).abs() <= 1e-12
        {
            return Ok(None);
        }
        let start_is_min = start[1] <= end[1];
        segments.push(VerticalOpenCutSegment {
            contour_id,
            x: start[0],
            y_min: start[1].min(end[1]),
            y_max: start[1].max(end[1]),
            z: start[2],
            min_vert_id: if start_is_min { 0 } else { 1 },
            max_vert_id: if start_is_min { 1 } else { 0 },
        });
    }
    if segments.len() != 2 {
        return Ok(None);
    }
    if (segments[0].z - segments[1].z).abs() > 1e-12
        || (segments[0].x - segments[1].x).abs() > 1e-12
    {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.y_min
            .total_cmp(&b.y_min)
            .then_with(|| a.y_max.total_cmp(&b.y_max))
            .then_with(|| a.contour_id.cmp(&b.contour_id))
    });
    let first = segments[0];
    let second = segments[1];
    if !((second.y_min - first.y_max).abs() <= 1e-10
        && second.y_max > first.y_max + 1e-12)
    {
        return Ok(None);
    }

    let x = first.x;
    let z = first.z;
    if first.min_vert_id != 0 {
        let points = vec![
            [x + magnitude, first.y_max, z],
            [x + magnitude, first.y_min, z],
            [x - magnitude, first.y_min, z],
            [x - magnitude, first.y_max, z],
            [x - magnitude, second.y_min, z],
            [x - magnitude, second.y_max, z],
            [x + magnitude, second.y_max, z],
            [x + magnitude, second.y_min, z],
            [x + magnitude, first.y_max, z],
        ];
        let origins = vec![
            OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
            OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
            OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
            OffsetContoursOrigin {
                l_org: source_index(second.contour_id, second.min_vert_id),
                l_dest: source_index(second.contour_id, second.min_vert_id),
                u_org: source_index(first.contour_id, first.max_vert_id),
                u_dest: source_index(first.contour_id, first.min_vert_id),
                l_ratio: 0.0,
                u_ratio: 0.0,
            },
            OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
            OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
            OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
            OffsetContoursOrigin {
                l_org: source_index(second.contour_id, second.min_vert_id),
                l_dest: source_index(second.contour_id, second.max_vert_id),
                u_org: source_index(first.contour_id, first.max_vert_id),
                u_dest: source_index(first.contour_id, first.max_vert_id),
                l_ratio: 0.0,
                u_ratio: 1.0,
            },
            OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
        ];

        return Ok(Some(OffsetContoursResult {
            contours: vec![points],
            origins: vec![origins],
        }));
    }

    let points = vec![
        [x - magnitude, first.y_min, z],
        [x - magnitude, first.y_max, z],
        [x - magnitude, first.y_max, z],
        [x - magnitude, second.y_max, z],
        [x + magnitude, second.y_max, z],
        [x + magnitude, second.y_min, z],
        [x + magnitude, second.y_min, z],
        [x + magnitude, first.y_min, z],
        [x - magnitude, first.y_min, z],
    ];
    let origins = vec![
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(second.contour_id, second.min_vert_id),
            l_dest: source_index(second.contour_id, second.min_vert_id),
            u_org: source_index(first.contour_id, first.max_vert_id),
            u_dest: source_index(first.contour_id, first.min_vert_id),
            l_ratio: 0.0,
            u_ratio: 0.0,
        },
        OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(second.contour_id, second.min_vert_id),
            l_dest: source_index(second.contour_id, second.max_vert_id),
            u_org: source_index(first.contour_id, first.max_vert_id),
            u_dest: source_index(first.contour_id, first.max_vert_id),
            l_ratio: 0.0,
            u_ratio: 1.0,
        },
        OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
    ];

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}

