use super::super::super::exact_boolean::ExactBooleanOperand;
use super::super::super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
use super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::super::super::exact_meshlib_near_stitch::ExactMeshlibNearStitchEndpoint;
use super::super::source_records::ExactMeshlibPreparedSourceRecord;
use super::OutputFaceTopology;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibRecordRewriteTargetDiagnostic {
    pub stitch_pair_index: usize,
    pub target_edge_id: usize,
    pub target_was_near_stitch_target: bool,
    pub target_origin_before: Option<usize>,
    pub target_left_before: Option<usize>,
    pub target_right_before: Option<usize>,
    pub target_next_edge_id_before: usize,
    pub target_prev_edge_id_before: usize,
    pub target_origin_after: Option<usize>,
    pub target_left_after: Option<usize>,
    pub target_right_after: Option<usize>,
    pub target_next_edge_id_after: usize,
    pub target_prev_edge_id_after: usize,
    pub record_next_edge_id: usize,
    pub record_left: Option<usize>,
    pub record_sym_prev_edge_id: usize,
}

impl OutputFaceTopology {
    pub(crate) fn apply_meshlib_record_rewrite_command(
        &mut self,
        command: &ExactMeshlibRecordRewriteCommand,
    ) -> Result<ExactMeshlibRecordRewriteTargetDiagnostic, &'static str> {
        let from_operand = self.meshlib_operand_filter(command.from_operand);
        let this_operand = self.meshlib_operand_filter(command.this_operand);
        let source_record = self.meshlib_record_rewrite_source_record(command, from_operand)?;
        let target =
            self.meshlib_record_rewrite_target(command, source_record.left, this_operand)?;
        let before = self.meshlib_record_rewrite_target_state(target);
        let target_was_near_stitch_target = self.meshlib_near_stitch_target_edge_registered(target);
        self.topology.apply_meshlib_stitched_edge_record_rewrite(
            target,
            source_record.next,
            source_record.left,
            source_record.sym_prev,
            self.meshlib_patch_record_reciprocals,
        )?;
        if self.meshlib_patch_record_reciprocals {
            if let Some(face) = source_record.left {
                if let Some(face_edge) = self.face_edges.get_mut(face) {
                    *face_edge = target;
                    self.meshlib_translated_face_records += 1;
                }
            }
        } else if source_record.left.is_some() {
            self.meshlib_translated_face_records += 1;
        }
        let after = self.meshlib_record_rewrite_target_state(target);
        Ok(ExactMeshlibRecordRewriteTargetDiagnostic {
            stitch_pair_index: command.stitch_pair_index,
            target_edge_id: target.0,
            target_was_near_stitch_target,
            target_origin_before: before.origin,
            target_left_before: before.left,
            target_right_before: before.right,
            target_next_edge_id_before: before.next.0,
            target_prev_edge_id_before: before.prev.0,
            target_origin_after: after.origin,
            target_left_after: after.left,
            target_right_after: after.right,
            target_next_edge_id_after: after.next.0,
            target_prev_edge_id_after: after.prev.0,
            record_next_edge_id: source_record.next.0,
            record_left: source_record.left,
            record_sym_prev_edge_id: source_record.sym_prev.0,
        })
    }

    pub(crate) fn prepare_meshlib_record_rewrite_command_map(
        &mut self,
        command: &ExactMeshlibRecordRewriteCommand,
    ) -> Result<(), &'static str> {
        let from_operand = self.meshlib_operand_filter(command.from_operand);
        let this_operand = self.meshlib_operand_filter(command.this_operand);
        let source_record = self.meshlib_record_rewrite_source_record(command, from_operand)?;
        let _ = self.meshlib_record_rewrite_target(command, source_record.left, this_operand)?;
        Ok(())
    }

    pub(crate) fn prepare_meshlib_record_rewrite_target_map(
        &mut self,
        command: &ExactMeshlibRecordRewriteCommand,
    ) -> Result<(), &'static str> {
        let this_operand = self.meshlib_operand_filter(command.this_operand);
        let _ = self.meshlib_record_rewrite_target(command, None, this_operand)?;
        Ok(())
    }

    fn meshlib_rewrite_target_edge(
        &mut self,
        edge: [usize; 2],
        synthetic: bool,
        mapped_from_left: Option<usize>,
        operand: Option<ExactBooleanOperand>,
    ) -> Result<ExactHalfEdgeId, &'static str> {
        if synthetic {
            return Ok(self.add_synthetic_stitch_edge(edge));
        }
        let candidates = self.meshlib_rewrite_target_candidates(edge, mapped_from_left, operand);
        let has_directed_face_candidates = candidates.has_directed_face_candidates;
        self.best_meshlib_rewrite_target(candidates.targets)
            .or_else(|| {
                has_directed_face_candidates.then(|| {
                    self.meshlib_synthetic_target_edges += 1;
                    self.add_synthetic_stitch_edge(edge)
                })
            })
            .ok_or("missing MeshLib rewrite target contour edge")
    }

    fn meshlib_rewrite_target_candidates(
        &self,
        edge: [usize; 2],
        _mapped_from_left: Option<usize>,
        operand: Option<ExactBooleanOperand>,
    ) -> MeshlibRewriteTargetCandidates {
        let mut candidates = self.directed_face_edge_candidates(edge, operand);
        if candidates.is_empty() && operand.is_some() {
            candidates = self.directed_face_edge_candidates(edge, None);
        }
        if candidates.is_empty() {
            let reversed = reverse_edge(edge);
            candidates = self.directed_face_edge_candidates(reversed, operand);
            if candidates.is_empty() && operand.is_some() {
                candidates = self.directed_face_edge_candidates(reversed, None);
            }
        }
        let has_directed_face_candidates = !candidates.is_empty();
        let targets = candidates
            .iter()
            .copied()
            .map(ExactHalfEdgeTopology::sym)
            .filter(|candidate| self.topology.left(*candidate).is_none())
            .collect::<Vec<_>>();
        MeshlibRewriteTargetCandidates {
            has_directed_face_candidates,
            targets,
        }
    }

    fn meshlib_record_rewrite_target(
        &mut self,
        command: &ExactMeshlibRecordRewriteCommand,
        mapped_from_left: Option<usize>,
        this_operand: Option<ExactBooleanOperand>,
    ) -> Result<ExactHalfEdgeId, &'static str> {
        let contour_edge = self.meshlib_contour_edge_key(command);
        if let Some(target) = self
            .meshlib_indexed_this_contour_edge_target(command)
            .filter(|target| self.meshlib_rewrite_target_accepts_record(*target, mapped_from_left))
        {
            let target = self.prepare_meshlib_mapped_rewrite_target(
                command,
                command.stitch_pair_index,
                target,
                mapped_from_left,
                this_operand,
            )?;
            self.register_meshlib_mapped_contour_edge(
                command.from_operand,
                command.from_source_edge_index,
                contour_edge,
                command.from_source_edge,
                target,
            );
            return Ok(target);
        }
        if let Some(target) = self
            .meshlib_indexed_contour_edge_target(command)
            .or_else(|| {
                self.meshlib_mapped_contour_edges
                    .get(&(command.from_operand, contour_edge))
                    .copied()
            })
            .filter(|target| self.meshlib_rewrite_target_accepts_record(*target, mapped_from_left))
        {
            return self.prepare_meshlib_mapped_rewrite_target(
                command,
                command.stitch_pair_index,
                target,
                mapped_from_left,
                this_operand,
            );
        }

        let target_candidates = self.meshlib_rewrite_target_candidates(
            command.this_contour_edge,
            mapped_from_left,
            this_operand,
        );
        let target = self.meshlib_rewrite_target_edge(
            command.this_contour_edge,
            command.this_side_synthetic,
            mapped_from_left,
            this_operand,
        )?;
        self.register_meshlib_near_stitch_target_edges(command.stitch_pair_index, target);
        if !command.this_side_synthetic {
            self.register_meshlib_near_stitch_target_edge_candidates(
                command.stitch_pair_index,
                target_candidates.targets,
            );
        }
        self.register_meshlib_mapped_contour_edge(
            command.from_operand,
            command.from_source_edge_index,
            contour_edge,
            command.from_source_edge,
            target,
        );
        Ok(target)
    }

    fn meshlib_rewrite_target_accepts_record(
        &self,
        target: ExactHalfEdgeId,
        _mapped_from_left: Option<usize>,
    ) -> bool {
        self.topology.left(target).is_none()
    }

    fn prepare_meshlib_mapped_rewrite_target(
        &mut self,
        command: &ExactMeshlibRecordRewriteCommand,
        stitch_pair_index: usize,
        target: ExactHalfEdgeId,
        mapped_from_left: Option<usize>,
        this_operand: Option<ExactBooleanOperand>,
    ) -> Result<ExactHalfEdgeId, &'static str> {
        let target_was_registered = self.meshlib_near_stitch_target_registered(stitch_pair_index);
        if !target_was_registered {
            self.register_meshlib_near_stitch_target_edges(stitch_pair_index, target);
        }
        if !target_was_registered && !command.this_side_synthetic {
            let target_candidates = self.meshlib_rewrite_target_candidates(
                command.this_contour_edge,
                mapped_from_left,
                this_operand,
            );
            self.register_meshlib_near_stitch_target_edge_candidates(
                stitch_pair_index,
                target_candidates.targets,
            );
        }
        Ok(target)
    }

    fn meshlib_near_stitch_target_registered(&self, stitch_pair_index: usize) -> bool {
        self.meshlib_near_stitch_target_edges
            .contains_key(&(stitch_pair_index, ExactMeshlibNearStitchEndpoint::Start))
            || self
                .meshlib_near_stitch_target_edges
                .contains_key(&(stitch_pair_index, ExactMeshlibNearStitchEndpoint::End))
    }

    fn best_meshlib_rewrite_target(
        &self,
        candidates: Vec<ExactHalfEdgeId>,
    ) -> Option<ExactHalfEdgeId> {
        candidates
            .iter()
            .copied()
            .find(|candidate| self.meshlib_near_stitch_boundary_score(*candidate) == 2)
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|candidate| self.meshlib_near_stitch_boundary_score(*candidate) == 1)
            })
            .or_else(|| candidates.first().copied())
    }

    fn meshlib_near_stitch_boundary_score(&self, target: ExactHalfEdgeId) -> u8 {
        let start_edge = self.topology.prev(ExactHalfEdgeTopology::sym(target));
        let end_edge = self.topology.next(target);
        let start_ready = self.topology.left(start_edge).is_none();
        let end_ready = self.topology.right(end_edge).is_none();
        u8::from(start_ready) + u8::from(end_ready)
    }

    fn meshlib_rewrite_source_edge(
        &mut self,
        edge: [usize; 2],
        synthetic: bool,
        operand: Option<ExactBooleanOperand>,
    ) -> Result<ExactHalfEdgeId, &'static str> {
        if synthetic {
            return Ok(self.add_synthetic_stitch_edge(edge));
        }
        self.first_directed_face_edge(edge, operand)
            .or_else(|| {
                operand
                    .is_some()
                    .then(|| self.first_directed_face_edge(edge, None))
                    .flatten()
            })
            .ok_or("missing MeshLib rewrite source contour edge")
    }

    fn meshlib_record_rewrite_source_record(
        &mut self,
        command: &ExactMeshlibRecordRewriteCommand,
        operand: Option<ExactBooleanOperand>,
    ) -> Result<ExactMeshlibPreparedSourceRecord, &'static str> {
        if !command.from_side_synthetic {
            if let Some(record) = self
                .meshlib_indexed_prepared_source_record(command)
                .or_else(|| {
                    self.meshlib_prepared_source_records
                        .get(&(command.from_operand, self.meshlib_contour_edge_key(command)))
                        .copied()
                })
            {
                return Ok(record);
            }
        }
        let mapped_from = self.meshlib_rewrite_source_edge(
            command.from_contour_edge,
            command.from_side_synthetic,
            operand,
        )?;
        Ok(ExactMeshlibPreparedSourceRecord {
            next: self.topology.next(mapped_from),
            left: self.topology.left(mapped_from),
            sym_prev: self.topology.prev(ExactHalfEdgeTopology::sym(mapped_from)),
        })
    }

    fn meshlib_contour_edge_key(&self, command: &ExactMeshlibRecordRewriteCommand) -> [usize; 2] {
        if self.meshlib_use_source_edge_identity {
            command.from_source_edge
        } else {
            command.from_contour_edge
        }
    }

    fn meshlib_indexed_contour_edge_target(
        &self,
        command: &ExactMeshlibRecordRewriteCommand,
    ) -> Option<ExactHalfEdgeId> {
        self.meshlib_use_source_edge_identity
            .then(|| {
                self.meshlib_mapped_contour_edge_indices
                    .get(&(command.from_operand, command.from_source_edge_index))
                    .copied()
            })
            .flatten()
    }

    fn meshlib_indexed_this_contour_edge_target(
        &self,
        command: &ExactMeshlibRecordRewriteCommand,
    ) -> Option<ExactHalfEdgeId> {
        self.meshlib_use_source_edge_identity
            .then(|| {
                self.meshlib_mapped_contour_edge_indices
                    .get(&(command.this_operand, command.this_source_edge_index))
                    .copied()
            })
            .flatten()
    }

    fn meshlib_indexed_prepared_source_record(
        &self,
        command: &ExactMeshlibRecordRewriteCommand,
    ) -> Option<ExactMeshlibPreparedSourceRecord> {
        self.meshlib_use_source_edge_identity
            .then(|| {
                self.meshlib_prepared_source_records_by_index
                    .get(&(command.from_operand, command.from_source_edge_index))
                    .copied()
            })
            .flatten()
    }

    pub(in crate::spatial::exact_splice_apply) fn apply_meshlib_prepared_mapped_source_records(
        &mut self,
        replays: Vec<(ExactHalfEdgeId, ExactMeshlibPreparedSourceRecord)>,
    ) -> Result<usize, &'static str> {
        let mut applied = 0;
        for (target, record) in replays {
            let target_was_near_stitch_target =
                self.meshlib_near_stitch_target_edge_registered(target);
            let before = self.meshlib_record_rewrite_target_state(target);
            if self.topology.left(target).is_some() {
                self.record_meshlib_prepared_source_replay(
                    target,
                    record,
                    target_was_near_stitch_target,
                    before,
                    false,
                    Some("target already has left face"),
                );
                continue;
            }
            if target_was_near_stitch_target {
                self.meshlib_prepared_mapped_source_record_replays_on_near_stitch_targets += 1;
            }
            self.topology.apply_meshlib_stitched_edge_record_rewrite(
                target,
                record.next,
                record.left,
                record.sym_prev,
                false,
            )?;
            self.record_meshlib_prepared_source_replay(
                target,
                record,
                target_was_near_stitch_target,
                before,
                true,
                None,
            );
            applied += 1;
        }
        self.meshlib_prepared_mapped_source_record_replays += applied;
        Ok(applied)
    }

    fn record_meshlib_prepared_source_replay(
        &mut self,
        target: ExactHalfEdgeId,
        record: ExactMeshlibPreparedSourceRecord,
        target_was_near_stitch_target: bool,
        before: MeshlibRecordRewriteTargetState,
        applied: bool,
        skipped_reason: Option<&'static str>,
    ) {
        let after = self.meshlib_record_rewrite_target_state(target);
        self.meshlib_prepared_mapped_source_record_replay_details
            .push(super::ExactMeshlibPreparedSourceRecordReplayDiagnostic {
                target_edge_id: target.0,
                target_was_near_stitch_target,
                target_origin_before: before.origin,
                target_left_before: before.left,
                target_right_before: before.right,
                target_origin_after: after.origin,
                target_left_after: after.left,
                target_right_after: after.right,
                record_next_edge_id: record.next.0,
                record_left: record.left,
                record_sym_prev_edge_id: record.sym_prev.0,
                applied,
                skipped_reason,
            });
    }

    fn meshlib_record_rewrite_target_state(
        &self,
        target: ExactHalfEdgeId,
    ) -> MeshlibRecordRewriteTargetState {
        MeshlibRecordRewriteTargetState {
            origin: self.topology.origin(target),
            left: self.topology.left(target),
            right: self.topology.right(target),
            next: self.topology.next(target),
            prev: self.topology.prev(target),
        }
    }

    fn meshlib_near_stitch_target_edge_registered(&self, target: ExactHalfEdgeId) -> bool {
        self.meshlib_near_stitch_target_edges
            .values()
            .any(|targets| targets.contains(&target))
    }
}

#[derive(Clone, Copy)]
struct MeshlibRecordRewriteTargetState {
    origin: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    next: ExactHalfEdgeId,
    prev: ExactHalfEdgeId,
}

fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

struct MeshlibRewriteTargetCandidates {
    has_directed_face_candidates: bool,
    targets: Vec<ExactHalfEdgeId>,
}
