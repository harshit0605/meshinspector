#[derive(Debug, Clone, Copy)]
struct HorizontalOpenCutSegment {
    contour_id: usize,
    x_min: f64,
    x_max: f64,
    y: f64,
    z: f64,
    min_vert_id: usize,
    max_vert_id: usize,
}

#[derive(Debug, Clone, Copy)]
struct VerticalOpenCutSegment {
    contour_id: usize,
    x: f64,
    y_min: f64,
    y_max: f64,
    z: f64,
    min_vert_id: usize,
    max_vert_id: usize,
}

#[derive(Debug, Clone, Copy)]
struct DirectedHorizontalOpenCutSegment {
    contour_id: usize,
    x_min: f64,
    x_max: f64,
    y: f64,
    z: f64,
    min_vert_id: usize,
    max_vert_id: usize,
}

#[derive(Debug, Clone, Copy)]
struct DirectedVerticalOpenCutSegment {
    contour_id: usize,
    x: f64,
    y_min: f64,
    y_max: f64,
    z: f64,
    min_vert_id: usize,
    max_vert_id: usize,
}

#[derive(Debug, Clone, Copy)]
struct LocalOpenCutSegment {
    contour_id: usize,
    along_min: f64,
    along_max: f64,
    cross: f64,
    z: f64,
    min_vert_id: usize,
    max_vert_id: usize,
}

