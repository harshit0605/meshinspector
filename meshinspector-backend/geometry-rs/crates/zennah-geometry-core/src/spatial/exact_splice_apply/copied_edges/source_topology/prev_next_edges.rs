use super::SourcePreparedTopology;
use crate::spatial::exact_boolean::ExactBooleanOperand;
use crate::spatial::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use crate::spatial::exact_splice_apply::output_topology::{
    ExactMeshlibCopiedPrevNextEdgeUpdate, OutputFaceTopology,
};
use std::collections::BTreeMap;

struct ContourUpdateContext<'a> {
    output: &'a OutputFaceTopology,
    edge_map: &'a BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    face_map: &'a [Option<usize>],
    from_mapped_edges: &'a [usize],
    flip_orientation: bool,
}

impl SourcePreparedTopology {
    pub(in crate::spatial::exact_splice_apply::copied_edges) fn mapped_contour_prev_next_edges(
        &self,
        output: &OutputFaceTopology,
        incoming_operand: ExactBooleanOperand,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        face_map: &[Option<usize>],
        flip_orientation: bool,
    ) -> Vec<ExactMeshlibCopiedPrevNextEdgeUpdate> {
        let contour_edges =
            self.mapped_contour_edges(output, incoming_operand, edge_map, flip_orientation);
        let from_mapped_edges = contour_edges
            .iter()
            .map(|(source_edge, _)| undirected_edge_key(*source_edge))
            .collect::<Vec<_>>();
        let context = ContourUpdateContext {
            output,
            edge_map,
            face_map,
            from_mapped_edges: &from_mapped_edges,
            flip_orientation,
        };
        let mut pairs = Vec::new();
        for (source_edge, target_edge) in contour_edges {
            self.push_next_contour_update(&context, &mut pairs, source_edge, target_edge);
            self.push_previous_contour_update(&context, &mut pairs, source_edge, target_edge);
        }
        pairs
    }

    fn push_next_contour_update(
        &self,
        context: &ContourUpdateContext<'_>,
        pairs: &mut Vec<ExactMeshlibCopiedPrevNextEdgeUpdate>,
        source_edge: ExactHalfEdgeId,
        target_edge: ExactHalfEdgeId,
    ) {
        let mut next_source = ExactHalfEdgeTopology::sym(source_edge);
        for _ in 0..=self.edge_vertices.len() {
            next_source = if context.flip_orientation {
                self.topology.prev(next_source)
            } else {
                self.topology.next(next_source)
            };
            let copied_face = if context.flip_orientation {
                self.topology.right(next_source)
            } else {
                self.topology.left(next_source)
            };
            if face_is_mapped(context.face_map, copied_face)
                || next_source == ExactHalfEdgeTopology::sym(source_edge)
            {
                break;
            }
        }
        if context
            .from_mapped_edges
            .contains(&undirected_edge_key(next_source))
        {
            return;
        }
        if let Some(mapped_next) = self.map_edge_like_meshlib(next_source, context.edge_map) {
            push_unique_update(
                pairs,
                ExactMeshlibCopiedPrevNextEdgeUpdate {
                    previous: context
                        .output
                        .topology
                        .prev(ExactHalfEdgeTopology::sym(target_edge)),
                    next: mapped_next,
                    source_contour_edge: source_edge,
                    target_contour_edge: target_edge,
                    walked_source_edge: next_source,
                    update_kind: "next",
                },
            );
        }
    }

    fn push_previous_contour_update(
        &self,
        context: &ContourUpdateContext<'_>,
        pairs: &mut Vec<ExactMeshlibCopiedPrevNextEdgeUpdate>,
        source_edge: ExactHalfEdgeId,
        target_edge: ExactHalfEdgeId,
    ) {
        let mut previous_source = source_edge;
        for _ in 0..=self.edge_vertices.len() {
            previous_source = if context.flip_orientation {
                self.topology.next(previous_source)
            } else {
                self.topology.prev(previous_source)
            };
            let copied_face = if context.flip_orientation {
                self.topology.left(previous_source)
            } else {
                self.topology.right(previous_source)
            };
            if face_is_mapped(context.face_map, copied_face) || previous_source == source_edge {
                break;
            }
        }
        if context
            .from_mapped_edges
            .contains(&undirected_edge_key(previous_source))
        {
            return;
        }
        if let Some(mapped_previous) = self.map_edge_like_meshlib(previous_source, context.edge_map)
        {
            push_unique_update(
                pairs,
                ExactMeshlibCopiedPrevNextEdgeUpdate {
                    previous: mapped_previous,
                    next: context.output.topology.next(target_edge),
                    source_contour_edge: source_edge,
                    target_contour_edge: target_edge,
                    walked_source_edge: previous_source,
                    update_kind: "previous",
                },
            );
        }
    }

    fn mapped_contour_edges(
        &self,
        output: &OutputFaceTopology,
        incoming_operand: ExactBooleanOperand,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        flip_orientation: bool,
    ) -> Vec<(ExactHalfEdgeId, ExactHalfEdgeId)> {
        let mut contour_edges = Vec::new();
        if output.meshlib_use_source_edge_identity {
            for (operand, edge_index) in output.meshlib_mapped_contour_edge_indices.keys() {
                if *operand != incoming_operand {
                    continue;
                }
                let Some(source_edge) =
                    self.part_contour_edge_for_cut_index(*edge_index, flip_orientation)
                else {
                    continue;
                };
                let Some(target_edge) = self.map_edge_like_meshlib(source_edge, edge_map) else {
                    continue;
                };
                push_unique_pair(&mut contour_edges, (source_edge, target_edge));
            }
            return contour_edges;
        }
        for (operand, source_vertices) in output.meshlib_mapped_contour_edges.keys() {
            if *operand != incoming_operand {
                continue;
            }
            let Some(source_edge) =
                self.part_contour_edge_for_vertices(*source_vertices, flip_orientation)
            else {
                continue;
            };
            let Some(target_edge) = self.map_edge_like_meshlib(source_edge, edge_map) else {
                continue;
            };
            push_unique_pair(&mut contour_edges, (source_edge, target_edge));
        }
        contour_edges
    }
}

fn undirected_edge_key(edge: ExactHalfEdgeId) -> usize {
    edge.0 & !1
}

fn face_is_mapped(face_map: &[Option<usize>], face: Option<usize>) -> bool {
    face.and_then(|face| face_map.get(face).copied().flatten())
        .is_some()
}

fn push_unique_pair(
    pairs: &mut Vec<(ExactHalfEdgeId, ExactHalfEdgeId)>,
    pair: (ExactHalfEdgeId, ExactHalfEdgeId),
) {
    if !pairs.contains(&pair) {
        pairs.push(pair);
    }
}

fn push_unique_update(
    updates: &mut Vec<ExactMeshlibCopiedPrevNextEdgeUpdate>,
    update: ExactMeshlibCopiedPrevNextEdgeUpdate,
) {
    if !updates
        .iter()
        .any(|stored| stored.previous == update.previous && stored.next == update.next)
    {
        updates.push(update);
    }
}
