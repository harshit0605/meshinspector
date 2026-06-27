use super::super::super::super::exact_boolean::ExactBooleanOperand;
use super::super::super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::super::super::output_topology::OutputFaceTopology;
use super::super::super::source_records::ExactMeshlibPreparedSourceRecord;
use super::super::translate_stitched_record;
use super::SourcePreparedTopology;
use std::collections::{BTreeMap, BTreeSet};

impl SourcePreparedTopology {
    pub(in crate::spatial::exact_splice_apply::copied_edges) fn initial_edge_map(
        &self,
        output: &OutputFaceTopology,
        incoming_operand: ExactBooleanOperand,
        flip_orientation: bool,
    ) -> BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId> {
        let mut edge_map = BTreeMap::new();
        if output.meshlib_use_source_edge_identity {
            for ((operand, source_edge_index), output_edge) in
                &output.meshlib_mapped_contour_edge_indices
            {
                if *operand != incoming_operand {
                    continue;
                }
                if output.topology.left(*output_edge).is_some() {
                    continue;
                }
                if let Some(source_contour_edge) =
                    self.part_contour_edge_for_cut_index(*source_edge_index, flip_orientation)
                {
                    edge_map.insert(source_contour_edge, *output_edge);
                    edge_map.insert(
                        ExactHalfEdgeTopology::sym(source_contour_edge),
                        ExactHalfEdgeTopology::sym(*output_edge),
                    );
                }
            }
        }
        for ((operand, source_edge), output_edge) in &output.meshlib_mapped_contour_edges {
            if *operand != incoming_operand {
                continue;
            }
            if output.topology.left(*output_edge).is_some() {
                continue;
            }
            if let Some(source_contour_edge) =
                self.part_contour_edge_for_vertices(*source_edge, flip_orientation)
            {
                edge_map.entry(source_contour_edge).or_insert(*output_edge);
                edge_map
                    .entry(ExactHalfEdgeTopology::sym(source_contour_edge))
                    .or_insert(ExactHalfEdgeTopology::sym(*output_edge));
            }
        }
        edge_map
    }

    pub(super) fn cut_edge_ids(
        &self,
        cut_mesh: &ExactCutMeshResult,
    ) -> Vec<Option<ExactHalfEdgeId>> {
        let mut seen = BTreeMap::<[usize; 2], usize>::new();
        cut_mesh
            .cut_edges
            .iter()
            .map(|edge| {
                let occurrence = seen.entry(*edge).or_default();
                let candidates = self.directed_edge_candidates(*edge);
                let result = candidates
                    .get(*occurrence)
                    .copied()
                    .or_else(|| candidates.first().copied());
                *occurrence += 1;
                result
            })
            .collect()
    }

