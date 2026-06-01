use super::super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::ExactMeshlibSourceHalfedgeKey;
use std::collections::{BTreeMap, BTreeSet};

mod helpers;
pub(super) use helpers::ordered_edge;
use helpers::{
    copied_vertex_map, cut_edge_ids, link_face_ring, mapped_edge_vertices, operand_edge_id,
    push_unique_edge_id, reverse_edge, undirected_edge_key,
};

pub(super) struct OperandTopology {
    pub(super) topology: ExactHalfEdgeTopology,
    directed_edges: BTreeMap<[usize; 2], Vec<ExactHalfEdgeId>>,
    edge_vertices: Vec<Option<[usize; 2]>>,
    source_edge_vertices: Vec<Option<[usize; 2]>>,
    source_cut_edge_ids: Vec<Option<ExactHalfEdgeId>>,
    region_faces: BTreeSet<usize>,
}

pub(super) enum SourceEdgeWalkResult {
    Edge(ExactHalfEdgeId),
    BlockedOpenSide,
    Missing,
}

impl OperandTopology {
    pub(super) fn from_output(assembly: &ExactBooleanAssemblyResult) -> Self {
        Self::from_faces(assembly, |_| true)
    }

    pub(super) fn from_assembly(
        assembly: &ExactBooleanAssemblyResult,
        operand: ExactBooleanOperand,
    ) -> Self {
        Self::from_faces(assembly, |source| source.operand == operand)
    }

    fn from_faces(
        assembly: &ExactBooleanAssemblyResult,
        mut include_source: impl FnMut(
            &super::super::exact_boolean::ExactBooleanOutputFaceSource,
        ) -> bool,
    ) -> Self {
        let mut topology = ExactHalfEdgeTopology::new();
        let mut directed_edges = BTreeMap::<[usize; 2], Vec<ExactHalfEdgeId>>::new();
        let mut undirected_edges =
            BTreeMap::<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>::new();
        let mut edge_vertices = Vec::new();
        let mut source_edge_vertices = Vec::new();
        let mut region_faces = BTreeSet::new();
        let mut local_face = 0;

        for (face, source) in assembly.faces.iter().zip(&assembly.face_sources) {
            if !include_source(source) {
                continue;
            }
            let face = [face[0] as usize, face[1] as usize, face[2] as usize];
            let edges = [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]];
            let mut face_edge_ids = Vec::with_capacity(3);
            for edge in edges {
                let edge_id = operand_edge_id(
                    &mut topology,
                    &mut undirected_edges,
                    &mut edge_vertices,
                    &mut source_edge_vertices,
                    edge,
                    Some(edge),
                    Some(edge),
                );
                directed_edges.entry(edge).or_default().push(edge_id);
                face_edge_ids.push(edge_id);
            }
            if link_face_ring(&mut topology, &face_edge_ids, edges).is_ok() {
                let _ = topology.set_left(face_edge_ids[0], Some(local_face));
                region_faces.insert(local_face);
            }
            local_face += 1;
        }

