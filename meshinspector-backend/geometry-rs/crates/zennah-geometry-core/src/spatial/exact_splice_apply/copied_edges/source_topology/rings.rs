use crate::spatial::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};

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
