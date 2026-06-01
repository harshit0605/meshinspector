use super::ExactHalfEdgeId;
use super::ExactMeshlibNearStitchRingDiagnostic;
use super::ExactMeshlibNearStitchTargetSnapshot;
use crate::spatial::exact_splice_apply::ExactMeshlibCopiedSourceEdgeLookupDiagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactMeshlibNearStitchCandidate {
    pub(super) edge: ExactHalfEdgeId,
    pub(super) source: &'static str,
    pub(super) key: Option<usize>,
    pub(super) source_edge: Option<[usize; 2]>,
    pub(super) target_snapshot: Option<ExactMeshlibNearStitchTargetSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactMeshlibNearStitchSourceCandidate {
    pub(super) edge: ExactHalfEdgeId,
    pub(super) source_edge: Option<[usize; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibNearStitchLinkedEdgeDiagnostic {
    pub edge_id: usize,
    pub origin: Option<usize>,
    pub left: Option<usize>,
    pub right: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactMeshlibNearStitchCandidateFailure {
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
    pub previous_next_edge: ExactMeshlibNearStitchLinkedEdgeDiagnostic,
    pub next_prev_edge: ExactMeshlibNearStitchLinkedEdgeDiagnostic,
    pub previous_left_ring: ExactMeshlibNearStitchRingDiagnostic,
    pub next_right_ring: ExactMeshlibNearStitchRingDiagnostic,
    pub previous_target_snapshot: Option<ExactMeshlibNearStitchTargetSnapshot>,
    pub next_target_snapshot: Option<ExactMeshlibNearStitchTargetSnapshot>,
    pub captured_open_target_reopened_previous: bool,
    pub captured_open_target_reopened_next: bool,
    pub captured_open_target_retry_error: Option<&'static str>,
    pub error: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactMeshlibNearStitchCandidateDiagnostics {
    pub attempt: &'static str,
    pub previous_candidates: usize,
    pub next_candidates: usize,
    pub failures: Vec<ExactMeshlibNearStitchCandidateFailure>,
    pub fallback_from: Option<ExactMeshlibNearStitchCandidateAttemptDiagnostics>,
    pub previous_source_lookup: Option<ExactMeshlibNearStitchSourceLookupDiagnostics>,
    pub next_source_lookup: Option<ExactMeshlibNearStitchSourceLookupDiagnostics>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibNearStitchCandidateAttemptDiagnostics {
    pub attempt: &'static str,
    pub error: &'static str,
    pub previous_candidates: usize,
    pub next_candidates: usize,
    pub failure_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibNearStitchSourceLookupDiagnostics {
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
    pub copied_source_edge: Option<ExactMeshlibCopiedSourceEdgeLookupDiagnostic>,
}

impl ExactMeshlibNearStitchCandidateDiagnostics {
    pub(super) fn fallback_attempt_summary(
        &self,
        error: &'static str,
    ) -> ExactMeshlibNearStitchCandidateAttemptDiagnostics {
        ExactMeshlibNearStitchCandidateAttemptDiagnostics {
            attempt: self.attempt,
            error,
            previous_candidates: self.previous_candidates,
            next_candidates: self.next_candidates,
            failure_count: self.failures.len(),
        }
    }
}

pub(super) fn labeled_near_stitch_candidates(
    edges: Vec<ExactHalfEdgeId>,
    source: &'static str,
) -> Vec<ExactMeshlibNearStitchCandidate> {
    edges
        .into_iter()
        .map(|edge| ExactMeshlibNearStitchCandidate {
            edge,
            source,
            key: None,
            source_edge: None,
            target_snapshot: None,
        })
        .collect()
}

pub(super) fn extend_unique_labeled_candidates(
    candidates: &mut Vec<ExactMeshlibNearStitchCandidate>,
    incoming: Vec<ExactHalfEdgeId>,
    source: &'static str,
) {
    for edge in incoming {
        if candidates.iter().any(|candidate| candidate.edge == edge) {
            continue;
        }
        candidates.push(ExactMeshlibNearStitchCandidate {
            edge,
            source,
            key: None,
            source_edge: None,
            target_snapshot: None,
        });
    }
}

pub(super) fn push_unique_labeled_target_candidate(
    candidates: &mut Vec<ExactMeshlibNearStitchCandidate>,
    edge: ExactHalfEdgeId,
    source: &'static str,
    target_snapshot: Option<ExactMeshlibNearStitchTargetSnapshot>,
) {
    if candidates.iter().any(|candidate| candidate.edge == edge) {
        return;
    }
    candidates.push(ExactMeshlibNearStitchCandidate {
        edge,
        source,
        key: None,
        source_edge: None,
        target_snapshot,
    });
}

pub(super) fn push_unique_labeled_candidate(
    candidates: &mut Vec<ExactMeshlibNearStitchCandidate>,
    edge: ExactHalfEdgeId,
    source: &'static str,
    key: Option<usize>,
    source_edge: Option<[usize; 2]>,
) {
    if candidates.iter().any(|candidate| candidate.edge == edge) {
        return;
    }
    candidates.push(ExactMeshlibNearStitchCandidate {
        edge,
        source,
        key,
        source_edge,
        target_snapshot: None,
    });
}
