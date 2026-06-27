use super::super::exact_boolean::{ExactBooleanOperand, ExactBooleanOutputFaceSource};
use super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::super::exact_meshlib_near_stitch::{
    ExactMeshlibNearStitchEndpoint, ExactMeshlibSourceHalfedgeKey,
};
use super::copied_edges::{
    finalize_meshlib_copied_edges, prepare_meshlib_copied_edges,
    ExactMeshlibCopiedEdgeTranslationInput, ExactMeshlibCopiedEdgeTranslationSummary,
    ExactMeshlibCopiedSourceEdgeDiagnostic, ExactMeshlibPreparedCopiedEdges,
};
use super::source_records::ExactMeshlibPreparedSourceRecord;
use std::collections::BTreeMap;
mod copied_edge_tracking;
mod export;
mod near_stitch;
mod near_stitch_diagnostics;
mod rewrite_records;
pub(crate) use copied_edge_tracking::{
    ExactMeshlibCopiedFaceRecordCandidateDiagnostic, ExactMeshlibCopiedFaceRecordDiagnostic,
};
pub(crate) use export::ExactMeshlibFaceExportFailureDiagnostic;
pub(crate) use near_stitch::{
    ExactMeshlibNearStitchCandidateDiagnostics, ExactMeshlibNearStitchLinkedEdgeDiagnostic,
    ExactMeshlibNearStitchSourceLookupDiagnostics, ExactMeshlibNearStitchTargetSnapshot,
};
pub(crate) use near_stitch_diagnostics::ExactMeshlibNearStitchRingDiagnostic;
pub(crate) use rewrite_records::ExactMeshlibRecordRewriteTargetDiagnostic;

type OutputMergeKey = ([usize; 2], Option<ExactBooleanOperand>);
type OutputMergeEdges = BTreeMap<OutputMergeKey, Vec<([usize; 2], ExactHalfEdgeId)>>;

