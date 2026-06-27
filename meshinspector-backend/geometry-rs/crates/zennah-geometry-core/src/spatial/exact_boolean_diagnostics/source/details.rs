use super::super::super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use super::super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::super::exact_stitch::{ExactStitchEdgePair, ExactStitchPlan};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct StitchSourcePaths {
    pub(super) path_lengths: Vec<usize>,
    pub(super) first_path_source_faces: Vec<Vec<usize>>,
    pub(super) second_path_source_faces: Vec<Vec<usize>>,
    pub(super) first_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub(super) second_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub(super) first_path_meshlib_removed_face_owner_candidates: Vec<Vec<usize>>,
    pub(super) second_path_meshlib_removed_face_owner_candidates: Vec<Vec<usize>>,
    pub(super) first_path_meshlib_removed_face_owner_candidate_runs: Vec<Vec<[usize; 2]>>,
    pub(super) second_path_meshlib_removed_face_owner_candidate_runs: Vec<Vec<[usize; 2]>>,
    pub(super) first_source_faces: Vec<usize>,
    pub(super) second_source_faces: Vec<usize>,
    pub(super) first_source_face_runs: Vec<[usize; 2]>,
    pub(super) second_source_face_runs: Vec<[usize; 2]>,
    pub(super) first_meshlib_removed_face_owner_candidates: Vec<usize>,
    pub(super) second_meshlib_removed_face_owner_candidates: Vec<usize>,
    pub(super) first_meshlib_removed_face_owner_candidate_runs: Vec<[usize; 2]>,
    pub(super) second_meshlib_removed_face_owner_candidate_runs: Vec<[usize; 2]>,
    pub(super) meshlib_removed_face_owner_candidate_missing_records: [usize; 2],
    pub(super) missing_source_records: [usize; 2],
}

pub(super) fn stitch_source_paths_from_pair_groups(
    pair_groups: &[Vec<usize>],
    stitch_plan: &ExactStitchPlan,
    first_source_faces_by_edge: &[Option<usize>],
    second_source_faces_by_edge: &[Option<usize>],
    first_owner_candidates_by_edge: &[Option<usize>],
    second_owner_candidates_by_edge: &[Option<usize>],
) -> StitchSourcePaths {
    let mut path_lengths = Vec::with_capacity(pair_groups.len());
    let mut first_path_source_faces = Vec::with_capacity(pair_groups.len());
    let mut second_path_source_faces = Vec::with_capacity(pair_groups.len());
    let mut first_path_owner_candidates = Vec::with_capacity(pair_groups.len());
    let mut second_path_owner_candidates = Vec::with_capacity(pair_groups.len());
    let mut first_source_faces = Vec::new();
    let mut second_source_faces = Vec::new();
    let mut first_owner_candidates = Vec::new();
    let mut second_owner_candidates = Vec::new();
    let mut missing_source_records = [0_usize; 2];
    let mut missing_owner_candidate_records = [0_usize; 2];

    for pair_group in pair_groups {
        let mut first_path_sources = Vec::with_capacity(pair_group.len());
        let mut second_path_sources = Vec::with_capacity(pair_group.len());
        let mut first_path_owners = Vec::with_capacity(pair_group.len());
        let mut second_path_owners = Vec::with_capacity(pair_group.len());
        for pair_index in pair_group {
            let Some(pair) = stitch_plan.pairs.get(*pair_index) else {
                missing_source_records[0] += 1;
                missing_source_records[1] += 1;
                missing_owner_candidate_records[0] += 1;
                missing_owner_candidate_records[1] += 1;
                continue;
            };
            if let Some(source_face) = first_source_faces_by_edge
                .get(pair.first_edge_index)
                .copied()
                .flatten()
            {
                first_path_sources.push(source_face);
            } else {
                missing_source_records[0] += 1;
            }
            if let Some(source_face) = second_source_faces_by_edge
                .get(pair.second_edge_index)
                .copied()
                .flatten()
            {
                second_path_sources.push(source_face);
            } else {
                missing_source_records[1] += 1;
            }
            if let Some(owner_candidate) = first_owner_candidates_by_edge
                .get(pair.first_edge_index)
                .copied()
                .flatten()
            {
                first_path_owners.push(owner_candidate);
            } else {
                missing_owner_candidate_records[0] += 1;
            }
            if let Some(owner_candidate) = second_owner_candidates_by_edge
                .get(pair.second_edge_index)
                .copied()
                .flatten()
            {
                second_path_owners.push(owner_candidate);
            } else {
                missing_owner_candidate_records[1] += 1;
            }
        }
        path_lengths.push(pair_group.len());
        first_source_faces.extend(first_path_sources.iter().copied());
        second_source_faces.extend(second_path_sources.iter().copied());
        first_owner_candidates.extend(first_path_owners.iter().copied());
        second_owner_candidates.extend(second_path_owners.iter().copied());
        first_path_source_faces.push(first_path_sources);
        second_path_source_faces.push(second_path_sources);
        first_path_owner_candidates.push(first_path_owners);
        second_path_owner_candidates.push(second_path_owners);
    }

    let first_path_source_face_runs = first_path_source_faces
        .iter()
        .map(|source_faces| source_face_runs(source_faces))
        .collect();
    let second_path_source_face_runs = second_path_source_faces
        .iter()
        .map(|source_faces| source_face_runs(source_faces))
        .collect();
    let first_source_face_runs = source_face_runs(&first_source_faces);
    let second_source_face_runs = source_face_runs(&second_source_faces);
    let first_path_owner_candidate_runs = first_path_owner_candidates
        .iter()
        .map(|source_faces| source_face_runs(source_faces))
        .collect();
    let second_path_owner_candidate_runs = second_path_owner_candidates
        .iter()
        .map(|source_faces| source_face_runs(source_faces))
        .collect();
    let first_owner_candidate_runs = source_face_runs(&first_owner_candidates);
    let second_owner_candidate_runs = source_face_runs(&second_owner_candidates);

    StitchSourcePaths {
        path_lengths,
        first_path_source_faces,
        second_path_source_faces,
        first_path_source_face_runs,
        second_path_source_face_runs,
        first_path_meshlib_removed_face_owner_candidates: first_path_owner_candidates,
        second_path_meshlib_removed_face_owner_candidates: second_path_owner_candidates,
        first_path_meshlib_removed_face_owner_candidate_runs: first_path_owner_candidate_runs,
        second_path_meshlib_removed_face_owner_candidate_runs: second_path_owner_candidate_runs,
        first_source_faces,
        second_source_faces,
        first_source_face_runs,
        second_source_face_runs,
        first_meshlib_removed_face_owner_candidates: first_owner_candidates,
        second_meshlib_removed_face_owner_candidates: second_owner_candidates,
        first_meshlib_removed_face_owner_candidate_runs: first_owner_candidate_runs,
        second_meshlib_removed_face_owner_candidate_runs: second_owner_candidate_runs,
        meshlib_removed_face_owner_candidate_missing_records: missing_owner_candidate_records,
        missing_source_records,
    }
}