    fn source_contour_edge_for_cut_index(
        &self,
        source_edge_index: usize,
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let indexed = self
            .cut_edge_id(source_edge_index)
            .map(|edge| self.source_contour_edge(edge, flip_orientation));
        if indexed.is_some_and(|edge| self.is_meshlib_prepared_contour_side(edge, flip_orientation))
        {
            return indexed;
        }
        let edge = *self.cut_edges.get(source_edge_index)?;
        let candidates = self
            .directed_edge_candidates(edge)
            .into_iter()
            .map(|edge| self.source_contour_edge(edge, flip_orientation))
            .collect::<Vec<_>>();
        candidates
            .iter()
            .copied()
            .find(|edge| self.is_meshlib_prepared_contour_side(*edge, flip_orientation))
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|edge| self.has_meshlib_prepared_face_side(*edge, flip_orientation))
            })
            .or(indexed)
    }

    pub(in crate::spatial::exact_splice_apply::copied_edges) fn part_contour_edge_for_cut_index(
        &self,
        source_edge_index: usize,
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let indexed = self
            .cut_edge_id(source_edge_index)
            .map(|edge| self.source_contour_edge(edge, flip_orientation));
        if indexed.is_some_and(|edge| self.is_meshlib_prepared_contour_side(edge, flip_orientation))
        {
            return indexed;
        }
        let edge = *self.cut_edges.get(source_edge_index)?;
        self.directed_edge_candidates(edge)
            .into_iter()
            .map(|edge| self.source_contour_edge(edge, flip_orientation))
            .find(|edge| self.is_meshlib_prepared_contour_side(*edge, flip_orientation))
    }

    pub(in crate::spatial::exact_splice_apply::copied_edges) fn part_contour_edge_for_vertices(
        &self,
        edge: [usize; 2],
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        self.directed_edge_candidates(edge)
            .into_iter()
            .map(|edge| self.source_contour_edge(edge, flip_orientation))
            .find(|edge| self.is_meshlib_prepared_contour_side(*edge, flip_orientation))
    }

    pub(in crate::spatial::exact_splice_apply::copied_edges) fn oriented_contour_vertex_maps(
        &self,
        contour_vertex_maps: &[([usize; 2], [usize; 2])],
        source_edge_indices: &[Option<usize>],
        flip_orientation: bool,
    ) -> Vec<([usize; 2], [usize; 2])> {
        contour_vertex_maps
            .iter()
            .enumerate()
            .filter_map(|(index, (source_edge, output_edge))| {
                let oriented_source_edge = self
                    .source_edge_index_part_contour_vertices(
                        source_edge_indices.get(index).copied().flatten(),
                        flip_orientation,
                    )
                    .or_else(|| {
                        self.part_contour_edge_for_vertices(*source_edge, flip_orientation)
                            .and_then(|edge| self.source_vertices_for_edge(edge))
                    })?;
                Some((oriented_source_edge, *output_edge))
            })
            .collect()
    }

    fn source_edge_index_part_contour_vertices(
        &self,
        source_edge_index: Option<usize>,
        flip_orientation: bool,
    ) -> Option<[usize; 2]> {
        let source_edge =
            self.part_contour_edge_for_cut_index(source_edge_index?, flip_orientation)?;
        self.source_vertices_for_edge(source_edge)
    }

    pub(in crate::spatial::exact_splice_apply::copied_edges) fn register_mapped_contour_source_records(
        &self,
        output: &mut OutputFaceTopology,
        incoming_operand: ExactBooleanOperand,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        face_map: &[Option<usize>],
        flip_orientation: bool,
    ) -> Result<(), &'static str> {
        if output.meshlib_use_source_edge_identity {
            let mut contour_edges = output
                .meshlib_mapped_contour_edge_indices
                .iter()
                .filter_map(|((operand, edge_index), _)| {
                    (*operand == incoming_operand).then_some(*edge_index)
                })
                .collect::<BTreeSet<_>>();
            contour_edges.extend(0..self.cut_edges.len());
            for edge_index in contour_edges {
                let Some(source_contour_edge) =
                    self.source_contour_edge_for_cut_index(edge_index, flip_orientation)
                else {
                    continue;
                };
                let Some(record) = translate_stitched_record(
                    self,
                    source_contour_edge,
                    edge_map,
                    face_map,
                    flip_orientation,
                ) else {
                    continue;
                };
                output.register_meshlib_prepared_source_record_by_index(
                    incoming_operand,
                    edge_index,
                    record,
                );
                if let Some(edge) = self.cut_edges.get(edge_index).copied() {
                    output.register_meshlib_prepared_source_record(incoming_operand, edge, record);
                }
            }
        }
        let contour_edges = output
            .meshlib_mapped_contour_edges
            .keys()
            .filter_map(|(operand, edge)| (*operand == incoming_operand).then_some(*edge))
            .collect::<Vec<_>>();
        for edge in contour_edges {
            let Some(source_edge) = self.directed_edge_candidates(edge).first().copied() else {
                continue;
            };
            let Some(record) =
                translate_stitched_record(self, source_edge, edge_map, face_map, flip_orientation)
            else {
                continue;
            };
            output.register_meshlib_prepared_source_record(incoming_operand, edge, record);
        }
        Ok(())
    }

    pub(in crate::spatial::exact_splice_apply::copied_edges) fn mapped_contour_source_record_replays(
        &self,
        output: &OutputFaceTopology,
        incoming_operand: ExactBooleanOperand,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        face_map: &[Option<usize>],
        flip_orientation: bool,
    ) -> Result<Vec<(ExactHalfEdgeId, ExactMeshlibPreparedSourceRecord)>, &'static str> {
        let mut replays = Vec::new();
        if output.meshlib_use_source_edge_identity {
            let contour_edges = output
                .meshlib_mapped_contour_edge_indices
                .iter()
                .filter_map(|((operand, edge_index), _)| {
                    (*operand == incoming_operand).then_some(*edge_index)
                })
                .collect::<Vec<_>>();
            for edge_index in contour_edges {
                let Some(source_contour_edge) =
                    self.source_contour_edge_for_cut_index(edge_index, flip_orientation)
                else {
                    continue;
                };
                let Some(target) = self.map_edge_like_meshlib(source_contour_edge, edge_map) else {
                    continue;
                };
                if output.topology.left(target).is_some() {
                    continue;
                }
                let Some(record) = translate_stitched_record(
                    self,
                    source_contour_edge,
                    edge_map,
                    face_map,
                    flip_orientation,
                ) else {
                    continue;
                };
                replays.push((target, record));
            }
        }
        let contour_edges = output
            .meshlib_mapped_contour_edges
            .keys()
            .filter_map(|(operand, edge)| (*operand == incoming_operand).then_some(*edge))
            .collect::<Vec<_>>();
        for edge in contour_edges {
            let Some(source_edge) = self.directed_edge_candidates(edge).first().copied() else {
                continue;
            };
            let Some(target) = self.map_edge_like_meshlib(source_edge, edge_map) else {
                continue;
            };
            if output.topology.left(target).is_some() {
                continue;
            }
            let Some(record) =
                translate_stitched_record(self, source_edge, edge_map, face_map, flip_orientation)
            else {
                continue;
            };
            if !replays.iter().any(|(stored, _)| *stored == target) {
                replays.push((target, record));
            }
        }
        Ok(replays)
    }

    fn source_contour_edge(
        &self,
        source_edge: ExactHalfEdgeId,
        flip_orientation: bool,
    ) -> ExactHalfEdgeId {
        let source_sym = ExactHalfEdgeTopology::sym(source_edge);
        [source_edge, source_sym]
            .into_iter()
            .find(|candidate| self.is_meshlib_prepared_contour_side(*candidate, flip_orientation))
            .or_else(|| {
                [source_edge, source_sym].into_iter().find(|candidate| {
                    if flip_orientation {
                        self.topology.left(*candidate).is_none()
                    } else {
                        self.topology.right(*candidate).is_none()
                    }
                })
            })
            .unwrap_or(if flip_orientation {
                source_sym
            } else {
                source_edge
            })
    }

    fn is_meshlib_prepared_contour_side(
        &self,
        edge: ExactHalfEdgeId,
        flip_orientation: bool,
    ) -> bool {
        if flip_orientation {
            self.topology.left(edge).is_none() && self.topology.right(edge).is_some()
        } else {
            self.topology.right(edge).is_none() && self.topology.left(edge).is_some()
        }
    }

    fn has_meshlib_prepared_face_side(
        &self,
        edge: ExactHalfEdgeId,
        flip_orientation: bool,
    ) -> bool {
        if flip_orientation {
            self.topology.right(edge).is_some()
        } else {
            self.topology.left(edge).is_some()
        }
    }
}