        Self {
            topology,
            directed_edges,
            edge_vertices,
            source_edge_vertices,
            source_cut_edge_ids: Vec::new(),
            region_faces,
        }
    }

    pub(super) fn from_cut_mesh(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
        vertex_map: &[Option<usize>],
        contour_vertex_maps: &[([usize; 2], [usize; 2])],
        contour_vertex_map_source_indices: &[Option<usize>],
        first_virtual_vertex: usize,
        flip_orientation: bool,
    ) -> Self {
        let contour_vertex_maps = Self::oriented_contour_vertex_maps(
            cut_mesh,
            prepared_faces,
            contour_vertex_maps,
            contour_vertex_map_source_indices,
            flip_orientation,
        );
        let copied_vertex_map = copied_vertex_map(
            vertex_map,
            cut_mesh,
            prepared_faces,
            first_virtual_vertex,
            &contour_vertex_maps,
        );
        Self::from_cut_mesh_with_vertex_map(cut_mesh, prepared_faces, &copied_vertex_map)
    }

    #[cfg(test)]
    pub(super) fn from_cut_mesh_with_fresh_vertex_map(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
        contour_vertex_maps: &[([usize; 2], [usize; 2])],
        contour_vertex_map_source_indices: &[Option<usize>],
        first_virtual_vertex: usize,
        flip_orientation: bool,
    ) -> Self {
        Self::from_cut_mesh_with_fresh_vertex_map_and_orientation(
            cut_mesh,
            prepared_faces,
            contour_vertex_maps,
            contour_vertex_map_source_indices,
            first_virtual_vertex,
            false,
            flip_orientation,
        )
    }

    pub(super) fn from_cut_mesh_with_fresh_vertex_map_and_orientation(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
        contour_vertex_maps: &[([usize; 2], [usize; 2])],
        contour_vertex_map_source_indices: &[Option<usize>],
        first_virtual_vertex: usize,
        source_flip_orientation: bool,
        contour_flip_orientation: bool,
    ) -> Self {
        let contour_vertex_maps = Self::oriented_contour_vertex_maps(
            cut_mesh,
            prepared_faces,
            contour_vertex_maps,
            contour_vertex_map_source_indices,
            contour_flip_orientation,
        );
        let copied_vertex_map = copied_vertex_map(
            &[],
            cut_mesh,
            prepared_faces,
            first_virtual_vertex,
            &contour_vertex_maps,
        );
        Self::from_cut_mesh_with_vertex_map_and_orientation(
            cut_mesh,
            prepared_faces,
            &copied_vertex_map,
            source_flip_orientation,
        )
    }

    fn from_cut_mesh_with_vertex_map(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
        copied_vertex_map: &[Option<usize>],
    ) -> Self {
        Self::from_cut_mesh_with_vertex_map_and_orientation(
            cut_mesh,
            prepared_faces,
            copied_vertex_map,
            false,
        )
    }

    fn from_cut_mesh_with_vertex_map_and_orientation(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
        copied_vertex_map: &[Option<usize>],
        flip_orientation: bool,
    ) -> Self {
        let mut topology = ExactHalfEdgeTopology::new();
        let mut directed_edges = BTreeMap::<[usize; 2], Vec<ExactHalfEdgeId>>::new();
        let mut undirected_edges =
            BTreeMap::<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>::new();
        let mut edge_vertices = Vec::new();
        let mut source_edge_vertices = Vec::new();
        let region_faces = prepared_faces.iter().copied().collect::<BTreeSet<_>>();

        for face_index in prepared_faces {
            let Some(face) = cut_mesh.faces.get(*face_index) else {
                continue;
            };
            let mut source_face = [face[0] as usize, face[1] as usize, face[2] as usize];
            if flip_orientation {
                source_face.swap(1, 2);
            }
            let source_edges = [
                [source_face[0], source_face[1]],
                [source_face[1], source_face[2]],
                [source_face[2], source_face[0]],
            ];
            let mut face_edge_ids = Vec::with_capacity(3);
            for source_edge in source_edges {
                let edge_id = operand_edge_id(
                    &mut topology,
                    &mut undirected_edges,
                    &mut edge_vertices,
                    &mut source_edge_vertices,
                    source_edge,
                    mapped_edge_vertices(source_edge, copied_vertex_map),
                    Some(source_edge),
                );
                directed_edges.entry(source_edge).or_default().push(edge_id);
                face_edge_ids.push(edge_id);
            }
            if link_face_ring(&mut topology, &face_edge_ids, source_edges).is_ok() {
                let _ = topology.set_left(face_edge_ids[0], Some(*face_index));
            }
        }
        let source_cut_edge_ids = cut_edge_ids(cut_mesh, &directed_edges);

        Self {
            topology,
            directed_edges,
            edge_vertices,
            source_edge_vertices,
            source_cut_edge_ids,
            region_faces,
        }
    }

    fn oriented_contour_vertex_maps(
        cut_mesh: &ExactCutMeshResult,
        prepared_faces: &[usize],
        contour_vertex_maps: &[([usize; 2], [usize; 2])],
        source_edge_indices: &[Option<usize>],
        flip_orientation: bool,
    ) -> Vec<([usize; 2], [usize; 2])> {
        if contour_vertex_maps.is_empty() {
            return Vec::new();
        }
        let source = Self::source_identity_topology(cut_mesh, prepared_faces);
        contour_vertex_maps
            .iter()
            .enumerate()
            .map(|(index, (source_edge, output_edge))| {
                let oriented_source_edge = source
                    .oriented_contour_source_edge(
                        *source_edge,
                        source_edge_indices.get(index).copied().flatten(),
                        flip_orientation,
                    )
                    .unwrap_or(*source_edge);
                (oriented_source_edge, *output_edge)
            })
            .collect()
    }

    fn source_identity_topology(cut_mesh: &ExactCutMeshResult, prepared_faces: &[usize]) -> Self {
        let vertex_map = cut_mesh
            .vertices
            .iter()
            .enumerate()
            .map(|(index, _)| Some(index))
            .collect::<Vec<_>>();
        Self::from_cut_mesh_with_vertex_map(cut_mesh, prepared_faces, &vertex_map)
    }

    fn oriented_contour_source_edge(
        &self,
        source_edge: [usize; 2],
        source_edge_index: Option<usize>,
        flip_orientation: bool,
    ) -> Option<[usize; 2]> {
        source_edge_index
            .and_then(|index| self.source_contour_edge_by_source_index(index, flip_orientation))
            .and_then(|edge| self.source_directed_edge(edge))
            .or_else(|| {
                self.source_contour_edge(source_edge, flip_orientation)
                    .and_then(|edge| self.source_directed_edge(edge))
            })
    }

    pub(super) fn contour_boundary_edge(&self, edge: [usize; 2]) -> Option<ExactHalfEdgeId> {
        self.first_directed_face_edge(edge)
            .map(ExactHalfEdgeTopology::sym)
    }

    pub(super) fn first_directed_face_edge(&self, edge: [usize; 2]) -> Option<ExactHalfEdgeId> {
        self.directed_edges
            .get(&edge)
            .and_then(|edges| edges.first().copied())
    }

    pub(super) fn previous_unmapped_source_edge(
        &self,
        from_contour_edge: [usize; 2],
        mapped_edges: &BTreeSet<[usize; 2]>,
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let source = self.source_contour_edge(from_contour_edge, flip_orientation)?;
        let candidate = self.walk_previous_source_edge(source, flip_orientation);
        let edge = self.mapped_edge_key(candidate)?;
        (!mapped_edges.contains(&ordered_edge(edge))).then_some(candidate)
    }

    pub(super) fn previous_unmapped_source_edge_by_source_index(
        &self,
        from_source_edge: [usize; 2],
        from_source_edge_index: usize,
        mapped_source_edges: &BTreeSet<usize>,
        flip_orientation: bool,
    ) -> SourceEdgeWalkResult {
        let mut blocked_open_side = false;
        for source in self.source_contour_edge_candidates_from_directed_or_index(
            from_source_edge,
            from_source_edge_index,
            flip_orientation,
        ) {
            let candidate = self.walk_previous_source_edge(source, flip_orientation);
            if candidate == source {
                continue;
            }
            if !mapped_source_edges.contains(&undirected_edge_key(candidate)) {
                if self.connect_left(candidate, flip_orientation).is_none() {
                    return SourceEdgeWalkResult::Edge(candidate);
                }
                blocked_open_side = true;
            }
        }
        if blocked_open_side {
            SourceEdgeWalkResult::BlockedOpenSide
        } else {
            SourceEdgeWalkResult::Missing
        }
    }

    pub(super) fn next_unmapped_source_edge(
        &self,
        from_contour_edge: [usize; 2],
        mapped_edges: &BTreeSet<[usize; 2]>,
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let source = self.source_contour_edge(from_contour_edge, flip_orientation)?;
        let candidate = self.walk_next_source_edge(source, flip_orientation);
        let edge = self.mapped_edge_key(candidate)?;
        (!mapped_edges.contains(&ordered_edge(edge))).then_some(candidate)
    }

    pub(super) fn next_unmapped_source_edge_by_source_index(
        &self,
        from_source_edge: [usize; 2],
        from_source_edge_index: usize,
        mapped_source_edges: &BTreeSet<usize>,
        flip_orientation: bool,
    ) -> SourceEdgeWalkResult {
        let mut blocked_open_side = false;
        for source in self.source_contour_edge_candidates_from_directed_or_index(
            from_source_edge,
            from_source_edge_index,
            flip_orientation,
        ) {
            let candidate = self.walk_next_source_edge(source, flip_orientation);
            if candidate == ExactHalfEdgeTopology::sym(source) {
                continue;
            }
            if !mapped_source_edges.contains(&undirected_edge_key(candidate)) {
                if self.connect_right(candidate, flip_orientation).is_none() {
                    return SourceEdgeWalkResult::Edge(candidate);
                }
                blocked_open_side = true;
            }
        }
        if blocked_open_side {
            SourceEdgeWalkResult::BlockedOpenSide
        } else {
            SourceEdgeWalkResult::Missing
        }
    }

    pub(super) fn source_contour_edge(
        &self,
        edge: [usize; 2],
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let candidates = self.contour_edge_candidates(edge);
        candidates
            .iter()
            .copied()
            .find(|candidate| self.is_meshlib_prepared_contour_side(*candidate, flip_orientation))
            .or_else(|| {
                candidates.iter().copied().find(|candidate| {
                    if flip_orientation {
                        self.topology.left(*candidate).is_none()
                    } else {
                        self.topology.right(*candidate).is_none()
                    }
                })
            })
            .or_else(|| candidates.first().copied())
    }

    fn contour_edge_candidates(&self, edge: [usize; 2]) -> Vec<ExactHalfEdgeId> {
        let mut candidates = Vec::new();
        if let Some(edges) = self.directed_edges.get(&edge) {
            for edge_id in edges {
                push_unique_edge_id(&mut candidates, *edge_id);
                push_unique_edge_id(&mut candidates, ExactHalfEdgeTopology::sym(*edge_id));
            }
        }
        let reversed = reverse_edge(edge);
        if let Some(edges) = self.directed_edges.get(&reversed) {
            for edge_id in edges {
                push_unique_edge_id(&mut candidates, ExactHalfEdgeTopology::sym(*edge_id));
                push_unique_edge_id(&mut candidates, *edge_id);
            }
        }
        candidates
    }

    fn walk_previous_source_edge(
        &self,
        source: ExactHalfEdgeId,
        flip_orientation: bool,
    ) -> ExactHalfEdgeId {
        let mut candidate = source;
        for _ in 0..=self.edge_vertices.len() {
            candidate = if flip_orientation {
                self.topology.next(candidate)
            } else {
                self.topology.prev(candidate)
            };
            let face_in_region = if flip_orientation {
                self.topology.left(candidate)
            } else {
                self.topology.right(candidate)
            }
            .is_some_and(|face| self.region_faces.contains(&face));
            if face_in_region || candidate == source {
                return candidate;
            }
        }
        candidate
    }

    fn walk_next_source_edge(
        &self,
        source: ExactHalfEdgeId,
        flip_orientation: bool,
    ) -> ExactHalfEdgeId {
        let source_sym = ExactHalfEdgeTopology::sym(source);
        let mut candidate = source_sym;
        for _ in 0..=self.edge_vertices.len() {
            candidate = if flip_orientation {
                self.topology.prev(candidate)
            } else {
                self.topology.next(candidate)
            };
            let face_in_region = if flip_orientation {
                self.topology.right(candidate)
            } else {
                self.topology.left(candidate)
            }
            .is_some_and(|face| self.region_faces.contains(&face));
            if face_in_region || candidate == source_sym {
                return candidate;
            }
        }
        candidate
    }

    pub(super) fn directed_edge(&self, edge: ExactHalfEdgeId) -> Option<[usize; 2]> {
        self.edge_vertices.get(edge.0).copied().flatten()
    }

    pub(super) fn source_directed_edge(&self, edge: ExactHalfEdgeId) -> Option<[usize; 2]> {
        self.source_edge_vertices.get(edge.0).copied().flatten()
    }

    pub(super) fn source_halfedge_index(&self, edge: ExactHalfEdgeId) -> Option<usize> {
        self.source_directed_edge(edge).map(|_| edge.0)
    }

    pub(super) fn source_halfedge_key(
        &self,
        edge: ExactHalfEdgeId,
    ) -> Option<ExactMeshlibSourceHalfedgeKey> {
        let source_edge = self.source_directed_edge(edge)?;
        let face = self
            .topology
            .left(edge)
            .or_else(|| self.topology.right(edge))?;
        Some(ExactMeshlibSourceHalfedgeKey {
            face,
            edge: source_edge,
        })
    }

    #[cfg(test)]
    pub(super) fn source_cut_undirected_edge_key(&self, source_edge_index: usize) -> Option<usize> {
        self.source_cut_edge_ids
            .get(source_edge_index)
            .copied()
            .flatten()
            .map(undirected_edge_key)
    }

    pub(super) fn source_contour_undirected_edge_keys(
        &self,
        edge: [usize; 2],
        source_edge_index: usize,
        flip_orientation: bool,
    ) -> Vec<usize> {
        let mut keys = Vec::new();
        for edge in self.source_contour_edge_candidates_from_directed_or_index(
            edge,
            source_edge_index,
            flip_orientation,
        ) {
            let key = undirected_edge_key(edge);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }

    pub(super) fn source_contour_edge_by_source_index(
        &self,
        source_edge_index: usize,
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let edge = self
            .source_cut_edge_ids
            .get(source_edge_index)
            .copied()
            .flatten()?;
        Some(self.source_contour_edge_from_halfedge(edge, flip_orientation))
    }

    fn source_contour_edge_candidates_from_directed_or_index(
        &self,
        edge: [usize; 2],
        source_edge_index: usize,
        flip_orientation: bool,
    ) -> Vec<ExactHalfEdgeId> {
        let mut candidates = Vec::new();
        if let Some(edge) =
            self.source_contour_edge_by_source_index(source_edge_index, flip_orientation)
        {
            push_unique_edge_id(&mut candidates, edge);
        }
        if let Some(edge) = self.source_contour_edge(edge, flip_orientation) {
            push_unique_edge_id(&mut candidates, edge);
        }
        candidates
    }

    fn source_contour_edge_from_halfedge(
        &self,
        edge: ExactHalfEdgeId,
        flip_orientation: bool,
    ) -> ExactHalfEdgeId {
        let edge_sym = ExactHalfEdgeTopology::sym(edge);
        [edge, edge_sym]
            .into_iter()
            .find(|candidate| self.is_meshlib_prepared_contour_side(*candidate, flip_orientation))
            .or_else(|| {
                [edge, edge_sym].into_iter().find(|candidate| {
                    if flip_orientation {
                        self.topology.left(*candidate).is_none()
                    } else {
                        self.topology.right(*candidate).is_none()
                    }
                })
            })
            .unwrap_or(if flip_orientation { edge_sym } else { edge })
    }

    fn is_meshlib_prepared_contour_side(
        &self,
        edge: ExactHalfEdgeId,
        flip_orientation: bool,
    ) -> bool {
        if flip_orientation {
            self.topology.left(edge).is_none()
                && self
                    .topology
                    .right(edge)
                    .is_some_and(|face| self.region_faces.contains(&face))
        } else {
            self.topology.right(edge).is_none()
                && self
                    .topology
                    .left(edge)
                    .is_some_and(|face| self.region_faces.contains(&face))
        }
    }

    fn mapped_edge_key(&self, edge: ExactHalfEdgeId) -> Option<[usize; 2]> {
        self.directed_edge(edge)
            .or_else(|| self.source_directed_edge(edge))
    }

    fn connect_left(&self, edge: ExactHalfEdgeId, flip_orientation: bool) -> Option<usize> {
        if flip_orientation {
            self.topology.right(edge)
        } else {
            self.topology.left(edge)
        }
    }

    fn connect_right(&self, edge: ExactHalfEdgeId, flip_orientation: bool) -> Option<usize> {
        if flip_orientation {
            self.topology.left(edge)
        } else {
            self.topology.right(edge)
        }
    }
}
