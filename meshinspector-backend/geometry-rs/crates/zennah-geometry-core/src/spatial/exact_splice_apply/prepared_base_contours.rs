use super::super::exact_boolean::ExactBooleanOperand;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::output_topology::OutputFaceTopology;

pub(super) fn register_prepared_base_contour_edge_indices(
    topology: &mut OutputFaceTopology,
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    vertex_map: &[Option<usize>],
    operand: ExactBooleanOperand,
    flip_orientation: bool,
) {
    for (edge_index, source_edge) in cut_mesh.cut_edges.iter().copied().enumerate() {
        let Some(target) = prepared_base_contour_target_edge(
            topology,
            cut_mesh,
            prepared_faces,
            vertex_map,
            source_edge,
            flip_orientation,
        ) else {
            continue;
        };
        topology.register_meshlib_mapped_contour_edge_index(operand, edge_index, target);
    }
}

fn prepared_base_contour_target_edge(
    topology: &OutputFaceTopology,
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    vertex_map: &[Option<usize>],
    source_edge: [usize; 2],
    flip_orientation: bool,
) -> Option<ExactHalfEdgeId> {
    let mut output_face_index = 0;
    for cut_face_index in prepared_faces {
        let Some(face) = cut_mesh.faces.get(*cut_face_index) else {
            continue;
        };
        let source_face = [face[0] as usize, face[1] as usize, face[2] as usize];
        for face_edge in source_face_edges(source_face) {
            if ordered_edge(face_edge) != ordered_edge(source_edge) {
                continue;
            }
            let Some(output_face_edge) =
                mapped_output_face_edge(face_edge, vertex_map, flip_orientation)
            else {
                continue;
            };
            let Some(edge_id) = topology.directed_face_edge(output_face_index, output_face_edge)
            else {
                continue;
            };
            let target = ExactHalfEdgeTopology::sym(edge_id);
            if topology.topology.left(target).is_none() {
                return Some(target);
            }
        }
        output_face_index += 1;
    }
    None
}

fn mapped_output_face_edge(
    edge: [usize; 2],
    vertex_map: &[Option<usize>],
    flip_orientation: bool,
) -> Option<[usize; 2]> {
    let mapped = [
        *vertex_map.get(edge[0])?.as_ref()?,
        *vertex_map.get(edge[1])?.as_ref()?,
    ];
    Some(if flip_orientation {
        [mapped[1], mapped[0]]
    } else {
        mapped
    })
}

fn source_face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}
