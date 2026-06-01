use super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::super::super::exact_meshlib_near_stitch::{
    ExactMeshlibNearStitchEdgeUpdateCommand, ExactMeshlibNearStitchEndpoint,
};
use super::near_stitch_diagnostics::ExactMeshlibNearStitchRingDiagnostic;
use super::OutputFaceTopology;

mod candidates;
use candidates::{
    extend_unique_labeled_candidates, labeled_near_stitch_candidates,
    push_unique_labeled_target_candidate, ExactMeshlibNearStitchCandidate,
    ExactMeshlibNearStitchCandidateFailure,
};
pub(crate) use candidates::{
    ExactMeshlibNearStitchCandidateDiagnostics, ExactMeshlibNearStitchLinkedEdgeDiagnostic,
    ExactMeshlibNearStitchSourceLookupDiagnostics,
};
mod source_candidates;
mod target_snapshot;
pub(crate) use target_snapshot::ExactMeshlibNearStitchTargetSnapshot;

#[derive(Clone, Copy)]
struct ExactMeshlibNearStitchCandidateAttemptContext {
    missing_previous_error: &'static str,
    missing_next_error: &'static str,
    attempt: &'static str,
    previous_source_lookup: Option<ExactMeshlibNearStitchSourceLookupDiagnostics>,
    next_source_lookup: Option<ExactMeshlibNearStitchSourceLookupDiagnostics>,
}

#[derive(Clone, Copy, Default)]
struct ExactMeshlibCapturedOpenTargetRetry {
    reopened_previous: bool,
    reopened_next: bool,
    error: Option<&'static str>,
}

