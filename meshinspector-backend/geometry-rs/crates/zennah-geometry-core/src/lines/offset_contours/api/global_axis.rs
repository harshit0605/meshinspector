#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AxisAlignedSegmentAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
struct AxisAlignedOffsetRect {
    axis: AxisAlignedSegmentAxis,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    z: f64,
}

#[derive(Debug, Clone, Copy)]
struct LocalTouchingSegment {
    along_min: f64,
    along_max: f64,
}

#[derive(Debug, Clone, Copy)]
struct LocalOffsetRect {
    along_min: f64,
    along_max: f64,
    cross_min: f64,
    cross_max: f64,
}

#[derive(Debug, Clone, Copy)]
struct AxisAlignedOpenCutSegment {
    axis: AxisAlignedSegmentAxis,
    along_min: f64,
    along_max: f64,
    cross: f64,
    z: f64,
    min_vert_id: usize,
}

fn offset_open_cut_axis_aligned_collinear_overlapping_global_outlines(
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

    let mut segments = Vec::new();
    for contour in contours {
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

        if (start[1] - end[1]).abs() <= 1e-12 {
            let start_is_min = start[0] <= end[0];
            segments.push(AxisAlignedOpenCutSegment {
                axis: AxisAlignedSegmentAxis::Horizontal,
                along_min: start[0].min(end[0]),
                along_max: start[0].max(end[0]),
                cross: start[1],
                z: start[2],
                min_vert_id: if start_is_min { 0 } else { 1 },
            });
        } else if (start[0] - end[0]).abs() <= 1e-12 {
            let start_is_min = start[1] <= end[1];
            segments.push(AxisAlignedOpenCutSegment {
                axis: AxisAlignedSegmentAxis::Vertical,
                along_min: start[1].min(end[1]),
                along_max: start[1].max(end[1]),
                cross: start[0],
                z: start[2],
                min_vert_id: if start_is_min { 0 } else { 1 },
            });
        } else {
            return Ok(None);
        }
    }

    if segments.len() < 2 {
        return Ok(None);
    }
    let axis = segments[0].axis;
    if segments.iter().any(|segment| {
        segment.axis != axis
            || (segment.cross - segments[0].cross).abs() > 1e-12
            || (segment.z - segments[0].z).abs() > 1e-12
    }) {
        return Ok(None);
    }

    segments.sort_by(|a, b| {
        a.along_min
            .total_cmp(&b.along_min)
            .then_with(|| a.along_max.total_cmp(&b.along_max))
    });
    let magnitude = offset.abs();
    if segments.len() > 2 {
        if axis == AxisAlignedSegmentAxis::Vertical {
            return Ok(offset_open_cut_vertical_collinear_chain_global_outline(
                &segments, magnitude,
            )
            .map(|outline| vec![outline]));
        }
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

    let z = first.z;
    let points = match axis {
        AxisAlignedSegmentAxis::Horizontal => {
            let y = first.cross;
            if first.min_vert_id != 0 {
                vec![
                    [first.along_min, y - magnitude, z],
                    [first.along_min, y + magnitude, z],
                    [first.along_max, y + magnitude, z],
                    [first.along_max, y + magnitude, z],
                    [second.along_max, y + magnitude, z],
                    [second.along_max, y - magnitude, z],
                    [second.along_min, y - magnitude, z],
                    [second.along_min, y - magnitude, z],
                    [first.along_min, y - magnitude, z],
                ]
            } else {
                vec![
                    [first.along_min, y + magnitude, z],
                    [first.along_max, y + magnitude, z],
                    [first.along_max, y + magnitude, z],
                    [second.along_max, y + magnitude, z],
                    [second.along_max, y - magnitude, z],
                    [second.along_min, y - magnitude, z],
                    [second.along_min, y - magnitude, z],
                    [first.along_min, y - magnitude, z],
                    [first.along_min, y + magnitude, z],
                ]
            }
        }
        AxisAlignedSegmentAxis::Vertical => {
            let x = first.cross;
            if first.min_vert_id != 0 {
                vec![
                    [x + magnitude, first.along_max, z],
                    [x + magnitude, first.along_min, z],
                    [x - magnitude, first.along_min, z],
                    [x - magnitude, second.along_min, z],
                    [x - magnitude, second.along_min, z],
                    [x - magnitude, second.along_max, z],
                    [x + magnitude, second.along_max, z],
                    [x + magnitude, first.along_max, z],
                    [x + magnitude, first.along_max, z],
                ]
            } else {
                vec![
                    [x - magnitude, first.along_min, z],
                    [x - magnitude, second.along_min, z],
                    [x - magnitude, second.along_min, z],
                    [x - magnitude, second.along_max, z],
                    [x + magnitude, second.along_max, z],
                    [x + magnitude, first.along_max, z],
                    [x + magnitude, first.along_max, z],
                    [x + magnitude, first.along_min, z],
                    [x - magnitude, first.along_min, z],
                ]
            }
        }
    };
    Ok(Some(vec![points]))
}

fn offset_open_cut_vertical_collinear_chain_global_outline(
    segments: &[AxisAlignedOpenCutSegment],
    magnitude: f64,
) -> Option<Vec<[f64; 3]>> {
    if segments
        .iter()
        .any(|segment| segment.axis != AxisAlignedSegmentAxis::Vertical || segment.min_vert_id != 0)
    {
        return None;
    }
    for pair in segments.windows(2) {
        let first = pair[0];
        let second = pair[1];
        if !(second.along_min > first.along_min + 1e-12
            && second.along_min <= first.along_max + 1e-12
            && second.along_max > first.along_max + 1e-12)
        {
            return None;
        }
    }

    let first = segments[0];
    let last = *segments.last()?;
    let x = first.cross;
    let z = first.z;
    let mut points = Vec::with_capacity(segments.len() * 4 + 1);

    points.push([x - magnitude, first.along_min, z]);
    for segment in &segments[1..] {
        points.push([x - magnitude, segment.along_min, z]);
        points.push([x - magnitude, segment.along_min, z]);
    }
    points.push([x - magnitude, last.along_max, z]);

    points.push([x + magnitude, last.along_max, z]);
    for index in (1..segments.len()).rev() {
        let previous = segments[index - 1];
        points.push([x + magnitude, previous.along_max, z]);
        points.push([x + magnitude, previous.along_max, z]);
    }
    points.push([x + magnitude, first.along_min, z]);
    points.push([x - magnitude, first.along_min, z]);

    Some(points)
}

fn offset_open_cut_collinear_overlapping_global_outlines(
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
    let common_cross = local_frame_coordinates(frame_origin, frame_origin, tangent, normal).1;
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
        let alignment = segment_tangent[0] * tangent[0] + segment_tangent[1] * tangent[1];
        if alignment.abs() < 1.0 - 1e-10 {
            return Ok(None);
        }

        let (a_along, a_cross) = local_frame_coordinates(a, frame_origin, tangent, normal);
        let (b_along, b_cross) = local_frame_coordinates(b, frame_origin, tangent, normal);
        if (a_cross - common_cross).abs() > 1e-10 || (b_cross - common_cross).abs() > 1e-10 {
            return Ok(None);
        }
        segments.push(LocalTouchingSegment {
            along_min: a_along.min(b_along),
            along_max: a_along.max(b_along),
        });
    }

    segments.sort_by(|a, b| {
        a.along_min
            .total_cmp(&b.along_min)
            .then_with(|| a.along_max.total_cmp(&b.along_max))
    });
    if segments.len() > 2 {
        return Ok(offset_open_cut_non_axis_three_collinear_overlap_global_outline(
            &segments,
            common_cross,
            offset.abs(),
            frame_origin,
            tangent,
            normal,
            z,
        )
        .map(|outline| vec![outline]));
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
    let forward_cross = common_cross + magnitude;
    let reverse_cross = common_cross - magnitude;
    let local_outline = [
        [first.along_min, reverse_cross, 0.0],
        [first.along_min, forward_cross, 0.0],
        [second.along_min, forward_cross, 0.0],
        [second.along_min, forward_cross, 0.0],
        [second.along_max, forward_cross, 0.0],
        [second.along_max, reverse_cross, 0.0],
        [second.along_min, reverse_cross, 0.0],
        [second.along_min, reverse_cross, 0.0],
        [first.along_min, reverse_cross, 0.0],
    ];
    let outline = local_outline
        .into_iter()
        .map(|[along, cross, _]| local_frame_point(along, cross, frame_origin, tangent, normal, z))
        .collect::<Vec<_>>();
    Ok(Some(vec![outline]))
}

