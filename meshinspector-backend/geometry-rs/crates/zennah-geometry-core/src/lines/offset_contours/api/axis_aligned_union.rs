fn offset_open_cut_axis_aligned_global_outlines(
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

    let magnitude = offset.abs();
    let mut rects = Vec::new();
    for contour in contours {
        if contour.is_empty() {
            continue;
        }
        if is_closed_contour(contour) || contour.len() != 2 {
            return Ok(None);
        }
        validate_contour(contour)?;
        let a = contour[0];
        let b = contour[1];
        if (a[2] - b[2]).abs() > 1e-12
            || rects
                .first()
                .is_some_and(|first: &AxisAlignedOffsetRect| (first.z - a[2]).abs() > 1e-12)
        {
            return Ok(None);
        }

        if (a[1] - b[1]).abs() <= 1e-12 {
            rects.push(AxisAlignedOffsetRect {
                axis: AxisAlignedSegmentAxis::Horizontal,
                x_min: a[0].min(b[0]),
                x_max: a[0].max(b[0]),
                y_min: a[1] - magnitude,
                y_max: a[1] + magnitude,
                z: a[2],
            });
        } else if (a[0] - b[0]).abs() <= 1e-12 {
            rects.push(AxisAlignedOffsetRect {
                axis: AxisAlignedSegmentAxis::Vertical,
                x_min: a[0] - magnitude,
                x_max: a[0] + magnitude,
                y_min: a[1].min(b[1]),
                y_max: a[1].max(b[1]),
                z: a[2],
            });
        } else {
            return Ok(None);
        }
    }

    if rects.len() <= 1 {
        return Ok(None);
    }

    if let Some(outline) = axis_aligned_collinear_chain_outline(&rects) {
        return Ok(Some(vec![outline]));
    }

    let outlines = axis_aligned_rect_union_outlines(&rects)?;
    if outlines.len() >= rects.len() {
        return Ok(None);
    }
    Ok(Some(outlines))
}

fn axis_aligned_collinear_chain_outline(rects: &[AxisAlignedOffsetRect]) -> Option<Vec<[f64; 3]>> {
    let axis = rects.first()?.axis;
    if rects.iter().any(|rect| rect.axis != axis) {
        return None;
    }

    match axis {
        AxisAlignedSegmentAxis::Horizontal => {
            let y_min = rects[0].y_min;
            let y_max = rects[0].y_max;
            if rects
                .iter()
                .any(|rect| (rect.y_min - y_min).abs() > 1e-12 || (rect.y_max - y_max).abs() > 1e-12)
            {
                return None;
            }
            let mut segments = rects.to_vec();
            segments.sort_by(|a, b| a.x_min.total_cmp(&b.x_min).then_with(|| a.x_max.total_cmp(&b.x_max)));
            if !axis_aligned_segments_form_connected_expanding_chain(
                &segments,
                |rect| rect.x_min,
                |rect| rect.x_max,
            ) {
                return None;
            }
            Some(axis_aligned_chain_outline(
                &segments,
                y_max,
                y_min,
                |along, cross, z| [along, cross, z],
            ))
        }
        AxisAlignedSegmentAxis::Vertical => {
            let x_min = rects[0].x_min;
            let x_max = rects[0].x_max;
            if rects
                .iter()
                .any(|rect| (rect.x_min - x_min).abs() > 1e-12 || (rect.x_max - x_max).abs() > 1e-12)
            {
                return None;
            }
            let mut segments = rects.to_vec();
            segments.sort_by(|a, b| a.y_min.total_cmp(&b.y_min).then_with(|| a.y_max.total_cmp(&b.y_max)));
            if !axis_aligned_segments_form_connected_expanding_chain(
                &segments,
                |rect| rect.y_min,
                |rect| rect.y_max,
            ) {
                return None;
            }
            Some(axis_aligned_chain_outline(
                &segments,
                x_min,
                x_max,
                |along, cross, z| [cross, along, z],
            ))
        }
    }
}

fn axis_aligned_segments_form_connected_expanding_chain(
    segments: &[AxisAlignedOffsetRect],
    min_coordinate: impl Fn(&AxisAlignedOffsetRect) -> f64,
    max_coordinate: impl Fn(&AxisAlignedOffsetRect) -> f64,
) -> bool {
    if segments.len() <= 1 {
        return false;
    }
    let mut current_max = max_coordinate(&segments[0]);
    for segment in &segments[1..] {
        if min_coordinate(segment) > current_max + 1e-12 {
            return false;
        }
        let segment_max = max_coordinate(segment);
        if segment_max <= current_max + 1e-12 {
            return false;
        }
        current_max = segment_max;
    }
    true
}

fn axis_aligned_chain_outline(
    segments: &[AxisAlignedOffsetRect],
    forward_cross: f64,
    reverse_cross: f64,
    point: impl Fn(f64, f64, f64) -> [f64; 3],
) -> Vec<[f64; 3]> {
    let z = segments[0].z;
    let along_min = |rect: &AxisAlignedOffsetRect| match rect.axis {
        AxisAlignedSegmentAxis::Horizontal => rect.x_min,
        AxisAlignedSegmentAxis::Vertical => rect.y_min,
    };
    let along_max = |rect: &AxisAlignedOffsetRect| match rect.axis {
        AxisAlignedSegmentAxis::Horizontal => rect.x_max,
        AxisAlignedSegmentAxis::Vertical => rect.y_max,
    };

    let mut outline = Vec::with_capacity(segments.len() * 4 + 1);
    outline.push(point(along_min(&segments[0]), forward_cross, z));
    for (index, segment) in segments.iter().enumerate() {
        let forward = point(along_max(segment), forward_cross, z);
        outline.push(forward);
        if index + 1 < segments.len() {
            outline.push(forward);
        }
    }

    outline.push(point(
        along_max(segments.last().expect("segments are non-empty")),
        reverse_cross,
        z,
    ));
    for (index, segment) in segments.iter().enumerate().rev() {
        let reverse = point(along_min(segment), reverse_cross, z);
        outline.push(reverse);
        if index > 0 {
            outline.push(reverse);
        }
    }
    outline.push(outline[0]);
    outline
}

