use super::super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOutputFaceSource, ExactBooleanStitchedEdgeSource,
};
use super::super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_halfedge::ExactHalfEdgeTopology;
use super::super::exact_meshlib_rewrite_apply::exact_meshlib_record_rewrite_apply_plan;
use super::super::exact_stitch::ExactStitchPath;
use super::topology::SourceEdgeWalkResult;
use super::*;

fn open_single_edge_assembly() -> ExactBooleanAssemblyResult {
    ExactBooleanAssemblyResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2], [2, 1, 3]],
        face_sources: vec![
            ExactBooleanOutputFaceSource {
                operand: ExactBooleanOperand::First,
                cut_face: 0,
                source_face: 0,
            },
            ExactBooleanOutputFaceSource {
                operand: ExactBooleanOperand::Second,
                cut_face: 0,
                source_face: 0,
            },
        ],
        first_output_vertex_for_cut_vertex: Vec::new(),
        second_output_vertex_for_cut_vertex: Vec::new(),
        stitched_edge_sources: vec![ExactBooleanStitchedEdgeSource {
            output_edge: [1, 2],
            first_output_edge: Some([1, 2]),
            second_output_edge: Some([2, 1]),
            first_stitch_edge: Some([1, 2]),
            second_stitch_edge: Some([2, 1]),
            first_stitch_edge_synthetic: false,
            second_stitch_edge_synthetic: false,
            first_edge_index: 0,
            second_edge_index: 0,
            first_cut_edge: [1, 2],
            second_cut_edge: [2, 1],
        }],
        stitched_edge_paths: vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }],
        prepare_first_faces: vec![0],
        prepare_second_faces: vec![0],
        selected_first_faces: vec![0],
        selected_second_faces: vec![0],
        flipped_first: false,
        flipped_second: false,
        first_cut_paths_consistent: true,
        second_cut_paths_consistent: true,
        first_cut_path_side_components: [1, 1],
        second_cut_path_side_components: [1, 1],
        first_cut_path_overlap_components: 0,
        second_cut_path_overlap_components: 0,
        result_cut_paths: Vec::new(),
        result_cut_path_closed: Vec::new(),
        result_cut_paths_complete: true,
    }
}

fn record_rewrite_command() -> ExactMeshlibRecordRewriteCommand {
    ExactMeshlibRecordRewriteCommand {
        stitch_pair_index: 0,
        this_operand: ExactBooleanOperand::First,
        from_operand: ExactBooleanOperand::Second,
        output_edge: [1, 2],
        this_contour_edge: [1, 2],
        from_contour_edge: [2, 1],
        this_source_edge_index: 0,
        from_source_edge_index: 0,
        this_source_edge: [1, 2],
        from_source_edge: [2, 1],
        this_side_synthetic: false,
        from_side_synthetic: false,
        synthetic_sides: 0,
    }
}