fn offset_open_cut_axis_aligned_crossing_origins(
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

    let mut horizontal = None;
    let mut vertical = None;
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
        if (start[2] - end[2]).abs() > 1e-12 {
            return Ok(None);
        }
        if (start[1] - end[1]).abs() <= 1e-12 && (start[0] - end[0]).abs() > 1e-12 {
            if horizontal.is_some() {
                return Ok(None);
            }
            let start_is_min = start[0] <= end[0];
            horizontal = Some(DirectedHorizontalOpenCutSegment {
                contour_id,
                x_min: start[0].min(end[0]),
                x_max: start[0].max(end[0]),
                y: start[1],
                z: start[2],
                min_vert_id: if start_is_min { 0 } else { 1 },
                max_vert_id: if start_is_min { 1 } else { 0 },
            });
        } else if (start[0] - end[0]).abs() <= 1e-12 && (start[1] - end[1]).abs() > 1e-12 {
            if vertical.is_some() {
                return Ok(None);
            }
            let start_is_min = start[1] <= end[1];
            vertical = Some(DirectedVerticalOpenCutSegment {
                contour_id,
                x: start[0],
                y_min: start[1].min(end[1]),
                y_max: start[1].max(end[1]),
                z: start[2],
                min_vert_id: if start_is_min { 0 } else { 1 },
                max_vert_id: if start_is_min { 1 } else { 0 },
            });
        } else {
            return Ok(None);
        }
    }

    let (Some(horizontal), Some(vertical)) = (horizontal, vertical) else {
        return Ok(None);
    };
    if (horizontal.z - vertical.z).abs() > 1e-12 {
        return Ok(None);
    }

    let magnitude = offset.abs();
    if !(vertical.x - magnitude > horizontal.x_min + 1e-12
        && vertical.x + magnitude < horizontal.x_max - 1e-12
        && horizontal.y - magnitude > vertical.y_min + 1e-12
        && horizontal.y + magnitude < vertical.y_max - 1e-12)
    {
        return Ok(None);
    }

    let z = horizontal.z;
    let x_left = vertical.x - magnitude;
    let x_right = vertical.x + magnitude;
    let y_low = horizontal.y - magnitude;
    let y_high = horizontal.y + magnitude;
    let points = vec![
        [x_right, y_high, z],
        [horizontal.x_max, y_high, z],
        [horizontal.x_max, y_low, z],
        [x_right, y_low, z],
        [x_right, vertical.y_min, z],
        [x_left, vertical.y_min, z],
        [x_left, y_low, z],
        [horizontal.x_min, y_low, z],
        [horizontal.x_min, y_high, z],
        [x_left, y_high, z],
        [x_left, vertical.y_max, z],
        [x_right, vertical.y_max, z],
        [x_right, y_high, z],
    ];

    let h_len = horizontal.x_max - horizontal.x_min;
    let v_len = vertical.y_max - vertical.y_min;
    let h_left_ratio = (x_left - horizontal.x_min) / h_len;
    let h_right_ratio = (x_right - horizontal.x_min) / h_len;
    let v_low_ratio = (y_low - vertical.y_min) / v_len;
    let v_high_ratio = (y_high - vertical.y_min) / v_len;
    let h_min = source_index(horizontal.contour_id, horizontal.min_vert_id);
    let h_max = source_index(horizontal.contour_id, horizontal.max_vert_id);
    let v_min = source_index(vertical.contour_id, vertical.min_vert_id);
    let v_max = source_index(vertical.contour_id, vertical.max_vert_id);
    let upper_right_origin = OffsetContoursOrigin {
        l_org: v_min,
        l_dest: v_max,
        u_org: h_min,
        u_dest: h_max,
        l_ratio: v_high_ratio,
        u_ratio: h_right_ratio,
    };
    let lower_right_origin = OffsetContoursOrigin {
        l_org: v_min,
        l_dest: v_max,
        u_org: h_min,
        u_dest: h_max,
        l_ratio: v_low_ratio,
        u_ratio: h_right_ratio,
    };
    let lower_left_origin = OffsetContoursOrigin {
        l_org: h_min,
        l_dest: h_max,
        u_org: v_max,
        u_dest: v_min,
        l_ratio: h_left_ratio,
        u_ratio: 1.0 - v_low_ratio,
    };
    let upper_left_origin = OffsetContoursOrigin {
        l_org: h_min,
        l_dest: h_max,
        u_org: v_max,
        u_dest: v_min,
        l_ratio: h_left_ratio,
        u_ratio: 1.0 - v_high_ratio,
    };
    let origins = vec![
        upper_right_origin,
        OffsetContoursOrigin::source_vertex(horizontal.contour_id, horizontal.max_vert_id),
        OffsetContoursOrigin::source_vertex(horizontal.contour_id, horizontal.max_vert_id),
        lower_right_origin,
        OffsetContoursOrigin::source_vertex(vertical.contour_id, vertical.min_vert_id),
        OffsetContoursOrigin::source_vertex(vertical.contour_id, vertical.min_vert_id),
        lower_left_origin,
        OffsetContoursOrigin::source_vertex(horizontal.contour_id, horizontal.min_vert_id),
        OffsetContoursOrigin::source_vertex(horizontal.contour_id, horizontal.min_vert_id),
        upper_left_origin,
        OffsetContoursOrigin::source_vertex(vertical.contour_id, vertical.max_vert_id),
        OffsetContoursOrigin::source_vertex(vertical.contour_id, vertical.max_vert_id),
        upper_right_origin,
    ];

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}

