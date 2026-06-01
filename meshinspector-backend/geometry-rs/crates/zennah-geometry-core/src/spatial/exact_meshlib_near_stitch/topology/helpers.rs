use super::super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use std::collections::BTreeMap;

pub(super) fn cut_edge_ids(
    cut_mesh: &ExactCutMeshResult,
    directed_edges: &BTreeMap<[usize; 2], Vec<ExactHalfEdgeId>>,
) -> Vec<Option<ExactHalfEdgeId>> {
    let mut seen = BTreeMap::<[usize; 2], usize>::new();
    cut_mesh
        .cut_edges
        .iter()
        .map(|edge| {
            let occurrence = seen.entry(*edge).or_default();
            let result = directed_edges
                .get(edge)
                .and_then(|edges| edges.get(*occurrence).copied())
                .or_else(|| {
                    directed_edges
                        .get(edge)
                        .and_then(|edges| edges.first().copied())
                });
            *occurrence += 1;
            result
        })
        .collect()
}

pub(super) fn undirected_edge_key(edge: ExactHalfEdgeId) -> usize {
    edge.0 / 2
}

pub(super) fn operand_edge_id(
    topology: &mut ExactHalfEdgeTopology,
    undirected_edges: &mut BTreeMap<[usize; 2], Vec<([usize; 2], ExactHalfEdgeId)>>,
    edge_vertices: &mut Vec<Option<[usize; 2]>>,
    source_edge_vertices: &mut Vec<Option<[usize; 2]>>,
    edge: [usize; 2],
    output_edge: Option<[usize; 2]>,
    source_edge: Option<[usize; 2]>,
) -> ExactHalfEdgeId {
    let key = ordered_edge(edge);
    if let Some(edge_ids) = undirected_edges.get(&key) {
        for (stored_edge, edge_id) in edge_ids {
            let candidate = if *stored_edge == edge {
                *edge_id
            } else {
                ExactHalfEdgeTopology::sym(*edge_id)
            };
            if topology.left(candidate).is_none() {
                remember_edge_vertices(edge_vertices, candidate, output_edge);
                remember_edge_vertices(source_edge_vertices, candidate, source_edge);
                return candidate;
            }
        }
    }
    let edge_id = topology.make_edge(None, None);
    remember_edge_vertices(edge_vertices, edge_id, output_edge);
    remember_edge_vertices(source_edge_vertices, edge_id, source_edge);
    undirected_edges
        .entry(key)
        .or_default()
        .push((edge, edge_id));
    edge_id
}

fn remember_edge_vertices(
    edge_vertices: &mut Vec<Option<[usize; 2]>>,
    edge_id: ExactHalfEdgeId,
    edge: Option<[usize; 2]>,
) {
    let Some(edge) = edge else {
        return;
    };
    let sym = ExactHalfEdgeTopology::sym(edge_id);
    if edge_vertices.len() <= sym.0 {
        edge_vertices.resize(sym.0 + 1, None);
    }
    edge_vertices[edge_id.0] = Some(edge);
    edge_vertices[sym.0] = Some(reverse_edge(edge));
}

pub(super) fn copied_vertex_map(
    vertex_map: &[Option<usize>],
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    first_virtual_vertex: usize,
    contour_vertex_maps: &[([usize; 2], [usize; 2])],
) -> Vec<Option<usize>> {
    let mut copied_map = vertex_map.to_vec();
    for (source_edge, output_edge) in contour_vertex_maps {
        set_copied_vertex(&mut copied_map, source_edge[0], output_edge[0]);
        set_copied_vertex(&mut copied_map, source_edge[1], output_edge[1]);
    }
    let mut next_virtual_vertex = first_virtual_vertex;
    for vertex in prepared_region_vertices(cut_mesh, prepared_faces) {
        if copied_map.len() <= vertex {
            copied_map.resize(vertex + 1, None);
        }
        if copied_map[vertex].is_none() {
            copied_map[vertex] = Some(next_virtual_vertex);
            next_virtual_vertex += 1;
        }
    }
    copied_map
}

fn set_copied_vertex(copied_map: &mut Vec<Option<usize>>, source: usize, output: usize) {
    if copied_map.len() <= source {
        copied_map.resize(source + 1, None);
    }
    if copied_map[source].is_none() {
        copied_map[source] = Some(output);
    }
}

fn prepared_region_vertices(cut_mesh: &ExactCutMeshResult, prepared_faces: &[usize]) -> Vec<usize> {
    let mut vertices = Vec::new();
    for face_index in prepared_faces {
        let Some(face) = cut_mesh.faces.get(*face_index) else {
            continue;
        };
        for vertex in face {
            let vertex = *vertex as usize;
            if !vertices.contains(&vertex) {
                vertices.push(vertex);
            }
        }
    }
    vertices.sort_unstable();
    vertices
}

pub(super) fn mapped_edge_vertices(
    source_edge: [usize; 2],
    vertex_map: &[Option<usize>],
) -> Option<[usize; 2]> {
    Some([
        *vertex_map.get(source_edge[0])?.as_ref()?,
        *vertex_map.get(source_edge[1])?.as_ref()?,
    ])
}

pub(super) fn link_face_ring(
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

pub(in crate::spatial::exact_meshlib_near_stitch) fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

pub(super) fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

pub(super) fn push_unique_edge_id(edges: &mut Vec<ExactHalfEdgeId>, edge: ExactHalfEdgeId) {
    if !edges.contains(&edge) {
        edges.push(edge);
    }
}