fn two_edge_open_path_assembly() -> ExactBooleanAssemblyResult {
    ExactBooleanAssemblyResult {
        vertices: vec![[0.0; 3]; 6],
        faces: vec![[0, 1, 3], [1, 2, 3], [1, 0, 4], [2, 1, 5]],
        face_sources: vec![
            ExactBooleanOutputFaceSource {
                operand: ExactBooleanOperand::First,
                cut_face: 0,
                source_face: 0,
            },
            ExactBooleanOutputFaceSource {
                operand: ExactBooleanOperand::First,
                cut_face: 1,
                source_face: 1,
            },
            ExactBooleanOutputFaceSource {
                operand: ExactBooleanOperand::Second,
                cut_face: 0,
                source_face: 0,
            },
            ExactBooleanOutputFaceSource {
                operand: ExactBooleanOperand::Second,
                cut_face: 1,
                source_face: 1,
            },
        ],
        first_output_vertex_for_cut_vertex: Vec::new(),
        second_output_vertex_for_cut_vertex: Vec::new(),
        stitched_edge_sources: vec![
            ExactBooleanStitchedEdgeSource {
                output_edge: [0, 1],
                first_output_edge: Some([0, 1]),
                second_output_edge: Some([1, 0]),
                first_stitch_edge: Some([0, 1]),
                second_stitch_edge: Some([1, 0]),
                first_stitch_edge_synthetic: false,
                second_stitch_edge_synthetic: false,
                first_edge_index: 0,
                second_edge_index: 0,
                first_cut_edge: [0, 1],
                second_cut_edge: [1, 0],
            },
            ExactBooleanStitchedEdgeSource {
                output_edge: [1, 2],
                first_output_edge: Some([1, 2]),
                second_output_edge: Some([2, 1]),
                first_stitch_edge: Some([1, 2]),
                second_stitch_edge: Some([2, 1]),
                first_stitch_edge_synthetic: false,
                second_stitch_edge_synthetic: false,
                first_edge_index: 1,
                second_edge_index: 1,
                first_cut_edge: [1, 2],
                second_cut_edge: [2, 1],
            },
        ],
        stitched_edge_paths: vec![ExactStitchPath {
            pair_indices: vec![0, 1],
            closed: false,
        }],
        prepare_first_faces: vec![0, 1],
        prepare_second_faces: vec![0, 1],
        selected_first_faces: vec![0, 1],
        selected_second_faces: vec![0, 1],
        flipped_first: false,
        flipped_second: false,
        first_cut_paths_consistent: true,
        second_cut_paths_consistent: true,
        first_cut_path_side_components: [1, 1],
        second_cut_path_side_components: [1, 1],
        first_cut_path_overlap_components: 0,
        second_cut_path_overlap_components: 0,
        result_cut_paths: Vec::new(),
        result_cut_path_closed: Vec::new(),
        result_cut_paths_complete: true,
    }
}

fn two_edge_record_rewrite_commands() -> [ExactMeshlibRecordRewriteCommand; 2] {
    [
        ExactMeshlibRecordRewriteCommand {
            stitch_pair_index: 0,
            this_operand: ExactBooleanOperand::First,
            from_operand: ExactBooleanOperand::Second,
            output_edge: [0, 1],
            this_contour_edge: [0, 1],
            from_contour_edge: [1, 0],
            this_source_edge_index: 0,
            from_source_edge_index: 0,
            this_source_edge: [0, 1],
            from_source_edge: [1, 0],
            this_side_synthetic: false,
            from_side_synthetic: false,
            synthetic_sides: 0,
        },
        ExactMeshlibRecordRewriteCommand {
            stitch_pair_index: 1,
            this_operand: ExactBooleanOperand::First,
            from_operand: ExactBooleanOperand::Second,
            output_edge: [1, 2],
            this_contour_edge: [1, 2],
            from_contour_edge: [2, 1],
            this_source_edge_index: 1,
            from_source_edge_index: 1,
            this_source_edge: [1, 2],
            from_source_edge: [2, 1],
            this_side_synthetic: false,
            from_side_synthetic: false,
            synthetic_sides: 0,
        },
    ]
}

#[test]
fn near_stitch_plan_derives_meshlib_open_contour_endpoint_updates() {
    let assembly = open_single_edge_assembly();
    let command = record_rewrite_command();

    let plan = exact_meshlib_near_stitch_plan(
        &assembly,
        ExactBooleanOperand::First,
        ExactBooleanOperand::Second,
        &[command],
    );

    assert_eq!(plan.open_paths, 1);
    assert_eq!(plan.expected_updates, 2);
    assert_eq!(plan.blocked_updates, 0);
    assert_eq!(plan.commands.len(), 2);
    assert!(plan
        .commands
        .iter()
        .all(|command| command.source_operand == Some(ExactBooleanOperand::Second)));
    assert!(plan.commands.iter().any(|command| command.endpoint
        == Some(ExactMeshlibNearStitchEndpoint::Start)
        && command.next_source_edge.is_some()));
    assert!(plan.commands.iter().any(|command| command.endpoint
        == Some(ExactMeshlibNearStitchEndpoint::End)
        && command.previous_source_edge.is_some()));
    let end_command = plan
        .commands
        .iter()
        .find(|command| command.endpoint == Some(ExactMeshlibNearStitchEndpoint::End))
        .unwrap();
    assert_eq!(end_command.next_edge, [2, 0]);

    let apply_plan =
        exact_meshlib_record_rewrite_apply_plan(&assembly.faces, &[command], &plan.commands);
    assert_eq!(apply_plan.failed_commands, 0);
    assert_eq!(apply_plan.applied_near_stitch_updates, 2);
    assert_eq!(apply_plan.failed_near_stitch_updates, 0);
    assert!(apply_plan.ready_for_export, "{apply_plan:#?}");
}

