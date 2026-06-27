use super::super::exact_coplanar::coplanar_overlap_contours;
use super::super::exact_cut::exact_cut_preplan;
use super::super::exact_cut_apply::exact_cut_mesh_from_preplan;
use super::super::exact_cut_apply::{exact_cut_mesh_by_contours, ExactCutMeshResult};
use super::super::exact_lone_retry::exact_pair_intersection_contours_with_coplanar;
use super::super::exact_one_mesh::{exact_one_mesh_intersection_contours, ExactOneMeshContours};
use super::super::exact_stitch::exact_stitch_plan_from_cut_meshes;
use super::source_preserving::*;
use super::types::{
    CoplanarContourCutTrialResult, ExactCoplanarContourCutTrial, ExactCoplanarCutCandidate,
};
use super::ExactMeshPairCutMeshes;
use crate::GeometryError;

pub(super) fn coplanar_contour_cut_trial(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
    stable: &ExactMeshPairCutMeshes,
) -> Result<Option<CoplanarContourCutTrialResult>, GeometryError> {
    let coplanar = coplanar_overlap_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        epsilon,
    )?;
    let contours = coplanar.merged_contours.first.len();
    if contours == 0 {
        return Ok(None);
    }
    let contour_edges = coplanar
        .merged_contours
        .first
        .iter()
        .map(|contour| contour.intersections.len())
        .sum();
    let combined = exact_pair_intersection_contours_with_coplanar(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    let first =
        exact_cut_mesh_by_contours(first_vertices, first_faces_i64, &combined.first, epsilon)?;
    let second =
        exact_cut_mesh_by_contours(second_vertices, second_faces_i64, &combined.second, epsilon)?;
    let regular_contours = exact_one_mesh_intersection_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    let regular_first_paths = regular_contours.first.len();
    let regular_second_paths = regular_contours.second.len();
    let mut paired_combined = regular_contours;
    append_one_mesh_contours(
        &mut paired_combined,
        coplanar.paired_merged_contours.clone(),
    );
    let paired_first_preplan = exact_cut_preplan(
        first_vertices,
        first_faces_i64,
        &paired_combined.first,
        epsilon,
    )?;
    let paired_second_preplan = exact_cut_preplan(
        second_vertices,
        second_faces_i64,
        &paired_combined.second,
        epsilon,
    )?;
    let mut paired_first = exact_cut_mesh_from_preplan(
        first_vertices,
        first_faces_i64,
        &paired_first_preplan,
        epsilon,
    )?;
    let mut paired_second = exact_cut_mesh_from_preplan(
        second_vertices,
        second_faces_i64,
        &paired_second_preplan,
        epsilon,
    )?;
    let paired_combined_first_cut_path_lengths = cut_path_lengths(&paired_first.cut_edge_paths);
    let paired_combined_second_cut_path_lengths = cut_path_lengths(&paired_second.cut_edge_paths);
    let paired_combined_first_cut_path_source_faces = cut_path_source_faces(&paired_first);
    let paired_combined_second_cut_path_source_faces = cut_path_source_faces(&paired_second);
    let paired_combined_first_cut_path_source_face_runs =
        source_face_runs_by_path(&paired_combined_first_cut_path_source_faces);
    let paired_combined_second_cut_path_source_face_runs =
        source_face_runs_by_path(&paired_combined_second_cut_path_source_faces);
    let paired_combined_first_collapsed_cut_path_lengths =
        cut_path_lengths(&paired_first.collapsed_cut_segment_paths);
    let paired_combined_second_collapsed_cut_path_lengths =
        cut_path_lengths(&paired_second.collapsed_cut_segment_paths);
    let paired_combined_first_collapsed_cut_path_source_faces =
        collapsed_cut_path_source_faces(&paired_first);
    let paired_combined_second_collapsed_cut_path_source_faces =
        collapsed_cut_path_source_faces(&paired_second);
    let paired_combined_first_collapsed_cut_path_source_face_runs =
        source_face_runs_by_path(&paired_combined_first_collapsed_cut_path_source_faces);
    let paired_combined_second_collapsed_cut_path_source_face_runs =
        source_face_runs_by_path(&paired_combined_second_collapsed_cut_path_source_faces);
    let paired_combined_first_source_preserving_cut_paths =
        source_preserving_cut_segment_paths(&paired_first_preplan);
    let paired_combined_second_source_preserving_cut_paths =
        source_preserving_cut_segment_paths(&paired_second_preplan);
    let paired_combined_first_source_preserving_cut_path_lengths =
        source_preserving_cut_path_lengths(&paired_combined_first_source_preserving_cut_paths);
    let paired_combined_second_source_preserving_cut_path_lengths =
        source_preserving_cut_path_lengths(&paired_combined_second_source_preserving_cut_paths);
    let paired_combined_first_source_preserving_cut_path_source_faces =
        source_preserving_cut_path_source_faces(&paired_combined_first_source_preserving_cut_paths);
    let paired_combined_second_source_preserving_cut_path_source_faces =
        source_preserving_cut_path_source_faces(
            &paired_combined_second_source_preserving_cut_paths,
        );
    let paired_combined_first_source_preserving_cut_path_source_face_runs =
        source_face_runs_by_path(&paired_combined_first_source_preserving_cut_path_source_faces);
    let paired_combined_second_source_preserving_cut_path_source_face_runs =
        source_face_runs_by_path(&paired_combined_second_source_preserving_cut_path_source_faces);
    let paired_combined_first_source_preserving_cut_path_collapsed =
        source_preserving_cut_path_collapsed(&paired_combined_first_source_preserving_cut_paths);
    let paired_combined_second_source_preserving_cut_path_collapsed =
        source_preserving_cut_path_collapsed(&paired_combined_second_source_preserving_cut_paths);
    let paired_combined_first_source_preserving_cut_path_start_primitive_kinds =
        source_preserving_cut_path_start_primitive_kinds(
            &paired_combined_first_source_preserving_cut_paths,
        );
    let paired_combined_second_source_preserving_cut_path_start_primitive_kinds =
        source_preserving_cut_path_start_primitive_kinds(
            &paired_combined_second_source_preserving_cut_paths,
        );
    let paired_combined_first_source_preserving_cut_path_start_primitive_faces =
        source_preserving_cut_path_start_primitive_faces(
            &paired_combined_first_source_preserving_cut_paths,
        );
    let paired_combined_second_source_preserving_cut_path_start_primitive_faces =
        source_preserving_cut_path_start_primitive_faces(
            &paired_combined_second_source_preserving_cut_paths,
        );
    let paired_combined_first_source_preserving_meshlib_like_order_rotations =
        source_preserving_meshlib_like_order_rotations(
            &paired_combined_first_source_preserving_cut_paths,
        );
    let paired_combined_second_source_preserving_meshlib_like_order_rotations =
        source_preserving_meshlib_like_order_rotations(
            &paired_combined_second_source_preserving_cut_paths,
        );
    let paired_combined_first_source_preserving_meshlib_like_cut_path_start_primitive_faces =
        rotate_paths(
            &paired_combined_first_source_preserving_cut_path_start_primitive_faces,
            &paired_combined_first_source_preserving_meshlib_like_order_rotations,
        );
    let paired_combined_second_source_preserving_meshlib_like_cut_path_start_primitive_faces =
        rotate_paths(
            &paired_combined_second_source_preserving_cut_path_start_primitive_faces,
            &paired_combined_second_source_preserving_meshlib_like_order_rotations,
        );
    let paired_combined_first_source_preserving_meshlib_like_cut_path_collapsed = rotate_paths(
        &paired_combined_first_source_preserving_cut_path_collapsed,
        &paired_combined_first_source_preserving_meshlib_like_order_rotations,
    );
    let paired_combined_second_source_preserving_meshlib_like_cut_path_collapsed = rotate_paths(
        &paired_combined_second_source_preserving_cut_path_collapsed,
        &paired_combined_second_source_preserving_meshlib_like_order_rotations,
    );
    let paired_combined_first_source_preserving_meshlib_like_cut_edge_paths =
        rotated_source_preserving_cut_path_edges(
            &paired_combined_first_source_preserving_cut_paths,
            &paired_combined_first_source_preserving_meshlib_like_order_rotations,
        );
    let paired_combined_second_source_preserving_meshlib_like_cut_edge_paths =
        rotated_source_preserving_cut_path_edges(
            &paired_combined_second_source_preserving_cut_paths,
            &paired_combined_second_source_preserving_meshlib_like_order_rotations,
        );
    let (
        paired_combined_first_source_preserving_meshlib_removed_face_owner_candidates,
        first_owner_missing_records,
    ) = source_preserving_meshlib_removed_face_owner_candidates(
        &paired_first,
        first_faces_i64,
        &paired_combined_first_source_preserving_cut_paths,
    );
    let (
        paired_combined_second_source_preserving_meshlib_removed_face_owner_candidates,
        second_owner_missing_records,
    ) = source_preserving_meshlib_removed_face_owner_candidates(
        &paired_second,
        second_faces_i64,
        &paired_combined_second_source_preserving_cut_paths,
    );
    let paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates =
        rotate_paths(
            &paired_combined_first_source_preserving_meshlib_removed_face_owner_candidates,
            &paired_combined_first_source_preserving_meshlib_like_order_rotations,
        );
    let paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates =
        rotate_paths(
            &paired_combined_second_source_preserving_meshlib_removed_face_owner_candidates,
            &paired_combined_second_source_preserving_meshlib_like_order_rotations,
        );
    let paired_combined_first_source_preserving_meshlib_removed_face_owner_candidate_runs =
        source_face_runs_by_path(
            &paired_combined_first_source_preserving_meshlib_removed_face_owner_candidates,
        );
    let paired_combined_second_source_preserving_meshlib_removed_face_owner_candidate_runs =
        source_face_runs_by_path(
            &paired_combined_second_source_preserving_meshlib_removed_face_owner_candidates,
        );
    let paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidate_runs =
        source_face_runs_by_path(
            &paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates,
        );
    let paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidate_runs =
        source_face_runs_by_path(
            &paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates,
        );
    let paired_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates =
        collapsed_owner_candidates(
            &paired_combined_first_source_preserving_meshlib_like_cut_path_collapsed,
            &paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates,
        );
    let paired_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates =
        collapsed_owner_candidates(
            &paired_combined_second_source_preserving_meshlib_like_cut_path_collapsed,
            &paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates,
        );
    let paired_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs =
        source_face_runs_by_path(
            &paired_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates,
        );
    let paired_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs =
        source_face_runs_by_path(
            &paired_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates,
        );
    let paired_combined_first_source_preserving_meshlib_like_replacement_source_faces =
        source_preserving_meshlib_like_replacement_source_faces(
            &paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates,
        );
    let paired_combined_second_source_preserving_meshlib_like_replacement_source_faces =
        source_preserving_meshlib_like_replacement_source_faces(
            &paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates,
        );
    let paired_combined_first_source_preserving_meshlib_like_replacement_source_face_counts =
        source_face_counts_by_path(
            &paired_combined_first_source_preserving_meshlib_like_replacement_source_faces,
        );
    let paired_combined_second_source_preserving_meshlib_like_replacement_source_face_counts =
        source_face_counts_by_path(
            &paired_combined_second_source_preserving_meshlib_like_replacement_source_faces,
        );
    let paired_combined_first_source_preserving_meshlib_like_replacement_source_face_runs =
        source_face_runs_by_path(
            &paired_combined_first_source_preserving_meshlib_like_replacement_source_faces,
        );
    let paired_combined_second_source_preserving_meshlib_like_replacement_source_face_runs =
        source_face_runs_by_path(
            &paired_combined_second_source_preserving_meshlib_like_replacement_source_faces,
        );
    let paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_runs =
        source_preserving_meshlib_like_replacement_lifecycle_runs(
            &paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates,
            &paired_combined_first_source_preserving_meshlib_like_cut_path_collapsed,
        );
    let paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_runs =
        source_preserving_meshlib_like_replacement_lifecycle_runs(
            &paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates,
            &paired_combined_second_source_preserving_meshlib_like_cut_path_collapsed,
        );
    let paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs =
        source_preserving_meshlib_like_replacement_lifecycle_slot_runs(
            first_faces_i64.len(),
            &paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_runs,
        );
    let paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs =
        source_preserving_meshlib_like_replacement_lifecycle_slot_runs(
            second_faces_i64.len(),
            &paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_runs,
        );
    let paired_combined_first_source_preserving_meshlib_like_cut2origin_source_faces =
        source_preserving_meshlib_like_cut2origin_source_faces(
            first_faces_i64.len(),
            &paired_combined_first_source_preserving_meshlib_like_replacement_source_faces,
        );
    let paired_combined_second_source_preserving_meshlib_like_cut2origin_source_faces =
        source_preserving_meshlib_like_cut2origin_source_faces(
            second_faces_i64.len(),
            &paired_combined_second_source_preserving_meshlib_like_replacement_source_faces,
        );
    let paired_combined_first_source_preserving_meshlib_like_cut2origin_source_face_counts =
        source_face_counts_by_path(
            &paired_combined_first_source_preserving_meshlib_like_cut2origin_source_faces,
        );
    let paired_combined_second_source_preserving_meshlib_like_cut2origin_source_face_counts =
        source_face_counts_by_path(
            &paired_combined_second_source_preserving_meshlib_like_cut2origin_source_faces,
        );
    let paired_combined_first_source_preserving_meshlib_like_cut2origin_source_face_runs =
        source_face_runs_by_path(
            &paired_combined_first_source_preserving_meshlib_like_cut2origin_source_faces,
        );
    let paired_combined_second_source_preserving_meshlib_like_cut2origin_source_face_runs =
        source_face_runs_by_path(
            &paired_combined_second_source_preserving_meshlib_like_cut2origin_source_faces,
        );
    let paired_combined_source_preserving_meshlib_removed_face_owner_missing_records =
        [first_owner_missing_records, second_owner_missing_records];
    let paired_combined_duplicate_first_path_edge_occurrences =
        duplicate_path_edge_occurrences(&paired_first.cut_edge_paths);
    let paired_combined_duplicate_second_path_edge_occurrences =
        duplicate_path_edge_occurrences(&paired_second.cut_edge_paths);
    let paired_combined_duplicate_first_path_edge_path_indices =
        duplicate_path_edge_path_indices(&paired_first.cut_edge_paths);
    let paired_combined_duplicate_second_path_edge_path_indices =
        duplicate_path_edge_path_indices(&paired_second.cut_edge_paths);
    let mut first_shadow_repair_paths = Vec::new();
    let mut second_shadow_repair_paths = Vec::new();
    if paired_combined_duplicate_first_path_edge_occurrences > 0
        || paired_combined_duplicate_second_path_edge_occurrences > 0
    {
        let paired_first_with_regular = paired_first;
        let paired_second_with_regular = paired_second;
        first_shadow_repair_paths =
            unique_regular_path_repairs(&paired_first_with_regular, regular_first_paths);
        second_shadow_repair_paths =
            unique_regular_path_repairs(&paired_second_with_regular, regular_second_paths);
        paired_first = exact_cut_mesh_by_contours(
            first_vertices,
            first_faces_i64,
            &coplanar.paired_merged_contours.first,
            epsilon,
        )?;
        paired_second = exact_cut_mesh_by_contours(
            second_vertices,
            second_faces_i64,
            &coplanar.paired_merged_contours.second,
            epsilon,
        )?;
    }
    let paired_stitch_plan =
        exact_stitch_plan_from_cut_meshes(&paired_first, &paired_second, epsilon);
    let first_duplicate_path_edge_occurrences =
        duplicate_path_edge_occurrences(&first.cut_edge_paths);
    let second_duplicate_path_edge_occurrences =
        duplicate_path_edge_occurrences(&second.cut_edge_paths);
    let paired_duplicate_first_path_edge_occurrences =
        duplicate_path_edge_occurrences(&paired_first.cut_edge_paths);
    let paired_duplicate_second_path_edge_occurrences =
        duplicate_path_edge_occurrences(&paired_second.cut_edge_paths);
    let accepted = coplanar_trial_is_no_regression(stable, &first, &second)
        && first_duplicate_path_edge_occurrences == 0
        && second_duplicate_path_edge_occurrences == 0;
    let paired_candidate_is_clean = paired_duplicate_first_path_edge_occurrences == 0
        && paired_duplicate_second_path_edge_occurrences == 0;
    let summary = ExactCoplanarContourCutTrial {
        contours,
        contour_edges,
        first_cut_edges: first.cut_edges.len(),
        second_cut_edges: second.cut_edges.len(),
        paired_contours: coplanar.paired_merged_contours.first.len(),
        paired_contour_edges: paired_contour_edges(&coplanar.paired_merged_contours.first),
        paired_first_cut_edges: paired_first.cut_edges.len(),
        paired_second_cut_edges: paired_second.cut_edges.len(),
        paired_combined_first_cut_path_lengths,
        paired_combined_second_cut_path_lengths,
        paired_combined_first_cut_path_source_faces,
        paired_combined_second_cut_path_source_faces,
        paired_combined_first_cut_path_source_face_runs,
        paired_combined_second_cut_path_source_face_runs,
        paired_combined_first_collapsed_cut_path_lengths,
        paired_combined_second_collapsed_cut_path_lengths,
        paired_combined_first_collapsed_cut_path_source_faces,
        paired_combined_second_collapsed_cut_path_source_faces,
        paired_combined_first_collapsed_cut_path_source_face_runs,
        paired_combined_second_collapsed_cut_path_source_face_runs,
        paired_combined_first_source_preserving_cut_path_lengths,
        paired_combined_second_source_preserving_cut_path_lengths,
        paired_combined_first_source_preserving_cut_path_source_faces,
        paired_combined_second_source_preserving_cut_path_source_faces,
        paired_combined_first_source_preserving_cut_path_source_face_runs,
        paired_combined_second_source_preserving_cut_path_source_face_runs,
        paired_combined_first_source_preserving_cut_path_collapsed,
        paired_combined_second_source_preserving_cut_path_collapsed,
        paired_combined_first_source_preserving_cut_path_start_primitive_kinds,
        paired_combined_second_source_preserving_cut_path_start_primitive_kinds,
        paired_combined_first_source_preserving_cut_path_start_primitive_faces,
        paired_combined_second_source_preserving_cut_path_start_primitive_faces,
        paired_combined_first_source_preserving_meshlib_like_order_rotations,
        paired_combined_second_source_preserving_meshlib_like_order_rotations,
        paired_combined_first_source_preserving_meshlib_like_cut_path_start_primitive_faces,
        paired_combined_second_source_preserving_meshlib_like_cut_path_start_primitive_faces,
        paired_combined_first_source_preserving_meshlib_like_cut_path_collapsed,
        paired_combined_second_source_preserving_meshlib_like_cut_path_collapsed,
        paired_combined_first_source_preserving_meshlib_like_cut_edge_paths,
        paired_combined_second_source_preserving_meshlib_like_cut_edge_paths,
        paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates,
        paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates,
        paired_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates,
        paired_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates,
        paired_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs,
        paired_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs,
        paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidate_runs,
        paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidate_runs,
        paired_combined_first_source_preserving_meshlib_like_replacement_source_faces,
        paired_combined_second_source_preserving_meshlib_like_replacement_source_faces,
        paired_combined_first_source_preserving_meshlib_like_replacement_source_face_counts,
        paired_combined_second_source_preserving_meshlib_like_replacement_source_face_counts,
        paired_combined_first_source_preserving_meshlib_like_replacement_source_face_runs,
        paired_combined_second_source_preserving_meshlib_like_replacement_source_face_runs,
        paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_runs,
        paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_runs,
        paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
        paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
        paired_combined_first_source_preserving_meshlib_like_cut2origin_source_faces,
        paired_combined_second_source_preserving_meshlib_like_cut2origin_source_faces,
        paired_combined_first_source_preserving_meshlib_like_cut2origin_source_face_counts,
        paired_combined_second_source_preserving_meshlib_like_cut2origin_source_face_counts,
        paired_combined_first_source_preserving_meshlib_like_cut2origin_source_face_runs,
        paired_combined_second_source_preserving_meshlib_like_cut2origin_source_face_runs,
        paired_combined_first_source_preserving_meshlib_removed_face_owner_candidates,
        paired_combined_second_source_preserving_meshlib_removed_face_owner_candidates,
        paired_combined_first_source_preserving_meshlib_removed_face_owner_candidate_runs,
        paired_combined_second_source_preserving_meshlib_removed_face_owner_candidate_runs,
        paired_combined_source_preserving_meshlib_removed_face_owner_missing_records,
        paired_combined_duplicate_first_path_edge_occurrences,
        paired_combined_duplicate_second_path_edge_occurrences,
        paired_combined_duplicate_first_path_edge_path_indices,
        paired_combined_duplicate_second_path_edge_path_indices,
        paired_stitch_cut_path_length_mismatches: cut_path_length_mismatches(
            &paired_first.cut_edge_paths,
            &paired_second.cut_edge_paths,
        ),
        paired_stitch_unmatched_first_edges: paired_stitch_plan.unmatched_first_edges.len(),
        paired_stitch_unmatched_second_edges: paired_stitch_plan.unmatched_second_edges.len(),
        paired_duplicate_first_path_edges: duplicate_path_edges(&paired_first.cut_edge_paths),
        paired_duplicate_second_path_edges: duplicate_path_edges(&paired_second.cut_edge_paths),
        paired_duplicate_first_path_edge_occurrences,
        paired_duplicate_second_path_edge_occurrences,
        paired_duplicate_first_path_edge_path_indices: duplicate_path_edge_path_indices(
            &paired_first.cut_edge_paths,
        ),
        paired_duplicate_second_path_edge_path_indices: duplicate_path_edge_path_indices(
            &paired_second.cut_edge_paths,
        ),
        first_skipped_source_faces: first.skipped_source_faces.clone(),
        second_skipped_source_faces: second.skipped_source_faces.clone(),
        accepted,
    };
    let paired_candidate = paired_candidate_is_clean.then_some(ExactCoplanarCutCandidate {
        first: paired_first,
        second: paired_second,
        first_shadow_repair_paths,
        second_shadow_repair_paths,
    });
    Ok(Some(CoplanarContourCutTrialResult {
        first,
        second,
        summary,
        paired_candidate,
    }))
}

