use super::exact_boolean::{
    ExactBooleanOperand, ExactBooleanOperation, ExactBooleanStitchedEdgeSource,
};
use super::exact_cut_apply::ExactCutMeshResult;
use super::exact_stitch::{ExactStitchPath, ExactStitchPlan};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct MappedStitchedEdges {
    pub(super) sources: Vec<ExactBooleanStitchedEdgeSource>,
    pub(super) paths: Vec<ExactStitchPath>,
}

pub(super) struct MappedResultCutPaths {
    pub(super) paths: Vec<Vec<[usize; 2]>>,
    pub(super) closed: Vec<bool>,
    pub(super) complete: bool,
}

pub(super) fn mapped_result_cut_paths(
    operation: ExactBooleanOperation,
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    stitch_plan: Option<&ExactStitchPlan>,
    first_vertex_map: &[Option<usize>],
    second_vertex_map: &[Option<usize>],
    epsilon: f64,
) -> MappedResultCutPaths {
    let (source_operand, source, vertex_map) = result_cut_source(
        operation,
        first,
        second,
        first_vertex_map,
        second_vertex_map,
    );
    let vertex_lookup = ResultCutVertexLookup::new(
        source_operand,
        first,
        second,
        first_vertex_map,
        second_vertex_map,
        epsilon,
    );
    let stitch_fallbacks = result_cut_stitch_fallbacks(
        source_operand,
        stitch_plan,
        first_vertex_map,
        second_vertex_map,
    );
    let mut complete = true;
    let mut paths = Vec::new();
    let mut closed = Vec::new();
    for (path_index, path) in source.cut_edge_paths.iter().enumerate() {
        let source_closed = source
            .cut_edge_path_closed
            .get(path_index)
            .copied()
            .unwrap_or(false);
        let mut mapped_path = Vec::with_capacity(path.len());
        let mut path_complete = true;
        for edge in path {
            match mapped_directed_edge(vertex_map, *edge)
                .or_else(|| vertex_lookup.mapped_source_edge(*edge))
                .or_else(|| stitch_fallbacks.get(edge).copied())
            {
                Some(mapped_edge) => mapped_path.push(mapped_edge),
                None => {
                    complete = false;
                    path_complete = false;
                    if !mapped_path.is_empty() {
                        paths.push(std::mem::take(&mut mapped_path));
                        closed.push(false);
                    }
                }
            }
        }
        if !mapped_path.is_empty() || path.is_empty() {
            paths.push(mapped_path);
            closed.push(source_closed && path_complete);
        }
    }
    MappedResultCutPaths {
        paths,
        closed,
        complete,
    }
}

pub(super) fn mapped_prepare_result_cut_paths(
    operation: ExactBooleanOperation,
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    first_prepare_faces: &[usize],
    second_prepare_faces: &[usize],
) -> MappedResultCutPaths {
    let source_operand = result_cut_source_operand(operation);
    let (source, prepare_faces) = match source_operand {
        ExactBooleanOperand::First => (first, first_prepare_faces),
        ExactBooleanOperand::Second => (second, second_prepare_faces),
    };
    let prepared_map = PreparedPartMap::new(source, prepare_faces);
    let mut complete = true;
    let mut paths = Vec::new();
    let mut closed = Vec::new();
    for (path_index, path) in source.cut_edge_paths.iter().enumerate() {
        let source_closed = source
            .cut_edge_path_closed
            .get(path_index)
            .copied()
            .unwrap_or(false);
        let mut mapped_path = Vec::with_capacity(path.len());
        let mut path_complete = true;
        for edge in path {
            match prepared_map.mapped_directed_edge(*edge) {
                Some(mapped_edge) => mapped_path.push(mapped_edge),
                None => {
                    complete = false;
                    path_complete = false;
                    if !mapped_path.is_empty() {
                        paths.push(std::mem::take(&mut mapped_path));
                        closed.push(false);
                    }
                }
            }
        }
        if !mapped_path.is_empty() || path.is_empty() {
            paths.push(mapped_path);
            closed.push(source_closed && path_complete);
        }
    }
    MappedResultCutPaths {
        paths,
        closed,
        complete,
    }
}

