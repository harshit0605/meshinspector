use super::super::exact_cut::ExactCutPreplan;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_one_mesh::ExactOneMeshPrimitive;
use super::types::{ExactCutShadowRepairPath, SourcePreservingCutSegment};

pub(super) fn source_preserving_cut_segment_paths(
    preplan: &ExactCutPreplan,
) -> Vec<Vec<SourcePreservingCutSegment>> {
    let mut segments_by_points =
        std::collections::BTreeMap::<(usize, usize, usize), SourcePreservingCutSegment>::new();
    for segment in &preplan.path_segments {
        let (start_coordinate, start_primitive_kind, start_primitive_face, start_primitive_edge) =
            source_preserving_start_primitive(preplan, segment.from_point);
        segments_by_points.insert(
            (segment.contour_index, segment.from_point, segment.to_point),
            SourcePreservingCutSegment {
                edge: preplan_segment_edge(preplan, segment.from_point, segment.to_point),
                source_face: segment.source_faces.first().copied(),
                collapsed: false,
                start_coordinate,
                start_primitive_kind,
                start_primitive_face,
                start_primitive_edge,
            },
        );
    }
    for segment in &preplan.collapsed_segments {
        let (start_coordinate, start_primitive_kind, start_primitive_face, start_primitive_edge) =
            source_preserving_start_primitive(preplan, segment.from_point);
        segments_by_points.insert(
            (segment.contour_index, segment.from_point, segment.to_point),
            SourcePreservingCutSegment {
                edge: preplan_segment_edge(preplan, segment.from_point, segment.to_point),
                source_face: segment.source_faces.first().copied(),
                collapsed: true,
                start_coordinate,
                start_primitive_kind,
                start_primitive_face,
                start_primitive_edge,
            },
        );
    }

    preplan
        .contour_points
        .iter()
        .enumerate()
        .map(|(contour_index, point_ids)| {
            contour_segment_pairs(
                point_ids,
                preplan
                    .contour_closed
                    .get(contour_index)
                    .copied()
                    .unwrap_or_default(),
            )
            .into_iter()
            .filter_map(|(from_point, to_point)| {
                segments_by_points
                    .get(&(contour_index, from_point, to_point))
                    .cloned()
            })
            .collect()
        })
        .collect()
}

pub(super) fn source_preserving_start_primitive(
    preplan: &ExactCutPreplan,
    point: usize,
) -> ([f64; 3], usize, Option<usize>, Option<[usize; 2]>) {
    let Some(cut_point) = preplan.cut_points.get(point) else {
        return ([0.0, 0.0, 0.0], 3, None, None);
    };
    match cut_point.original_primitive.clone() {
        ExactOneMeshPrimitive::Edge(edge) => (cut_point.coordinate, 1, None, Some(edge)),
        ExactOneMeshPrimitive::Face(face) => (cut_point.coordinate, 2, Some(face), None),
    }
}

pub(super) fn preplan_segment_edge(
    preplan: &ExactCutPreplan,
    from_point: usize,
    to_point: usize,
) -> Option<[usize; 2]> {
    let from = preplan.cut_points.get(from_point)?;
    let to = preplan.cut_points.get(to_point)?;
    Some([from.vertex_index, to.vertex_index])
}

pub(super) fn contour_segment_pairs(point_ids: &[usize], closed: bool) -> Vec<(usize, usize)> {
    if point_ids.len() < 2 {
        return Vec::new();
    }
    let mut pairs = point_ids
        .windows(2)
        .map(|window| (window[0], window[1]))
        .collect::<Vec<_>>();
    if closed && point_ids.len() > 2 {
        pairs.push((*point_ids.last().unwrap(), point_ids[0]));
    }
    pairs
}

pub(super) fn source_preserving_cut_path_lengths(
    paths: &[Vec<SourcePreservingCutSegment>],
) -> Vec<usize> {
    paths.iter().map(Vec::len).collect()
}

pub(super) fn source_preserving_cut_path_source_faces(
    paths: &[Vec<SourcePreservingCutSegment>],
) -> Vec<Vec<usize>> {
    paths
        .iter()
        .map(|path| {
            path.iter()
                .filter_map(|segment| segment.source_face)
                .collect()
        })
        .collect()
}

