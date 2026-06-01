use super::*;
use crate::spatial::exact_meshlib_near_stitch::ExactMeshlibSourceHalfedgeKey;

#[test]
fn source_halfedge_keys_keep_both_meshlib_face_sides() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2], [2, 1, 3]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0, 1],
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
