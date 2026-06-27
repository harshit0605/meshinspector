use super::super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use std::collections::{BTreeMap, BTreeSet};

mod contours;
mod face_records;
mod keys;
mod prev_next_edges;
mod rings;
#[cfg(test)]
mod tests;
use rings::link_face_ring;

pub(super) struct SourcePreparedTopology {
    pub(super) topology: ExactHalfEdgeTopology,
    directed_edges: BTreeMap<[usize; 2], Vec<ExactHalfEdgeId>>,
    cut_edges: Vec<[usize; 2]>,
    cut_edge_ids: Vec<Option<ExactHalfEdgeId>>,
    pub(super) edge_vertices: BTreeMap<ExactHalfEdgeId, [usize; 2]>,
    pub(super) face_edges: BTreeMap<usize, Vec<ExactHalfEdgeId>>,
    face_source_faces: BTreeMap<usize, usize>,
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
            face_source_faces: BTreeMap::new(),
            base_edges: Vec::new(),
        };
        let mut undirected_edges =
            BTreeMap::<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>::new();
        let mut prepared_face_set = BTreeSet::new();
        for face_index in prepared_faces {
            prepared_face_set.insert(*face_index);
            source.add_face(
                cut_mesh,
                *face_index,
                flip_orientation,
                true,
                &mut undirected_edges,
            )?;
        }
        for face_index in contour_support_face_indices(cut_mesh) {
            if prepared_face_set.contains(&face_index) {
                continue;
            }
            source.add_face(
                cut_mesh,
                face_index,
                flip_orientation,
                false,
                &mut undirected_edges,
            )?;
        }
        source.cut_edge_ids = source.cut_edge_ids(cut_mesh);
        Ok(source)
    }

    fn add_face(
        &mut self,
        cut_mesh: &ExactCutMeshResult,
        face_index: usize,
        flip_orientation: bool,
        copy_base_edges: bool,
        undirected_edges: &mut BTreeMap<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>,
    ) -> Result<(), &'static str> {
        if self.face_edges.contains_key(&face_index) {
            return Ok(());
        }
        let Some(face) = cut_mesh.faces.get(face_index) else {
            return Ok(());
        };
        let mut face = [face[0] as usize, face[1] as usize, face[2] as usize];
        if flip_orientation {
            face.swap(1, 2);
        }
        let edges = [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]];
        let mut face_edge_ids = Vec::with_capacity(3);
        for edge in edges {
            face_edge_ids.push(self.edge_id(undirected_edges, edge, copy_base_edges));
        }
        link_face_ring(&mut self.topology, &face_edge_ids, edges)?;
        self.topology.set_left(face_edge_ids[0], Some(face_index))?;
        self.face_edges.insert(face_index, face_edge_ids);
        if let Some(source_face) = cut_mesh.source_face_for_faces.get(face_index) {
            self.face_source_faces.insert(face_index, *source_face);
        }
        Ok(())
    }

    fn edge_id(
        &mut self,
        undirected_edges: &mut BTreeMap<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>,
        edge: [usize; 2],
        copy_base_edge: bool,
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
                    if copy_base_edge {
                        self.register_base_edge(candidate);
                    }
                    return candidate;
                }
            }
        }

        let edge_id = self.topology.make_edge(Some(edge[0]), Some(edge[1]));
        if copy_base_edge {
            self.register_base_edge(edge_id);
        }
        undirected_edges
            .entry(key)
            .or_default()
            .push((edge, edge_id));
        self.register_directed_edge(edge, edge_id);
        edge_id
    }

    fn register_base_edge(&mut self, edge: ExactHalfEdgeId) {
        let undirected = ExactHalfEdgeId(edge.0 & !1);
        if !self
            .base_edges
            .iter()
            .any(|stored| stored.0 & !1 == undirected.0)
        {
            self.base_edges.push(undirected);
        }
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

    pub(super) fn cut_edge_id(&self, source_edge_index: usize) -> Option<ExactHalfEdgeId> {
        self.cut_edge_ids.get(source_edge_index).copied().flatten()
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

    pub(super) fn source_face_for_face(&self, face: usize) -> Option<usize> {
        self.face_source_faces.get(&face).copied()
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

fn contour_support_face_indices(cut_mesh: &ExactCutMeshResult) -> BTreeSet<usize> {
    let cut_edges = cut_mesh
        .cut_edges
        .iter()
        .copied()
        .map(ordered_edge)
        .collect::<BTreeSet<_>>();
    cut_mesh
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            let face = [face[0] as usize, face[1] as usize, face[2] as usize];
            let edges = [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]];
            edges
                .into_iter()
                .any(|edge| cut_edges.contains(&ordered_edge(edge)))
                .then_some(face_index)
        })
        .collect()
}
