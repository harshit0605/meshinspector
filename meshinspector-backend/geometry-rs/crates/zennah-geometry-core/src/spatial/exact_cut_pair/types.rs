use super::super::exact_cut_apply::ExactCutMeshResult;

#[derive(Debug, Clone, PartialEq)]
pub struct ExactMeshPairCutMeshes {
    pub first: ExactCutMeshResult,
    pub second: ExactCutMeshResult,
    pub(in crate::spatial) coplanar_cut_trial: Option<ExactCoplanarContourCutTrial>,
    pub(in crate::spatial) paired_coplanar_candidate: Option<ExactCoplanarCutCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::spatial) struct ExactCoplanarContourCutTrial {
    pub contours: usize,
    pub contour_edges: usize,
    pub first_cut_edges: usize,
    pub second_cut_edges: usize,
    pub paired_contours: usize,
    pub paired_contour_edges: usize,
    pub paired_first_cut_edges: usize,
    pub paired_second_cut_edges: usize,
    pub paired_combined_first_cut_path_lengths: Vec<usize>,
    pub paired_combined_second_cut_path_lengths: Vec<usize>,
    pub paired_combined_first_cut_path_source_faces: Vec<Vec<usize>>,
    pub paired_combined_second_cut_path_source_faces: Vec<Vec<usize>>,
    pub paired_combined_first_cut_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_cut_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_collapsed_cut_path_lengths: Vec<usize>,
    pub paired_combined_second_collapsed_cut_path_lengths: Vec<usize>,
    pub paired_combined_first_collapsed_cut_path_source_faces: Vec<Vec<usize>>,
    pub paired_combined_second_collapsed_cut_path_source_faces: Vec<Vec<usize>>,
    pub paired_combined_first_collapsed_cut_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_collapsed_cut_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_cut_path_lengths: Vec<usize>,
    pub paired_combined_second_source_preserving_cut_path_lengths: Vec<usize>,
    pub paired_combined_first_source_preserving_cut_path_source_faces: Vec<Vec<usize>>,
    pub paired_combined_second_source_preserving_cut_path_source_faces: Vec<Vec<usize>>,
    pub paired_combined_first_source_preserving_cut_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_cut_path_source_face_runs: Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_cut_path_collapsed: Vec<Vec<bool>>,
    pub paired_combined_second_source_preserving_cut_path_collapsed: Vec<Vec<bool>>,
    pub paired_combined_first_source_preserving_cut_path_start_primitive_kinds: Vec<Vec<usize>>,
    pub paired_combined_second_source_preserving_cut_path_start_primitive_kinds: Vec<Vec<usize>>,
    pub paired_combined_first_source_preserving_cut_path_start_primitive_faces: Vec<Vec<i64>>,
    pub paired_combined_second_source_preserving_cut_path_start_primitive_faces: Vec<Vec<i64>>,
    pub paired_combined_first_source_preserving_meshlib_like_order_rotations: Vec<usize>,
    pub paired_combined_second_source_preserving_meshlib_like_order_rotations: Vec<usize>,
    pub paired_combined_first_source_preserving_meshlib_like_cut_path_start_primitive_faces:
        Vec<Vec<i64>>,
    pub paired_combined_second_source_preserving_meshlib_like_cut_path_start_primitive_faces:
        Vec<Vec<i64>>,
    pub paired_combined_first_source_preserving_meshlib_like_cut_path_collapsed: Vec<Vec<bool>>,
    pub paired_combined_second_source_preserving_meshlib_like_cut_path_collapsed: Vec<Vec<bool>>,
    pub paired_combined_first_source_preserving_meshlib_like_cut_edge_paths: Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_like_cut_edge_paths: Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates:
        Vec<Vec<usize>>,
    pub paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates:
        Vec<Vec<usize>>,
    pub paired_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates:
        Vec<Vec<usize>>,
    pub paired_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates:
        Vec<Vec<usize>>,
    pub paired_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidate_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidate_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_meshlib_like_replacement_source_faces:
        Vec<Vec<usize>>,
    pub paired_combined_second_source_preserving_meshlib_like_replacement_source_faces:
        Vec<Vec<usize>>,
    pub paired_combined_first_source_preserving_meshlib_like_replacement_source_face_counts:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_like_replacement_source_face_counts:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_meshlib_like_replacement_source_face_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_like_replacement_source_face_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_runs:
        Vec<Vec<[usize; 4]>>,
    pub paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_runs:
        Vec<Vec<[usize; 4]>>,
    pub paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs:
        Vec<Vec<[usize; 8]>>,
    pub paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs:
        Vec<Vec<[usize; 8]>>,
    pub paired_combined_first_source_preserving_meshlib_like_cut2origin_source_faces:
        Vec<Vec<usize>>,
    pub paired_combined_second_source_preserving_meshlib_like_cut2origin_source_faces:
        Vec<Vec<usize>>,
    pub paired_combined_first_source_preserving_meshlib_like_cut2origin_source_face_counts:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_like_cut2origin_source_face_counts:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_meshlib_like_cut2origin_source_face_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_like_cut2origin_source_face_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_first_source_preserving_meshlib_removed_face_owner_candidates:
        Vec<Vec<usize>>,
    pub paired_combined_second_source_preserving_meshlib_removed_face_owner_candidates:
        Vec<Vec<usize>>,
    pub paired_combined_first_source_preserving_meshlib_removed_face_owner_candidate_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_second_source_preserving_meshlib_removed_face_owner_candidate_runs:
        Vec<Vec<[usize; 2]>>,
    pub paired_combined_source_preserving_meshlib_removed_face_owner_missing_records: [usize; 2],
    pub paired_combined_duplicate_first_path_edge_occurrences: usize,
    pub paired_combined_duplicate_second_path_edge_occurrences: usize,
    pub paired_combined_duplicate_first_path_edge_path_indices: Vec<Vec<usize>>,
    pub paired_combined_duplicate_second_path_edge_path_indices: Vec<Vec<usize>>,
    pub paired_stitch_cut_path_length_mismatches: usize,
    pub paired_stitch_unmatched_first_edges: usize,
    pub paired_stitch_unmatched_second_edges: usize,
    pub paired_duplicate_first_path_edges: usize,
    pub paired_duplicate_second_path_edges: usize,
    pub paired_duplicate_first_path_edge_occurrences: usize,
    pub paired_duplicate_second_path_edge_occurrences: usize,
    pub paired_duplicate_first_path_edge_path_indices: Vec<Vec<usize>>,
    pub paired_duplicate_second_path_edge_path_indices: Vec<Vec<usize>>,
    pub first_skipped_source_faces: Vec<usize>,
    pub second_skipped_source_faces: Vec<usize>,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::spatial) struct ExactCoplanarCutCandidate {
    pub first: ExactCutMeshResult,
    pub second: ExactCutMeshResult,
    pub first_shadow_repair_paths: Vec<ExactCutShadowRepairPath>,
    pub second_shadow_repair_paths: Vec<ExactCutShadowRepairPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::spatial) struct ExactCutShadowRepairPath {
    pub path: Vec<[usize; 2]>,
    pub source_faces: Vec<Option<usize>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourcePreservingCutSegment {
    pub(super) edge: Option<[usize; 2]>,
    pub(super) source_face: Option<usize>,
    pub(super) collapsed: bool,
    pub(super) start_coordinate: [f64; 3],
    pub(super) start_primitive_kind: usize,
    pub(super) start_primitive_face: Option<usize>,
    pub(super) start_primitive_edge: Option<[usize; 2]>,
}

pub(super) struct CoplanarContourCutTrialResult {
    pub(super) first: ExactCutMeshResult,
    pub(super) second: ExactCutMeshResult,
    pub(super) summary: ExactCoplanarContourCutTrial,
    pub(super) paired_candidate: Option<ExactCoplanarCutCandidate>,
}