pub(super) fn edge_grouped_stitch_pair_paths(
    pairs: &[ExactStitchEdgePair],
) -> Vec<(Vec<usize>, bool)> {
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
        let component = stitch_pair_component(pair_index, pairs, &vertex_pairs);
        let start_vertex = stitch_pair_component_start_vertex(&component, pairs, &vertex_pairs);
        let (ordered, closed) =
            walk_stitch_pair_path(start_vertex, pairs, &vertex_pairs, &mut visited);
        if !ordered.is_empty() {
            paths.push((ordered, closed));
        }
    }
    paths
}

pub(super) fn stitch_pair_component(
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

pub(super) fn stitch_pair_component_start_vertex(
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
        .find(|vertex| {
            stitch_pair_degree_in_component(*vertex, vertex_pairs, &component_pairs) == 1
        })
        .unwrap_or(pairs[component[0]].first_edge[0])
}

pub(super) fn walk_stitch_pair_path(
    start_vertex: usize,
    pairs: &[ExactStitchEdgePair],
    vertex_pairs: &BTreeMap<usize, Vec<usize>>,
    visited: &mut BTreeSet<usize>,
) -> (Vec<usize>, bool) {
    let mut ordered = Vec::new();
    let mut current_vertex = start_vertex;
    let mut previous_pair = None;
    while let Some(pair_index) =
        next_unvisited_stitch_pair(current_vertex, previous_pair, vertex_pairs, visited)
    {
        visited.insert(pair_index);
        ordered.push(pair_index);
        previous_pair = Some(pair_index);
        current_vertex = other_stitch_pair_endpoint(pairs[pair_index].first_edge, current_vertex);
    }
    let closed = !ordered.is_empty() && current_vertex == start_vertex;
    (ordered, closed)
}

pub(super) fn next_unvisited_stitch_pair(
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

pub(super) fn stitch_pair_degree_in_component(
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

pub(super) fn other_stitch_pair_endpoint(edge: [usize; 2], vertex: usize) -> usize {
    if edge[0] == vertex {
        edge[1]
    } else {
        edge[0]
    }
}

pub(super) fn cut_edge_source_faces_by_index(cut: &ExactCutMeshResult) -> Vec<Option<usize>> {
    let edge_indices = cut
        .cut_edges
        .iter()
        .copied()
        .enumerate()
        .map(|(index, edge)| (ordered_edge(edge), index))
        .collect::<BTreeMap<_, _>>();
    let mut source_faces_by_edge = vec![None; cut.cut_edges.len()];
    for (path, source_faces) in cut
        .cut_edge_paths
        .iter()
        .zip(&cut.cut_edge_path_source_faces)
    {
        for (edge, source_face) in path.iter().zip(source_faces) {
            let Some(source_face) = source_face else {
                continue;
            };
            if let Some(index) = edge_indices.get(&ordered_edge(*edge)) {
                source_faces_by_edge[*index].get_or_insert(*source_face);
            }
        }
    }
    source_faces_by_edge
}

pub(super) fn cut_edge_meshlib_removed_face_owner_candidates_by_index(
    cut: &ExactCutMeshResult,
    source_faces_by_edge: &[Option<usize>],
) -> Vec<Option<usize>> {
    cut.cut_edges
        .iter()
        .copied()
        .enumerate()
        .map(|(edge_index, edge)| {
            meshlib_removed_face_owner_candidate(
                source_faces_by_edge.get(edge_index).copied().flatten(),
                cut_edge_side_source_faces(cut, edge, DirectedEdgeSide::Left)
                    .first()
                    .copied(),
                cut_edge_side_source_faces(cut, edge, DirectedEdgeSide::Right)
                    .first()
                    .copied(),
            )
        })
        .collect()
}

pub(super) fn cut_edge_adjacent_source_faces(
    cut: &ExactCutMeshResult,
    edge: [usize; 2],
) -> Vec<usize> {
    let edge = ordered_edge(edge);
    let mut source_faces = cut
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            face_has_edge(*face, edge)
                .then(|| cut.source_face_for_faces.get(face_index).copied())
                .flatten()
        })
        .collect::<Vec<_>>();
    source_faces.sort_unstable();
    source_faces.dedup();
    source_faces
}

pub(super) fn source_face_runs(source_faces: &[usize]) -> Vec<[usize; 2]> {
    let mut runs = Vec::<[usize; 2]>::new();
    for source_face in source_faces {
        match runs.last_mut() {
            Some([run_source_face, count]) if run_source_face == source_face => *count += 1,
            _ => runs.push([*source_face, 1]),
        }
    }
    runs
}

pub(super) fn source_face_runs_by_path(source_face_paths: &[Vec<usize>]) -> Vec<Vec<[usize; 2]>> {
    source_face_paths
        .iter()
        .map(|source_faces| source_face_runs(source_faces))
        .collect()
}

pub(super) fn primary_edge_source_faces(edge_source_faces: &[Vec<Vec<usize>>]) -> Vec<Vec<usize>> {
    edge_source_faces
        .iter()
        .map(|path| {
            path.iter()
                .filter_map(|source_faces| source_faces.first().copied())
                .collect()
        })
        .collect()
}

pub(super) fn closed_path_meshlib_removed_face_owner_candidates(
    cut: &ExactCutMeshResult,
    closed_left_primary_sources: &[Vec<usize>],
    closed_right_primary_sources: &[Vec<usize>],
) -> Vec<Vec<usize>> {
    let mut candidates = Vec::new();
    let mut closed_index = 0;
    for (path_index, path) in cut.cut_edge_paths.iter().enumerate() {
        if !cut
            .cut_edge_path_closed
            .get(path_index)
            .copied()
            .unwrap_or_default()
        {
            continue;
        }
        let source_faces = cut
            .cut_edge_path_source_faces
            .get(path_index)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let left_sources = closed_left_primary_sources
            .get(closed_index)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let right_sources = closed_right_primary_sources
            .get(closed_index)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let path_candidates = (0..path.len())
            .filter_map(|edge_index| {
                meshlib_removed_face_owner_candidate(
                    source_faces.get(edge_index).copied().flatten(),
                    left_sources.get(edge_index).copied(),
                    right_sources.get(edge_index).copied(),
                )
            })
            .collect::<Vec<_>>();
        candidates.push(path_candidates);
        closed_index += 1;
    }
    candidates
}

pub(super) fn meshlib_removed_face_owner_candidate(
    path_source: Option<usize>,
    left_source: Option<usize>,
    right_source: Option<usize>,
) -> Option<usize> {
    match (path_source, left_source, right_source) {
        (Some(path), Some(left), Some(right)) => {
            let left_is_missing_side = left != path;
            let right_is_missing_side = right != path;
            match (left_is_missing_side, right_is_missing_side) {
                (true, false) => Some(left),
                (false, true) => Some(right),
                (true, true) => Some(left),
                (false, false) => Some(path),
            }
        }
        (Some(path), Some(left), None) if left != path => Some(left),
        (Some(path), None, Some(right)) if right != path => Some(right),
        (Some(path), _, _) => Some(path),
        (None, Some(left), _) => Some(left),
        (None, None, Some(right)) => Some(right),
        (None, None, None) => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DirectedEdgeSide {
    Left,
    Right,
}

pub(super) fn cut_edge_side_source_faces(
    cut: &ExactCutMeshResult,
    edge: [usize; 2],
    side: DirectedEdgeSide,
) -> Vec<usize> {
    let directed_edge = match side {
        DirectedEdgeSide::Left => edge,
        DirectedEdgeSide::Right => [edge[1], edge[0]],
    };
    let mut source_faces = cut
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            face_has_directed_edge(*face, directed_edge)
                .then(|| cut.source_face_for_faces.get(face_index).copied())
                .flatten()
        })
        .collect::<Vec<_>>();
    source_faces.sort_unstable();
    source_faces.dedup();
    source_faces
}

pub(super) fn boundary_misses(
    input: &super::super::ExactBooleanPipelineDiagnosticInputs<'_>,
    first_raw_faces: &[usize],
    second_raw_faces: &[usize],
) -> [[usize; 2]; 2] {
    [
        [
            contour_boundary_misses(
                &input.first_cut.mesh.faces,
                first_raw_faces,
                input
                    .first_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
            contour_boundary_misses(
                &input.second_cut.mesh.faces,
                second_raw_faces,
                input
                    .second_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
        ],
        [
            contour_boundary_misses(
                &input.first_cut.mesh.faces,
                &input.assembly.selected_first_faces,
                input
                    .first_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
            contour_boundary_misses(
                &input.second_cut.mesh.faces,
                &input.assembly.selected_second_faces,
                input
                    .second_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
        ],
    ]
}

pub(in crate::spatial::exact_boolean_diagnostics) fn duplicate_face_counts(
    faces: &[[i64; 3]],
) -> (usize, usize) {
    let mut face_keys = BTreeMap::<[i64; 3], usize>::new();
    for face in faces {
        let mut key = *face;
        key.sort_unstable();
        *face_keys.entry(key).or_default() += 1;
    }
    let duplicate_groups = face_keys.values().filter(|&&count| count > 1).count();
    let duplicate_faces = face_keys
        .values()
        .filter(|&&count| count > 1)
        .map(|count| count - 1)
        .sum();
    (duplicate_groups, duplicate_faces)
}

pub(super) fn contour_boundary_misses(
    faces: &[[i64; 3]],
    selected_faces: &[usize],
    contour_edges: impl Iterator<Item = [usize; 2]>,
) -> usize {
    let selected_faces = selected_faces.iter().copied().collect::<BTreeSet<_>>();
    contour_edges
        .filter(|edge| selected_edge_incidence(faces, &selected_faces, *edge) != 1)
        .count()
}

pub(super) fn selected_edge_incidence(
    faces: &[[i64; 3]],
    selected_faces: &BTreeSet<usize>,
    edge: [usize; 2],
) -> usize {
    let edge = ordered_edge(edge);
    selected_faces
        .iter()
        .filter(|&&face_index| {
            faces
                .get(face_index)
                .map(|face| face_has_edge(*face, edge))
                .unwrap_or(false)
        })
        .count()
}

pub(super) fn face_has_edge(face: [i64; 3], edge: [usize; 2]) -> bool {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
        .into_iter()
        .map(|edge| [edge[0] as usize, edge[1] as usize])
        .any(|candidate| ordered_edge(candidate) == edge)
}

pub(super) fn face_has_directed_edge(face: [i64; 3], edge: [usize; 2]) -> bool {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
        .into_iter()
        .map(|edge| [edge[0] as usize, edge[1] as usize])
        .any(|candidate| candidate == edge)
}

pub(super) fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

pub(super) fn duplicate_source_faces(
    assembly: &ExactBooleanAssemblyResult,
    operand: ExactBooleanOperand,
) -> (usize, usize) {
    let mut source_faces = BTreeMap::<usize, usize>::new();
    for source in &assembly.face_sources {
        if source.operand == operand {
            *source_faces.entry(source.source_face).or_default() += 1;
        }
    }
    let duplicate_groups = source_faces.values().filter(|&&count| count > 1).count();
    let duplicate_faces = source_faces
        .values()
        .filter(|&&count| count > 1)
        .map(|count| count - 1)
        .sum();
    (duplicate_groups, duplicate_faces)
}
