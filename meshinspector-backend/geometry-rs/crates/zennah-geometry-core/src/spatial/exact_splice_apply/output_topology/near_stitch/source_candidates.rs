use super::super::super::super::exact_boolean::ExactBooleanOperand;
use super::super::super::super::exact_halfedge::ExactHalfEdgeTopology;
use super::super::super::super::exact_meshlib_near_stitch::ExactMeshlibSourceHalfedgeKey;
use super::super::OutputFaceTopology;
use super::candidates::{
    extend_unique_labeled_candidates, push_unique_labeled_candidate,
    ExactMeshlibNearStitchCandidate, ExactMeshlibNearStitchSourceCandidate,
    ExactMeshlibNearStitchSourceLookupDiagnostics,
};

pub(super) struct ExactMeshlibNearStitchSourceLookup {
    pub(super) candidates: Vec<ExactMeshlibNearStitchCandidate>,
    pub(super) diagnostics: ExactMeshlibNearStitchSourceLookupDiagnostics,
}

impl OutputFaceTopology {
    fn meshlib_source_edge_candidates(
        &self,
        operand: Option<ExactBooleanOperand>,
        source_edge: Option<[usize; 2]>,
    ) -> Option<Vec<ExactMeshlibNearStitchSourceCandidate>> {
        let source_edge = source_edge?;
        let candidates = self
            .meshlib_source_directed_edges
            .get(&(operand?, source_edge))?
            .clone();
        (!candidates.is_empty()).then(|| {
            candidates
                .into_iter()
                .map(|edge| ExactMeshlibNearStitchSourceCandidate {
                    edge,
                    source_edge: Some(source_edge),
                })
                .collect()
        })
    }

    fn meshlib_source_halfedge_candidates(
        &self,
        operand: Option<ExactBooleanOperand>,
        source_halfedge: Option<usize>,
    ) -> Option<Vec<ExactMeshlibNearStitchSourceCandidate>> {
        let operand = operand?;
        let source_halfedge = source_halfedge?;
        let key = (operand, source_halfedge);
        let candidates = self.meshlib_source_halfedges.get(&key)?.clone();
        let source_edges = self.meshlib_source_halfedge_edges.get(&key);
        (!candidates.is_empty()).then(|| {
            candidates
                .into_iter()
                .enumerate()
                .map(|(index, edge)| ExactMeshlibNearStitchSourceCandidate {
                    edge,
                    source_edge: source_edges.and_then(|edges| edges.get(index).copied()),
                })
                .collect()
        })
    }

    fn meshlib_exact_source_halfedge_key_candidates(
        &self,
        operand: Option<ExactBooleanOperand>,
        source_key: Option<ExactMeshlibSourceHalfedgeKey>,
    ) -> Option<Vec<ExactMeshlibNearStitchSourceCandidate>> {
        let operand = operand?;
        let source_key = source_key?;
        let key = (operand, source_key);
        let candidates = self.meshlib_source_halfedge_keys.get(&key)?;
        let source_edges = self.meshlib_source_halfedge_key_edges.get(&key);
        (!candidates.is_empty()).then(|| {
            candidates
                .iter()
                .copied()
                .enumerate()
                .map(|(index, edge)| ExactMeshlibNearStitchSourceCandidate {
                    edge,
                    source_edge: source_edges.and_then(|edges| edges.get(index).copied()),
                })
                .collect()
        })
    }

    fn meshlib_same_source_edge_key_candidates(
        &self,
        operand: Option<ExactBooleanOperand>,
        source_key: Option<ExactMeshlibSourceHalfedgeKey>,
    ) -> Option<Vec<ExactMeshlibNearStitchSourceCandidate>> {
        let operand = operand?;
        let source_key = source_key?;
        let mut candidates = Vec::new();
        let mut source_edges = Vec::new();
        for ((stored_operand, stored_key), edges) in &self.meshlib_source_halfedge_keys {
            if *stored_operand != operand
                || !source_key_edges_match(stored_key.edge, source_key.edge)
            {
                continue;
            }
            let stored_source_edges = self
                .meshlib_source_halfedge_key_edges
                .get(&(*stored_operand, *stored_key));
            for (index, edge) in edges.iter().copied().enumerate() {
                if candidates.contains(&edge) {
                    continue;
                }
                candidates.push(edge);
                source_edges.push(stored_source_edges.and_then(|edges| edges.get(index).copied()));
            }
        }
        (!candidates.is_empty()).then(|| {
            candidates
                .into_iter()
                .enumerate()
                .map(|(index, edge)| ExactMeshlibNearStitchSourceCandidate {
                    edge,
                    source_edge: source_edges.get(index).copied().flatten(),
                })
                .collect()
        })
    }

