fn offset_open_cut_horizontal_overlapping_parallel_origins(
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
        if (start[1] - end[1]).abs() > 1e-12
            || (start[2] - end[2]).abs() > 1e-12
            || start[0] > end[0]
        {
            return Ok(None);
        }
        segments.push(HorizontalOpenCutSegment {
            contour_id,
            x_min: start[0],
            x_max: end[0],
            y: start[1],
            z: start[2],
            min_vert_id: 0,
            max_vert_id: 1,
        });
    }
    if segments.len() != 2 {
        return Ok(None);
    }
    if (segments[0].z - segments[1].z).abs() > 1e-12 {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.y.total_cmp(&b.y)
            .then_with(|| a.x_min.total_cmp(&b.x_min))
            .then_with(|| a.x_max.total_cmp(&b.x_max))
    });
    let lower = segments[0];
    let upper = segments[1];
    let y_gap = upper.y - lower.y;
    if y_gap <= 1e-12 || y_gap >= 2.0 * magnitude - 1e-12 {
        return Ok(None);
    }
    if !(upper.x_min > lower.x_min + 1e-12
        && upper.x_min < lower.x_max - 1e-12
        && upper.x_max > lower.x_max + 1e-12)
    {
        return Ok(None);
    }

    let z = lower.z;
    let points = vec![
        [lower.x_max, lower.y - magnitude, z],
        [lower.x_min, lower.y - magnitude, z],
        [lower.x_min, lower.y + magnitude, z],
        [upper.x_min, lower.y + magnitude, z],
        [upper.x_min, upper.y + magnitude, z],
        [upper.x_max, upper.y + magnitude, z],
        [upper.x_max, upper.y - magnitude, z],
        [lower.x_max, upper.y - magnitude, z],
        [lower.x_max, lower.y - magnitude, z],
    ];

    let lower_x_ratio = (upper.x_min - lower.x_min) / (lower.x_max - lower.x_min);
    let upper_x_ratio = (lower.x_max - upper.x_min) / (upper.x_max - upper.x_min);
    let first_y_ratio = (points[3][1] - (upper.y - magnitude)) / (2.0 * magnitude);
    let second_y_ratio = (points[7][1] - (lower.y - magnitude)) / (2.0 * magnitude);

    let origins = vec![
        OffsetContoursOrigin::source_vertex(lower.contour_id, 1),
        OffsetContoursOrigin::source_vertex(lower.contour_id, 0),
        OffsetContoursOrigin::source_vertex(lower.contour_id, 0),
        OffsetContoursOrigin {
            l_org: source_index(upper.contour_id, 0),
            l_dest: source_index(upper.contour_id, 0),
            u_org: source_index(lower.contour_id, 0),
            u_dest: source_index(lower.contour_id, 1),
            l_ratio: first_y_ratio,
            u_ratio: lower_x_ratio,
        },
        OffsetContoursOrigin::source_vertex(upper.contour_id, 0),
        OffsetContoursOrigin::source_vertex(upper.contour_id, 1),
        OffsetContoursOrigin::source_vertex(upper.contour_id, 1),
        OffsetContoursOrigin {
            l_org: source_index(lower.contour_id, 1),
            l_dest: source_index(lower.contour_id, 1),
            u_org: source_index(upper.contour_id, 0),
            u_dest: source_index(upper.contour_id, 1),
            l_ratio: second_y_ratio,
            u_ratio: upper_x_ratio,
        },
        OffsetContoursOrigin::source_vertex(lower.contour_id, 1),
    ];

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}