pub(super) fn source_preserving_cut_path_collapsed(
    paths: &[Vec<SourcePreservingCutSegment>],
) -> Vec<Vec<bool>> {
    paths
        .iter()
        .map(|path| path.iter().map(|segment| segment.collapsed).collect())
        .collect()
}

pub(super) fn source_preserving_cut_path_start_primitive_kinds(
    paths: &[Vec<SourcePreservingCutSegment>],
) -> Vec<Vec<usize>> {
    paths
        .iter()
        .map(|path| {
            path.iter()
                .map(|segment| segment.start_primitive_kind)
                .collect()
        })
        .collect()
}

pub(super) fn source_preserving_cut_path_start_primitive_faces(
    paths: &[Vec<SourcePreservingCutSegment>],
) -> Vec<Vec<i64>> {
    paths
        .iter()
        .map(|path| {
            path.iter()
                .map(|segment| {
                    segment
                        .start_primitive_face
                        .and_then(|face| i64::try_from(face).ok())
                        .unwrap_or(-1)
                })
                .collect()
        })
        .collect()
}

pub(super) fn source_preserving_meshlib_like_order_rotations(
    paths: &[Vec<SourcePreservingCutSegment>],
) -> Vec<usize> {
    paths
        .iter()
        .map(|path| source_preserving_meshlib_like_order_rotation(path))
        .collect()
}

pub(super) fn source_preserving_meshlib_like_order_rotation(
    path: &[SourcePreservingCutSegment],
) -> usize {
    let Some(max_coordinate) = path
        .iter()
        .map(|segment| segment.start_coordinate)
        .max_by(|left, right| compare_coordinates(*left, *right))
    else {
        return 0;
    };
    let max_coordinate_indices = path
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| {
            (compare_coordinates(segment.start_coordinate, max_coordinate)
                == std::cmp::Ordering::Equal)
                .then_some(index)
        })
        .collect::<Vec<_>>();
    max_coordinate_indices
        .iter()
        .copied()
        .find(|index| {
            let next = (*index + 1) % path.len();
            compare_coordinates(path[next].start_coordinate, max_coordinate)
                != std::cmp::Ordering::Equal
        })
        .or_else(|| max_coordinate_indices.last().copied())
        .unwrap_or_default()
}

pub(super) fn compare_coordinates(left: [f64; 3], right: [f64; 3]) -> std::cmp::Ordering {
    left[0]
        .total_cmp(&right[0])
        .then_with(|| left[1].total_cmp(&right[1]))
        .then_with(|| left[2].total_cmp(&right[2]))
}

pub(super) fn rotate_paths<T: Copy>(paths: &[Vec<T>], rotations: &[usize]) -> Vec<Vec<T>> {
    paths
        .iter()
        .enumerate()
        .map(|(path_index, path)| {
            let rotation = rotations.get(path_index).copied().unwrap_or_default();
            rotate_path(path, rotation)
        })
        .collect()
}

pub(super) fn rotate_path<T: Copy>(path: &[T], rotation: usize) -> Vec<T> {
    if path.is_empty() {
        return Vec::new();
    }
    let offset = rotation % path.len();
    path[offset..]
        .iter()
        .chain(path[..offset].iter())
        .copied()
        .collect()
}

pub(super) fn rotated_source_preserving_cut_path_edges(
    paths: &[Vec<SourcePreservingCutSegment>],
    rotations: &[usize],
) -> Vec<Vec<[usize; 2]>> {
    paths
        .iter()
        .enumerate()
        .map(|(path_index, path)| {
            if path.is_empty() {
                return Vec::new();
            }
            let offset = rotations.get(path_index).copied().unwrap_or_default() % path.len();
            (0..path.len())
                .filter_map(|index| path[(offset + index) % path.len()].edge)
                .collect()
        })
        .collect()
}