fn offset_open_cut_horizontal_collinear_overlapping_origins(
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
            || (start[0] - end[0]).abs() <= 1e-12
        {
            return Ok(None);
        }
        let start_is_min = start[0] <= end[0];
        segments.push(HorizontalOpenCutSegment {
            contour_id,
            x_min: start[0].min(end[0]),
            x_max: start[0].max(end[0]),
            y: start[1],
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
        .any(|segment| (segment.z - segments[0].z).abs() > 1e-12 || (segment.y - segments[0].y).abs() > 1e-12)
    {
        return Ok(None);
    }
    segments.sort_by(|a, b| {
        a.x_min
            .total_cmp(&b.x_min)
            .then_with(|| a.x_max.total_cmp(&b.x_max))
            .then_with(|| a.contour_id.cmp(&b.contour_id))
    });
    if segments.len() > 2 {
        return Ok(offset_open_cut_horizontal_collinear_chain_origins(
            &segments, magnitude,
        ));
    }
    let first = segments[0];
    let second = segments[1];
    if !(second.x_min > first.x_min + 1e-12
        && second.x_min <= first.x_max + 1e-12
        && second.x_max > first.x_max + 1e-12)
    {
        return Ok(None);
    }

    let y = first.y;
    let z = first.z;
    let first_overlap_ratio = (first.x_max - second.x_min) / (second.x_max - second.x_min);
    let second_overlap_ratio = (second.x_min - first.x_min) / (first.x_max - first.x_min);
    if first.min_vert_id != 0 {
        let points = vec![
            [first.x_min, y - magnitude, z],
            [first.x_min, y + magnitude, z],
            [first.x_max, y + magnitude, z],
            [first.x_max, y + magnitude, z],
            [second.x_max, y + magnitude, z],
            [second.x_max, y - magnitude, z],
            [second.x_min, y - magnitude, z],
            [second.x_min, y - magnitude, z],
            [first.x_min, y - magnitude, z],
        ];
        let origins = vec![
            OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
            OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
            OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
            OffsetContoursOrigin {
                l_org: source_index(second.contour_id, second.min_vert_id),
                l_dest: source_index(second.contour_id, second.max_vert_id),
                u_org: source_index(first.contour_id, first.max_vert_id),
                u_dest: source_index(first.contour_id, first.max_vert_id),
                l_ratio: first_overlap_ratio,
                u_ratio: 0.0,
            },
            OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
            OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
            OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
            if second.min_vert_id == 0 {
                OffsetContoursOrigin {
                    l_org: source_index(second.contour_id, second.min_vert_id),
                    l_dest: source_index(second.contour_id, second.min_vert_id),
                    u_org: source_index(first.contour_id, first.min_vert_id),
                    u_dest: source_index(first.contour_id, first.max_vert_id),
                    l_ratio: 0.0,
                    u_ratio: second_overlap_ratio,
                }
            } else {
                OffsetContoursOrigin {
                    l_org: source_index(first.contour_id, first.min_vert_id),
                    l_dest: source_index(first.contour_id, first.max_vert_id),
                    u_org: source_index(second.contour_id, second.min_vert_id),
                    u_dest: source_index(second.contour_id, second.min_vert_id),
                    l_ratio: second_overlap_ratio,
                    u_ratio: 1.0,
                }
            },
            OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        ];

        return Ok(Some(OffsetContoursResult {
            contours: vec![points],
            origins: vec![origins],
        }));
    }

    let points = vec![
        [first.x_min, y + magnitude, z],
        [first.x_max, y + magnitude, z],
        [first.x_max, y + magnitude, z],
        [second.x_max, y + magnitude, z],
        [second.x_max, y - magnitude, z],
        [second.x_min, y - magnitude, z],
        [second.x_min, y - magnitude, z],
        [first.x_min, y - magnitude, z],
        [first.x_min, y + magnitude, z],
    ];

    let origins = vec![
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.max_vert_id),
        OffsetContoursOrigin {
            l_org: source_index(first.contour_id, first.max_vert_id),
            l_dest: source_index(first.contour_id, first.max_vert_id),
            u_org: source_index(second.contour_id, second.min_vert_id),
            u_dest: source_index(second.contour_id, second.max_vert_id),
            l_ratio: 1.0,
            u_ratio: first_overlap_ratio,
        },
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.max_vert_id),
        OffsetContoursOrigin::source_vertex(second.contour_id, second.min_vert_id),
        if second.min_vert_id == 0 {
            OffsetContoursOrigin {
                l_org: source_index(second.contour_id, second.min_vert_id),
                l_dest: source_index(second.contour_id, second.min_vert_id),
                u_org: source_index(first.contour_id, first.min_vert_id),
                u_dest: source_index(first.contour_id, first.max_vert_id),
                l_ratio: 0.0,
                u_ratio: second_overlap_ratio,
            }
        } else {
            OffsetContoursOrigin {
                l_org: source_index(first.contour_id, first.min_vert_id),
                l_dest: source_index(first.contour_id, first.max_vert_id),
                u_org: source_index(second.contour_id, second.min_vert_id),
                u_dest: source_index(second.contour_id, second.min_vert_id),
                l_ratio: second_overlap_ratio,
                u_ratio: 1.0,
            }
        },
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
        OffsetContoursOrigin::source_vertex(first.contour_id, first.min_vert_id),
    ];

    Ok(Some(OffsetContoursResult {
        contours: vec![points],
        origins: vec![origins],
    }))
}