fn result_cut_source<'a>(
    operation: ExactBooleanOperation,
    first: &'a ExactCutMeshResult,
    second: &'a ExactCutMeshResult,
    first_vertex_map: &'a [Option<usize>],
    second_vertex_map: &'a [Option<usize>],
) -> (
    ExactBooleanOperand,
    &'a ExactCutMeshResult,
    &'a [Option<usize>],
) {
    match result_cut_source_operand(operation) {
        ExactBooleanOperand::First => (ExactBooleanOperand::First, first, first_vertex_map),
        ExactBooleanOperand::Second => (ExactBooleanOperand::Second, second, second_vertex_map),
    }
}

fn result_cut_source_operand(operation: ExactBooleanOperation) -> ExactBooleanOperand {
    if requires_stitched_result_cut(operation) {
        if operation == ExactBooleanOperation::Intersection {
            ExactBooleanOperand::Second
        } else {
            ExactBooleanOperand::First
        }
    } else if matches!(
        operation,
        ExactBooleanOperation::InsideA | ExactBooleanOperation::OutsideA
    ) {
        ExactBooleanOperand::First
    } else {
        ExactBooleanOperand::Second
    }
}

struct PreparedPartMap {
    vertices: Vec<Option<usize>>,
    edges: BTreeSet<[usize; 2]>,
}

impl PreparedPartMap {
    fn new(source: &ExactCutMeshResult, prepare_faces: &[usize]) -> Self {
        let mut vertices = vec![None; source.vertices.len()];
        let mut edges = BTreeSet::new();
        for face_index in prepare_faces {
            let Some(face) = source.faces.get(*face_index) else {
                continue;
            };
            let Some(face_vertices) = valid_face_vertices(*face, source.vertices.len()) else {
                continue;
            };
            for vertex in face_vertices {
                let next_vertex = vertices.iter().filter(|vertex| vertex.is_some()).count();
                vertices[vertex].get_or_insert(next_vertex);
            }
            edges.insert(ordered_edge([face_vertices[0], face_vertices[1]]));
            edges.insert(ordered_edge([face_vertices[1], face_vertices[2]]));
            edges.insert(ordered_edge([face_vertices[2], face_vertices[0]]));
        }
        Self { vertices, edges }
    }

    fn mapped_directed_edge(&self, edge: [usize; 2]) -> Option<[usize; 2]> {
        self.edges.contains(&ordered_edge(edge)).then(|| {
            let mapped = mapped_directed_edge(&self.vertices, edge)?;
            (mapped[0] != mapped[1]).then_some(mapped)
        })?
    }
}

fn valid_face_vertices(face: [i64; 3], vertex_count: usize) -> Option<[usize; 3]> {
    let vertices = [
        usize::try_from(face[0]).ok()?,
        usize::try_from(face[1]).ok()?,
        usize::try_from(face[2]).ok()?,
    ];
    vertices
        .iter()
        .all(|vertex| *vertex < vertex_count)
        .then_some(vertices)
}

struct ResultCutVertexLookup<'a> {
    source_vertices: &'a [[f64; 3]],
    source_map: &'a [Option<usize>],
    candidate_vertices: Vec<([f64; 3], usize)>,
    tolerance_sq: f64,
}

impl<'a> ResultCutVertexLookup<'a> {
    fn new(
        source_operand: ExactBooleanOperand,
        first: &'a ExactCutMeshResult,
        second: &'a ExactCutMeshResult,
        first_vertex_map: &'a [Option<usize>],
        second_vertex_map: &'a [Option<usize>],
        epsilon: f64,
    ) -> Self {
        let (source_vertices, source_map) = match source_operand {
            ExactBooleanOperand::First => (&first.vertices, first_vertex_map),
            ExactBooleanOperand::Second => (&second.vertices, second_vertex_map),
        };
        let mut candidate_vertices = Vec::new();
        push_mapped_vertices(&mut candidate_vertices, &first.vertices, first_vertex_map);
        push_mapped_vertices(&mut candidate_vertices, &second.vertices, second_vertex_map);
        Self {
            source_vertices,
            source_map,
            candidate_vertices,
            tolerance_sq: effective_epsilon(epsilon).powi(2),
        }
    }

    fn mapped_source_edge(&self, edge: [usize; 2]) -> Option<[usize; 2]> {
        let mapped = [
            self.mapped_source_vertex(edge[0])?,
            self.mapped_source_vertex(edge[1])?,
        ];
        (mapped[0] != mapped[1]).then_some(mapped)
    }

    fn mapped_source_vertex(&self, vertex: usize) -> Option<usize> {
        self.source_map
            .get(vertex)
            .copied()
            .flatten()
            .or_else(|| self.mapped_vertex_by_coordinate(*self.source_vertices.get(vertex)?))
    }

