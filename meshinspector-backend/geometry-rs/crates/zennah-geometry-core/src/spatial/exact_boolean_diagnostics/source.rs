use super::super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use super::super::exact_coplanar::same_oriented_coplanar_overlap_faces;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_fill_apply::ExactCutHoleFillResult;
use super::super::exact_stitch::ExactStitchPlan;
use crate::GeometryError;
use std::collections::BTreeMap;
mod details;
pub(super) use details::duplicate_face_counts;
use details::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct FaceSourceSummary {
    pub selected_first_faces: usize,
    pub selected_second_faces: usize,
    pub first_source_face_groups: usize,
    pub second_source_face_groups: usize,
    pub duplicate_first_source_faces: usize,
    pub duplicate_second_source_faces: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct RawFaceSelectionSummary {
    pub raw_selected_faces: [usize; 2],
    pub overlap_faces: [usize; 2],
    pub boundary_misses: [[usize; 2]; 2],
    pub selection_delta_faces: [i64; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct CutSourceFaceInventory {
    pub faces: usize,
    pub source_records: usize,
    pub unique_source_faces: usize,
    pub duplicate_source_records: usize,
    pub fill_plans: usize,
    pub added_faces: usize,
    pub source_face_counts: Vec<[usize; 2]>,
    pub fill_plan_source_faces: Vec<usize>,
    pub fill_plan_added_faces: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct CutPathInventory {
    pub path_lengths: Vec<usize>,
    pub closed_path_lengths: Vec<usize>,
    pub path_source_faces: Vec<Vec<usize>>,
    pub path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub closed_path_source_faces: Vec<Vec<usize>>,
    pub closed_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub path_edge_adjacent_source_faces: Vec<Vec<Vec<usize>>>,
    pub closed_path_edge_adjacent_source_faces: Vec<Vec<Vec<usize>>>,
    pub path_edge_left_source_faces: Vec<Vec<Vec<usize>>>,
    pub path_edge_right_source_faces: Vec<Vec<Vec<usize>>>,
    pub closed_path_edge_left_source_faces: Vec<Vec<Vec<usize>>>,
    pub closed_path_edge_right_source_faces: Vec<Vec<Vec<usize>>>,
    pub closed_path_edge_left_primary_source_faces: Vec<Vec<usize>>,
    pub closed_path_edge_right_primary_source_faces: Vec<Vec<usize>>,
    pub closed_path_edge_left_primary_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub closed_path_edge_right_primary_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub closed_path_meshlib_removed_face_owner_candidates: Vec<Vec<usize>>,
    pub closed_path_meshlib_removed_face_owner_candidate_runs: Vec<Vec<[usize; 2]>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct StitchResultCutSourceInventory {
    pub path_lengths: Vec<usize>,
    pub first_path_source_faces: Vec<Vec<usize>>,
    pub second_path_source_faces: Vec<Vec<usize>>,
    pub first_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub second_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub first_path_meshlib_removed_face_owner_candidates: Vec<Vec<usize>>,
    pub second_path_meshlib_removed_face_owner_candidates: Vec<Vec<usize>>,
    pub first_path_meshlib_removed_face_owner_candidate_runs: Vec<Vec<[usize; 2]>>,
    pub second_path_meshlib_removed_face_owner_candidate_runs: Vec<Vec<[usize; 2]>>,
    pub first_source_faces: Vec<usize>,
    pub second_source_faces: Vec<usize>,
    pub first_source_face_runs: Vec<[usize; 2]>,
    pub second_source_face_runs: Vec<[usize; 2]>,
    pub first_meshlib_removed_face_owner_candidates: Vec<usize>,
    pub second_meshlib_removed_face_owner_candidates: Vec<usize>,
    pub first_meshlib_removed_face_owner_candidate_runs: Vec<[usize; 2]>,
    pub second_meshlib_removed_face_owner_candidate_runs: Vec<[usize; 2]>,
    pub meshlib_removed_face_owner_candidate_missing_records: [usize; 2],
    pub missing_source_records: [usize; 2],
    pub edge_grouped_path_lengths: Vec<usize>,
    pub edge_grouped_closed_paths: usize,
    pub first_edge_grouped_path_source_faces: Vec<Vec<usize>>,
    pub second_edge_grouped_path_source_faces: Vec<Vec<usize>>,
    pub first_edge_grouped_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub second_edge_grouped_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub first_edge_grouped_source_faces: Vec<usize>,
    pub second_edge_grouped_source_faces: Vec<usize>,
    pub first_edge_grouped_source_face_runs: Vec<[usize; 2]>,
    pub second_edge_grouped_source_face_runs: Vec<[usize; 2]>,
    pub edge_grouped_missing_source_records: [usize; 2],
}

pub(super) fn face_source_summary(assembly: &ExactBooleanAssemblyResult) -> FaceSourceSummary {
    let (first_source_face_groups, duplicate_first_source_faces) =
        duplicate_source_faces(assembly, ExactBooleanOperand::First);
    let (second_source_face_groups, duplicate_second_source_faces) =
        duplicate_source_faces(assembly, ExactBooleanOperand::Second);
    FaceSourceSummary {
        selected_first_faces: assembly.selected_first_faces.len(),
        selected_second_faces: assembly.selected_second_faces.len(),
        first_source_face_groups,
        second_source_face_groups,
        duplicate_first_source_faces,
        duplicate_second_source_faces,
    }
}

pub(super) fn raw_face_selection_summary(
    input: &super::ExactBooleanPipelineDiagnosticInputs<'_>,
    actual: &FaceSourceSummary,
) -> Result<RawFaceSelectionSummary, GeometryError> {
    let first = input.first_cut;
    let second = input.second_cut;
    let first_prepare_faces = &input.assembly.prepare_first_faces;
    let second_prepare_faces = &input.assembly.prepare_second_faces;
    let first_raw = first_prepare_faces.len();
    let second_raw = second_prepare_faces.len();
    let first_overlap_faces = same_oriented_coplanar_overlap_faces(
        &first.mesh.vertices,
        &first.mesh.faces,
        &second.mesh.vertices,
        &second.mesh.faces,
        input.epsilon,
    )?
    .len();
    let second_overlap_faces = same_oriented_coplanar_overlap_faces(
        &second.mesh.vertices,
        &second.mesh.faces,
        &first.mesh.vertices,
        &first.mesh.faces,
        input.epsilon,
    )?
    .len();
    Ok(RawFaceSelectionSummary {
        raw_selected_faces: [first_raw, second_raw],
        overlap_faces: [first_overlap_faces, second_overlap_faces],
        boundary_misses: boundary_misses(input, first_prepare_faces, second_prepare_faces),
        selection_delta_faces: [
            actual.selected_first_faces as i64 - first_raw as i64,
            actual.selected_second_faces as i64 - second_raw as i64,
        ],
    })
}

pub(super) fn cut_source_face_inventory(cut: &ExactCutHoleFillResult) -> CutSourceFaceInventory {
    let mut counts = BTreeMap::<usize, usize>::new();
    for source_face in &cut.mesh.source_face_for_faces {
        *counts.entry(*source_face).or_default() += 1;
    }
    let source_records = cut.mesh.source_face_for_faces.len();
    let added_faces = cut
        .added_face_ranges
        .iter()
        .map(|[start, end]| end.saturating_sub(*start))
        .sum();

    CutSourceFaceInventory {
        faces: cut.mesh.faces.len(),
        source_records,
        unique_source_faces: counts.len(),
        duplicate_source_records: source_records.saturating_sub(counts.len()),
        fill_plans: cut.fill_plans.len(),
        added_faces,
        source_face_counts: counts
            .into_iter()
            .map(|(source_face, count)| [source_face, count])
            .collect(),
        fill_plan_source_faces: cut.fill_plans.iter().map(|plan| plan.source_face).collect(),
        fill_plan_added_faces: cut
            .fill_plans
            .iter()
            .map(|plan| plan.fill_plan.num_tris)
            .collect(),
    }
}

pub(super) fn cut_path_inventory(cut: &ExactCutMeshResult) -> CutPathInventory {
    let path_lengths = cut.cut_edge_paths.iter().map(Vec::len).collect::<Vec<_>>();
    let closed_path_lengths = cut
        .cut_edge_paths
        .iter()
        .zip(&cut.cut_edge_path_closed)
        .filter_map(|(path, closed)| closed.then_some(path.len()))
        .collect::<Vec<_>>();
    let path_source_faces = cut
        .cut_edge_path_source_faces
        .iter()
        .map(|source_faces| source_faces.iter().flatten().copied().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let path_source_face_runs = path_source_faces
        .iter()
        .map(|source_faces| source_face_runs(source_faces))
        .collect::<Vec<_>>();
    let closed_path_source_faces = path_source_faces
        .iter()
        .zip(&cut.cut_edge_path_closed)
        .filter_map(|(source_faces, closed)| closed.then_some(source_faces.clone()))
        .collect::<Vec<_>>();
    let closed_path_source_face_runs = path_source_face_runs
        .iter()
        .zip(&cut.cut_edge_path_closed)
        .filter_map(|(source_face_runs, closed)| closed.then_some(source_face_runs.clone()))
        .collect::<Vec<_>>();
    let path_edge_adjacent_source_faces = cut
        .cut_edge_paths
        .iter()
        .map(|path| {
            path.iter()
                .map(|edge| cut_edge_adjacent_source_faces(cut, *edge))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let closed_path_edge_adjacent_source_faces = path_edge_adjacent_source_faces
        .iter()
        .zip(&cut.cut_edge_path_closed)
        .filter_map(|(source_faces, closed)| closed.then_some(source_faces.clone()))
        .collect::<Vec<_>>();
    let path_edge_left_source_faces = cut
        .cut_edge_paths
        .iter()
        .map(|path| {
            path.iter()
                .map(|edge| cut_edge_side_source_faces(cut, *edge, DirectedEdgeSide::Left))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let path_edge_right_source_faces = cut
        .cut_edge_paths
        .iter()
        .map(|path| {
            path.iter()
                .map(|edge| cut_edge_side_source_faces(cut, *edge, DirectedEdgeSide::Right))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let closed_path_edge_left_source_faces = path_edge_left_source_faces
        .iter()
        .zip(&cut.cut_edge_path_closed)
        .filter_map(|(source_faces, closed)| closed.then_some(source_faces.clone()))
        .collect::<Vec<_>>();
    let closed_path_edge_right_source_faces = path_edge_right_source_faces
        .iter()
        .zip(&cut.cut_edge_path_closed)
        .filter_map(|(source_faces, closed)| closed.then_some(source_faces.clone()))
        .collect::<Vec<_>>();
    let closed_path_edge_left_primary_source_faces =
        primary_edge_source_faces(&closed_path_edge_left_source_faces);
    let closed_path_edge_right_primary_source_faces =
        primary_edge_source_faces(&closed_path_edge_right_source_faces);
    let closed_path_edge_left_primary_source_face_runs =
        source_face_runs_by_path(&closed_path_edge_left_primary_source_faces);
    let closed_path_edge_right_primary_source_face_runs =
        source_face_runs_by_path(&closed_path_edge_right_primary_source_faces);
    let closed_path_meshlib_removed_face_owner_candidates =
        closed_path_meshlib_removed_face_owner_candidates(
            cut,
            &closed_path_edge_left_primary_source_faces,
            &closed_path_edge_right_primary_source_faces,
        );
    let closed_path_meshlib_removed_face_owner_candidate_runs =
        source_face_runs_by_path(&closed_path_meshlib_removed_face_owner_candidates);
    CutPathInventory {
        path_lengths,
        closed_path_lengths,
        path_source_faces,
        path_source_face_runs,
        closed_path_source_faces,
        closed_path_source_face_runs,
        path_edge_adjacent_source_faces,
        closed_path_edge_adjacent_source_faces,
        path_edge_left_source_faces,
        path_edge_right_source_faces,
        closed_path_edge_left_source_faces,
        closed_path_edge_right_source_faces,
        closed_path_edge_left_primary_source_faces,
        closed_path_edge_right_primary_source_faces,
        closed_path_edge_left_primary_source_face_runs,
        closed_path_edge_right_primary_source_face_runs,
        closed_path_meshlib_removed_face_owner_candidates,
        closed_path_meshlib_removed_face_owner_candidate_runs,
    }
}

pub(super) fn stitch_result_cut_source_inventory(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    stitch_plan: &ExactStitchPlan,
) -> StitchResultCutSourceInventory {
    let first_source_faces_by_edge = cut_edge_source_faces_by_index(first);
    let second_source_faces_by_edge = cut_edge_source_faces_by_index(second);
    let first_owner_candidates_by_edge =
        cut_edge_meshlib_removed_face_owner_candidates_by_index(first, &first_source_faces_by_edge);
    let second_owner_candidates_by_edge = cut_edge_meshlib_removed_face_owner_candidates_by_index(
        second,
        &second_source_faces_by_edge,
    );
    let pair_groups = stitch_plan
        .paths
        .iter()
        .map(|path| path.pair_indices.clone())
        .collect::<Vec<_>>();
    let source_paths = stitch_source_paths_from_pair_groups(
        &pair_groups,
        stitch_plan,
        &first_source_faces_by_edge,
        &second_source_faces_by_edge,
        &first_owner_candidates_by_edge,
        &second_owner_candidates_by_edge,
    );
    let edge_grouped_pair_paths = edge_grouped_stitch_pair_paths(&stitch_plan.pairs);
    let edge_grouped_closed_paths = edge_grouped_pair_paths
        .iter()
        .filter(|(_, closed)| *closed)
        .count();
    let edge_grouped_pair_groups = edge_grouped_pair_paths
        .into_iter()
        .map(|(pair_indices, _)| pair_indices)
        .collect::<Vec<_>>();
    let edge_grouped_source_paths = stitch_source_paths_from_pair_groups(
        &edge_grouped_pair_groups,
        stitch_plan,
        &first_source_faces_by_edge,
        &second_source_faces_by_edge,
        &first_owner_candidates_by_edge,
        &second_owner_candidates_by_edge,
    );

    StitchResultCutSourceInventory {
        path_lengths: source_paths.path_lengths,
        first_path_source_faces: source_paths.first_path_source_faces,
        second_path_source_faces: source_paths.second_path_source_faces,
        first_path_source_face_runs: source_paths.first_path_source_face_runs,
        second_path_source_face_runs: source_paths.second_path_source_face_runs,
        first_path_meshlib_removed_face_owner_candidates: source_paths
            .first_path_meshlib_removed_face_owner_candidates,
        second_path_meshlib_removed_face_owner_candidates: source_paths
            .second_path_meshlib_removed_face_owner_candidates,
        first_path_meshlib_removed_face_owner_candidate_runs: source_paths
            .first_path_meshlib_removed_face_owner_candidate_runs,
        second_path_meshlib_removed_face_owner_candidate_runs: source_paths
            .second_path_meshlib_removed_face_owner_candidate_runs,
        first_source_faces: source_paths.first_source_faces,
        second_source_faces: source_paths.second_source_faces,
        first_source_face_runs: source_paths.first_source_face_runs,
        second_source_face_runs: source_paths.second_source_face_runs,
        first_meshlib_removed_face_owner_candidates: source_paths
            .first_meshlib_removed_face_owner_candidates,
        second_meshlib_removed_face_owner_candidates: source_paths
            .second_meshlib_removed_face_owner_candidates,
        first_meshlib_removed_face_owner_candidate_runs: source_paths
            .first_meshlib_removed_face_owner_candidate_runs,
        second_meshlib_removed_face_owner_candidate_runs: source_paths
            .second_meshlib_removed_face_owner_candidate_runs,
        meshlib_removed_face_owner_candidate_missing_records: source_paths
            .meshlib_removed_face_owner_candidate_missing_records,
        missing_source_records: source_paths.missing_source_records,
        edge_grouped_path_lengths: edge_grouped_source_paths.path_lengths,
        edge_grouped_closed_paths,
        first_edge_grouped_path_source_faces: edge_grouped_source_paths.first_path_source_faces,
        second_edge_grouped_path_source_faces: edge_grouped_source_paths.second_path_source_faces,
        first_edge_grouped_path_source_face_runs: edge_grouped_source_paths
            .first_path_source_face_runs,
        second_edge_grouped_path_source_face_runs: edge_grouped_source_paths
            .second_path_source_face_runs,
        first_edge_grouped_source_faces: edge_grouped_source_paths.first_source_faces,
        second_edge_grouped_source_faces: edge_grouped_source_paths.second_source_faces,
        first_edge_grouped_source_face_runs: edge_grouped_source_paths.first_source_face_runs,
        second_edge_grouped_source_face_runs: edge_grouped_source_paths.second_source_face_runs,
        edge_grouped_missing_source_records: edge_grouped_source_paths.missing_source_records,
    }
}

#[cfg(test)]
mod tests;