#[test]
fn near_stitch_apply_handles_multi_edge_open_path() {
    let assembly = two_edge_open_path_assembly();
    let commands = two_edge_record_rewrite_commands();

    let plan = exact_meshlib_near_stitch_plan(
        &assembly,
        ExactBooleanOperand::First,
        ExactBooleanOperand::Second,
        &commands,
    );

    assert_eq!(plan.open_paths, 1);
    assert_eq!(plan.expected_updates, 2);
    assert_eq!(plan.blocked_updates, 0);
    assert_eq!(plan.commands.len(), 2);
    assert_eq!(
        plan.commands[0].endpoint,
        Some(ExactMeshlibNearStitchEndpoint::Start)
    );
    assert_eq!(
        plan.commands[1].endpoint,
        Some(ExactMeshlibNearStitchEndpoint::End)
    );

    let apply_plan =
        exact_meshlib_record_rewrite_apply_plan(&assembly.faces, &commands, &plan.commands);
    assert_eq!(apply_plan.failed_commands, 0);
    assert_eq!(apply_plan.failed_near_stitch_updates, 0);
    assert_eq!(apply_plan.applied_near_stitch_updates, 2);
    assert!(apply_plan.ready_for_export, "{apply_plan:#?}");
}

#[test]
fn prepared_near_stitch_source_path_carries_meshlib_source_halfedge_ids() {
    let assembly = two_edge_open_path_assembly();
    let commands = two_edge_record_rewrite_commands();
    let base_cut = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 6],
        faces: vec![[0, 1, 3], [1, 2, 3]],
        cut_edges: vec![[0, 1], [1, 2]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0, 1],
        skipped_source_faces: Vec::new(),
    };
    let incoming_cut = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 6],
        faces: vec![[1, 0, 4], [2, 1, 5]],
        cut_edges: vec![[1, 0], [2, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0, 1],
        skipped_source_faces: Vec::new(),
    };

    let plan = exact_meshlib_near_stitch_plan_with_prepared_parts(
        &assembly,
        ExactBooleanOperand::Second,
        &commands,
        ExactMeshlibNearStitchSourceInput {
            cut_mesh: &base_cut,
            prepared_faces: &[0, 1],
            vertex_map: &[],
            contour_vertex_maps: vec![([0, 1], [0, 1]), ([1, 2], [1, 2])],
            contour_vertex_map_source_indices: vec![Some(0), Some(1)],
            first_virtual_vertex: 6,
            flip_orientation: false,
        },
        ExactMeshlibNearStitchSourceInput {
            cut_mesh: &incoming_cut,
            prepared_faces: &[0, 1],
            vertex_map: &[],
            contour_vertex_maps: vec![([1, 0], [1, 0]), ([2, 1], [2, 1])],
            contour_vertex_map_source_indices: vec![Some(0), Some(1)],
            first_virtual_vertex: 6,
            flip_orientation: false,
        },
    );

    let start = plan
        .commands
        .iter()
        .find(|command| command.endpoint == Some(ExactMeshlibNearStitchEndpoint::Start))
        .expect("start near-stitch command");
    let end = plan
        .commands
        .iter()
        .find(|command| command.endpoint == Some(ExactMeshlibNearStitchEndpoint::End))
        .expect("end near-stitch command");
    assert!(start.next_source_halfedge.is_some());
    assert!(end.previous_source_halfedge.is_some());
}