    pub(super) fn meshlib_near_stitch_source_candidates(
        &self,
        operand: Option<ExactBooleanOperand>,
        source_halfedge: Option<usize>,
        source_halfedge_key: Option<ExactMeshlibSourceHalfedgeKey>,
        source_edge: Option<[usize; 2]>,
        fallback_edge: [usize; 2],
    ) -> ExactMeshlibNearStitchSourceLookup {
        let mut candidates = Vec::new();
        let mut diagnostics = ExactMeshlibNearStitchSourceLookupDiagnostics {
            requested_halfedge: source_halfedge,
            requested_key_face: source_halfedge_key.map(|key| key.face),
            requested_key_edge: source_halfedge_key.map(|key| key.edge),
            requested_source_edge: source_edge,
            fallback_edge,
            exact_key_candidates: 0,
            same_edge_key_candidates: 0,
            halfedge_candidates: 0,
            source_edge_candidates: 0,
            topology_candidates: 0,
            total_candidates: 0,
            copied_source_edge: self.meshlib_copied_source_edge_lookup(operand, source_edge),
        };
        if let Some(key_candidates) =
            self.meshlib_exact_source_halfedge_key_candidates(operand, source_halfedge_key)
        {
            diagnostics.exact_key_candidates = key_candidates.len();
            self.extend_oriented_source_candidates(
                &mut candidates,
                key_candidates,
                "source-halfedge-key",
                "source-halfedge-key-sym",
                source_halfedge,
                fallback_edge,
            );
        } else if let Some(key_candidates) =
            self.meshlib_same_source_edge_key_candidates(operand, source_halfedge_key)
        {
            diagnostics.same_edge_key_candidates = key_candidates.len();
            self.extend_oriented_source_candidates(
                &mut candidates,
                key_candidates,
                "source-halfedge-key-edge",
                "source-halfedge-key-edge-sym",
                source_halfedge,
                fallback_edge,
            );
        }
        if let Some(halfedge_candidates) =
            self.meshlib_source_halfedge_candidates(operand, source_halfedge)
        {
            diagnostics.halfedge_candidates = halfedge_candidates.len();
            self.extend_oriented_source_candidates(
                &mut candidates,
                halfedge_candidates,
                "source-halfedge",
                "source-halfedge-sym",
                source_halfedge,
                fallback_edge,
            );
        }
        if let Some(edge_candidates) = self.meshlib_source_edge_candidates(operand, source_edge) {
            diagnostics.source_edge_candidates = edge_candidates.len();
            self.extend_oriented_source_candidates(
                &mut candidates,
                edge_candidates,
                "source-edge",
                "source-edge-sym",
                None,
                fallback_edge,
            );
        }
        let topology_candidates = self.topology_edge_candidates_for_directed_edge(fallback_edge);
        diagnostics.topology_candidates = topology_candidates.len();
        extend_unique_labeled_candidates(&mut candidates, topology_candidates, "topology-fallback");
        diagnostics.total_candidates = candidates.len();
        ExactMeshlibNearStitchSourceLookup {
            candidates,
            diagnostics,
        }
    }

    fn extend_oriented_source_candidates(
        &self,
        candidates: &mut Vec<ExactMeshlibNearStitchCandidate>,
        incoming: Vec<ExactMeshlibNearStitchSourceCandidate>,
        source: &'static str,
        sym_source: &'static str,
        key: Option<usize>,
        fallback_edge: [usize; 2],
    ) {
        for incoming in incoming {
            push_unique_labeled_candidate(
                candidates,
                incoming.edge,
                source,
                key,
                incoming.source_edge,
            );
            let sym = ExactHalfEdgeTopology::sym(incoming.edge);
            if self.topology.origin(incoming.edge) != Some(fallback_edge[0])
                && self.topology.origin(sym) == Some(fallback_edge[0])
            {
                push_unique_labeled_candidate(
                    candidates,
                    sym,
                    sym_source,
                    key,
                    incoming.source_edge.map(super::super::reverse_edge),
                );
            }
        }
    }
}

fn source_key_edges_match(stored: [usize; 2], requested: [usize; 2]) -> bool {
    stored == requested || stored == super::super::reverse_edge(requested)
}