impl OutputFaceTopology {
    #[cfg(test)]
    pub(crate) fn apply_meshlib_near_stitch_edge_update(
        &mut self,
        previous_edge: [usize; 2],
        next_edge: [usize; 2],
    ) -> Result<(), &'static str> {
        self.apply_meshlib_near_stitch_edge_update_attempt(previous_edge, next_edge, "vertex-pair")
    }

    fn apply_meshlib_near_stitch_edge_update_attempt(
        &mut self,
        previous_edge: [usize; 2],
        next_edge: [usize; 2],
        attempt: &'static str,
    ) -> Result<(), &'static str> {
        self.meshlib_last_near_stitch_candidate_diagnostics = None;
        let previous_candidates = self.topology_edge_candidates_for_directed_edge(previous_edge);
        let next_candidates = self.topology_edge_candidates_for_directed_edge(next_edge);
        let previous_candidates =
            labeled_near_stitch_candidates(previous_candidates, "topology-fallback");
        let next_candidates = labeled_near_stitch_candidates(next_candidates, "topology-fallback");
        self.apply_meshlib_near_stitch_labeled_candidates(
            &previous_candidates,
            &next_candidates,
            ExactMeshlibNearStitchCandidateAttemptContext {
                missing_previous_error: "missing MeshLib near stitch previous edge",
                missing_next_error: "missing MeshLib near stitch next edge",
                attempt,
                previous_source_lookup: None,
                next_source_lookup: None,
            },
        )
    }

    pub(crate) fn apply_meshlib_near_stitch_edge_update_command(
        &mut self,
        command: &ExactMeshlibNearStitchEdgeUpdateCommand,
    ) -> Result<(), &'static str> {
        self.meshlib_last_near_stitch_candidate_diagnostics = None;
        let id_result = match (command.stitch_pair_index, command.endpoint) {
            (Some(stitch_pair_index), Some(ExactMeshlibNearStitchEndpoint::Start)) => {
                let previous_candidates = self.meshlib_target_near_stitch_candidates(
                    stitch_pair_index,
                    ExactMeshlibNearStitchEndpoint::Start,
                    command.previous_edge,
                );
                let next_lookup = self.meshlib_near_stitch_source_candidates(
                    command.source_operand,
                    command.next_source_halfedge,
                    command.next_source_halfedge_key,
                    command.next_source_edge,
                    command.next_edge,
                );
                self.apply_meshlib_near_stitch_labeled_candidates(
                    &previous_candidates,
                    &next_lookup.candidates,
                    ExactMeshlibNearStitchCandidateAttemptContext {
                        missing_previous_error: "missing MeshLib near stitch target edge",
                        missing_next_error: "missing MeshLib near stitch next edge",
                        attempt: "identity-target-source",
                        previous_source_lookup: None,
                        next_source_lookup: Some(next_lookup.diagnostics),
                    },
                )
            }
            (Some(stitch_pair_index), Some(ExactMeshlibNearStitchEndpoint::End)) => {
                let next_candidates = self.meshlib_target_near_stitch_candidates(
                    stitch_pair_index,
                    ExactMeshlibNearStitchEndpoint::End,
                    command.next_edge,
                );
                let previous_lookup = self.meshlib_near_stitch_source_candidates(
                    command.source_operand,
                    command.previous_source_halfedge,
                    command.previous_source_halfedge_key,
                    command.previous_source_edge,
                    command.previous_edge,
                );
                self.apply_meshlib_near_stitch_labeled_candidates(
                    &previous_lookup.candidates,
                    &next_candidates,
                    ExactMeshlibNearStitchCandidateAttemptContext {
                        missing_previous_error: "missing MeshLib near stitch previous edge",
                        missing_next_error: "missing MeshLib near stitch target edge",
                        attempt: "identity-target-source",
                        previous_source_lookup: Some(previous_lookup.diagnostics),
                        next_source_lookup: None,
                    },
                )
            }
            _ => Err("missing MeshLib near stitch target edge"),
        };
        match id_result {
            Ok(()) => Ok(()),
            Err(error) => {
                let identity_diagnostics = self.take_meshlib_near_stitch_candidate_diagnostics();
                if !command.strict_source_identity
                    || should_fallback_to_vertex_pair_near_stitch(error)
                {
                    let fallback_result = self.apply_meshlib_near_stitch_edge_update_attempt(
                        command.previous_edge,
                        command.next_edge,
                        "vertex-pair-fallback",
                    );
                    if fallback_result.is_err() {
                        self.attach_meshlib_near_stitch_fallback_attempt(
                            identity_diagnostics,
                            error,
                        );
                    }
                    fallback_result
                } else {
                    self.meshlib_last_near_stitch_candidate_diagnostics = identity_diagnostics;
                    Err(error)
                }
            }
        }
    }

    fn attach_meshlib_near_stitch_fallback_attempt(
        &mut self,
        identity_diagnostics: Option<ExactMeshlibNearStitchCandidateDiagnostics>,
        identity_error: &'static str,
    ) {
        let Some(identity_diagnostics) = identity_diagnostics else {
            return;
        };
        let identity_attempt = identity_diagnostics.fallback_attempt_summary(identity_error);
        if let Some(fallback_diagnostics) =
            self.meshlib_last_near_stitch_candidate_diagnostics.as_mut()
        {
            fallback_diagnostics.previous_source_lookup =
                identity_diagnostics.previous_source_lookup;
            fallback_diagnostics.next_source_lookup = identity_diagnostics.next_source_lookup;
            fallback_diagnostics.fallback_from = Some(identity_attempt);
        } else {
            self.meshlib_last_near_stitch_candidate_diagnostics = Some(identity_diagnostics);
        }
    }

    pub(crate) fn take_meshlib_near_stitch_candidate_diagnostics(
        &mut self,
    ) -> Option<ExactMeshlibNearStitchCandidateDiagnostics> {
        self.meshlib_last_near_stitch_candidate_diagnostics.take()
    }

    pub(super) fn register_meshlib_near_stitch_target_edges(
        &mut self,
        stitch_pair_index: usize,
        target: ExactHalfEdgeId,
    ) {
        self.register_meshlib_near_stitch_target_edge_candidates(stitch_pair_index, [target]);
    }

    pub(super) fn register_meshlib_near_stitch_target_edge_candidates(
        &mut self,
        stitch_pair_index: usize,
        targets: impl IntoIterator<Item = ExactHalfEdgeId>,
    ) {
        for target in targets {
            let start = self.topology.prev(ExactHalfEdgeTopology::sym(target));
            let end = self.topology.next(target);
            self.capture_meshlib_near_stitch_target_snapshot(
                stitch_pair_index,
                ExactMeshlibNearStitchEndpoint::Start,
                start,
            );
            push_unique_target_edge(
                self.meshlib_near_stitch_target_edges
                    .entry((stitch_pair_index, ExactMeshlibNearStitchEndpoint::Start))
                    .or_default(),
                start,
            );
            self.capture_meshlib_near_stitch_target_snapshot(
                stitch_pair_index,
                ExactMeshlibNearStitchEndpoint::End,
                end,
            );
            push_unique_target_edge(
                self.meshlib_near_stitch_target_edges
                    .entry((stitch_pair_index, ExactMeshlibNearStitchEndpoint::End))
                    .or_default(),
                end,
            );
        }
    }

    #[cfg(test)]
    pub(super) fn meshlib_near_stitch_target_edge_count(
        &self,
        stitch_pair_index: usize,
        endpoint: ExactMeshlibNearStitchEndpoint,
    ) -> usize {
        self.meshlib_near_stitch_target_edges
            .get(&(stitch_pair_index, endpoint))
            .map(Vec::len)
            .unwrap_or_default()
    }

    fn meshlib_target_near_stitch_candidates(
        &self,
        stitch_pair_index: usize,
        endpoint: ExactMeshlibNearStitchEndpoint,
        fallback_edge: [usize; 2],
    ) -> Vec<ExactMeshlibNearStitchCandidate> {
        let mut candidates = Vec::new();
        if let Some(targets) = self
            .meshlib_near_stitch_target_edges
            .get(&(stitch_pair_index, endpoint))
        {
            for target in targets {
                let snapshot =
                    self.meshlib_near_stitch_target_snapshot(stitch_pair_index, endpoint, *target);
                push_unique_labeled_target_candidate(
                    &mut candidates,
                    *target,
                    "target-registered",
                    snapshot,
                );
            }
        }
        extend_unique_labeled_candidates(
            &mut candidates,
            self.topology_face_edge_candidates_for_directed_edge(fallback_edge),
            "target-face-fallback",
        );
        extend_unique_labeled_candidates(
            &mut candidates,
            self.topology_edge_candidates_for_directed_edge(fallback_edge),
            "target-topology-fallback",
        );
        candidates
    }

    #[cfg(test)]
    fn apply_meshlib_near_stitch_candidates(
        &mut self,
        previous_candidates: &[ExactHalfEdgeId],
        next_candidates: &[ExactHalfEdgeId],
        missing_error: &'static str,
    ) -> Result<(), &'static str> {
        let previous_candidates =
            labeled_near_stitch_candidates(previous_candidates.to_vec(), "candidate");
        let next_candidates = labeled_near_stitch_candidates(next_candidates.to_vec(), "candidate");
        self.apply_meshlib_near_stitch_labeled_candidates(
            &previous_candidates,
            &next_candidates,
            ExactMeshlibNearStitchCandidateAttemptContext {
                missing_previous_error: missing_error,
                missing_next_error: missing_error,
                attempt: "test-candidates",
                previous_source_lookup: None,
                next_source_lookup: None,
            },
        )
    }

    fn apply_meshlib_near_stitch_labeled_candidates(
        &mut self,
        previous_candidates: &[ExactMeshlibNearStitchCandidate],
        next_candidates: &[ExactMeshlibNearStitchCandidate],
        context: ExactMeshlibNearStitchCandidateAttemptContext,
    ) -> Result<(), &'static str> {
        if previous_candidates.is_empty() || next_candidates.is_empty() {
            self.meshlib_last_near_stitch_candidate_diagnostics =
                Some(ExactMeshlibNearStitchCandidateDiagnostics {
                    attempt: context.attempt,
                    previous_candidates: previous_candidates.len(),
                    next_candidates: next_candidates.len(),
                    failures: Vec::new(),
                    fallback_from: None,
                    previous_source_lookup: context.previous_source_lookup,
                    next_source_lookup: context.next_source_lookup,
                });
            return if previous_candidates.is_empty() {
                Err(context.missing_previous_error)
            } else {
                Err(context.missing_next_error)
            };
        }
        let mut best_error = None;
        let mut failures = Vec::new();
        for previous in previous_candidates {
            for next in next_candidates {
                match self
                    .topology
                    .validate_meshlib_near_stitch_edge_update(previous.edge, next.edge)
                {
                    Ok(()) => {
                        return self
                            .topology
                            .apply_meshlib_near_stitch_edge_update(previous.edge, next.edge);
                    }
                    Err(error) => {
                        let retry = self
                            .apply_meshlib_near_stitch_with_captured_open_target(*previous, *next);
                        if retry.error.is_none() && (retry.reopened_previous || retry.reopened_next)
                        {
                            return Ok(());
                        }
                        failures.push(
                            self.near_stitch_candidate_failure(*previous, *next, error, retry),
                        );
                        best_error = Some(prefer_near_stitch_guard_error(best_error, error))
                    }
                };
            }
        }
        self.meshlib_last_near_stitch_candidate_diagnostics =
            Some(ExactMeshlibNearStitchCandidateDiagnostics {
                attempt: context.attempt,
                previous_candidates: previous_candidates.len(),
                next_candidates: next_candidates.len(),
                failures,
                fallback_from: None,
                previous_source_lookup: context.previous_source_lookup,
                next_source_lookup: context.next_source_lookup,
            });
        Err(best_error.unwrap_or("no MeshLib near stitch candidate satisfies boundary guards"))
    }

    fn apply_meshlib_near_stitch_with_captured_open_target(
        &mut self,
        previous: ExactMeshlibNearStitchCandidate,
        next: ExactMeshlibNearStitchCandidate,
    ) -> ExactMeshlibCapturedOpenTargetRetry {
        if self.topology.origin(previous.edge) != self.topology.origin(next.edge) {
            return ExactMeshlibCapturedOpenTargetRetry::default();
        }
        let previous_left = self.topology.left(previous.edge);
        let next_right_edge = ExactHalfEdgeTopology::sym(next.edge);
        let next_right = self.topology.left(next_right_edge);
        let can_reopen_previous = previous
            .target_snapshot
            .is_some_and(|snapshot| snapshot.left.is_none())
            && previous_left.is_some();
        let can_reopen_next = next
            .target_snapshot
            .is_some_and(|snapshot| snapshot.right.is_none())
            && next_right.is_some();
        if !can_reopen_previous && !can_reopen_next {
            return ExactMeshlibCapturedOpenTargetRetry::default();
        }
        let mut retry = ExactMeshlibCapturedOpenTargetRetry {
            reopened_previous: can_reopen_previous,
            reopened_next: can_reopen_next,
            error: None,
        };
        let result = (|| {
            if can_reopen_previous {
                self.topology.set_left_direct(previous.edge, None)?;
            }
            if can_reopen_next {
                self.topology.set_left_direct(next_right_edge, None)?;
            }
            self.topology
                .apply_meshlib_near_stitch_edge_update(previous.edge, next.edge)
        })();
        if let Err(retry_error) = result {
            retry.error = Some(retry_error);
            if can_reopen_previous {
                let _ = self.topology.set_left_direct(previous.edge, previous_left);
            }
            if can_reopen_next {
                let _ = self.topology.set_left_direct(next_right_edge, next_right);
            }
        }
        retry
    }

    fn near_stitch_candidate_failure(
        &self,
        previous: ExactMeshlibNearStitchCandidate,
        next: ExactMeshlibNearStitchCandidate,
        error: &'static str,
        retry: ExactMeshlibCapturedOpenTargetRetry,
    ) -> ExactMeshlibNearStitchCandidateFailure {
        let next_sym = ExactHalfEdgeTopology::sym(next.edge);
        let previous_next = self.topology.next(previous.edge);
        let next_prev = self.topology.prev(next.edge);
        ExactMeshlibNearStitchCandidateFailure {
            previous_edge_id: previous.edge.0,
            next_edge_id: next.edge.0,
            previous_candidate_source: previous.source,
            next_candidate_source: next.source,
            previous_candidate_key: previous.key,
            next_candidate_key: next.key,
            previous_candidate_source_edge: previous.source_edge,
            next_candidate_source_edge: next.source_edge,
            previous_origin: self.topology.origin(previous.edge),
            next_origin: self.topology.origin(next.edge),
            previous_left: self.topology.left(previous.edge),
            previous_right: self.topology.right(previous.edge),
            next_left: self.topology.left(next.edge),
            next_right: self.topology.right(next.edge),
            previous_next_edge_id: previous_next.0,
            next_prev_edge_id: next_prev.0,
            previous_next_edge: self.near_stitch_linked_edge_diagnostic(previous_next),
            next_prev_edge: self.near_stitch_linked_edge_diagnostic(next_prev),
            previous_left_ring: self.meshlib_near_stitch_left_ring_diagnostic(previous.edge),
            next_right_ring: self.meshlib_near_stitch_left_ring_diagnostic(next_sym),
            previous_target_snapshot: previous.target_snapshot,
            next_target_snapshot: next.target_snapshot,
            captured_open_target_reopened_previous: retry.reopened_previous,
            captured_open_target_reopened_next: retry.reopened_next,
            captured_open_target_retry_error: retry.error,
            error,
        }
    }

    fn near_stitch_linked_edge_diagnostic(
        &self,
        edge: ExactHalfEdgeId,
    ) -> ExactMeshlibNearStitchLinkedEdgeDiagnostic {
        ExactMeshlibNearStitchLinkedEdgeDiagnostic {
            edge_id: edge.0,
            origin: self.topology.origin(edge),
            left: self.topology.left(edge),
            right: self.topology.right(edge),
        }
    }
}

fn should_fallback_to_vertex_pair_near_stitch(error: &str) -> bool {
    matches!(
        error,
        "missing MeshLib near stitch target edge"
            | "missing MeshLib near stitch previous edge"
            | "missing MeshLib near stitch next edge"
            | "missing MeshLib near stitch edge"
    )
}

fn prefer_near_stitch_guard_error(
    current: Option<&'static str>,
    candidate: &'static str,
) -> &'static str {
    match current {
        Some(current)
            if near_stitch_guard_error_rank(current) >= near_stitch_guard_error_rank(candidate) =>
        {
            current
        }
        _ => candidate,
    }
}

fn near_stitch_guard_error_rank(error: &str) -> u8 {
    match error {
        "previous near stitch edge must not have a left face"
        | "next near stitch edge must not have a right face" => 3,
        "near stitch edges must share origin" => 2,
        _ => 1,
    }
}

fn push_unique_target_edge(edges: &mut Vec<ExactHalfEdgeId>, edge: ExactHalfEdgeId) {
    if !edges.contains(&edge) {
        edges.push(edge);
    }
}

#[cfg(test)]
mod tests;
