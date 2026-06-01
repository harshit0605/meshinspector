use super::*;

#[test]
fn initial_edge_map_uses_actual_meshlib_contour_side_when_flipped() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0]).unwrap();
    let source_edge = source.cut_edge_id(0).unwrap();
    let mut output = OutputFaceTopology::from_faces(&[[0, 1, 2]]).unwrap();
    output.use_meshlib_source_edge_identity();
    let output_edge = output.directed_face_edge(0, [0, 1]).unwrap();
    output
        .meshlib_mapped_contour_edge_indices
        .insert((ExactBooleanOperand::Second, 0), output_edge);

    let non_flipped = source.initial_edge_map(&output, ExactBooleanOperand::Second, false);
    assert_eq!(non_flipped.get(&source_edge), Some(&output_edge));
    assert_eq!(
        non_flipped.get(&ExactHalfEdgeTopology::sym(source_edge)),
        Some(&ExactHalfEdgeTopology::sym(output_edge))
    );

    let flipped = source.initial_edge_map(&output, ExactBooleanOperand::Second, true);
    assert_eq!(
        flipped.get(&ExactHalfEdgeTopology::sym(source_edge)),
        Some(&output_edge)
    );
    assert_eq!(
        flipped.get(&source_edge),
        Some(&ExactHalfEdgeTopology::sym(output_edge))
    );
}

#[test]
fn contour_vertex_maps_use_actual_meshlib_source_side_when_flipped() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0]).unwrap();
    let raw_source_edge = source.cut_edge_id(0).unwrap();
    let actual_flipped_side = ExactHalfEdgeTopology::sym(raw_source_edge);

    let maps = source.oriented_contour_vertex_maps(&[([0, 1], [10, 11])], &[None], true);

    assert_eq!(
        maps,
        vec![(
            source
                .source_vertices_for_edge(actual_flipped_side)
                .unwrap(),
            [10, 11]
        )]
    );
}

#[test]
fn contour_vertex_maps_prefer_meshlib_cut_edge_index_over_directed_fallback() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1], [1, 2]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0]).unwrap();
    let indexed_edge = ExactHalfEdgeTopology::sym(source.cut_edge_id(1).unwrap());

    let maps = source.oriented_contour_vertex_maps(&[([0, 1], [10, 11])], &[Some(1)], true);

    assert_eq!(
        maps[0].0,
        source.source_vertices_for_edge(indexed_edge).unwrap(),
        "MeshLib remaps contours by prepared cut-edge index before falling back to the raw edge"
    );
}

#[test]
fn initial_edge_map_selects_open_contour_side_for_reversed_nonflip_edge() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[1, 0]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0]).unwrap();
    let raw_source_edge = source.cut_edge_id(0).unwrap();
    let actual_contour_edge = ExactHalfEdgeTopology::sym(raw_source_edge);
    let mut output = OutputFaceTopology::from_faces(&[[0, 1, 2]]).unwrap();
    output.use_meshlib_source_edge_identity();
    let output_edge = output.directed_face_edge(0, [0, 1]).unwrap();
    output
        .meshlib_mapped_contour_edge_indices
        .insert((ExactBooleanOperand::Second, 0), output_edge);

    let non_flipped = source.initial_edge_map(&output, ExactBooleanOperand::Second, false);
    assert_eq!(source.topology.right(actual_contour_edge), None);
    assert_eq!(non_flipped.get(&actual_contour_edge), Some(&output_edge));
    assert_eq!(
        non_flipped.get(&raw_source_edge),
        Some(&ExactHalfEdgeTopology::sym(output_edge))
    );
}

#[test]
fn map_edge_like_meshlib_restores_halfedge_parity_from_undirected_map() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0]).unwrap();
    let source_edge = source.face_edges.get(&0).unwrap()[0];
    let output = OutputFaceTopology::from_faces(&[[0, 1, 2]]).unwrap();
    let output_edge = output.directed_face_edge(0, [0, 1]).unwrap();
    let mut edge_map = BTreeMap::new();
    edge_map.insert(source_edge, output_edge);

    assert_eq!(
        source.map_edge_like_meshlib(source_edge, &edge_map),
        Some(output_edge)
    );
    assert_eq!(
        source.map_edge_like_meshlib(ExactHalfEdgeTopology::sym(source_edge), &edge_map),
        Some(ExactHalfEdgeTopology::sym(output_edge))
    );

    edge_map.clear();
    edge_map.insert(
        ExactHalfEdgeTopology::sym(source_edge),
        ExactHalfEdgeTopology::sym(output_edge),
    );
    assert_eq!(
        source.map_edge_like_meshlib(source_edge, &edge_map),
        Some(output_edge)
    );
}