#[test]
fn near_stitch_plan_blocks_open_endpoint_without_record_command() {
    let assembly = open_single_edge_assembly();

    let plan = exact_meshlib_near_stitch_plan(
        &assembly,
        ExactBooleanOperand::First,
        ExactBooleanOperand::Second,
        &[],
    );

    assert_eq!(plan.open_paths, 1);
    assert_eq!(plan.expected_updates, 2);
    assert_eq!(plan.commands.len(), 0);
    assert_eq!(plan.blocked_updates, 2);
}

#[test]
fn near_stitch_source_topology_uses_prepared_part_faces_only() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2], [2, 1, 3]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0, 1],
        skipped_source_faces: Vec::new(),
    };

    let topology = OperandTopology::from_cut_mesh(
        &cut_mesh,
        &[0],
        &[Some(0), Some(1), Some(2), Some(3)],
        &[],
        &[],
        4,
        false,
    );

    assert!(topology.first_directed_face_edge([1, 2]).is_some());
    assert!(
        topology.first_directed_face_edge([1, 3]).is_none(),
        "connectPreparedParts stitches an already prepared MeshLib part, so raw-cut faces outside the prepared part must not contribute source edges"
    );
}

#[test]
fn near_stitch_source_lookup_selects_meshlib_contour_side_for_flip() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let topology = OperandTopology::from_cut_mesh(
        &cut_mesh,
        &[0],
        &[Some(0), Some(1), Some(2)],
        &[],
        &[],
        3,
        false,
    );
    let face_edge = topology
        .first_directed_face_edge([0, 1])
        .expect("face edge");
    let open_edge = ExactHalfEdgeTopology::sym(face_edge);

    assert_eq!(
        topology.source_contour_edge([0, 1], false),
        Some(face_edge),
        "non-flipped MeshLib fromContours use the side with no right face"
    );
    assert_eq!(
        topology.source_contour_edge([0, 1], true),
        Some(open_edge),
        "flipped MeshLib fromContours use the side with no left face"
    );
    assert_eq!(
        topology.source_contour_edge_by_source_index(0, false),
        Some(face_edge),
        "prepared source-index lookup must use the same non-flipped MeshLib open-side rule"
    );
    assert_eq!(
        topology.source_contour_edge_by_source_index(0, true),
        Some(open_edge),
        "prepared source-index lookup must use the same flipped MeshLib open-side rule"
    );
    assert_eq!(topology.contour_boundary_edge([0, 1]), Some(open_edge));
}

#[test]
fn near_stitch_source_identity_exclusion_uses_cut_edge_occurrence() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2], [0, 1, 3]],
        cut_edges: vec![[0, 1], [0, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0, 1],
        skipped_source_faces: Vec::new(),
    };
    let topology = OperandTopology::from_cut_mesh_with_fresh_vertex_map(
        &cut_mesh,
        &[0, 1],
        &[],
        &[],
        4,
        false,
    );
    let first_key = topology
        .source_cut_undirected_edge_key(0)
        .expect("first cut edge");
    let second_key = topology
        .source_cut_undirected_edge_key(1)
        .expect("second cut edge");
    assert_ne!(
        first_key, second_key,
        "MeshLib fromMappedEdges keys contour occurrences by edge id, not just vertex pair"
    );
    let second_contour_keys = topology.source_contour_undirected_edge_keys([0, 1], 1, false);
    assert_eq!(
        second_contour_keys.first().copied(),
        Some(second_key),
        "source-index lookup must keep the indexed contour occurrence as the primary key"
    );
    assert!(
        second_contour_keys.contains(&first_key),
        "fromMappedEdges construction should also include directed fallback contour candidates"
    );

    let mut mapped_source_edges = BTreeSet::new();
    mapped_source_edges.insert(first_key);
    assert!(
        !mapped_source_edges.contains(&second_key),
        "mapping the first same-vertex contour must not exclude the second occurrence"
    );
    let SourceEdgeWalkResult::Edge(second_edge) =
        topology.next_unmapped_source_edge_by_source_index([0, 1], 1, &mapped_source_edges, false)
    else {
        panic!("second same-vertex contour edge remains unmapped by source identity");
    };
    assert_ne!(
        topology
            .source_halfedge_index(second_edge)
            .map(|edge| edge / 2),
        Some(first_key)
    );
}