pub(crate) struct OutputFaceTopology {
    pub(super) topology: ExactHalfEdgeTopology,
    directed_face_edges: BTreeMap<(usize, [usize; 2]), ExactHalfEdgeId>,
    face_operands: Vec<Option<ExactBooleanOperand>>,
    face_cut_faces: Vec<Option<usize>>,
    face_source_faces: Vec<Option<usize>>,
    filter_face_operands: bool,
    pub(super) face_edges: Vec<ExactHalfEdgeId>,
    synthetic_stitch_edges: Vec<ExactHalfEdgeId>,
    pub(super) meshlib_mapped_contour_edges:
        BTreeMap<(ExactBooleanOperand, [usize; 2]), ExactHalfEdgeId>,
    pub(super) meshlib_mapped_contour_edge_indices:
        BTreeMap<(ExactBooleanOperand, usize), ExactHalfEdgeId>,
    meshlib_copied_directed_edges: BTreeMap<[usize; 2], Vec<ExactHalfEdgeId>>,
    meshlib_copied_source_edge_statuses:
        BTreeMap<(ExactBooleanOperand, [usize; 2]), Vec<ExactMeshlibCopiedSourceEdgeDiagnostic>>,
    meshlib_source_directed_edges:
        BTreeMap<(ExactBooleanOperand, [usize; 2]), Vec<ExactHalfEdgeId>>,
    meshlib_source_halfedges: BTreeMap<(ExactBooleanOperand, usize), Vec<ExactHalfEdgeId>>,
    meshlib_source_halfedge_edges: BTreeMap<(ExactBooleanOperand, usize), Vec<[usize; 2]>>,
    meshlib_source_halfedge_keys:
        BTreeMap<(ExactBooleanOperand, ExactMeshlibSourceHalfedgeKey), Vec<ExactHalfEdgeId>>,
    meshlib_source_halfedge_key_edges:
        BTreeMap<(ExactBooleanOperand, ExactMeshlibSourceHalfedgeKey), Vec<[usize; 2]>>,
    meshlib_prepared_source_records:
        BTreeMap<(ExactBooleanOperand, [usize; 2]), ExactMeshlibPreparedSourceRecord>,
    meshlib_prepared_source_records_by_index:
        BTreeMap<(ExactBooleanOperand, usize), ExactMeshlibPreparedSourceRecord>,
    meshlib_near_stitch_target_edges:
        BTreeMap<(usize, ExactMeshlibNearStitchEndpoint), Vec<ExactHalfEdgeId>>,
    meshlib_near_stitch_target_snapshots: BTreeMap<
        (usize, ExactMeshlibNearStitchEndpoint, ExactHalfEdgeId),
        ExactMeshlibNearStitchTargetSnapshot,
    >,
    meshlib_last_near_stitch_candidate_diagnostics:
        Option<ExactMeshlibNearStitchCandidateDiagnostics>,
    pub(super) meshlib_use_source_edge_identity: bool,
    pub(super) meshlib_patch_record_reciprocals: bool,
    pub(super) duplicated_directed_edges: usize,
    pub(crate) meshlib_synthetic_target_edges: usize,
    pub(crate) meshlib_translated_face_records: usize,
    pub(crate) meshlib_copied_edge_translation: ExactMeshlibCopiedEdgeTranslationSummary,
    pub(crate) meshlib_prepared_mapped_source_record_replays: usize,
    pub(crate) meshlib_prepared_mapped_source_record_replays_on_near_stitch_targets: usize,
    pub(crate) meshlib_prepared_mapped_source_record_replay_details:
        Vec<ExactMeshlibPreparedSourceRecordReplayDiagnostic>,
    pub(crate) meshlib_copied_prev_next_edge_update_attempts: usize,
    pub(crate) meshlib_copied_prev_next_edge_updates_applied: usize,
    pub(crate) meshlib_copied_prev_next_edge_updates_skipped: usize,
    pub(crate) meshlib_copied_prev_next_edge_update_details:
        Vec<ExactMeshlibCopiedPrevNextEdgeUpdateDiagnostic>,
    pub(crate) meshlib_copied_face_record_details: Vec<ExactMeshlibCopiedFaceRecordDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibPreparedSourceRecordReplayDiagnostic {
    pub target_edge_id: usize,
    pub target_was_near_stitch_target: bool,
    pub target_origin_before: Option<usize>,
    pub target_left_before: Option<usize>,
    pub target_right_before: Option<usize>,
    pub target_origin_after: Option<usize>,
    pub target_left_after: Option<usize>,
    pub target_right_after: Option<usize>,
    pub record_next_edge_id: usize,
    pub record_left: Option<usize>,
    pub record_sym_prev_edge_id: usize,
    pub applied: bool,
    pub skipped_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::spatial::exact_splice_apply) struct ExactMeshlibCopiedPrevNextEdgeUpdate {
    pub previous: ExactHalfEdgeId,
    pub next: ExactHalfEdgeId,
    pub source_contour_edge: ExactHalfEdgeId,
    pub target_contour_edge: ExactHalfEdgeId,
    pub walked_source_edge: ExactHalfEdgeId,
    pub update_kind: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibCopiedPrevNextEdgeUpdateDiagnostic {
    pub source_contour_edge_id: Option<usize>,
    pub target_contour_edge_id: Option<usize>,
    pub walked_source_edge_id: Option<usize>,
    pub update_kind: Option<&'static str>,
    pub previous_edge_id: usize,
    pub next_edge_id: usize,
    pub previous_origin: Option<usize>,
    pub next_origin: Option<usize>,
    pub previous_left: Option<usize>,
    pub next_right: Option<usize>,
    pub applied: bool,
    pub skipped_reason: Option<&'static str>,
}

impl OutputFaceTopology {
    pub(crate) fn from_faces(faces: &[[i64; 3]]) -> Result<Self, &'static str> {
        Self::from_faces_with_operands(
            faces,
            vec![None; faces.len()],
            vec![None; faces.len()],
            vec![None; faces.len()],
            false,
        )
    }

    pub(crate) fn from_faces_with_sources(
        faces: &[[i64; 3]],
        face_sources: &[ExactBooleanOutputFaceSource],
    ) -> Result<Self, &'static str> {
        if faces.len() != face_sources.len() {
            return Err("face source count must match face count");
        }
        let face_operands = face_sources
            .iter()
            .map(|source| Some(source.operand))
            .collect();
        let face_cut_faces = face_sources
            .iter()
            .map(|source| Some(source.cut_face))
            .collect();
        let face_source_faces = face_sources
            .iter()
            .map(|source| Some(source.source_face))
            .collect();
        Self::from_faces_with_operands(
            faces,
            face_operands,
            face_cut_faces,
            face_source_faces,
            true,
        )
    }

