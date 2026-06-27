use super::*;
use crate::spatial::exact_boolean::ExactBooleanOperand;
use crate::spatial::exact_meshlib_near_stitch::ExactMeshlibSourceHalfedgeKey;
use crate::spatial::exact_splice_apply::OutputFaceTopology;

#[test]
fn source_halfedge_keys_keep_both_meshlib_face_sides() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2], [2, 1, 3]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0, 1],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0, 1]).unwrap();
    let shared = source.face_edges.get(&0).unwrap()[1];

    let keys = source.source_halfedge_keys(shared);

    assert_eq!(
        keys,
        vec![
            ExactMeshlibSourceHalfedgeKey {
                face: 0,
                edge: [1, 2],
            },
            ExactMeshlibSourceHalfedgeKey {
                face: 1,
                edge: [1, 2],
            },
        ]
    );
}

#[test]
fn part_contour_edges_exclude_internal_copied_faces() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2], [2, 1, 3]],
        cut_edges: vec![[1, 2]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0, 1],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0, 1]).unwrap();
    let mut output = OutputFaceTopology::from_faces(&[[0, 1, 2]]).unwrap();
    output.use_meshlib_source_edge_identity();
    let output_edge = output.directed_face_edge(0, [1, 2]).unwrap();
    output.topology.set_left_direct(output_edge, None).unwrap();
    output
        .meshlib_mapped_contour_edge_indices
        .insert((ExactBooleanOperand::Second, 0), output_edge);

    assert_eq!(source.part_contour_edge_for_cut_index(0, false), None);
    assert_eq!(source.part_contour_edge_for_cut_index(0, true), None);
    assert!(source
        .initial_edge_map(&output, ExactBooleanOperand::Second, true)
        .is_empty());
}

#[test]
fn contour_support_edges_are_indexed_without_copying_unprepared_faces() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1], [1, 2], [2, 0]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };

    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[]).unwrap();

    assert!(source.base_edges.is_empty());
    for edge_index in 0..cut_mesh.cut_edges.len() {
        let contour_edge = source
            .part_contour_edge_for_cut_index(edge_index, false)
            .expect("cut-contour support halfedge");
        assert_eq!(source.topology.right(contour_edge), None);
        assert_eq!(source.topology.left(contour_edge), Some(0));
    }
}
