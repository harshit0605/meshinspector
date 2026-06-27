use super::metrics::{
    combine_fill_metric_with_mode, edge_fill_metric_with_mode, triangle_fill_metric_with_mode,
    FillMetricContext,
};
use super::FillHoleMetricMode;
use std::collections::{HashMap, HashSet};

pub(super) type BoundaryEdgeContexts = HashMap<(usize, usize), usize>;

#[derive(Clone, Copy, Debug)]
pub(super) struct CandidateWeights {
    pub left_weight: f64,
    pub left_prev: Option<usize>,
    pub right_weight: f64,
    pub right_prev: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CandidateRequest<'a> {
    pub vertices: &'a [[f64; 3]],
    pub points: &'a [[f64; 3]],
    pub boundary_loop: &'a [usize],
    pub boundary_contexts: &'a BoundaryEdgeContexts,
    pub fill_metric_mode: FillHoleMetricMode,
    pub metric_context: FillMetricContext,
    pub smooth_bd: bool,
    pub start: usize,
    pub split: usize,
    pub end: usize,
    pub include_final_edge: bool,
}

pub(super) fn boundary_edge_contexts(
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
) -> BoundaryEdgeContexts {
    let boundary_vertices = boundary_loop.iter().copied().collect::<HashSet<_>>();
    let boundary_edges = boundary_loop
        .iter()
        .enumerate()
        .map(|(index, &vertex)| {
            let next = boundary_loop[(index + 1) % boundary_loop.len()];
            ordered_pair(vertex, next)
        })
        .collect::<HashSet<_>>();

    let mut contexts = BoundaryEdgeContexts::new();
    for face in existing_faces {
        if face.iter().any(|vertex| *vertex < 0) {
            continue;
        }
        let face = [face[0] as usize, face[1] as usize, face[2] as usize];
        for edge_index in 0..3 {
            let a = face[edge_index];
            let b = face[(edge_index + 1) % 3];
            let opposite = face[(edge_index + 2) % 3];
            let edge = ordered_pair(a, b);
            if boundary_edges.contains(&edge) && !boundary_vertices.contains(&opposite) {
                contexts.entry(edge).or_insert(opposite);
            }
        }
    }
    contexts
}

pub(super) fn candidate_weight(request: CandidateRequest<'_>, weights: CandidateWeights) -> f64 {
    let mut weight = combine_fill_metric_with_mode(
        weights.left_weight,
        weights.right_weight,
        request.fill_metric_mode,
    );
    weight = combine_fill_metric_with_mode(
        weight,
        triangle_fill_metric_with_mode(
            request.points[request.start],
            request.points[request.split],
            request.points[request.end],
            request.fill_metric_mode,
            request.metric_context,
        ),
        request.fill_metric_mode,
    );
    for edge_weight in [
        edge_metric_or_zero(
            &request,
            request.start,
            request.split,
            weights.left_prev,
            request.end,
        ),
        edge_metric_or_zero(
            &request,
            request.split,
            request.end,
            weights.right_prev,
            request.start,
        ),
    ] {
        weight = combine_fill_metric_with_mode(weight, edge_weight, request.fill_metric_mode);
    }
    if request.include_final_edge {
        weight = combine_fill_metric_with_mode(
            weight,
            final_edge_metric_or_zero(&request, weights.left_prev, weights.right_prev),
            request.fill_metric_mode,
        );
    }
    weight
}

fn edge_metric_or_zero(
    request: &CandidateRequest<'_>,
    a_pos: usize,
    b_pos: usize,
    adjacent_prev: Option<usize>,
    other_pos: usize,
) -> f64 {
    let Some(left_vertex) = adjacent_prev
        .map(|pos| request.boundary_loop[pos])
        .or_else(|| boundary_context_vertex(request, a_pos, b_pos))
    else {
        return 0.0;
    };
    edge_fill_metric_with_mode(
        request.vertices,
        request.boundary_loop[a_pos],
        request.boundary_loop[b_pos],
        left_vertex,
        request.boundary_loop[other_pos],
        request.fill_metric_mode,
        request.metric_context,
    )
    .unwrap_or(0.0)
}

fn final_edge_metric_or_zero(
    request: &CandidateRequest<'_>,
    left_prev: Option<usize>,
    right_prev: Option<usize>,
) -> f64 {
    let Some(left_vertex) = left_prev
        .map(|pos| request.boundary_loop[pos])
        .or_else(|| boundary_context_vertex(request, request.start, request.end))
    else {
        return 0.0;
    };
    let Some(right_vertex) = right_prev
        .map(|pos| request.boundary_loop[pos])
        .or_else(|| boundary_context_vertex(request, request.start, request.end))
    else {
        return 0.0;
    };
    edge_fill_metric_with_mode(
        request.vertices,
        request.boundary_loop[request.start],
        request.boundary_loop[request.end],
        left_vertex,
        right_vertex,
        request.fill_metric_mode,
        request.metric_context,
    )
    .unwrap_or(0.0)
}

fn boundary_context_vertex(
    request: &CandidateRequest<'_>,
    a_pos: usize,
    b_pos: usize,
) -> Option<usize> {
    if !request.smooth_bd || !positions_are_adjacent(a_pos, b_pos, request.boundary_loop.len()) {
        return None;
    }
    let edge = ordered_pair(request.boundary_loop[a_pos], request.boundary_loop[b_pos]);
    request.boundary_contexts.get(&edge).copied()
}

fn positions_are_adjacent(a: usize, b: usize, n: usize) -> bool {
    a.abs_diff(b) == 1 || a.abs_diff(b) + 1 == n
}

fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