fn axis_aligned_rect_union_outlines(
    rects: &[AxisAlignedOffsetRect],
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    let xs = sorted_unique_coordinates(
        rects
            .iter()
            .flat_map(|rect| [rect.x_min, rect.x_max])
            .collect(),
    );
    let ys = sorted_unique_coordinates(
        rects
            .iter()
            .flat_map(|rect| [rect.y_min, rect.y_max])
            .collect(),
    );
    if xs.len() < 2 || ys.len() < 2 {
        return Ok(Vec::new());
    }

    let x_cell_count = xs.len() - 1;
    let y_cell_count = ys.len() - 1;
    let mut covered = vec![vec![false; y_cell_count]; x_cell_count];
    for x_index in 0..x_cell_count {
        let x_mid = (xs[x_index] + xs[x_index + 1]) * 0.5;
        for y_index in 0..y_cell_count {
            let y_mid = (ys[y_index] + ys[y_index + 1]) * 0.5;
            covered[x_index][y_index] = rects.iter().any(|rect| {
                x_mid >= rect.x_min - 1e-12
                    && x_mid <= rect.x_max + 1e-12
                    && y_mid >= rect.y_min - 1e-12
                    && y_mid <= rect.y_max + 1e-12
            });
        }
    }

    let mut edges: Vec<((usize, usize), (usize, usize))> = Vec::new();
    for x_index in 0..x_cell_count {
        for y_index in 0..y_cell_count {
            if !covered[x_index][y_index] {
                continue;
            }
            if y_index == 0 || !covered[x_index][y_index - 1] {
                edges.push(((x_index + 1, y_index), (x_index, y_index)));
            }
            if x_index == 0 || !covered[x_index - 1][y_index] {
                edges.push(((x_index, y_index), (x_index, y_index + 1)));
            }
            if y_index + 1 == y_cell_count || !covered[x_index][y_index + 1] {
                edges.push(((x_index, y_index + 1), (x_index + 1, y_index + 1)));
            }
            if x_index + 1 == x_cell_count || !covered[x_index + 1][y_index] {
                edges.push(((x_index + 1, y_index + 1), (x_index + 1, y_index)));
            }
        }
    }

    let mut outlines = Vec::new();
    while !edges.is_empty() {
        let (start, mut end) = edges.remove(0);
        let mut grid_points = vec![start, end];
        while end != start {
            let Some(next_edge_index) = edges.iter().position(|(edge_start, _)| *edge_start == end)
            else {
                return Err("OffsetContours global cut outline boundary is not closed".to_string());
            };
            let (_, next_end) = edges.remove(next_edge_index);
            end = next_end;
            grid_points.push(end);
        }

        let mut outline = grid_points
            .iter()
            .map(|(x_index, y_index)| [xs[*x_index], ys[*y_index], rects[0].z])
            .collect::<Vec<_>>();
        orient_and_rotate_global_axis_aligned_outline(&mut outline);
        outlines.push(outline);
    }

    outlines.sort_by(|a, b| {
        let a_key = outline_sort_key(a);
        let b_key = outline_sort_key(b);
        a_key
            .0
            .total_cmp(&b_key.0)
            .then_with(|| b_key.1.total_cmp(&a_key.1))
    });
    Ok(outlines)
}

fn sorted_unique_coordinates(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(|a, b| a.total_cmp(b));
    values.dedup_by(|a, b| (*a - *b).abs() <= 1e-12);
    values
}

fn orient_and_rotate_global_axis_aligned_outline(outline: &mut Vec<[f64; 3]>) {
    if outline.len() <= 2 {
        return;
    }
    if signed_area_xy(outline) > 0.0 {
        outline.pop();
        outline.reverse();
        outline.push(outline[0]);
    }
    remove_axis_aligned_collinear_points(outline);

    let open_len = outline.len() - 1;
    let start_index = (0..open_len)
        .min_by(|left, right| {
            outline[*left][1]
                .total_cmp(&outline[*right][1])
                .then_with(|| outline[*right][0].total_cmp(&outline[*left][0]))
        })
        .unwrap_or(0);
    if start_index == 0 {
        return;
    }

    let mut rotated = outline[..open_len].to_vec();
    rotated.rotate_left(start_index);
    rotated.push(rotated[0]);
    *outline = rotated;
}

fn remove_axis_aligned_collinear_points(outline: &mut Vec<[f64; 3]>) {
    if outline.len() <= 3 {
        return;
    }
    outline.pop();
    loop {
        let len = outline.len();
        let Some(remove_index) = (0..len).find(|index| {
            let previous = outline[(*index + len - 1) % len];
            let current = outline[*index];
            let next = outline[(*index + 1) % len];
            ((previous[0] - current[0]).abs() <= 1e-12
                && (current[0] - next[0]).abs() <= 1e-12)
                || ((previous[1] - current[1]).abs() <= 1e-12
                    && (current[1] - next[1]).abs() <= 1e-12)
        }) else {
            break;
        };
        outline.remove(remove_index);
        if outline.len() <= 2 {
            break;
        }
    }
    outline.push(outline[0]);
}

fn outline_sort_key(outline: &[[f64; 3]]) -> (f64, f64) {
    outline
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min_y, max_x), point| {
            (min_y.min(point[1]), max_x.max(point[0]))
        })
}
