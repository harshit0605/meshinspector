use super::super::super::super::exact_boolean::ExactBooleanOperand;
use super::super::super::super::exact_halfedge::ExactHalfEdgeId;
use super::super::super::super::exact_meshlib_near_stitch::ExactMeshlibSourceHalfedgeKey;
use super::super::super::output_topology::OutputFaceTopology;
use super::SourcePreparedTopology;
use std::collections::BTreeMap;

impl SourcePreparedTopology {
    pub(in crate::spatial::exact_splice_apply::copied_edges) fn register_source_halfedge_candidates(
        &self,
        output: &mut OutputFaceTopology,
        incoming_operand: ExactBooleanOperand,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    ) {
        for (source_edge, output_edge) in edge_map {
            let source_vertices = self.source_vertices_for_edge(*source_edge);
            let source_keys = self.source_halfedge_keys(*source_edge);
            if source_keys.is_empty() {
                output.register_meshlib_source_halfedge(
                    incoming_operand,
                    *source_edge,
                    None,
                    source_vertices,
                    *output_edge,
                );
            } else {
                for source_key in source_keys {
                    output.register_meshlib_source_halfedge(
                        incoming_operand,
                        *source_edge,
                        Some(source_key),
                        source_vertices,
                        *output_edge,
                    );
                }
            }
            if let Some(source_vertices) = source_vertices {
                output.register_meshlib_source_edge(
                    incoming_operand,
                    source_vertices,
                    *output_edge,
                );
            }
        }
    }

    pub(super) fn source_halfedge_keys(
        &self,
        source_edge: ExactHalfEdgeId,
    ) -> Vec<ExactMeshlibSourceHalfedgeKey> {
        let Some(edge) = self.source_vertices_for_edge(source_edge) else {
            return Vec::new();
        };
        let mut keys = Vec::new();
        if let Some(face) = self.topology.left(source_edge) {
            keys.push(ExactMeshlibSourceHalfedgeKey { face, edge });
        }
        if let Some(face) = self.topology.right(source_edge) {
            let right_key = ExactMeshlibSourceHalfedgeKey { face, edge };
            if !keys.contains(&right_key) {
                keys.push(right_key);
            }
        }
        keys
    }
}