    fn from_faces_with_operands(
        faces: &[[i64; 3]],
        face_operands: Vec<Option<ExactBooleanOperand>>,
        face_cut_faces: Vec<Option<usize>>,
        face_source_faces: Vec<Option<usize>>,
        filter_face_operands: bool,
    ) -> Result<Self, &'static str> {
        let mut topology = ExactHalfEdgeTopology::new();
        let mut directed_face_edges = BTreeMap::new();
        let mut undirected_edges = OutputMergeEdges::new();
        let mut face_edges = Vec::with_capacity(faces.len());
        let mut duplicated_directed_edges = 0;
        for (face_index, face) in faces.iter().enumerate() {
            let face = [face[0] as usize, face[1] as usize, face[2] as usize];
            let edges = [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]];
            let mut face_edge_ids = Vec::with_capacity(3);
            for edge in edges {
                let (edge_id, duplicated) = output_edge_id(
                    &mut topology,
                    &mut undirected_edges,
                    edge,
                    face_operands[face_index],
                );
                if duplicated {
                    duplicated_directed_edges += 1;
                }
                directed_face_edges.insert((face_index, edge), edge_id);
                face_edge_ids.push(edge_id);
            }
            link_face_ring(&mut topology, &face_edge_ids, edges)?;
            topology.set_left(face_edge_ids[0], Some(face_index))?;
            face_edges.push(face_edge_ids[0]);
        }
        Ok(Self {
            topology,
            directed_face_edges,
            face_operands,
            face_cut_faces,
            face_source_faces,
            filter_face_operands,
            face_edges,
            synthetic_stitch_edges: Vec::new(),
            meshlib_mapped_contour_edges: BTreeMap::new(),
            meshlib_mapped_contour_edge_indices: BTreeMap::new(),
            meshlib_copied_directed_edges: BTreeMap::new(),
            meshlib_copied_source_edge_statuses: BTreeMap::new(),
            meshlib_source_directed_edges: BTreeMap::new(),
            meshlib_source_halfedges: BTreeMap::new(),
            meshlib_source_halfedge_edges: BTreeMap::new(),
            meshlib_source_halfedge_keys: BTreeMap::new(),
            meshlib_source_halfedge_key_edges: BTreeMap::new(),
            meshlib_prepared_source_records: BTreeMap::new(),
            meshlib_prepared_source_records_by_index: BTreeMap::new(),
            meshlib_near_stitch_target_edges: BTreeMap::new(),
            meshlib_near_stitch_target_snapshots: BTreeMap::new(),
            meshlib_last_near_stitch_candidate_diagnostics: None,
            meshlib_use_source_edge_identity: false,
            meshlib_patch_record_reciprocals: true,
            duplicated_directed_edges,
            meshlib_synthetic_target_edges: 0,
            meshlib_translated_face_records: 0,
            meshlib_copied_edge_translation: ExactMeshlibCopiedEdgeTranslationSummary::default(),
            meshlib_prepared_mapped_source_record_replays: 0,
            meshlib_prepared_mapped_source_record_replays_on_near_stitch_targets: 0,
            meshlib_prepared_mapped_source_record_replay_details: Vec::new(),
            meshlib_copied_prev_next_edge_update_attempts: 0,
            meshlib_copied_prev_next_edge_updates_applied: 0,
            meshlib_copied_prev_next_edge_updates_skipped: 0,
            meshlib_copied_prev_next_edge_update_details: Vec::new(),
            meshlib_copied_face_record_details: Vec::new(),
        })
    }

    pub(super) fn directed_face_edge(
        &self,
        face_index: usize,
        edge: [usize; 2],
    ) -> Option<ExactHalfEdgeId> {
        self.directed_face_edges.get(&(face_index, edge)).copied()
    }

    pub(super) fn add_synthetic_stitch_edge(&mut self, edge: [usize; 2]) -> ExactHalfEdgeId {
        let edge_id = self.topology.make_edge(Some(edge[0]), Some(edge[1]));
        self.synthetic_stitch_edges.push(edge_id);
        edge_id
    }

    pub(crate) fn prepare_meshlib_copied_edges(
        &mut self,
        input: ExactMeshlibCopiedEdgeTranslationInput<'_>,
    ) -> Result<ExactMeshlibPreparedCopiedEdges, &'static str> {
        prepare_meshlib_copied_edges(self, input)
    }

    pub(crate) fn finalize_meshlib_copied_edges(
        &mut self,
        prepared: ExactMeshlibPreparedCopiedEdges,
    ) -> Result<ExactMeshlibCopiedEdgeTranslationSummary, &'static str> {
        let summary = finalize_meshlib_copied_edges(self, prepared)?;
        self.meshlib_copied_edge_translation = summary;
        Ok(summary)
    }

    pub(crate) fn use_meshlib_source_edge_identity(&mut self) {
        self.meshlib_use_source_edge_identity = true;
    }

    pub(crate) fn use_meshlib_direct_record_rewrite(&mut self) {
        self.meshlib_patch_record_reciprocals = false;
    }

    pub(super) fn set_meshlib_copied_face_record(
        &mut self,
        face_index: usize,
        edge: ExactHalfEdgeId,
        operand: ExactBooleanOperand,
        cut_face: usize,
        source_face: Option<usize>,
    ) -> Result<(), &'static str> {
        if face_index < self.face_edges.len() {
            self.face_edges[face_index] = edge;
            if let Some(face_operand) = self.face_operands.get_mut(face_index) {
                *face_operand = Some(operand);
            }
            if let Some(face_cut_face) = self.face_cut_faces.get_mut(face_index) {
                *face_cut_face = Some(cut_face);
            }
            if let Some(face_source_face) = self.face_source_faces.get_mut(face_index) {
                *face_source_face = source_face;
            }
            return Ok(());
        }
        if face_index == self.face_edges.len() {
            self.face_edges.push(edge);
            self.face_operands.push(Some(operand));
            self.face_cut_faces.push(Some(cut_face));
            self.face_source_faces.push(source_face);
            return Ok(());
        }
        Err("MeshLib copied face records must append contiguously")
    }

    pub(crate) fn refresh_meshlib_face_records(&mut self) -> usize {
        let mut refreshed = 0;
        for face_index in 0..self.face_edges.len() {
            let current = self.face_edges[face_index];
            if self
                .topology
                .validate_meshlib_face_left_ring(current, face_index)
                .is_ok()
            {
                continue;
            }
            let Some(replacement) = self.meshlib_valid_face_record_edge(face_index) else {
                continue;
            };
            self.face_edges[face_index] = replacement;
            refreshed += 1;
        }
        refreshed
    }

    fn meshlib_valid_face_record_edge(&self, face_index: usize) -> Option<ExactHalfEdgeId> {
        self.topology.edge_ids().find(|edge| {
            self.topology.left(*edge) == Some(face_index)
                && self
                    .topology
                    .validate_meshlib_face_left_ring(*edge, face_index)
                    .is_ok()
        })
    }

    fn meshlib_operand_filter(&self, operand: ExactBooleanOperand) -> Option<ExactBooleanOperand> {
        self.filter_face_operands.then_some(operand)
    }

    fn first_directed_face_edge(
        &self,
        edge: [usize; 2],
        operand: Option<ExactBooleanOperand>,
    ) -> Option<ExactHalfEdgeId> {
        self.directed_face_edges
            .iter()
            .find_map(|((face_index, stored_edge), edge_id)| {
                (*stored_edge == edge && self.face_matches_operand(*face_index, operand))
                    .then_some(*edge_id)
            })
    }

    fn topology_edge_candidates_for_directed_edge(&self, edge: [usize; 2]) -> Vec<ExactHalfEdgeId> {
        let mut candidates = self.directed_face_edge_candidates(edge, None);
        self.extend_copied_edge_candidates(edge, &mut candidates);
        for candidate in self
            .directed_face_edge_candidates(reverse_edge(edge), None)
            .into_iter()
            .map(ExactHalfEdgeTopology::sym)
        {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
        candidates
    }

    fn topology_face_edge_candidates_for_directed_edge(
        &self,
        edge: [usize; 2],
    ) -> Vec<ExactHalfEdgeId> {
        let mut candidates = self.directed_face_edge_candidates(edge, None);
        for candidate in self
            .directed_face_edge_candidates(reverse_edge(edge), None)
            .into_iter()
            .map(ExactHalfEdgeTopology::sym)
        {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
        candidates
    }

    pub(super) fn register_meshlib_mapped_contour_edge_index(
        &mut self,
        operand: ExactBooleanOperand,
        edge_index: usize,
        edge_id: ExactHalfEdgeId,
    ) {
        self.meshlib_mapped_contour_edge_indices
            .insert((operand, edge_index), edge_id);
    }

    fn register_meshlib_mapped_contour_edge(
        &mut self,
        operand: ExactBooleanOperand,
        edge_index: usize,
        edge: [usize; 2],
        source_edge: [usize; 2],
        edge_id: ExactHalfEdgeId,
    ) {
        self.register_meshlib_mapped_contour_edge_index(operand, edge_index, edge_id);
        self.meshlib_mapped_contour_edges
            .insert((operand, edge), edge_id);
        self.meshlib_mapped_contour_edges.insert(
            (operand, reverse_edge(edge)),
            ExactHalfEdgeTopology::sym(edge_id),
        );
        self.register_meshlib_source_edge(operand, source_edge, edge_id);
    }

    pub(super) fn register_meshlib_source_edge(
        &mut self,
        operand: ExactBooleanOperand,
        source_edge: [usize; 2],
        edge_id: ExactHalfEdgeId,
    ) {
        push_unique_edge_id(
            self.meshlib_source_directed_edges
                .entry((operand, source_edge))
                .or_default(),
            edge_id,
        );
        push_unique_edge_id(
            self.meshlib_source_directed_edges
                .entry((operand, reverse_edge(source_edge)))
                .or_default(),
            ExactHalfEdgeTopology::sym(edge_id),
        );
    }

    pub(super) fn register_meshlib_source_halfedge(
        &mut self,
        operand: ExactBooleanOperand,
        source_edge_id: ExactHalfEdgeId,
        source_key: Option<ExactMeshlibSourceHalfedgeKey>,
        source_edge: Option<[usize; 2]>,
        edge_id: ExactHalfEdgeId,
    ) {
        let source_edge_id_key = (operand, source_edge_id.0);
        let candidates = self
            .meshlib_source_halfedges
            .entry(source_edge_id_key)
            .or_default();
        if !candidates.contains(&edge_id) {
            candidates.push(edge_id);
        }
        let Some(source_edge) = source_edge else {
            return;
        };
        if let Some(source_key) = source_key {
            let source_key = (operand, source_key);
            let candidates = self
                .meshlib_source_halfedge_keys
                .entry(source_key)
                .or_default();
            if !candidates.contains(&edge_id) {
                candidates.push(edge_id);
            }
            set_parallel_source_edge(
                self.meshlib_source_halfedge_key_edges
                    .entry(source_key)
                    .or_default(),
                candidates,
                edge_id,
                source_edge,
            );
        }
        let source_edges = self
            .meshlib_source_halfedge_edges
            .entry(source_edge_id_key)
            .or_default();
        set_parallel_source_edge(source_edges, candidates, edge_id, source_edge);
    }

    pub(super) fn register_meshlib_prepared_source_record(
        &mut self,
        operand: ExactBooleanOperand,
        edge: [usize; 2],
        record: ExactMeshlibPreparedSourceRecord,
    ) {
        self.meshlib_prepared_source_records
            .insert((operand, edge), record);
    }

    pub(super) fn register_meshlib_prepared_source_record_by_index(
        &mut self,
        operand: ExactBooleanOperand,
        edge_index: usize,
        record: ExactMeshlibPreparedSourceRecord,
    ) {
        self.meshlib_prepared_source_records_by_index
            .insert((operand, edge_index), record);
    }

    fn extend_copied_edge_candidates(
        &self,
        edge: [usize; 2],
        candidates: &mut Vec<ExactHalfEdgeId>,
    ) {
        if let Some(copied_candidates) = self.meshlib_copied_directed_edges.get(&edge) {
            for candidate in copied_candidates {
                if !candidates.contains(candidate) {
                    candidates.push(*candidate);
                }
            }
        }
    }

    fn directed_face_edge_candidates(
        &self,
        edge: [usize; 2],
        operand: Option<ExactBooleanOperand>,
    ) -> Vec<ExactHalfEdgeId> {
        self.directed_face_edges
            .iter()
            .filter_map(|((face_index, stored_edge), edge_id)| {
                (*stored_edge == edge && self.face_matches_operand(*face_index, operand))
                    .then_some(*edge_id)
            })
            .collect()
    }

    fn face_matches_operand(
        &self,
        face_index: usize,
        operand: Option<ExactBooleanOperand>,
    ) -> bool {
        if !self.filter_face_operands {
            return true;
        }
        operand.is_none_or(|operand| self.face_operands.get(face_index) == Some(&Some(operand)))
    }

    pub(super) fn export_faces(&self) -> Result<Vec<[i64; 3]>, &'static str> {
        self.export_face_results().into_iter().collect()
    }

    pub(crate) fn export_face_results(&self) -> Vec<Result<[i64; 3], &'static str>> {
        self.face_edges
            .iter()
            .enumerate()
            .map(|(face_index, edge)| {
                self.topology
                    .validate_meshlib_face_left_ring(*edge, face_index)?;
                let origins = match self.topology.left_ring_origins(*edge) {
                    Ok(origins) => origins,
                    Err(error) => return Err(error),
                };
                match origins.as_slice() {
                    [a, b, c] => Ok([*a as i64, *b as i64, *c as i64]),
                    _ => Err("exported face is not triangular"),
                }
            })
            .collect()
    }

    pub(crate) fn exported_face_operands_for_results(
        &self,
        export_results: &[Result<[i64; 3], &'static str>],
    ) -> Vec<Option<ExactBooleanOperand>> {
        export_results
            .iter()
            .enumerate()
            .filter_map(|(face_index, result)| {
                result
                    .is_ok()
                    .then(|| self.face_operands.get(face_index).copied().flatten())
            })
            .collect()
    }

    pub(crate) fn exported_face_cut_faces_for_results(
        &self,
        export_results: &[Result<[i64; 3], &'static str>],
    ) -> Vec<Option<usize>> {
        export_results
            .iter()
            .enumerate()
            .filter_map(|(face_index, result)| {
                result
                    .is_ok()
                    .then(|| self.face_cut_faces.get(face_index).copied().flatten())
            })
            .collect()
    }

    pub(crate) fn exported_face_source_faces_for_results(
        &self,
        export_results: &[Result<[i64; 3], &'static str>],
    ) -> Vec<Option<usize>> {
        export_results
            .iter()
            .enumerate()
            .filter_map(|(face_index, result)| {
                result
                    .is_ok()
                    .then(|| self.face_source_faces.get(face_index).copied().flatten())
            })
            .collect()
    }

    pub(crate) fn not_lone_undirected_edge_count(&self) -> usize {
        self.topology
            .not_lone_undirected_edge_count()
            .unwrap_or_default()
    }

    pub(super) fn deleted_synthetic_stitch_edges(&self) -> usize {
        self.synthetic_stitch_edges
            .iter()
            .filter(|edge| self.topology.is_lone_edge(**edge).unwrap_or(false))
            .count()
    }
}

fn output_edge_id(
    topology: &mut ExactHalfEdgeTopology,
    undirected_edges: &mut OutputMergeEdges,
    edge: [usize; 2],
    operand: Option<ExactBooleanOperand>,
) -> (ExactHalfEdgeId, bool) {
    let key = (ordered_edge(edge), operand);
    if let Some(edge_ids) = undirected_edges.get(&key) {
        for (stored_edge, edge_id) in edge_ids {
            let candidate = if *stored_edge == edge {
                *edge_id
            } else {
                ExactHalfEdgeTopology::sym(*edge_id)
            };
            if topology.left(candidate).is_none() {
                return (candidate, false);
            }
        }
    }
    let edge_id = topology.make_edge(None, None);
    let duplicated = undirected_edges.contains_key(&key);
    undirected_edges
        .entry(key)
        .or_default()
        .push((edge, edge_id));
    (edge_id, duplicated)
}

fn link_face_ring(
    topology: &mut ExactHalfEdgeTopology,
    face_edge_ids: &[ExactHalfEdgeId],
    edges: [[usize; 2]; 3],
) -> Result<(), &'static str> {
    for (index, edge) in face_edge_ids.iter().copied().enumerate() {
        let previous_edge = face_edge_ids[(index + face_edge_ids.len() - 1) % face_edge_ids.len()];
        let previous_sym = ExactHalfEdgeTopology::sym(previous_edge);
        let target = topology.prev(previous_sym);
        if topology.next(edge) != previous_sym {
            topology.splice(edge, target)?;
        }
        topology.set_origin(edge, Some(edges[index][0]))?;
    }
    Ok(())
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

fn push_unique_edge_id(edges: &mut Vec<ExactHalfEdgeId>, edge: ExactHalfEdgeId) {
    if !edges.contains(&edge) {
        edges.push(edge);
    }
}

fn set_parallel_source_edge(
    source_edges: &mut Vec<[usize; 2]>,
    candidates: &[ExactHalfEdgeId],
    edge_id: ExactHalfEdgeId,
    source_edge: [usize; 2],
) {
    if source_edges.len() < candidates.len() {
        source_edges.resize(candidates.len(), source_edge);
    }
    if let Some(position) = candidates
        .iter()
        .position(|candidate| *candidate == edge_id)
    {
        source_edges[position] = source_edge;
    }
}