pub(super) fn collapsed_owner_candidates(
    collapsed_paths: &[Vec<bool>],
    owner_paths: &[Vec<usize>],
) -> Vec<Vec<usize>> {
    collapsed_paths
        .iter()
        .zip(owner_paths)
        .map(|(collapsed, owners)| {
            collapsed
                .iter()
                .zip(owners)
                .filter_map(|(is_collapsed, owner)| is_collapsed.then_some(*owner))
                .collect()
        })
        .collect()
}

pub(super) fn source_preserving_meshlib_removed_face_owner_candidates(
    cut: &ExactCutMeshResult,
    faces_i64: &[[i64; 3]],
    paths: &[Vec<SourcePreservingCutSegment>],
) -> (Vec<Vec<usize>>, usize) {
    let cut_edges = cut
        .cut_edges
        .iter()
        .copied()
        .map(ordered_edge)
        .collect::<std::collections::BTreeSet<_>>();
    let mut missing_records = 0_usize;
    let candidates = paths
        .iter()
        .map(|path| {
            path.iter()
                .filter_map(|segment| {
                    let owner = segment
                        .start_primitive_face
                        .or_else(|| {
                            segment
                                .start_primitive_edge
                                .and_then(|edge| directed_edge_left_face(faces_i64, edge))
                        })
                        .or_else(|| {
                            segment
                                .edge
                                .filter(|edge| {
                                    !segment.collapsed && cut_edges.contains(&ordered_edge(*edge))
                                })
                                .and_then(|edge| {
                                    meshlib_removed_face_owner_candidate(
                                        segment.source_face,
                                        cut_edge_side_source_faces(
                                            cut,
                                            edge,
                                            DirectedEdgeSide::Left,
                                        )
                                        .first()
                                        .copied(),
                                        cut_edge_side_source_faces(
                                            cut,
                                            edge,
                                            DirectedEdgeSide::Right,
                                        )
                                        .first()
                                        .copied(),
                                    )
                                })
                        });
                    if owner.is_none() {
                        missing_records += 1;
                    }
                    owner.or(segment.source_face)
                })
                .collect()
        })
        .collect();
    (candidates, missing_records)
}

