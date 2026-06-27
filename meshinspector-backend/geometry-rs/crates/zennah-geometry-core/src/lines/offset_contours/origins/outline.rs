#[derive(Debug, Clone, Copy)]
struct OutlineIntersection {
    first: usize,
    second: usize,
    point: [f64; 3],
    first_ratio: f64,
    second_ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutlineOriginMode {
    CanonicalSourceEdges,
    PositiveVariableSourceEdges,
    OpenCanonicalSourceEdges,
    OpenPositiveVariableSourceEdges,
}

fn simplify_self_overlapping_outline_with_origins(
    source_points: &[[f64; 3]],
    contour_id: usize,
    points: Vec<[f64; 3]>,
    origins: Vec<OffsetContoursOrigin>,
    origin_mode: OutlineOriginMode,
) -> (Vec<[f64; 3]>, Vec<OffsetContoursOrigin>) {
    if points.len() < 5 || points.len() != origins.len() {
        return (points, origins);
    }
    if origin_mode == OutlineOriginMode::OpenCanonicalSourceEdges {
        return simplify_open_canonical_outline_with_origins(
            source_points,
            contour_id,
            points,
            origins,
        );
    }

    let segment_count = points.len() - 1;
    let mut output = Vec::with_capacity(points.len());
    let mut output_origins = Vec::with_capacity(origins.len());
    let mut index = 0;
    let mut started_at_open_intersection = false;
    while index < segment_count {
        if let Some(intersection) = find_next_outline_intersection(&points, index) {
            let origin = outline_intersection_origin(
                source_points,
                contour_id,
                &origins,
                intersection.first,
                intersection.second,
                intersection.point,
                intersection.first_ratio,
                intersection.second_ratio,
                origin_mode,
            );
            if origin_mode == OutlineOriginMode::OpenPositiveVariableSourceEdges
                && index == 0
                && output.is_empty()
            {
                output.push(restore_outline_intersection_z(
                    source_points,
                    intersection.point,
                    &origin,
                ));
                output_origins.push(origin);
                started_at_open_intersection = true;
                index = intersection.second + 1;
                continue;
            }

            output.push(points[index]);
            output_origins.push(origins[index]);
            output.push(restore_outline_intersection_z(
                source_points,
                intersection.point,
                &origin,
            ));
            output_origins.push(origin);
            index = intersection.second + 1;
        } else {
            output.push(points[index]);
            output_origins.push(origins[index]);
            index += 1;
        }
    }

    if started_at_open_intersection {
        if let (Some(last_input), Some(last_origin)) =
            (points.get(segment_count).copied(), origins.get(segment_count).copied())
        {
            if output.last().is_none_or(|last| !same_xy(*last, last_input)) {
                output.push(last_input);
                output_origins.push(last_origin);
            }
        }
    }

    if let Some(first) = output.first().copied() {
        if output.last().is_none_or(|last| !same_xy(*last, first)) {
            output.push(first);
            output_origins.push(
                output_origins
                    .first()
                    .copied()
                    .unwrap_or_else(|| OffsetContoursOrigin::source_vertex(contour_id, 0)),
            );
        }
    }
    (output, output_origins)
}

fn simplify_open_canonical_outline_with_origins(
    source_points: &[[f64; 3]],
    contour_id: usize,
    points: Vec<[f64; 3]>,
    origins: Vec<OffsetContoursOrigin>,
) -> (Vec<[f64; 3]>, Vec<OffsetContoursOrigin>) {
    let segment_count = points.len() - 1;
    let Some(anchor) = find_open_canonical_anchor_intersection(&points) else {
        return (points, origins);
    };
    let start = if anchor.second + 1 == segment_count {
        anchor.first + 1
    } else {
        anchor.second + 1
    };
    if start >= segment_count {
        return (points, origins);
    }

    let mut output = Vec::with_capacity(points.len());
    let mut output_origins = Vec::with_capacity(origins.len());
    let mut index = start;
    while index < segment_count {
        if let Some(intersection) = find_next_outline_intersection(&points, index) {
            let origin = outline_intersection_origin(
                source_points,
                contour_id,
                &origins,
                intersection.first,
                intersection.second,
                intersection.point,
                intersection.first_ratio,
                intersection.second_ratio,
                OutlineOriginMode::OpenCanonicalSourceEdges,
            );
            output.push(points[index]);
            output_origins.push(origins[index]);
            output.push(restore_outline_intersection_z(
                source_points,
                intersection.point,
                &origin,
            ));
            output_origins.push(origin);
            index = intersection.second + 1;
        } else {
            output.push(points[index]);
            output_origins.push(origins[index]);
            index += 1;
        }
    }

    if anchor.second + 1 != segment_count {
        if let (Some(point), Some(origin)) = (
            points.first().copied(),
            origins.first().copied(),
        ) {
            if output.last().is_none_or(|last| !same_xy(*last, point)) {
                output.push(point);
                output_origins.push(origin);
            }
        }
    }

    let anchor_origin = open_canonical_anchor_intersection_origin(
        source_points,
        contour_id,
        &origins,
        &anchor,
        segment_count,
    );
    output.push(restore_outline_intersection_z(
        source_points,
        anchor.point,
        &anchor_origin,
    ));
    output_origins.push(anchor_origin);
    if let Some(first) = output.first().copied() {
        output.push(first);
        output_origins.push(
            output_origins
                .first()
                .copied()
                .unwrap_or_else(|| OffsetContoursOrigin::source_vertex(contour_id, 0)),
        );
    }
    (output, output_origins)
}

fn find_open_canonical_anchor_intersection(points: &[[f64; 3]]) -> Option<OutlineIntersection> {
    let segment_count = points.len().checked_sub(1)?;
    let mut first_segment_anchor = None;
    let mut closing_segment_anchor = None;
    for first in 0..segment_count {
        for second in first + 2..segment_count {
            if first == 0 && second + 1 == segment_count {
                continue;
            }
            let Some((point, first_ratio, second_ratio)) = segment_intersection_xy(
                points[first],
                points[first + 1],
                points[second],
                points[second + 1],
            ) else {
                continue;
            };
            let intersection = OutlineIntersection {
                first,
                second,
                point,
                first_ratio,
                second_ratio,
            };
            if second + 1 == segment_count && first > 0 {
                if closing_segment_anchor
                    .as_ref()
                    .is_none_or(|candidate: &OutlineIntersection| first < candidate.first)
                {
                    closing_segment_anchor = Some(intersection);
                }
            } else if first == 0
                && first_segment_anchor
                    .as_ref()
                    .is_none_or(|candidate: &OutlineIntersection| second < candidate.second)
            {
                first_segment_anchor = Some(intersection);
            }
        }
    }
    closing_segment_anchor.or(first_segment_anchor)
}

fn open_canonical_anchor_intersection_origin(
    source_points: &[[f64; 3]],
    contour_id: usize,
    origins: &[OffsetContoursOrigin],
    intersection: &OutlineIntersection,
    segment_count: usize,
) -> OffsetContoursOrigin {
    if intersection.second + 1 == segment_count {
        if let (Some(first_edge), Some(second_edge)) = (
            raw_source_edge_from_output_segment_allow_degenerate(
                source_points,
                contour_id,
                origins,
                intersection.first,
            ),
            raw_source_edge_from_output_segment_allow_degenerate(
                source_points,
                contour_id,
                origins,
                intersection.second,
            ),
        ) {
            let reversed_first = SourceEdge {
                org: first_edge.dest,
                dest: first_edge.org,
            };
            return OffsetContoursOrigin {
                l_org: source_index(contour_id, second_edge.org),
                l_dest: source_index(contour_id, second_edge.dest),
                u_org: source_index(contour_id, reversed_first.org),
                u_dest: source_index(contour_id, reversed_first.dest),
                l_ratio: if second_edge.org == second_edge.dest {
                    intersection.second_ratio
                } else {
                    source_edge_ratio(second_edge, source_points, intersection.point)
                },
                u_ratio: if first_edge.org == first_edge.dest {
                    1.0 - intersection.first_ratio
                } else {
                    source_edge_ratio(reversed_first, source_points, intersection.point)
                },
            };
        }
    }

    outline_intersection_origin(
        source_points,
        contour_id,
        origins,
        intersection.first,
        intersection.second,
        intersection.point,
        intersection.first_ratio,
        intersection.second_ratio,
        OutlineOriginMode::OpenCanonicalSourceEdges,
    )
}

fn find_next_outline_intersection(
    points: &[[f64; 3]],
    first: usize,
) -> Option<OutlineIntersection> {
    let segment_count = points.len().checked_sub(1)?;
    let mut best: Option<(f64, OutlineIntersection)> = None;
    for second in first + 2..segment_count {
        if first == 0 && second + 1 == segment_count {
            continue;
        }
        let Some((point, first_ratio, second_ratio)) =
            segment_intersection_xy(points[first], points[first + 1], points[second], points[second + 1])
        else {
            continue;
        };
        if best.is_none_or(|(best_ratio, _)| first_ratio < best_ratio) {
            best = Some((
                first_ratio,
                OutlineIntersection {
                    first,
                    second,
                    point,
                    first_ratio,
                    second_ratio,
                },
            ));
        }
    }
    best.map(|(_, intersection)| intersection)
}

fn segment_intersection_xy(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
) -> Option<([f64; 3], f64, f64)> {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let cd = [d[0] - c[0], d[1] - c[1]];
    let denominator = ab[0] * cd[1] - ab[1] * cd[0];
    if denominator.abs() <= 1e-12 {
        return None;
    }
    let ac = [c[0] - a[0], c[1] - a[1]];
    let first_ratio = (ac[0] * cd[1] - ac[1] * cd[0]) / denominator;
    let second_ratio = (ac[0] * ab[1] - ac[1] * ab[0]) / denominator;
    if !(1e-9..=1.0 - 1e-9).contains(&first_ratio)
        || !(1e-9..=1.0 - 1e-9).contains(&second_ratio)
    {
        return None;
    }
    Some((
        [
            a[0] + ab[0] * first_ratio,
            a[1] + ab[1] * first_ratio,
            (1.0 - first_ratio) * a[2] + first_ratio * b[2],
        ],
        first_ratio,
        second_ratio,
    ))
}

fn outline_intersection_origin(
    source_points: &[[f64; 3]],
    contour_id: usize,
    origins: &[OffsetContoursOrigin],
    first: usize,
    second: usize,
    intersection: [f64; 3],
    first_ratio: f64,
    second_ratio: f64,
    mode: OutlineOriginMode,
) -> OffsetContoursOrigin {
    if mode == OutlineOriginMode::OpenCanonicalSourceEdges && first == 0 {
        if let Some(first_edge) =
            raw_source_edge_from_output_segment(source_points, contour_id, origins, first)
        {
            let second_edge = raw_source_edge_from_output_segment_allow_degenerate(
                source_points,
                contour_id,
                origins,
                second,
            );
            let Some(second_edge) = second_edge else {
                return OffsetContoursOrigin {
                    l_org: source_index(contour_id, first_edge.org),
                    l_dest: source_index(contour_id, first_edge.dest),
                    u_org: OffsetContourIndex::unknown(),
                    u_dest: OffsetContourIndex::unknown(),
                    l_ratio: source_edge_ratio(first_edge, source_points, intersection),
                    u_ratio: 0.0,
                };
            };
            let reversed_second = SourceEdge {
                org: second_edge.dest,
                dest: second_edge.org,
            };
            return OffsetContoursOrigin {
                l_org: source_index(contour_id, first_edge.org),
                l_dest: source_index(contour_id, first_edge.dest),
                u_org: source_index(contour_id, reversed_second.org),
                u_dest: source_index(contour_id, reversed_second.dest),
                l_ratio: source_edge_ratio(first_edge, source_points, intersection),
                u_ratio: if second_edge.org == second_edge.dest {
                    1.0 - second_ratio
                } else {
                    source_edge_ratio(reversed_second, source_points, intersection)
                },
            };
        }
    }

    if mode == OutlineOriginMode::PositiveVariableSourceEdges
        || mode == OutlineOriginMode::OpenPositiveVariableSourceEdges
    {
        if let (Some(first_edge), Some(second_edge)) = (
            raw_source_edge_from_output_segment(source_points, contour_id, origins, first),
            raw_source_edge_from_output_segment(source_points, contour_id, origins, second),
        ) {
            return OffsetContoursOrigin {
                l_org: source_index(contour_id, second_edge.org),
                l_dest: source_index(contour_id, second_edge.dest),
                u_org: source_index(contour_id, first_edge.org),
                u_dest: source_index(contour_id, first_edge.dest),
                l_ratio: second_ratio,
                u_ratio: first_ratio,
            };
        }
    }

    let Some(first_edge) = source_edge_from_output_segment(source_points, contour_id, origins, first)
    else {
        return origins
            .get(first)
            .copied()
            .unwrap_or_else(|| OffsetContoursOrigin::source_vertex(contour_id, 0));
    };
    let Some(second_edge) =
        source_edge_from_output_segment(source_points, contour_id, origins, second)
    else {
        return origins
            .get(first)
            .copied()
            .unwrap_or_else(|| OffsetContoursOrigin::source_vertex(contour_id, 0));
    };
    let (lower, upper) =
        if source_edge_angle(first_edge, source_points) >= source_edge_angle(second_edge, source_points) {
            (first_edge, second_edge)
        } else {
            (second_edge, first_edge)
        };
    OffsetContoursOrigin {
        l_org: source_index(contour_id, lower.org),
        l_dest: source_index(contour_id, lower.dest),
        u_org: source_index(contour_id, upper.org),
        u_dest: source_index(contour_id, upper.dest),
        l_ratio: source_edge_ratio(lower, source_points, intersection),
        u_ratio: source_edge_ratio(upper, source_points, intersection),
    }
}

fn source_edge_from_output_segment(
    source_points: &[[f64; 3]],
    contour_id: usize,
    origins: &[OffsetContoursOrigin],
    segment: usize,
) -> Option<SourceEdge> {
    let raw = raw_source_edge_from_output_segment(source_points, contour_id, origins, segment)?;
    Some(canonical_ascending_edge(raw.org, raw.dest, source_points))
}

fn raw_source_edge_from_output_segment(
    source_points: &[[f64; 3]],
    contour_id: usize,
    origins: &[OffsetContoursOrigin],
    segment: usize,
) -> Option<SourceEdge> {
    let edge =
        raw_source_edge_from_output_segment_allow_degenerate(source_points, contour_id, origins, segment)?;
    if edge.org == edge.dest {
        return None;
    }
    Some(edge)
}

fn raw_source_edge_from_output_segment_allow_degenerate(
    source_points: &[[f64; 3]],
    contour_id: usize,
    origins: &[OffsetContoursOrigin],
    segment: usize,
) -> Option<SourceEdge> {
    let org = origins.get(segment)?.l_org;
    let dest = origins.get(segment + 1)?.l_org;
    if org.contour_id != contour_id as i32 || dest.contour_id != contour_id as i32 {
        return None;
    }
    let org = usize::try_from(org.vert_id).ok()?;
    let dest = usize::try_from(dest.vert_id).ok()?;
    if org >= source_points.len() || dest >= source_points.len() {
        return None;
    }
    Some(SourceEdge { org, dest })
}

fn restore_outline_intersection_z(
    source_points: &[[f64; 3]],
    mut point: [f64; 3],
    origin: &OffsetContoursOrigin,
) -> [f64; 3] {
    if !origin.is_intersection() {
        return point;
    }
    let Some(l_org) = source_points.get(origin.l_org.vert_id as usize) else {
        return point;
    };
    let Some(l_dest) = source_points.get(origin.l_dest.vert_id as usize) else {
        return point;
    };
    let Some(u_org) = source_points.get(origin.u_org.vert_id as usize) else {
        return point;
    };
    let Some(u_dest) = source_points.get(origin.u_dest.vert_id as usize) else {
        return point;
    };
    let lower_z = (1.0 - origin.l_ratio) * l_org[2] + origin.l_ratio * l_dest[2];
    let upper_z = (1.0 - origin.u_ratio) * u_org[2] + origin.u_ratio * u_dest[2];
    point[2] = (lower_z + upper_z) * 0.5;
    point
}