fn paired_contour_edges(contours: &[super::super::exact_one_mesh::ExactOneMeshContour]) -> usize {
    contours
        .iter()
        .map(|contour| contour.intersections.len())
        .sum()
}

fn append_one_mesh_contours(target: &mut ExactOneMeshContours, mut source: ExactOneMeshContours) {
    target.first.append(&mut source.first);
    target.second.append(&mut source.second);
    target
        .coordinates_in_first_space
        .append(&mut source.coordinates_in_first_space);
}

fn cut_path_length_mismatches(
    first_paths: &[Vec<[usize; 2]>],
    second_paths: &[Vec<[usize; 2]>],
) -> usize {
    let shared_mismatches = first_paths
        .iter()
        .zip(second_paths)
        .filter(|(first, second)| first.len() != second.len())
        .count();
    shared_mismatches + first_paths.len().abs_diff(second_paths.len())
}

fn cut_path_lengths(paths: &[Vec<[usize; 2]>]) -> Vec<usize> {
    paths.iter().map(Vec::len).collect()
}

fn cut_path_source_faces(cut: &ExactCutMeshResult) -> Vec<Vec<usize>> {
    cut.cut_edge_path_source_faces
        .iter()
        .map(|source_faces| source_faces.iter().flatten().copied().collect())
        .collect()
}

fn collapsed_cut_path_source_faces(cut: &ExactCutMeshResult) -> Vec<Vec<usize>> {
    cut.collapsed_cut_segment_path_source_faces
        .iter()
        .map(|source_faces| source_faces.iter().flatten().copied().collect())
        .collect()
}

fn coplanar_trial_is_no_regression(
    stable: &ExactMeshPairCutMeshes,
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
) -> bool {
    first.skipped_source_faces.is_empty()
        && second.skipped_source_faces.is_empty()
        && first.cut_edges.len() >= stable.first.cut_edges.len()
        && second.cut_edges.len() >= stable.second.cut_edges.len()
}