#[test]
fn translated_record_walks_use_meshlib_undirected_edge_map_parity() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0]).unwrap();
    let source_edge = source.face_edges.get(&0).unwrap()[0];
    let source_next = source.topology.next(source_edge);
    let source_prev = source.topology.prev(source_edge);
    let output_next = ExactHalfEdgeId(100);
    let output_prev = ExactHalfEdgeId(102);
    let mut next_edge_map = BTreeMap::new();
    next_edge_map.insert(
        ExactHalfEdgeTopology::sym(source_next),
        ExactHalfEdgeTopology::sym(output_next),
    );
    let mut prev_edge_map = BTreeMap::new();
    prev_edge_map.insert(
        ExactHalfEdgeTopology::sym(source_prev),
        ExactHalfEdgeTopology::sym(output_prev),
    );

    assert_eq!(
        mapped_next(&source, source_next, &next_edge_map),
        Some(output_next)
    );
    assert_eq!(
        mapped_prev(&source, source_prev, &prev_edge_map),
        Some(output_prev)
    );
}

#[test]
fn mapped_face_edge_prefers_meshlib_valid_left_ring_mapping() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let source = SourcePreparedTopology::from_cut_mesh(&cut_mesh, &[0]).unwrap();
    let source_edges = source.face_edges.get(&0).unwrap();
    let mut output = OutputFaceTopology::from_faces(&[[0, 1, 2]]).unwrap();
    let first_output_edge = output.topology.make_edge(Some(0), Some(1));
    let valid_output_edge = output.directed_face_edge(0, [1, 2]).unwrap();
    let mut edge_map = BTreeMap::new();
    edge_map.insert(source_edges[0], first_output_edge);
    edge_map.insert(
        ExactHalfEdgeTopology::sym(source_edges[0]),
        ExactHalfEdgeTopology::sym(first_output_edge),
    );
    edge_map.insert(source_edges[1], valid_output_edge);
    edge_map.insert(
        ExactHalfEdgeTopology::sym(source_edges[1]),
        ExactHalfEdgeTopology::sym(valid_output_edge),
    );

    assert_eq!(
        source.mapped_face_edge(&output, 0, 0, &edge_map, false),
        Some(valid_output_edge)
    );
}

#[test]
fn copied_edge_prepare_records_source_edge_copy_status() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let prepared = prepare_meshlib_copied_edges(
        &mut output,
        ExactMeshlibCopiedEdgeTranslationInput {
            cut_mesh: &cut_mesh,
            prepared_faces: &[0],
            vertex_map: &[Some(0), Some(1), Some(2)],
            contour_vertex_maps: Vec::new(),
            contour_vertex_map_source_indices: Vec::new(),
            face_sources: &[],
            incoming_operand: ExactBooleanOperand::Second,
            first_virtual_vertex: 3,
            append_prepared_faces: false,
            flip_orientation: false,
        },
    )
    .unwrap();

    assert_eq!(prepared.summary.copied_edges, 3);
    let lookup = output
        .meshlib_copied_source_edge_lookup(Some(ExactBooleanOperand::Second), Some([0, 1]))
        .unwrap();
    assert_eq!(lookup.status.label(), "copied");
    assert_eq!(lookup.matched_source_edge, Some([0, 1]));
    assert_eq!(lookup.matching_statuses, 1);
    assert!(lookup.source_halfedge.is_some());
    assert_eq!(lookup.source_origin, Some(0));
    assert_eq!(lookup.source_left, Some(0));
    assert_eq!(lookup.source_right, None);
    assert!(lookup.source_next_halfedge.is_some());
    assert!(lookup.source_prev_halfedge.is_some());
    assert!(lookup.output_edge_id.is_some());
    assert_eq!(lookup.output_origin, Some(0));
    assert_eq!(lookup.output_left, None);
    assert_eq!(lookup.output_right, None);
    assert!(lookup.output_next_edge_id.is_some());
    assert!(lookup.output_prev_edge_id.is_some());
}