pub(super) fn directed_edge_left_face(faces_i64: &[[i64; 3]], edge: [usize; 2]) -> Option<usize> {
    faces_i64
        .iter()
        .enumerate()
        .find_map(|(face_index, face)| face_has_directed_edge(*face, edge).then_some(face_index))
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

pub(super) fn face_has_directed_edge(face: [i64; 3], edge: [usize; 2]) -> bool {
    let Ok(a) = usize::try_from(face[0]) else {
        return false;
    };
    let Ok(b) = usize::try_from(face[1]) else {
        return false;
    };
    let Ok(c) = usize::try_from(face[2]) else {
        return false;
    };
    (a == edge[0] && b == edge[1])
        || (b == edge[0] && c == edge[1])
        || (c == edge[0] && a == edge[1])
}

pub(super) fn source_face_runs_by_path(source_face_paths: &[Vec<usize>]) -> Vec<Vec<[usize; 2]>> {
    source_face_paths
        .iter()
        .map(|source_faces| source_face_runs(source_faces))
        .collect()
}

pub(super) fn source_face_counts_by_path(source_face_paths: &[Vec<usize>]) -> Vec<Vec<[usize; 2]>> {
    source_face_paths
        .iter()
        .map(|source_faces| {
            let mut counts = std::collections::BTreeMap::<usize, usize>::new();
            for source_face in source_faces {
                *counts.entry(*source_face).or_default() += 1;
            }
            counts
                .into_iter()
                .map(|(source_face, count)| [source_face, count])
                .collect()
        })
        .collect()
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

pub(super) fn source_preserving_meshlib_like_replacement_source_faces(
    owner_paths: &[Vec<usize>],
) -> Vec<Vec<usize>> {
    owner_paths
        .iter()
        .map(|owner_path| {
            let runs = circular_source_face_runs(owner_path);
            let mut owner_counts = std::collections::BTreeMap::<usize, usize>::new();
            for source_face in owner_path {
                *owner_counts.entry(*source_face).or_default() += 1;
            }
            let mut replacement_sources = Vec::new();
            let mut consumed_primary_sources = std::collections::BTreeSet::<usize>::new();
            for (run_index, [source_face, run_len]) in runs.iter().copied().enumerate() {
                let source_hits = owner_counts.get(&source_face).copied().unwrap_or_default();
                let records = if consumed_primary_sources.insert(source_face) {
                    let later_run_hits = runs[(run_index + 1)..]
                        .iter()
                        .filter_map(|[candidate_source, candidate_len]| {
                            (*candidate_source == source_face).then_some(*candidate_len)
                        })
                        .sum::<usize>();
                    source_hits * 2 + 1 - later_run_hits
                } else {
                    run_len
                };
                replacement_sources.extend(std::iter::repeat(source_face).take(records));
            }
            replacement_sources
        })
        .collect()
}

pub(super) fn source_preserving_meshlib_like_replacement_lifecycle_runs(
    owner_paths: &[Vec<usize>],
    collapsed_paths: &[Vec<bool>],
) -> Vec<Vec<[usize; 4]>> {
    owner_paths
        .iter()
        .zip(collapsed_paths)
        .map(|(owner_path, collapsed_path)| {
            let runs = circular_source_face_lifecycle_runs(owner_path, collapsed_path);
            let mut owner_counts = std::collections::BTreeMap::<usize, usize>::new();
            for source_face in owner_path {
                *owner_counts.entry(*source_face).or_default() += 1;
            }
            let mut consumed_primary_sources = std::collections::BTreeSet::<usize>::new();
            runs.iter()
                .copied()
                .enumerate()
                .map(|(run_index, [source_face, run_len, collapsed_hits])| {
                    let source_hits = owner_counts.get(&source_face).copied().unwrap_or_default();
                    let replacement_records = if consumed_primary_sources.insert(source_face) {
                        let later_run_hits = runs[(run_index + 1)..]
                            .iter()
                            .filter_map(|[candidate_source, candidate_len, _]| {
                                (*candidate_source == source_face).then_some(*candidate_len)
                            })
                            .sum::<usize>();
                        source_hits * 2 + 1 - later_run_hits
                    } else {
                        run_len
                    };
                    [source_face, run_len, collapsed_hits, replacement_records]
                })
                .collect()
        })
        .collect()
}

pub(super) fn source_preserving_meshlib_like_replacement_lifecycle_slot_runs(
    original_source_faces: usize,
    lifecycle_paths: &[Vec<[usize; 4]>],
) -> Vec<Vec<[usize; 8]>> {
    lifecycle_paths
        .iter()
        .enumerate()
        .map(|(path_index, lifecycle_runs)| {
            let mut next_slot = original_source_faces;
            lifecycle_runs
                .iter()
                .copied()
                .enumerate()
                .map(
                    |(
                        run_index,
                        [source_face, contour_hits, collapsed_hits, replacement_records],
                    )| {
                        let start_slot = next_slot;
                        next_slot += replacement_records;
                        [
                            path_index,
                            run_index,
                            source_face,
                            contour_hits,
                            collapsed_hits,
                            replacement_records,
                            start_slot,
                            next_slot,
                        ]
                    },
                )
                .collect()
        })
        .collect()
}

pub(super) fn circular_source_face_lifecycle_runs(
    source_faces: &[usize],
    collapsed_path: &[bool],
) -> Vec<[usize; 3]> {
    let mut runs = Vec::<[usize; 3]>::new();
    for (source_face, is_collapsed) in source_faces.iter().zip(collapsed_path) {
        match runs.last_mut() {
            Some([run_source_face, count, collapsed_hits]) if run_source_face == source_face => {
                *count += 1;
                if *is_collapsed {
                    *collapsed_hits += 1;
                }
            }
            _ => runs.push([*source_face, 1, usize::from(*is_collapsed)]),
        }
    }
    if runs.len() > 1 && runs.first().map(|run| run[0]) == runs.last().map(|run| run[0]) {
        let tail = runs.pop().unwrap();
        runs[0][1] += tail[1];
        runs[0][2] += tail[2];
    }
    runs
}

pub(super) fn circular_source_face_runs(source_faces: &[usize]) -> Vec<[usize; 2]> {
    let mut runs = source_face_runs(source_faces);
    if runs.len() > 1 && runs.first().map(|run| run[0]) == runs.last().map(|run| run[0]) {
        let tail = runs.pop().unwrap();
        runs[0][1] += tail[1];
    }
    runs
}

pub(super) fn source_preserving_meshlib_like_cut2origin_source_faces(
    original_source_faces: usize,
    replacement_paths: &[Vec<usize>],
) -> Vec<Vec<usize>> {
    let prepared_sources = (0..original_source_faces).collect::<Vec<_>>();
    replacement_paths
        .iter()
        .map(|replacement_path| {
            prepared_sources
                .iter()
                .copied()
                .chain(replacement_path.iter().copied())
                .collect()
        })
        .collect()
}

pub(super) fn duplicate_path_edges(paths: &[Vec<[usize; 2]>]) -> usize {
    paths
        .iter()
        .map(|path| {
            let mut seen = std::collections::BTreeSet::new();
            path.iter()
                .copied()
                .map(ordered_edge)
                .filter(|edge| !seen.insert(*edge))
                .count()
        })
        .sum()
}

pub(super) fn duplicate_path_edge_occurrences(paths: &[Vec<[usize; 2]>]) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    paths
        .iter()
        .flatten()
        .copied()
        .map(ordered_edge)
        .filter(|edge| !seen.insert(*edge))
        .count()
}

pub(super) fn duplicate_path_edge_path_indices(paths: &[Vec<[usize; 2]>]) -> Vec<Vec<usize>> {
    let mut occurrences = std::collections::BTreeMap::<[usize; 2], Vec<usize>>::new();
    for (path_index, path) in paths.iter().enumerate() {
        for edge in path {
            let indices = occurrences.entry(ordered_edge(*edge)).or_default();
            if indices.last().copied() != Some(path_index) {
                indices.push(path_index);
            }
        }
    }
    occurrences
        .into_values()
        .filter(|indices| indices.len() > 1)
        .collect()
}

pub(super) fn unique_regular_path_repairs(
    combined: &ExactCutMeshResult,
    regular_path_count: usize,
) -> Vec<ExactCutShadowRepairPath> {
    let edge_counts = path_edge_occurrence_counts(&combined.cut_edge_paths);
    let mut repairs = Vec::new();
    for path_index in 0..regular_path_count.min(combined.cut_edge_paths.len()) {
        let path = &combined.cut_edge_paths[path_index];
        let empty_sources = Vec::new();
        let sources = combined
            .cut_edge_path_source_faces
            .get(path_index)
            .unwrap_or(&empty_sources);
        let mut repair_path = Vec::new();
        let mut repair_sources = Vec::new();
        for (edge_index, edge) in path.iter().copied().enumerate() {
            if edge_counts.get(&ordered_edge(edge)).copied().unwrap_or(0) == 1 {
                repair_path.push(edge);
                repair_sources.push(sources.get(edge_index).copied().unwrap_or(None));
                continue;
            }
            append_repair_path(&mut repairs, &mut repair_path, &mut repair_sources);
        }
        append_repair_path(&mut repairs, &mut repair_path, &mut repair_sources);
    }
    repairs
}

pub(super) fn append_repair_path(
    repairs: &mut Vec<ExactCutShadowRepairPath>,
    path: &mut Vec<[usize; 2]>,
    sources: &mut Vec<Option<usize>>,
) {
    if path.is_empty() {
        return;
    }
    repairs.push(ExactCutShadowRepairPath {
        path: std::mem::take(path),
        source_faces: std::mem::take(sources),
    });
}

pub(super) fn path_edge_occurrence_counts(
    paths: &[Vec<[usize; 2]>],
) -> std::collections::BTreeMap<[usize; 2], usize> {
    let mut counts = std::collections::BTreeMap::<[usize; 2], usize>::new();
    for edge in paths.iter().flatten().copied() {
        *counts.entry(ordered_edge(edge)).or_default() += 1;
    }
    counts
}

pub(super) fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}
