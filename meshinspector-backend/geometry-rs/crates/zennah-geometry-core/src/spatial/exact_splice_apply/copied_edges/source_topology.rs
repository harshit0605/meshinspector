use super::super::super::exact_boolean::ExactBooleanOperand;
use super::super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::super::output_topology::OutputFaceTopology;
use super::super::source_records::ExactMeshlibPreparedSourceRecord;
use super::translate_stitched_record;
use std::collections::BTreeMap;

mod keys;
#[cfg(test)]
mod tests;

pub(super) struct SourcePreparedTopology {
    pub(super) topology: ExactHalfEdgeTopology,
    directed_edges: BTreeMap<[usize; 2], Vec<ExactHalfEdgeId>>,
    cut_edges: Vec<[usize; 2]>,
    cut_edge_ids: Vec<Option<ExactHalfEdgeId>>,
    pub(super) edge_vertices: BTreeMap<ExactHalfEdgeId, [usize; 2]>,
    pub(super) face_edges: BTreeMap<usize, Vec<ExactHalfEdgeId>>,
    pub(super) base_edges: Vec<ExactHalfEdgeId>,
}

impl SourcePreparedTopology {
    pub(super) fn from_cut_mesh(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
    ) -> Result<Self, &'static str> {
        Self::from_cut_mesh_with_orientation(cut_mesh, prepared_faces, false)
    }

    pub(super) fn from_cut_mesh_with_orientation(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
        flip_orientation: bool,
    ) -> Result<Self, &'static str> {
        let mut source = Self {
            topology: ExactHalfEdgeTopology::new(),
            directed_edges: BTreeMap::new(),
            cut_edges: cut_mesh.cut_edges.clone(),
            cut_edge_ids: Vec::new(),
            edge_vertices: BTreeMap::new(),
            face_edges: BTreeMap::new(),
            base_edges: Vec::new(),
        };
        let mut undirected_edges =
            BTreeMap::<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>::new();
        for face_index in prepared_faces {
            let Some(face) = cut_mesh.faces.get(*face_index) else {
                continue;
            };
            let mut face = [face[0] as usize, face[1] as usize, face[2] as usize];
            if flip_orientation {
                face.swap(1, 2);
            }
            let edges = [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]];
            let mut face_edge_ids = Vec::with_capacity(3);
            for edge in edges {
                face_edge_ids.push(source.edge_id(&mut undirected_edges, edge));
            }
            link_face_ring(&mut source.topology, &face_edge_ids, edges)?;
            source
                .topology
                .set_left(face_edge_ids[0], Some(*face_index))?;
            source.face_edges.insert(*face_index, face_edge_ids);
        }
        source.cut_edge_ids = source.cut_edge_ids(cut_mesh);
        Ok(source)
    }

    fn edge_id(
        &mut self,
        undirected_edges: &mut BTreeMap<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>,
        edge: [usize; 2],
    ) -> ExactHalfEdgeId {
        let key = ordered_edge(edge);
        if let Some(edge_ids) = undirected_edges.get(&key) {
            for (stored_edge, edge_id) in edge_ids {
                let candidate = if *stored_edge == edge {
                    *edge_id
                } else {
                    ExactHalfEdgeTopology::sym(*edge_id)
                };
                if self.topology.left(candidate).is_none() {
                    self.register_directed_edge(edge, candidate);
                    return candidate;
                }
            }
        }

        let edge_id = self.topology.make_edge(Some(edge[0]), Some(edge[1]));
        self.base_edges.push(edge_id);
        undirected_edges
            .entry(key)
            .or_default()
            .push((edge, edge_id));
        self.register_directed_edge(edge, edge_id);
        edge_id
    }

    fn register_directed_edge(&mut self, edge: [usize; 2], edge_id: ExactHalfEdgeId) {
        self.directed_edges.entry(edge).or_default().push(edge_id);
        self.edge_vertices.insert(edge_id, edge);
        self.edge_vertices
            .insert(ExactHalfEdgeTopology::sym(edge_id), reverse_edge(edge));
    }

    fn directed_edge_candidates(&self, edge: [usize; 2]) -> Vec<ExactHalfEdgeId> {
        let mut candidates = Vec::new();
        if let Some(edges) = self.directed_edges.get(&edge) {
            candidates.extend(edges.iter().copied());
        }
        if let Some(edges) = self.directed_edges.get(&reverse_edge(edge)) {
            candidates.extend(edges.iter().copied().map(ExactHalfEdgeTopology::sym));
        }
        candidates
    }

    pub(super) fn initial_edge_map(
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
                if let Some(source_contour_edge) =
                    self.source_contour_edge_for_cut_index(*source_edge_index, flip_orientation)
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
            if let Some(source_edge_id) =
                self.directed_edge_candidates(*source_edge).first().copied()
            {
                let source_contour_edge =
                    self.source_contour_edge(source_edge_id, flip_orientation);
                edge_map.entry(source_contour_edge).or_insert(*output_edge);
                edge_map
                    .entry(ExactHalfEdgeTopology::sym(source_contour_edge))
                    .or_insert(ExactHalfEdgeTopology::sym(*output_edge));
            }
        }
        edge_map
    }

    pub(super) fn cut_edge_id(&self, source_edge_index: usize) -> Option<ExactHalfEdgeId> {
        self.cut_edge_ids.get(source_edge_index).copied().flatten()
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

    fn cut_edge_ids(&self, cut_mesh: &ExactCutMeshResult) -> Vec<Option<ExactHalfEdgeId>> {
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

    pub(super) fn output_vertices_for_edge(
        &self,
        source_edge: ExactHalfEdgeId,
        vertex_map: &[Option<usize>],
    ) -> Option<[usize; 2]> {
        let source_vertices = self.edge_vertices.get(&source_edge)?;
        Some([
            *vertex_map.get(source_vertices[0])?.as_ref()?,
            *vertex_map.get(source_vertices[1])?.as_ref()?,
        ])
    }

    pub(super) fn source_vertices_for_edge(
        &self,
        source_edge: ExactHalfEdgeId,
    ) -> Option<[usize; 2]> {
        self.edge_vertices.get(&source_edge).copied()
    }

    pub(super) fn oriented_contour_vertex_maps(
        &self,
        contour_vertex_maps: &[([usize; 2], [usize; 2])],
        source_edge_indices: &[Option<usize>],
        flip_orientation: bool,
    ) -> Vec<([usize; 2], [usize; 2])> {
        contour_vertex_maps
            .iter()
            .enumerate()
            .map(|(index, (source_edge, output_edge))| {
                let oriented_source_edge = self
                    .source_edge_index_contour_vertices(
                        source_edge_indices.get(index).copied().flatten(),
                        flip_orientation,
                    )
                    .or_else(|| {
                        self.directed_edge_candidates(*source_edge)
                            .first()
                            .copied()
                            .map(|edge| self.source_contour_edge(edge, flip_orientation))
                            .and_then(|edge| self.source_vertices_for_edge(edge))
                    })
                    .unwrap_or(*source_edge);
                (oriented_source_edge, *output_edge)
            })
            .collect()
    }

    fn source_edge_index_contour_vertices(
        &self,
        source_edge_index: Option<usize>,
        flip_orientation: bool,
    ) -> Option<[usize; 2]> {
        let source_edge =
            self.source_contour_edge_for_cut_index(source_edge_index?, flip_orientation)?;
        self.source_vertices_for_edge(source_edge)
    }

    pub(super) fn mapped_face_edge(
        &self,
        output: &OutputFaceTopology,
        face: usize,
        output_face: usize,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let stored_edges = self.face_edges.get(&face)?;
        let ring_edges = stored_edges
            .first()
            .and_then(|edge| self.topology.left_ring_edges(*edge).ok());
        let mut fallback = None;
        for edge in ring_edges.as_deref().unwrap_or(stored_edges) {
            let Some(mapped_edge) = self.map_edge_like_meshlib(*edge, edge_map) else {
                continue;
            };
            let face_edge = if flip_orientation {
                ExactHalfEdgeTopology::sym(mapped_edge)
            } else {
                mapped_edge
            };
            if fallback.is_none() {
                fallback = Some(face_edge);
            }
            if output
                .topology
                .validate_meshlib_face_left_ring(face_edge, output_face)
                .is_ok()
            {
                return Some(face_edge);
            }
        }
        fallback
    }

    pub(super) fn map_edge_like_meshlib(
        &self,
        source_edge: ExactHalfEdgeId,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    ) -> Option<ExactHalfEdgeId> {
        let undirected = ExactHalfEdgeId(source_edge.0 & !1);
        let mapped = edge_map.get(&undirected).copied().or_else(|| {
            let mapped_sym = edge_map.get(&ExactHalfEdgeTopology::sym(undirected))?;
            Some(ExactHalfEdgeTopology::sym(*mapped_sym))
        })?;
        if source_edge.0 % 2 == 1 {
            Some(ExactHalfEdgeTopology::sym(mapped))
        } else {
            Some(mapped)
        }
    }

    pub(super) fn register_mapped_contour_source_records(
        &self,
        output: &mut OutputFaceTopology,
        incoming_operand: ExactBooleanOperand,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        face_map: &[Option<usize>],
        flip_orientation: bool,
    ) -> Result<(), &'static str> {
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
                let Some(record) = translate_stitched_record(
                    self,
                    source_contour_edge,
                    edge_map,
                    face_map,
                    flip_orientation,
                ) else {
                    return Err("missing MeshLib prepared source record");
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
                return Err("missing MeshLib prepared source record");
            };
            output.register_meshlib_prepared_source_record(incoming_operand, edge, record);
        }
        Ok(())
    }

    pub(super) fn mapped_contour_source_record_replays(
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
                let Some(record) = translate_stitched_record(
                    self,
                    source_contour_edge,
                    edge_map,
                    face_map,
                    flip_orientation,
                ) else {
                    return Err("missing MeshLib prepared source record");
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
            let Some(record) =
                translate_stitched_record(self, source_edge, edge_map, face_map, flip_orientation)
            else {
                return Err("missing MeshLib prepared source record");
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