    fn mapped_vertex_by_coordinate(&self, point: [f64; 3]) -> Option<usize> {
        self.candidate_vertices
            .iter()
            .find_map(|(candidate, output_vertex)| {
                points_close(point, *candidate, self.tolerance_sq).then_some(*output_vertex)
            })
    }
}

fn push_mapped_vertices(
    output: &mut Vec<([f64; 3], usize)>,
    vertices: &[[f64; 3]],
    vertex_map: &[Option<usize>],
) {
    output.extend(
        vertex_map
            .iter()
            .enumerate()
            .filter_map(|(vertex, output_vertex)| {
                Some((*vertices.get(vertex)?, output_vertex.as_ref().copied()?))
            }),
    );
}

fn result_cut_stitch_fallbacks(
    source_operand: ExactBooleanOperand,
    stitch_plan: Option<&ExactStitchPlan>,
    first_vertex_map: &[Option<usize>],
    second_vertex_map: &[Option<usize>],
) -> BTreeMap<[usize; 2], [usize; 2]> {
    let mut fallbacks = BTreeMap::new();
    let Some(stitch_plan) = stitch_plan else {
        return fallbacks;
    };
    for pair in &stitch_plan.pairs {
        match source_operand {
            ExactBooleanOperand::First => {
                if let Some(mapped_second) =
                    mapped_directed_edge(second_vertex_map, pair.second_edge)
                {
                    insert_stitch_fallback(
                        &mut fallbacks,
                        pair.first_edge,
                        reverse_edge(mapped_second),
                    );
                }
            }
            ExactBooleanOperand::Second => {
                if let Some(mapped_first) = mapped_directed_edge(first_vertex_map, pair.first_edge)
                {
                    insert_stitch_fallback(
                        &mut fallbacks,
                        pair.second_edge,
                        reverse_edge(mapped_first),
                    );
                }
            }
        }
    }
    fallbacks
}

fn insert_stitch_fallback(
    fallbacks: &mut BTreeMap<[usize; 2], [usize; 2]>,
    source_edge: [usize; 2],
    mapped_edge: [usize; 2],
) {
    fallbacks.entry(source_edge).or_insert(mapped_edge);
    fallbacks
        .entry(reverse_edge(source_edge))
        .or_insert(reverse_edge(mapped_edge));
}

fn requires_stitched_result_cut(operation: ExactBooleanOperation) -> bool {
    matches!(
        operation,
        ExactBooleanOperation::Union
            | ExactBooleanOperation::Intersection
            | ExactBooleanOperation::DifferenceAB
            | ExactBooleanOperation::DifferenceBA
    )
}

pub(super) fn stitched_edge_sources(
    stitch_plan: &ExactStitchPlan,
    first_vertex_map: &[Option<usize>],
    second_vertex_map: &[Option<usize>],
) -> MappedStitchedEdges {
    let mut pair_to_source = BTreeMap::new();
    let sources = stitch_plan
        .pairs
        .iter()
        .enumerate()
        .filter_map(|(pair_index, pair)| {
            let first_output_edge = mapped_ordered_edge(first_vertex_map, pair.first_edge);
            let second_output_edge = mapped_ordered_edge(second_vertex_map, pair.second_edge);
            let first_directed_output_edge =
                mapped_directed_edge(first_vertex_map, pair.first_edge);
            let second_directed_output_edge =
                mapped_directed_edge(second_vertex_map, pair.second_edge);
            let output_edge = match (first_output_edge, second_output_edge) {
                (Some(first), Some(second)) if first == second => first,
                (Some(first), None) => first,
                (None, Some(second)) => second,
                _ => return None,
            };
            let first_stitch_edge = first_directed_output_edge
                .or_else(|| second_directed_output_edge.map(reverse_edge));
            let second_stitch_edge = first_stitch_edge.map(reverse_edge);
            let source_index = pair_to_source.len();
            pair_to_source.insert(pair_index, source_index);
            Some(ExactBooleanStitchedEdgeSource {
                output_edge,
                first_output_edge: first_directed_output_edge,
                second_output_edge: second_directed_output_edge,
                first_stitch_edge,
                second_stitch_edge,
                first_stitch_edge_synthetic: first_directed_output_edge.is_none()
                    && first_stitch_edge.is_some(),
                second_stitch_edge_synthetic: second_directed_output_edge.is_none()
                    && second_stitch_edge.is_some(),
                first_edge_index: pair.first_edge_index,
                second_edge_index: pair.second_edge_index,
                first_cut_edge: pair.first_edge,
                second_cut_edge: pair.second_edge,
            })
        })
        .collect::<Vec<_>>();
    let paths = mapped_stitched_paths(&stitch_plan.paths, &pair_to_source, &sources);
    MappedStitchedEdges { sources, paths }
}

