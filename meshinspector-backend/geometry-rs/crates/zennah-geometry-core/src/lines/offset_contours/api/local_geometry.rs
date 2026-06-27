fn local_frame_coordinates(
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

fn local_segments_form_touching_expanding_chain(segments: &[LocalTouchingSegment]) -> bool {
    if segments.len() <= 1 {
        return false;
    }
    let mut current_max = segments[0].along_max;
    for segment in &segments[1..] {
        if (segment.along_min - current_max).abs() > 1e-10 {
            return false;
        }
        if segment.along_max <= current_max + 1e-12 {
            return false;
        }
        current_max = segment.along_max;
    }
    true
}

fn local_touching_chain_outline(
    segments: &[LocalTouchingSegment],
    forward_cross: f64,
    reverse_cross: f64,
    origin: [f64; 3],
    tangent: [f64; 2],
    normal: [f64; 2],
    z: f64,
) -> Vec<[f64; 3]> {
    let mut outline = Vec::with_capacity(segments.len() * 4 + 1);
    outline.push(local_frame_point(
        segments[0].along_min,
        forward_cross,
        origin,
        tangent,
        normal,
        z,
    ));
    for (index, segment) in segments.iter().enumerate() {
        let forward = local_frame_point(
            segment.along_max,
            forward_cross,
            origin,
            tangent,
            normal,
            z,
        );
        outline.push(forward);
        if index + 1 < segments.len() {
            outline.push(forward);
        }
    }

    outline.push(local_frame_point(
        segments.last().expect("segments are non-empty").along_max,
        reverse_cross,
        origin,
        tangent,
        normal,
        z,
    ));
    for (index, segment) in segments.iter().enumerate().rev() {
        let reverse = local_frame_point(
            segment.along_min,
            reverse_cross,
            origin,
            tangent,
            normal,
            z,
        );
        outline.push(reverse);
        if index > 0 {
            outline.push(reverse);
        }
    }
    outline.push(outline[0]);
    outline
}

fn local_frame_point(
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

fn local_rect_union_outlines(rects: &[LocalOffsetRect]) -> Result<Vec<Vec<[f64; 3]>>, String> {
    let xs = sorted_unique_coordinates(
        rects
            .iter()
            .flat_map(|rect| [rect.along_min, rect.along_max])
            .collect(),
    );
    let ys = sorted_unique_coordinates(
        rects
            .iter()
            .flat_map(|rect| [rect.cross_min, rect.cross_max])
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
                x_mid >= rect.along_min - 1e-12
                    && x_mid <= rect.along_max + 1e-12
                    && y_mid >= rect.cross_min - 1e-12
                    && y_mid <= rect.cross_max + 1e-12
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
                return Err("OffsetContours local cut outline boundary is not closed".to_string());
            };
            let (_, next_end) = edges.remove(next_edge_index);
            end = next_end;
            grid_points.push(end);
        }

        let mut outline = grid_points
            .iter()
            .map(|(x_index, y_index)| [xs[*x_index], ys[*y_index], 0.0])
            .collect::<Vec<_>>();
        orient_and_rotate_global_axis_aligned_outline(&mut outline);
        outlines.push(outline);
    }
    Ok(outlines)
}

fn rotate_local_parallel_outline_to_meshlib_start(
    outline: &mut Vec<[f64; 3]>,
    rects: &[LocalOffsetRect],
) {
    if outline.len() <= 2 {
        return;
    }
    let open_len = outline.len() - 1;
    let mut sorted_rects = rects.to_vec();
    sorted_rects.sort_by(|a, b| {
        a.along_min
            .total_cmp(&b.along_min)
            .then_with(|| a.cross_min.total_cmp(&b.cross_min))
    });
    let first = sorted_rects[0];
    let target_cross = sorted_rects
        .iter()
        .map(|rect| rect.cross_min)
        .fold(f64::NEG_INFINITY, f64::max);
    let target_index = (0..open_len).find(|index| {
        (outline[*index][0] - first.along_max).abs() <= 1e-9
            && (outline[*index][1] - target_cross).abs() <= 1e-9
    });
    let Some(start_index) = target_index else {
        return;
    };
    if start_index == 0 {
        return;
    }
    let mut rotated = outline[..open_len].to_vec();
    rotated.rotate_left(start_index);
    rotated.push(rotated[0]);
    *outline = rotated;
}