fn offset_open_cut_non_axis_overlapping_parallel_origins(
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
    let tangent = [direction[0] / length, direction[1] / length];
    let normal = [-tangent[1], tangent[0]];
    let magnitude = offset.abs();
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
        if segment_tangent[0] * tangent[0] + segment_tangent[1] * tangent[1] < 1.0 - 1e-10 {
            return Ok(None);
        }
        let (a_along, a_cross) =
            local_frame_coordinates_2d(a, first_start, tangent, normal);
        let (b_along, b_cross) =
            local_frame_coordinates_2d(b, first_start, tangent, normal);
        if (a_cross - b_cross).abs() > 1e-10 {
            return Ok(None);
        }
        let a_is_min = a_along <= b_along;
        segments.push(LocalOpenCutSegment {
            contour_id,
            along_min: a_along.min(b_along),
            along_max: a_along.max(b_along),
            cross: (a_cross + b_cross) * 0.5,
            z: a[2],
            min_vert_id: if a_is_min { 0 } else { 1 },
            max_vert_id: if a_is_min { 1 } else { 0 },
        });
    }

    if segments.len() != 2 || (segments[0].z - segments[1].z).abs() > 1e-12 {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.cross
            .total_cmp(&b.cross)
            .then_with(|| a.along_min.total_cmp(&b.along_min))
            .then_with(|| a.along_max.total_cmp(&b.along_max))
    });
    let lower = segments[0];
    let upper = segments[1];
    let cross_gap = upper.cross - lower.cross;
    if cross_gap <= 1e-12 || cross_gap >= 2.0 * magnitude - 1e-12 {
        return Ok(None);
    }
    if !(upper.along_min > lower.along_min + 1e-12
        && upper.along_min < lower.along_max - 1e-12
        && upper.along_max > lower.along_max + 1e-12)
    {
        return Ok(None);
    }

    let local_points = vec![
        [lower.along_max, upper.cross - magnitude, 0.0],
        [lower.along_max, lower.cross - magnitude, 0.0],
        [lower.along_min, lower.cross - magnitude, 0.0],
        [lower.along_min, lower.cross + magnitude, 0.0],
        [upper.along_min, lower.cross + magnitude, 0.0],
        [upper.along_min, upper.cross + magnitude, 0.0],
        [upper.along_max, upper.cross + magnitude, 0.0],
        [upper.along_max, upper.cross - magnitude, 0.0],
        [lower.along_max, upper.cross - magnitude, 0.0],
    ];
    let points = local_points
        .into_iter()
        .map(|[along, cross, _]| {
            local_frame_point_2d(along, cross, first_start, tangent, normal, z)
        })
        .collect::<Vec<_>>();

    let upper_ratio_at_lower_end =
        (lower.along_max - upper.along_min) / (upper.along_max - upper.along_min);
    let lower_ratio_at_upper_start =
        (upper.along_min - lower.along_min) / (lower.along_max - lower.along_min);
    let cross_lower_to_upper = cross_gap / (2.0 * magnitude);
    let cross_upper_to_lower = 1.0 - cross_lower_to_upper;
    let start_origin = OffsetContoursOrigin {
        l_org: source_index(upper.contour_id, 0),
        l_dest: source_index(upper.contour_id, 1),
        u_org: source_index(lower.contour_id, 1),
        u_dest: source_index(lower.contour_id, 1),
        l_ratio: upper_ratio_at_lower_end,
        u_ratio: cross_upper_to_lower,
    };
    let inner_step_origin = OffsetContoursOrigin {
        l_org: source_index(lower.contour_id, 0),
        l_dest: source_index(lower.contour_id, 1),
        u_org: source_index(upper.contour_id, 0),
        u_dest: source_index(upper.contour_id, 0),
        l_ratio: lower_ratio_at_upper_start,
        u_ratio: cross_lower_to_upper,
    };
    let origins = vec![
        start_origin,
        OffsetContoursOrigin::source_vertex(lower.contour_id, 1),
        OffsetContoursOrigin::source_vertex(lower.contour_id, 0),
        OffsetContoursOrigin::source_vertex(lower.contour_id, 0),
        inner_step_origin,
        OffsetContoursOrigin::source_vertex(upper.contour_id, 0),
        OffsetContoursOrigin::source_vertex(upper.contour_id, 1),
        OffsetContoursOrigin::source_vertex(upper.contour_id, 1),
        start_origin,
    ];

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}

fn local_frame_coordinates_2d(
    point: [f64; 3],
    origin: [f64; 3],
    tangent: [f64; 2],
    normal: [f64; 2],
) -> (f64, f64) {
    let dx = point[0] - origin[0];
    let dy = point[1] - origin[1];
    (
        dx * tangent[0] + dy * tangent[1],
        dx * normal[0] + dy * normal[1],
    )
}

fn local_frame_point_2d(
    along: f64,
    cross: f64,
    origin: [f64; 3],
    tangent: [f64; 2],
    normal: [f64; 2],
    z: f64,
) -> [f64; 3] {
    [
        origin[0] + tangent[0] * along + normal[0] * cross,
        origin[1] + tangent[1] * along + normal[1] * cross,
        z,
    ]
}