fn mapped_stitched_paths(
    paths: &[ExactStitchPath],
    pair_to_source: &BTreeMap<usize, usize>,
    sources: &[ExactBooleanStitchedEdgeSource],
) -> Vec<ExactStitchPath> {
    let mut mapped_paths = Vec::new();
    for path in paths {
        let mut segment = Vec::new();
        let mut segment_start = 0;
        for (offset, pair_index) in path.pair_indices.iter().copied().enumerate() {
            let Some(source_index) = pair_to_source.get(&pair_index).copied() else {
                push_mapped_stitch_segment(
                    &mut mapped_paths,
                    &mut segment,
                    path,
                    segment_start,
                    offset,
                    sources,
                );
                segment_start = offset + 1;
                continue;
            };
            if let Some(previous_index) = segment.last().copied() {
                if !stitch_sources_are_contiguous(&sources[previous_index], &sources[source_index])
                {
                    push_mapped_stitch_segment(
                        &mut mapped_paths,
                        &mut segment,
                        path,
                        segment_start,
                        offset,
                        sources,
                    );
                    segment_start = offset;
                }
            }
            segment.push(source_index);
        }
        push_mapped_stitch_segment(
            &mut mapped_paths,
            &mut segment,
            path,
            segment_start,
            path.pair_indices.len(),
            sources,
        );
    }
    mapped_paths
}

fn push_mapped_stitch_segment(
    mapped_paths: &mut Vec<ExactStitchPath>,
    segment: &mut Vec<usize>,
    source_path: &ExactStitchPath,
    segment_start: usize,
    segment_end: usize,
    sources: &[ExactBooleanStitchedEdgeSource],
) {
    if segment.is_empty() {
        return;
    }
    let full_path = segment_start == 0 && segment_end == source_path.pair_indices.len();
    let closed = source_path.closed && full_path && stitch_segment_is_closed(segment, sources);
    mapped_paths.push(ExactStitchPath {
        pair_indices: std::mem::take(segment),
        closed,
    });
}

fn stitch_sources_are_contiguous(
    left: &ExactBooleanStitchedEdgeSource,
    right: &ExactBooleanStitchedEdgeSource,
) -> bool {
    match (
        left.first_stitch_edge,
        right.first_stitch_edge,
        left.second_stitch_edge,
        right.second_stitch_edge,
    ) {
        (Some(first_left), Some(first_right), Some(second_left), Some(second_right)) => {
            first_left[1] == first_right[0] && second_left[0] == second_right[1]
        }
        _ => false,
    }
}

fn stitch_segment_is_closed(segment: &[usize], sources: &[ExactBooleanStitchedEdgeSource]) -> bool {
    let Some(first) = segment.first().and_then(|index| sources.get(*index)) else {
        return false;
    };
    let Some(last) = segment.last().and_then(|index| sources.get(*index)) else {
        return false;
    };
    match (
        first.first_stitch_edge,
        last.first_stitch_edge,
        first.second_stitch_edge,
        last.second_stitch_edge,
    ) {
        (Some(first_start), Some(first_end), Some(second_start), Some(second_end)) => {
            first_end[1] == first_start[0] && second_end[0] == second_start[1]
        }
        _ => false,
    }
}

fn mapped_ordered_edge(vertex_map: &[Option<usize>], edge: [usize; 2]) -> Option<[usize; 2]> {
    mapped_directed_edge(vertex_map, edge).map(ordered_edge)
}

fn mapped_directed_edge(vertex_map: &[Option<usize>], edge: [usize; 2]) -> Option<[usize; 2]> {
    Some([
        vertex_map.get(edge[0]).copied().flatten()?,
        vertex_map.get(edge[1]).copied().flatten()?,
    ])
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

fn points_close(left: [f64; 3], right: [f64; 3], tolerance_sq: f64) -> bool {
    let delta = [left[0] - right[0], left[1] - right[1], left[2] - right[2]];
    delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2] <= tolerance_sq
}

fn effective_epsilon(epsilon: f64) -> f64 {
    let base = if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    };
    base * 4.0
}