#[test]
fn near_stitch_prepared_incoming_uses_meshlib_fresh_vmap() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let topology = OperandTopology::from_cut_mesh_with_fresh_vertex_map(
        &cut_mesh,
        &[0],
        &[([0, 1], [10, 11])],
        &[None],
        20,
        false,
    );

    let contour_edge = topology
        .first_directed_face_edge([0, 1])
        .expect("mapped contour edge");
    let copied_edge = topology
        .first_directed_face_edge([1, 2])
        .expect("copied edge after contour");

    assert_eq!(topology.directed_edge(contour_edge), Some([10, 11]));
    assert_eq!(topology.directed_edge(copied_edge), Some([11, 20]));
}

#[test]
fn near_stitch_fresh_vmap_uses_indexed_meshlib_contour_side_when_flipped() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let topology = OperandTopology::from_cut_mesh_with_fresh_vertex_map(
        &cut_mesh,
        &[0],
        &[([0, 1], [10, 11])],
        &[Some(0)],
        20,
        true,
    );

    let contour_edge = topology
        .first_directed_face_edge([0, 1])
        .expect("mapped contour edge");
    let copied_edge = topology
        .first_directed_face_edge([1, 2])
        .expect("copied edge after contour");

    assert_eq!(
        topology.directed_edge(contour_edge),
        Some([11, 10]),
        "flipped MeshLib source-indexed contour maps bind the actual open contour side"
    );
    assert_eq!(topology.directed_edge(copied_edge), Some([10, 20]));
}

#[test]
fn near_stitch_preflipped_source_uses_connect_open_side() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let topology = OperandTopology::from_cut_mesh_with_fresh_vertex_map_and_orientation(
        &cut_mesh,
        &[0],
        &[([0, 1], [10, 11])],
        &[Some(0)],
        20,
        true,
        false,
    );

    let contour_edge = topology
        .source_contour_edge([0, 1], false)
        .expect("prepared connect contour side");

    assert_eq!(topology.source_directed_edge(contour_edge), Some([1, 0]));
    assert_eq!(
        topology.directed_edge(contour_edge),
        Some([11, 10]),
        "MeshLib maps paths through the preflipped prepared part, then connects without a second flip"
    );
}

#[test]
fn near_stitch_source_walk_stops_at_first_mapped_face_like_meshlib() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 5],
        faces: vec![[0, 1, 2], [2, 1, 3], [3, 1, 4]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0, 1, 2],
        skipped_source_faces: Vec::new(),
    };
    let topology = OperandTopology::from_cut_mesh(
        &cut_mesh,
        &[0, 1, 2],
        &[Some(0), Some(1), Some(2), Some(3), Some(4)],
        &[],
        &[],
        5,
        false,
    );

    let next_edge = topology
        .next_unmapped_source_edge([2, 1], &BTreeSet::new(), false)
        .expect("next source edge");
    let previous_edge = topology
        .previous_unmapped_source_edge([2, 1], &BTreeSet::new(), false)
        .expect("previous source edge");
    let next = topology.source_directed_edge(next_edge);
    let previous = topology.source_directed_edge(previous_edge);

    assert_eq!(next, Some([1, 4]));
    assert_eq!(topology.topology.left(next_edge), Some(2));
    assert_eq!(previous, Some([2, 3]));
    assert_eq!(topology.topology.right(previous_edge), Some(1));
}
