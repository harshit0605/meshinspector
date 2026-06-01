use super::super::super::exact_meshlib_near_stitch::ExactMeshlibNearStitchEndpoint;
use super::super::super::exact_meshlib_rewrite_apply::ExactMeshlibRecordRewriteApplyPlan;
use super::super::super::exact_splice_apply::{
    ExactMeshlibCopiedSourceEdgeLookupDiagnostic as ExactCopiedSourceEdgeLookupDiagnostic,
    ExactMeshlibNearStitchLinkedEdgeDiagnostic as ExactLinkedEdgeDiagnostic,
    ExactMeshlibNearStitchRingDiagnostic,
    ExactMeshlibNearStitchSourceLookupDiagnostics as ExactSourceLookupDiagnostics,
    ExactMeshlibNearStitchTargetSnapshot,
};
use super::operand_label;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibNearStitchCandidateFailureDiagnostic {
    pub previous_edge_id: usize,
    pub next_edge_id: usize,
    pub previous_candidate_source: &'static str,
    pub next_candidate_source: &'static str,
    pub previous_candidate_key: Option<usize>,
    pub next_candidate_key: Option<usize>,
    pub previous_candidate_source_edge: Option<[usize; 2]>,
    pub next_candidate_source_edge: Option<[usize; 2]>,
    pub previous_origin: Option<usize>,
    pub next_origin: Option<usize>,
    pub previous_left: Option<usize>,
    pub previous_right: Option<usize>,
    pub next_left: Option<usize>,
    pub next_right: Option<usize>,
    pub previous_next_edge_id: usize,
    pub next_prev_edge_id: usize,
    pub previous_next_edge: MeshlibNearStitchLinkedEdgeDiagnostic,
    pub next_prev_edge: MeshlibNearStitchLinkedEdgeDiagnostic,
    pub previous_left_ring: MeshlibNearStitchRingDiagnostic,
    pub next_right_ring: MeshlibNearStitchRingDiagnostic,
    pub previous_target_snapshot: Option<MeshlibNearStitchTargetSnapshotDiagnostic>,
    pub next_target_snapshot: Option<MeshlibNearStitchTargetSnapshotDiagnostic>,
    pub captured_open_target_reopened_previous: bool,
    pub captured_open_target_reopened_next: bool,
    pub captured_open_target_retry_error: Option<&'static str>,
    pub error: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibNearStitchLinkedEdgeDiagnostic {
    pub edge_id: usize,
    pub origin: Option<usize>,
    pub left: Option<usize>,
    pub right: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibNearStitchTargetSnapshotDiagnostic {
    pub edge_id: usize,
    pub origin: Option<usize>,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub next_edge_id: usize,
    pub prev_edge_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibNearStitchRingDiagnostic {
    pub edge_ids: Vec<usize>,
    pub origins: Vec<Option<usize>>,
    pub left_faces: Vec<Option<usize>>,
    pub error: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibNearStitchCandidateDiagnostics {
    pub attempt: &'static str,
    pub previous_candidates: usize,
    pub next_candidates: usize,
    pub failures: Vec<MeshlibNearStitchCandidateFailureDiagnostic>,
    pub fallback_from: Option<MeshlibNearStitchCandidateAttemptDiagnostic>,
    pub previous_source_lookup: Option<MeshlibNearStitchSourceLookupDiagnostic>,
    pub next_source_lookup: Option<MeshlibNearStitchSourceLookupDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibNearStitchCandidateAttemptDiagnostic {
    pub attempt: &'static str,
    pub error: &'static str,
    pub previous_candidates: usize,
    pub next_candidates: usize,
    pub failure_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibNearStitchSourceLookupDiagnostic {
    pub requested_halfedge: Option<usize>,
    pub requested_key_face: Option<usize>,
    pub requested_key_edge: Option<[usize; 2]>,
    pub requested_source_edge: Option<[usize; 2]>,
    pub fallback_edge: [usize; 2],
    pub exact_key_candidates: usize,
    pub same_edge_key_candidates: usize,
    pub halfedge_candidates: usize,
    pub source_edge_candidates: usize,
    pub topology_candidates: usize,
    pub total_candidates: usize,
    pub copied_source_edge: Option<MeshlibCopiedSourceEdgeLookupDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibCopiedSourceEdgeLookupDiagnostic {
    pub status: &'static str,
    pub matched_source_edge: Option<[usize; 2]>,
    pub source_halfedge: Option<usize>,
    pub source_origin: Option<usize>,
    pub source_left: Option<usize>,
    pub source_right: Option<usize>,
    pub source_left_mapped_face: Option<usize>,
    pub source_right_mapped_face: Option<usize>,
    pub source_next_halfedge: Option<usize>,
    pub source_prev_halfedge: Option<usize>,
    pub output_edge_id: Option<usize>,
    pub output_origin: Option<usize>,
    pub output_left: Option<usize>,
    pub output_right: Option<usize>,
    pub output_next_edge_id: Option<usize>,
    pub output_prev_edge_id: Option<usize>,
    pub matching_statuses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibNearStitchFailureDiagnostic {
    pub stitch_pair_index: Option<usize>,
    pub endpoint: Option<&'static str>,
    pub source_operand: Option<&'static str>,
    pub previous_source_halfedge: Option<usize>,
    pub next_source_halfedge: Option<usize>,
    pub previous_source_halfedge_key_face: Option<usize>,
    pub previous_source_halfedge_key_edge: Option<[usize; 2]>,
    pub next_source_halfedge_key_face: Option<usize>,
    pub next_source_halfedge_key_edge: Option<[usize; 2]>,
    pub previous_source_edge: Option<[usize; 2]>,
    pub next_source_edge: Option<[usize; 2]>,
    pub previous_edge: [usize; 2],
    pub next_edge: [usize; 2],
    pub strict_source_identity: bool,
    pub error: &'static str,
    pub candidate_diagnostics: Option<MeshlibNearStitchCandidateDiagnostics>,
}

pub(in crate::spatial::exact_boolean_diagnostics) fn near_stitch_failure_details(
    plan: &ExactMeshlibRecordRewriteApplyPlan,
) -> Vec<MeshlibNearStitchFailureDiagnostic> {
    plan.near_stitch_update_entries
        .iter()
        .filter_map(|entry| {
            entry.error.map(|error| {
                let command = entry.command;
                MeshlibNearStitchFailureDiagnostic {
                    stitch_pair_index: command.stitch_pair_index,
                    endpoint: command.endpoint.map(near_stitch_endpoint_label),
                    source_operand: command.source_operand.map(operand_label),
                    previous_source_halfedge: command.previous_source_halfedge,
                    next_source_halfedge: command.next_source_halfedge,
                    previous_source_halfedge_key_face: command
                        .previous_source_halfedge_key
                        .map(|key| key.face),
                    previous_source_halfedge_key_edge: command
                        .previous_source_halfedge_key
                        .map(|key| key.edge),
                    next_source_halfedge_key_face: command
                        .next_source_halfedge_key
                        .map(|key| key.face),
                    next_source_halfedge_key_edge: command
                        .next_source_halfedge_key
                        .map(|key| key.edge),
                    previous_source_edge: command.previous_source_edge,
                    next_source_edge: command.next_source_edge,
                    previous_edge: command.previous_edge,
                    next_edge: command.next_edge,
                    strict_source_identity: command.strict_source_identity,
                    error,
                    candidate_diagnostics: entry.candidate_diagnostics.as_ref().map(
                        |diagnostics| MeshlibNearStitchCandidateDiagnostics {
                            attempt: diagnostics.attempt,
                            previous_candidates: diagnostics.previous_candidates,
                            next_candidates: diagnostics.next_candidates,
                            failures: diagnostics
                                .failures
                                .iter()
                                .map(|failure| MeshlibNearStitchCandidateFailureDiagnostic {
                                    previous_edge_id: failure.previous_edge_id,
                                    next_edge_id: failure.next_edge_id,
                                    previous_candidate_source: failure.previous_candidate_source,
                                    next_candidate_source: failure.next_candidate_source,
                                    previous_candidate_key: failure.previous_candidate_key,
                                    next_candidate_key: failure.next_candidate_key,
                                    previous_candidate_source_edge: failure
                                        .previous_candidate_source_edge,
                                    next_candidate_source_edge: failure.next_candidate_source_edge,
                                    previous_origin: failure.previous_origin,
                                    next_origin: failure.next_origin,
                                    previous_left: failure.previous_left,
                                    previous_right: failure.previous_right,
                                    next_left: failure.next_left,
                                    next_right: failure.next_right,
                                    previous_next_edge_id: failure.previous_next_edge_id,
                                    next_prev_edge_id: failure.next_prev_edge_id,
                                    previous_next_edge: near_stitch_linked_edge_diagnostic(
                                        failure.previous_next_edge,
                                    ),
                                    next_prev_edge: near_stitch_linked_edge_diagnostic(
                                        failure.next_prev_edge,
                                    ),
                                    previous_left_ring: near_stitch_ring_diagnostic(
                                        &failure.previous_left_ring,
                                    ),
                                    next_right_ring: near_stitch_ring_diagnostic(
                                        &failure.next_right_ring,
                                    ),
                                    previous_target_snapshot: failure
                                        .previous_target_snapshot
                                        .map(near_stitch_target_snapshot_diagnostic),
                                    next_target_snapshot: failure
                                        .next_target_snapshot
                                        .map(near_stitch_target_snapshot_diagnostic),
                                    captured_open_target_reopened_previous: failure
                                        .captured_open_target_reopened_previous,
                                    captured_open_target_reopened_next: failure
                                        .captured_open_target_reopened_next,
                                    captured_open_target_retry_error: failure
                                        .captured_open_target_retry_error,
                                    error: failure.error,
                                })
                                .collect(),
                            fallback_from: diagnostics.fallback_from.map(|attempt| {
                                MeshlibNearStitchCandidateAttemptDiagnostic {
                                    attempt: attempt.attempt,
                                    error: attempt.error,
                                    previous_candidates: attempt.previous_candidates,
                                    next_candidates: attempt.next_candidates,
                                    failure_count: attempt.failure_count,
                                }
                            }),
                            previous_source_lookup: diagnostics
                                .previous_source_lookup
                                .map(near_stitch_source_lookup_diagnostic),
                            next_source_lookup: diagnostics
                                .next_source_lookup
                                .map(near_stitch_source_lookup_diagnostic),
                        },
                    ),
                }
            })
        })
        .collect()
}

fn near_stitch_source_lookup_diagnostic(
    lookup: ExactSourceLookupDiagnostics,
) -> MeshlibNearStitchSourceLookupDiagnostic {
    MeshlibNearStitchSourceLookupDiagnostic {
        requested_halfedge: lookup.requested_halfedge,
        requested_key_face: lookup.requested_key_face,
        requested_key_edge: lookup.requested_key_edge,
        requested_source_edge: lookup.requested_source_edge,
        fallback_edge: lookup.fallback_edge,
        exact_key_candidates: lookup.exact_key_candidates,
        same_edge_key_candidates: lookup.same_edge_key_candidates,
        halfedge_candidates: lookup.halfedge_candidates,
        source_edge_candidates: lookup.source_edge_candidates,
        topology_candidates: lookup.topology_candidates,
        total_candidates: lookup.total_candidates,
        copied_source_edge: lookup
            .copied_source_edge
            .map(copied_source_edge_lookup_diagnostic),
    }
}

fn copied_source_edge_lookup_diagnostic(
    lookup: ExactCopiedSourceEdgeLookupDiagnostic,
) -> MeshlibCopiedSourceEdgeLookupDiagnostic {
    MeshlibCopiedSourceEdgeLookupDiagnostic {
        status: lookup.status.label(),
        matched_source_edge: lookup.matched_source_edge,
        source_halfedge: lookup.source_halfedge,
        source_origin: lookup.source_origin,
        source_left: lookup.source_left,
        source_right: lookup.source_right,
        source_left_mapped_face: lookup.source_left_mapped_face,
        source_right_mapped_face: lookup.source_right_mapped_face,
        source_next_halfedge: lookup.source_next_halfedge,
        source_prev_halfedge: lookup.source_prev_halfedge,
        output_edge_id: lookup.output_edge_id,
        output_origin: lookup.output_origin,
        output_left: lookup.output_left,
        output_right: lookup.output_right,
        output_next_edge_id: lookup.output_next_edge_id,
        output_prev_edge_id: lookup.output_prev_edge_id,
        matching_statuses: lookup.matching_statuses,
    }
}

fn near_stitch_linked_edge_diagnostic(
    detail: ExactLinkedEdgeDiagnostic,
) -> MeshlibNearStitchLinkedEdgeDiagnostic {
    MeshlibNearStitchLinkedEdgeDiagnostic {
        edge_id: detail.edge_id,
        origin: detail.origin,
        left: detail.left,
        right: detail.right,
    }
}

fn near_stitch_ring_diagnostic(
    detail: &ExactMeshlibNearStitchRingDiagnostic,
) -> MeshlibNearStitchRingDiagnostic {
    MeshlibNearStitchRingDiagnostic {
        edge_ids: detail.edge_ids.clone(),
        origins: detail.origins.clone(),
        left_faces: detail.left_faces.clone(),
        error: detail.error,
    }
}

fn near_stitch_target_snapshot_diagnostic(
    detail: ExactMeshlibNearStitchTargetSnapshot,
) -> MeshlibNearStitchTargetSnapshotDiagnostic {
    MeshlibNearStitchTargetSnapshotDiagnostic {
        edge_id: detail.edge_id,
        origin: detail.origin,
        left: detail.left,
        right: detail.right,
        next_edge_id: detail.next_edge_id,
        prev_edge_id: detail.prev_edge_id,
    }
}

fn near_stitch_endpoint_label(endpoint: ExactMeshlibNearStitchEndpoint) -> &'static str {
    match endpoint {
        ExactMeshlibNearStitchEndpoint::Start => "start",
        ExactMeshlibNearStitchEndpoint::End => "end",
    }
}
