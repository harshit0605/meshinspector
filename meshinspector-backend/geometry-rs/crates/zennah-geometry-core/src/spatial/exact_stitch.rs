use super::exact_cut_apply::ExactCutMeshResult;
use crate::math::{dot, sub};
use std::collections::{BTreeMap, BTreeSet};

const STITCH_EPSILON_FLOOR: f64 = 1e-8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactStitchEdgePair {
    pub first_edge_index: usize,
    pub second_edge_index: usize,
    pub first_edge: [usize; 2],
    pub second_edge: [usize; 2],
    pub second_reversed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactStitchPath {
    pub pair_indices: Vec<usize>,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactStitchPlan {
    pub pairs: Vec<ExactStitchEdgePair>,
    pub paths: Vec<ExactStitchPath>,
    pub unmatched_first_edges: Vec<usize>,
    pub unmatched_second_edges: Vec<usize>,
    pub compatible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactStitchVertexMap {
    pub second_to_first: Vec<Option<usize>>,
    pub conflicts: Vec<usize>,
}

pub fn exact_stitch_plan_from_cut_meshes(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    epsilon: f64,
) -> ExactStitchPlan {
    if let Some(path_plan) = exact_stitch_plan_by_cut_paths(first, second, epsilon) {
        if path_plan.compatible {
            return path_plan;
        }
        let edge_plan = exact_stitch_plan_by_edges(
            &first.vertices,
            &first.cut_edges,
            &second.vertices,
            &second.cut_edges,
            epsilon,
        );
        if path_plan.pairs.len() >= edge_plan.pairs.len() {
            return path_plan;
        }
        return edge_plan;
    }

    exact_stitch_plan_by_edges(
        &first.vertices,
        &first.cut_edges,
        &second.vertices,
        &second.cut_edges,
        epsilon,
    )
}

fn exact_stitch_plan_by_cut_paths(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    epsilon: f64,
) -> Option<ExactStitchPlan> {
    if first.cut_edge_paths.is_empty()
        || second.cut_edge_paths.is_empty()
        || first.cut_edge_paths.len() != second.cut_edge_paths.len()
    {
        return None;
    }

    let tolerance_sq = effective_epsilon(epsilon).powi(2);
    let first_edge_indices = cut_edge_indices(&first.cut_edges)?;
    let second_edge_indices = cut_edge_indices(&second.cut_edges)?;
    let first_path_edge_indices = path_edge_indices(&first.cut_edge_paths, &first_edge_indices)?;
    let second_path_edge_indices = path_edge_indices(&second.cut_edge_paths, &second_edge_indices)?;
    let mut used_first = vec![false; first.cut_edges.len()];
    let mut used_second = vec![false; second.cut_edges.len()];
    let mut pairs = Vec::new();
    let mut paths = Vec::new();
    let mut path_length_mismatch = false;

    for (path_index, first_path) in first.cut_edge_paths.iter().enumerate() {
        let second_path = second.cut_edge_paths.get(path_index)?;
        let path_lengths_match = first_path.len() == second_path.len();
        path_length_mismatch |= !path_lengths_match;

        let mut path_segments = Vec::<Vec<usize>>::new();
        let mut segment = Vec::new();
        for first_edge in first_path.iter().copied() {
            let Some(first_edge_index) = first_edge_indices.get(&ordered_edge(first_edge)).copied()
            else {
                push_stitch_path_segment(&mut path_segments, &mut segment);
                continue;
            };
            if used_first[first_edge_index] {
                push_stitch_path_segment(&mut path_segments, &mut segment);
                continue;
            }
            let Some((second_edge_index, second_edge, second_reversed)) = matching_path_edge(
                first_edge,
                second_path,
                second,
                &second_edge_indices,
                &used_second,
                &first.vertices,
                tolerance_sq,
            ) else {
                push_stitch_path_segment(&mut path_segments, &mut segment);
                continue;
            };
            used_first[first_edge_index] = true;
            used_second[second_edge_index] = true;
            let pair_index = pairs.len();
            pairs.push(ExactStitchEdgePair {
                first_edge_index,
                second_edge_index,
                first_edge,
                second_edge,
                second_reversed,
            });
            segment.push(pair_index);
        }
        push_stitch_path_segment(&mut path_segments, &mut segment);

        let full_path_matched = path_lengths_match
            && path_segments.len() == 1
            && path_segments[0].len() == first_path.len();
        for pair_indices in path_segments {
            paths.push(ExactStitchPath {
                pair_indices,
                closed: full_path_matched
                    && first
                        .cut_edge_path_closed
                        .get(path_index)
                        .copied()
                        .unwrap_or_else(|| directed_path_is_closed(first_path)),
            });
        }
    }

    let unmatched_first_edges = first_path_edge_indices
        .iter()
        .copied()
        .filter(|index| !used_first[*index])
        .collect::<Vec<_>>();
    let unmatched_second_edges = second_path_edge_indices
        .iter()
        .copied()
        .filter(|index| !used_second[*index])
        .collect::<Vec<_>>();
    let compatible = unmatched_first_edges.is_empty()
        && unmatched_second_edges.is_empty()
        && !path_length_mismatch;
    Some(ExactStitchPlan {
        pairs,
        paths,
        unmatched_first_edges,
        unmatched_second_edges,
        compatible,
    })
}

fn push_stitch_path_segment(output: &mut Vec<Vec<usize>>, segment: &mut Vec<usize>) {
    if !segment.is_empty() {
        output.push(std::mem::take(segment));
    }
}

pub fn exact_stitch_plan_by_edges(
    first_vertices: &[[f64; 3]],
    first_edges: &[[usize; 2]],
    second_vertices: &[[f64; 3]],
    second_edges: &[[usize; 2]],
    epsilon: f64,
) -> ExactStitchPlan {
    let tolerance_sq = effective_epsilon(epsilon).powi(2);
    let mut used_second = vec![false; second_edges.len()];
    let mut pairs = Vec::new();
    let mut unmatched_first_edges = Vec::new();

    for (first_edge_index, first_edge) in first_edges.iter().copied().enumerate() {
        let Some(first_points) = edge_points(first_vertices, first_edge) else {
            unmatched_first_edges.push(first_edge_index);
            continue;
        };
        let mut matched_second = None;
        for (second_edge_index, second_edge) in second_edges.iter().copied().enumerate() {
            if used_second[second_edge_index] {
                continue;
            }
            let Some(second_points) = edge_points(second_vertices, second_edge) else {
                continue;
            };
            if points_close(first_points[0], second_points[0], tolerance_sq)
                && points_close(first_points[1], second_points[1], tolerance_sq)
            {
                matched_second = Some((second_edge_index, second_edge, false));
                break;
            }
            if points_close(first_points[0], second_points[1], tolerance_sq)
                && points_close(first_points[1], second_points[0], tolerance_sq)
            {
                matched_second = Some((second_edge_index, second_edge, true));
                break;
            }
        }
        match matched_second {
            Some((second_edge_index, second_edge, second_reversed)) => {
                used_second[second_edge_index] = true;
                pairs.push(ExactStitchEdgePair {
                    first_edge_index,
                    second_edge_index,
                    first_edge,
                    second_edge,
                    second_reversed,
                });
            }
            None => unmatched_first_edges.push(first_edge_index),
        }
    }

    let unmatched_second_edges = used_second
        .into_iter()
        .enumerate()
        .filter_map(|(index, used)| (!used).then_some(index))
        .collect::<Vec<_>>();
    let paths = stitch_paths_from_pairs(&pairs);
    let compatible = unmatched_first_edges.is_empty()
        && unmatched_second_edges.is_empty()
        && first_edges.len() == second_edges.len();

    ExactStitchPlan {
        pairs,
        paths,
        unmatched_first_edges,
        unmatched_second_edges,
        compatible,
    }
}

fn cut_edge_indices(edges: &[[usize; 2]]) -> Option<BTreeMap<[usize; 2], usize>> {
    let mut indices = BTreeMap::new();
    for (index, edge) in edges.iter().copied().enumerate() {
        if indices.insert(ordered_edge(edge), index).is_some() {
            return None;
        }
    }
    Some(indices)
}

fn path_edge_indices(
    paths: &[Vec<[usize; 2]>],
    edge_indices: &BTreeMap<[usize; 2], usize>,
) -> Option<BTreeSet<usize>> {
    let mut output = BTreeSet::new();
    for edge in paths.iter().flatten().copied() {
        output.insert(*edge_indices.get(&ordered_edge(edge))?);
    }
    Some(output)
}

fn matching_path_edge(
    first_edge: [usize; 2],
    second_path: &[[usize; 2]],
    second: &ExactCutMeshResult,
    second_edge_indices: &BTreeMap<[usize; 2], usize>,
    used_second: &[bool],
    first_vertices: &[[f64; 3]],
    tolerance_sq: f64,
) -> Option<(usize, [usize; 2], bool)> {
    let first_points = edge_points(first_vertices, first_edge)?;
    for second_edge in second_path.iter().copied() {
        let second_edge_index = *second_edge_indices.get(&ordered_edge(second_edge))?;
        if used_second[second_edge_index] {
            continue;
        }
        let second_points = edge_points(&second.vertices, second_edge)?;
        if points_close(first_points[0], second_points[0], tolerance_sq)
            && points_close(first_points[1], second_points[1], tolerance_sq)
        {
            return Some((second_edge_index, second_edge, false));
        }
        if points_close(first_points[0], second_points[1], tolerance_sq)
            && points_close(first_points[1], second_points[0], tolerance_sq)
        {
            return Some((second_edge_index, second_edge, true));
        }
    }
    None
}

fn directed_path_is_closed(path: &[[usize; 2]]) -> bool {
    path.len() > 1
        && path.windows(2).all(|window| window[0][1] == window[1][0])
        && path.first().map(|edge| edge[0]) == path.last().map(|edge| edge[1])
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn stitch_paths_from_pairs(pairs: &[ExactStitchEdgePair]) -> Vec<ExactStitchPath> {
    let mut vertex_pairs = BTreeMap::<usize, Vec<usize>>::new();
    for (index, pair) in pairs.iter().enumerate() {
        vertex_pairs
            .entry(pair.first_edge[0])
            .or_default()
            .push(index);
        vertex_pairs
            .entry(pair.first_edge[1])
            .or_default()
            .push(index);
    }

    let mut visited = BTreeSet::new();
    let mut paths = Vec::new();
    for pair_index in 0..pairs.len() {
        if visited.contains(&pair_index) {
            continue;
        }
        let component = component_pair_indices(pair_index, pairs, &vertex_pairs);
        let start_vertex = path_start_vertex(&component, pairs, &vertex_pairs);
        let (ordered, closed) = walk_pair_path(start_vertex, pairs, &vertex_pairs, &mut visited);
        if !ordered.is_empty() {
            paths.push(ExactStitchPath {
                pair_indices: ordered,
                closed,
            });
        }
    }
    paths
}

fn component_pair_indices(
    start_pair: usize,
    pairs: &[ExactStitchEdgePair],
    vertex_pairs: &BTreeMap<usize, Vec<usize>>,
) -> Vec<usize> {
    let mut stack = vec![start_pair];
    let mut component = BTreeSet::new();
    while let Some(pair_index) = stack.pop() {
        if !component.insert(pair_index) {
            continue;
        }
        for vertex in pairs[pair_index].first_edge {
            if let Some(next_pairs) = vertex_pairs.get(&vertex) {
                stack.extend(next_pairs.iter().copied());
            }
        }
    }
    component.into_iter().collect()
}

fn path_start_vertex(
    component: &[usize],
    pairs: &[ExactStitchEdgePair],
    vertex_pairs: &BTreeMap<usize, Vec<usize>>,
) -> usize {
    let component_pairs = component.iter().copied().collect::<BTreeSet<_>>();
    let vertices = component
        .iter()
        .flat_map(|index| pairs[*index].first_edge)
        .collect::<BTreeSet<_>>();
    vertices
        .into_iter()
        .find(|vertex| degree_in_component(*vertex, vertex_pairs, &component_pairs) == 1)
        .unwrap_or(pairs[component[0]].first_edge[0])
}

fn walk_pair_path(
    start_vertex: usize,
    pairs: &[ExactStitchEdgePair],
    vertex_pairs: &BTreeMap<usize, Vec<usize>>,
    visited: &mut BTreeSet<usize>,
) -> (Vec<usize>, bool) {
    let mut ordered = Vec::new();
    let mut current_vertex = start_vertex;
    let mut previous_pair = None;
    while let Some(pair_index) =
        next_unvisited_pair(current_vertex, previous_pair, vertex_pairs, visited)
    {
        visited.insert(pair_index);
        ordered.push(pair_index);
        previous_pair = Some(pair_index);
        current_vertex = other_endpoint(pairs[pair_index].first_edge, current_vertex);
    }
    let closed = !ordered.is_empty() && current_vertex == start_vertex;
    (ordered, closed)
}

fn next_unvisited_pair(
    vertex: usize,
    previous_pair: Option<usize>,
    vertex_pairs: &BTreeMap<usize, Vec<usize>>,
    visited: &BTreeSet<usize>,
) -> Option<usize> {
    vertex_pairs
        .get(&vertex)?
        .iter()
        .copied()
        .find(|pair_index| Some(*pair_index) != previous_pair && !visited.contains(pair_index))
}

fn degree_in_component(
    vertex: usize,
    vertex_pairs: &BTreeMap<usize, Vec<usize>>,
    component_pairs: &BTreeSet<usize>,
) -> usize {
    vertex_pairs
        .get(&vertex)
        .map(|pairs| {
            pairs
                .iter()
                .filter(|pair_index| component_pairs.contains(pair_index))
                .count()
        })
        .unwrap_or_default()
}

fn other_endpoint(edge: [usize; 2], vertex: usize) -> usize {
    if edge[0] == vertex {
        edge[1]
    } else {
        edge[0]
    }
}

pub(super) fn exact_stitch_vertex_map(
    plan: &ExactStitchPlan,
    second_vertex_count: usize,
) -> ExactStitchVertexMap {
    let mut second_to_first = vec![None; second_vertex_count];
    let mut conflicts = Vec::new();
    for pair in &plan.pairs {
        let endpoint_pairs = if pair.second_reversed {
            [
                (pair.second_edge[0], pair.first_edge[1]),
                (pair.second_edge[1], pair.first_edge[0]),
            ]
        } else {
            [
                (pair.second_edge[0], pair.first_edge[0]),
                (pair.second_edge[1], pair.first_edge[1]),
            ]
        };
        for (second_vertex, first_vertex) in endpoint_pairs {
            let Some(slot) = second_to_first.get_mut(second_vertex) else {
                conflicts.push(second_vertex);
                continue;
            };
            match slot {
                Some(existing) if *existing != first_vertex => conflicts.push(second_vertex),
                Some(_) => {}
                None => *slot = Some(first_vertex),
            }
        }
    }
    ExactStitchVertexMap {
        second_to_first,
        conflicts,
    }
}

fn edge_points(vertices: &[[f64; 3]], edge: [usize; 2]) -> Option<[[f64; 3]; 2]> {
    Some([*vertices.get(edge[0])?, *vertices.get(edge[1])?])
}

fn points_close(left: [f64; 3], right: [f64; 3], tolerance_sq: f64) -> bool {
    let delta = sub(left, right);
    dot(delta, delta) <= tolerance_sq
}

fn effective_epsilon(epsilon: f64) -> f64 {
    let base = if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    };
    (base * 4.0).max(STITCH_EPSILON_FLOOR)
}

#[cfg(test)]
mod tests;
