use super::super::exact_boolean::{
    ExactBooleanOperand, ExactBooleanOutputFaceSource, ExactBooleanStitchedEdgeSource,
};
use super::super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_halfedge::ExactHalfEdgeTopology;
use super::super::exact_meshlib_near_stitch::ExactMeshlibNearStitchEdgeUpdateCommand;
use super::super::exact_meshlib_rewrite_apply::{
    exact_meshlib_record_rewrite_apply_plan,
    exact_meshlib_record_rewrite_apply_plan_with_copied_edges,
    exact_meshlib_record_rewrite_apply_plan_with_prepared_base,
    exact_meshlib_record_rewrite_apply_plan_with_sources, ExactMeshlibNearStitchUpdateStatus,
    ExactMeshlibRecordRewriteApplyStatus,
};
use super::super::exact_splice::exact_topology_splice_plan;
use super::super::exact_stitch::ExactStitchPath;
use super::*;

fn stitched_edge(output_edge: [usize; 2]) -> ExactBooleanStitchedEdgeSource {
    ExactBooleanStitchedEdgeSource {
        output_edge,
        first_output_edge: Some(output_edge),
        second_output_edge: Some([output_edge[1], output_edge[0]]),
        first_stitch_edge: Some(output_edge),
        second_stitch_edge: Some([output_edge[1], output_edge[0]]),
        first_stitch_edge_synthetic: false,
        second_stitch_edge_synthetic: false,
        first_edge_index: 0,
        second_edge_index: 0,
        first_cut_edge: output_edge,
        second_cut_edge: output_edge,
    }
}

fn meshlib_rewrite_command(
    this_contour_edge: [usize; 2],
    from_contour_edge: [usize; 2],
) -> ExactMeshlibRecordRewriteCommand {
    ExactMeshlibRecordRewriteCommand {
        stitch_pair_index: 0,
        this_operand: ExactBooleanOperand::First,
        from_operand: ExactBooleanOperand::Second,
        output_edge: ordered_edge(this_contour_edge),
        this_contour_edge,
        from_contour_edge,
        this_source_edge_index: 0,
        from_source_edge_index: 0,
        this_source_edge: this_contour_edge,
        from_source_edge: from_contour_edge,
        this_side_synthetic: false,
        from_side_synthetic: false,
        synthetic_sides: 0,
    }
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_applies_face_backed_command() {
    let faces = [[0, 1, 2], [2, 3, 4]];
    let command = meshlib_rewrite_command([1, 2], [2, 3]);

    let plan = exact_meshlib_record_rewrite_apply_plan(&faces, &[command], &[]);

    assert_eq!(plan.commands, 1);
    assert_eq!(plan.applied_commands, 1);
    assert_eq!(plan.failed_commands, 0);
    assert_eq!(plan.failed_closed_target_edges, 0);
    assert_eq!(plan.translated_face_records, 1);
    assert_eq!(plan.synthetic_side_edges, 0);
    assert_eq!(plan.exported_faces, 2);
    assert_eq!(plan.export_failed_faces, 0);
    assert_eq!(plan.export_non_triangular_faces, 0);
    assert_eq!(plan.export_left_ring_not_closed_faces, 0);
    assert_eq!(plan.export_missing_origin_faces, 0);
    assert_eq!(plan.export_face_record_left_mismatch_faces, 0);
    assert_eq!(plan.export_face_left_ring_mismatch_faces, 0);
    assert_eq!(plan.export_other_failed_faces, 0);
    assert!(plan.export_failed_face_indices.is_empty());
    assert_eq!(plan.topology_edges_before_rewrite, 6);
    assert_eq!(plan.topology_edges_after_rewrite, 6);
    assert!(!plan.export_changed_faces);
    assert!(plan.ready_for_export);
    assert_eq!(
        plan.entries[0].status,
        ExactMeshlibRecordRewriteApplyStatus::Applied
    );
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_with_sources_keeps_operands_separate() {
    let faces = [[0, 1, 2], [2, 1, 3]];
    let face_sources = [
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
    ];
    let command = meshlib_rewrite_command([1, 2], [2, 1]);

    let plan = exact_meshlib_record_rewrite_apply_plan_with_sources(
        &faces,
        &face_sources,
        &[command],
        &[],
    );

    assert_eq!(plan.topology_edges_before_rewrite, 6);
    assert_eq!(plan.applied_commands, 1);
    assert_eq!(plan.failed_commands, 0);
    assert_eq!(plan.topology_edges_after_rewrite, 6);
    assert!(plan.ready_for_export);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_translates_copied_edge_records() {
    let faces = [[0, 1, 2], [2, 1, 3]];
    let face_sources = [
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
    ];
    let command = meshlib_rewrite_command([1, 2], [2, 1]);
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        faces: vec![[2, 1, 3]],
        cut_edges: vec![[2, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [0];
    let vertex_map = [None, Some(1), Some(2), Some(3)];
    let copied_edges = ExactMeshlibCopiedEdgeTranslationInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: vec![([2, 1], [1, 2])],
        contour_vertex_map_source_indices: vec![Some(0)],
        face_sources: &face_sources,
        incoming_operand: ExactBooleanOperand::Second,
        first_virtual_vertex: 4,
        append_prepared_faces: false,
        flip_orientation: false,
    };

    let plan = exact_meshlib_record_rewrite_apply_plan_with_copied_edges(
        &faces,
        &face_sources,
        &[command],
        &[],
        copied_edges,
    );

    assert_eq!(plan.applied_commands, 1);
    assert_eq!(plan.translated_copied_edge_records, 4);
    assert_eq!(plan.failed_copied_edge_records, 0);
}

#[test]
fn meshlib_prepared_base_topology_maps_raw_prepare_part_faces() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 2.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 2.0],
        ],
        faces: vec![[0, 1, 2], [2, 1, 3]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![10, 11],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [1];
    let vertex_map = [None, Some(8), Some(9), None];
    let mut output_vertices = vec![[0.0; 3]; 20];
    output_vertices[8] = [1.0, 0.0, 0.0];
    output_vertices[9] = [0.0, 1.0, 0.0];

    let prepared = output_topology_from_prepared_base(ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: Vec::new(),
        output_vertices: &output_vertices,
        operand: ExactBooleanOperand::Second,
        first_virtual_vertex: 20,
        flip_orientation: false,
    })
    .expect("prepared base topology");

    assert_eq!(prepared.faces, vec![[9, 8, 20]]);
    assert_eq!(prepared.vertices.len(), 21);
    assert_eq!(prepared.vertices[20], [0.0, 0.0, 2.0]);
    assert_eq!(prepared.face_sources.len(), 1);
    assert_eq!(
        prepared.face_sources[0].operand,
        ExactBooleanOperand::Second
    );
    assert_eq!(prepared.face_sources[0].cut_face, 1);
    assert_eq!(prepared.face_sources[0].source_face, 11);
    assert_eq!(prepared.virtual_vertices, 1);
    assert_eq!(prepared.topology.export_faces().unwrap(), prepared.faces);
}

#[test]
fn meshlib_prepared_base_contour_maps_seed_unmapped_vertices_before_virtual_copy() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [0];
    let vertex_map = [Some(10), Some(8), None];
    let output_vertices = vec![[0.0; 3]; 12];

    let prepared = output_topology_from_prepared_base(ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: vec![([1, 2], [4, 5])],
        output_vertices: &output_vertices,
        operand: ExactBooleanOperand::First,
        first_virtual_vertex: 12,
        flip_orientation: false,
    })
    .expect("prepared base topology");

    assert_eq!(prepared.faces, vec![[10, 8, 5]]);
    assert_eq!(prepared.virtual_vertices, 0);
    assert_eq!(prepared.vertices.len(), output_vertices.len());
}

#[test]
fn meshlib_prepared_base_registers_indexed_contour_targets() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[1, 2]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [0];
    let vertex_map = [Some(0), Some(1), Some(2)];
    let output_vertices = vec![[0.0; 3]; 3];

    let prepared = output_topology_from_prepared_base(ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: Vec::new(),
        output_vertices: &output_vertices,
        operand: ExactBooleanOperand::First,
        first_virtual_vertex: 3,
        flip_orientation: false,
    })
    .expect("prepared base topology");

    let target = prepared
        .topology
        .meshlib_mapped_contour_edge_indices
        .get(&(ExactBooleanOperand::First, 0))
        .copied()
        .expect("indexed contour target");
    assert_eq!(prepared.topology.topology.left(target), None);
    assert_eq!(prepared.topology.topology.right(target), Some(0));
    assert_eq!(
        prepared
            .topology
            .topology
            .left(ExactHalfEdgeTopology::sym(target)),
        Some(0)
    );
}

#[test]
fn meshlib_prepared_base_uses_cut_paths_for_indexed_contour_targets() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1], [1, 2]],
        cut_edge_paths: vec![vec![[2, 1]]],
        cut_edge_path_closed: vec![false],
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [0];
    let vertex_map = [Some(0), Some(1), Some(2)];
    let output_vertices = vec![[0.0; 3]; 3];

    let prepared = output_topology_from_prepared_base(ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: Vec::new(),
        output_vertices: &output_vertices,
        operand: ExactBooleanOperand::First,
        first_virtual_vertex: 3,
        flip_orientation: false,
    })
    .expect("prepared base topology");

    assert!(!prepared
        .topology
        .meshlib_mapped_contour_edge_indices
        .contains_key(&(ExactBooleanOperand::First, 0)));
    let target = prepared
        .topology
        .meshlib_mapped_contour_edge_indices
        .get(&(ExactBooleanOperand::First, 1))
        .copied()
        .expect("path contour target");
    assert_eq!(prepared.topology.topology.left(target), None);
    assert_eq!(prepared.topology.topology.right(target), Some(0));
}

#[test]
fn meshlib_prepared_base_cut_paths_preserve_duplicate_occurrences() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2], [1, 2, 3]],
        cut_edges: vec![[1, 2], [1, 2]],
        cut_edge_paths: vec![vec![[1, 2], [1, 2]]],
        cut_edge_path_closed: vec![false],
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0, 1],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [0, 1];
    let vertex_map = [Some(0), Some(1), Some(2), Some(3)];
    let output_vertices = vec![[0.0; 3]; 4];

    let prepared = output_topology_from_prepared_base(ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: Vec::new(),
        output_vertices: &output_vertices,
        operand: ExactBooleanOperand::First,
        first_virtual_vertex: 4,
        flip_orientation: false,
    })
    .expect("prepared base topology");

    let first_target = prepared
        .topology
        .meshlib_mapped_contour_edge_indices
        .get(&(ExactBooleanOperand::First, 0))
        .copied()
        .expect("first duplicate target");
    let second_target = prepared
        .topology
        .meshlib_mapped_contour_edge_indices
        .get(&(ExactBooleanOperand::First, 1))
        .copied()
        .expect("second duplicate target");

    assert_ne!(first_target, second_target);
    assert_eq!(prepared.topology.topology.left(first_target), None);
    assert_eq!(prepared.topology.topology.left(second_target), None);
    assert_eq!(prepared.topology.topology.right(first_target), Some(0));
    assert_eq!(prepared.topology.topology.right(second_target), Some(1));
}

#[test]
fn meshlib_prepared_base_does_not_register_closed_indexed_contour_targets() {
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
    let prepared_faces = [0, 1];
    let vertex_map = [Some(0), Some(1), Some(2), Some(3)];
    let output_vertices = vec![[0.0; 3]; 4];

    let prepared = output_topology_from_prepared_base(ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: Vec::new(),
        output_vertices: &output_vertices,
        operand: ExactBooleanOperand::First,
        first_virtual_vertex: 4,
        flip_orientation: false,
    })
    .expect("prepared base topology");

    assert!(!prepared
        .topology
        .meshlib_mapped_contour_edge_indices
        .contains_key(&(ExactBooleanOperand::First, 0)));
    assert_eq!(prepared.topology.export_faces().unwrap(), prepared.faces);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_accepts_prepared_base_topology() {
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 3],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![5],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [0];
    let vertex_map = [Some(3), Some(4), Some(5)];
    let output_vertices = vec![[0.0; 3]; 6];

    let plan = exact_meshlib_record_rewrite_apply_plan_with_prepared_base(
        ExactMeshlibPreparedBaseTopologyInput {
            cut_mesh: &cut_mesh,
            prepared_faces: &prepared_faces,
            vertex_map: &vertex_map,
            contour_vertex_maps: Vec::new(),
            output_vertices: &output_vertices,
            operand: ExactBooleanOperand::First,
            first_virtual_vertex: 6,
            flip_orientation: false,
        },
        &[],
        &[],
        None,
    );

    assert_eq!(plan.exported_faces, 1);
    assert_eq!(plan.export_failed_faces, 0);
    assert_eq!(plan.exported_face_indices, vec![[3, 4, 5]]);
    assert_eq!(plan.topology_edges_before_rewrite, 3);
    assert_eq!(plan.topology_edges_after_rewrite, 3);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_appends_prepared_base_copied_faces() {
    let base_cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let incoming_cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[1, 3, 2]],
        cut_edges: vec![[2, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![7],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let base_faces = [0];
    let incoming_faces = [0];
    let base_vertex_map = [Some(0), Some(1), Some(2)];
    let incoming_vertex_map = [None, Some(1), Some(2), None];
    let output_vertices = vec![[0.0; 3]; 3];
    let face_sources: [ExactBooleanOutputFaceSource; 0] = [];
    let copied_edges = ExactMeshlibCopiedEdgeTranslationInput {
        cut_mesh: &incoming_cut_mesh,
        prepared_faces: &incoming_faces,
        vertex_map: &incoming_vertex_map,
        contour_vertex_maps: Vec::new(),
        contour_vertex_map_source_indices: Vec::new(),
        face_sources: &face_sources,
        incoming_operand: ExactBooleanOperand::Second,
        first_virtual_vertex: 3,
        append_prepared_faces: true,
        flip_orientation: false,
    };

    let plan = exact_meshlib_record_rewrite_apply_plan_with_prepared_base(
        ExactMeshlibPreparedBaseTopologyInput {
            cut_mesh: &base_cut_mesh,
            prepared_faces: &base_faces,
            vertex_map: &base_vertex_map,
            contour_vertex_maps: Vec::new(),
            output_vertices: &output_vertices,
            operand: ExactBooleanOperand::First,
            first_virtual_vertex: 3,
            flip_orientation: false,
        },
        &[],
        &[],
        Some(copied_edges),
    );

    assert_eq!(plan.export_failed_faces, 0);
    assert_eq!(plan.translated_copied_edge_records, 6);
    assert_eq!(plan.translated_copied_face_records, 1);
    assert_eq!(plan.exported_face_indices, vec![[0, 1, 2], [3, 5, 4]]);
    assert!(plan.export_changed_faces);
}

#[test]
fn prepared_base_copied_vertices_start_after_base_virtual_vertices() {
    let base_cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 3]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let incoming_cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 5],
        faces: vec![[1, 4, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![7],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let base_faces = [0];
    let incoming_faces = [0];
    let base_vertex_map = [Some(0), Some(1), Some(2), None];
    let incoming_vertex_map = [None, Some(1), Some(2), None, None];
    let output_vertices = vec![[0.0; 3]; 3];
    let face_sources: [ExactBooleanOutputFaceSource; 0] = [];
    let copied_edges = ExactMeshlibCopiedEdgeTranslationInput {
        cut_mesh: &incoming_cut_mesh,
        prepared_faces: &incoming_faces,
        vertex_map: &incoming_vertex_map,
        contour_vertex_maps: vec![([2, 1], [2, 1])],
        contour_vertex_map_source_indices: vec![Some(0)],
        face_sources: &face_sources,
        incoming_operand: ExactBooleanOperand::Second,
        first_virtual_vertex: 3,
        append_prepared_faces: true,
        flip_orientation: false,
    };

    let plan = exact_meshlib_record_rewrite_apply_plan_with_prepared_base(
        ExactMeshlibPreparedBaseTopologyInput {
            cut_mesh: &base_cut_mesh,
            prepared_faces: &base_faces,
            vertex_map: &base_vertex_map,
            contour_vertex_maps: Vec::new(),
            output_vertices: &output_vertices,
            operand: ExactBooleanOperand::First,
            first_virtual_vertex: 3,
            flip_orientation: false,
        },
        &[],
        &[],
        Some(copied_edges),
    );

    assert_eq!(plan.export_failed_faces, 0);
    assert_eq!(plan.exported_face_indices, vec![[0, 1, 3], [1, 4, 2]]);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_uses_prepared_source_records() {
    let base_cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let incoming_cut_mesh = ExactCutMeshResult {
        vertices: vec![[0.0; 3]; 4],
        faces: vec![[2, 1, 3]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![7],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let base_faces = [0];
    let incoming_faces = [0];
    let base_vertex_map = [Some(0), Some(1), Some(2)];
    let incoming_vertex_map = [None, Some(1), Some(2), None];
    let output_vertices = vec![[0.0; 3]; 3];
    let face_sources: [ExactBooleanOutputFaceSource; 0] = [];
    let command = meshlib_rewrite_command([1, 2], [2, 1]);
    let copied_edges = ExactMeshlibCopiedEdgeTranslationInput {
        cut_mesh: &incoming_cut_mesh,
        prepared_faces: &incoming_faces,
        vertex_map: &incoming_vertex_map,
        contour_vertex_maps: vec![(command.from_source_edge, command.this_source_edge)],
        contour_vertex_map_source_indices: vec![Some(command.from_source_edge_index)],
        face_sources: &face_sources,
        incoming_operand: ExactBooleanOperand::Second,
        first_virtual_vertex: 3,
        append_prepared_faces: true,
        flip_orientation: false,
    };

    let plan = exact_meshlib_record_rewrite_apply_plan_with_prepared_base(
        ExactMeshlibPreparedBaseTopologyInput {
            cut_mesh: &base_cut_mesh,
            prepared_faces: &base_faces,
            vertex_map: &base_vertex_map,
            contour_vertex_maps: Vec::new(),
            output_vertices: &output_vertices,
            operand: ExactBooleanOperand::First,
            first_virtual_vertex: 3,
            flip_orientation: false,
        },
        &[command],
        &[],
        Some(copied_edges),
    );

    assert_eq!(plan.applied_commands, 1);
    assert_eq!(plan.failed_commands, 0);
    assert_eq!(plan.failed_missing_source_edges, 0);
    assert_eq!(plan.translated_copied_edge_records, 4);
    assert_eq!(plan.translated_copied_face_records, 1);
    assert_eq!(plan.mapped_source_record_replays, 0);
    assert_eq!(plan.mapped_source_record_replays_on_near_stitch_targets, 0);
    assert_eq!(plan.translated_face_records, 1);
    assert_eq!(plan.export_failed_faces, 0);
    assert_eq!(plan.export_face_record_left_mismatch_faces, 0);
    assert_eq!(plan.export_face_left_ring_mismatch_faces, 0);
    assert_eq!(plan.export_other_failed_faces, 0);
    assert_eq!(plan.export_failed_face_indices, Vec::<usize>::new());
    assert_eq!(plan.exported_face_indices, vec![[0, 1, 2], [2, 2, 3]]);
    assert!(plan.ready_for_export);
}

#[test]
fn output_topology_refreshes_stale_meshlib_face_record() {
    let faces = [[0, 1, 2]];
    let mut output = OutputFaceTopology::from_faces(&faces).unwrap();
    let stale = output.topology.make_edge(Some(9), Some(10));
    output.face_edges[0] = stale;

    assert_eq!(
        output.export_face_results()[0],
        Err("MeshLib face record edge must have face on left")
    );
    assert_eq!(output.refresh_meshlib_face_records(), 1);
    assert_eq!(output.export_face_results(), vec![Ok(faces[0])]);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_applies_synthetic_from_side() {
    let faces = [[0, 1, 2]];
    let mut command = meshlib_rewrite_command([1, 2], [2, 1]);
    command.from_side_synthetic = true;
    command.synthetic_sides = 1;

    let plan = exact_meshlib_record_rewrite_apply_plan(&faces, &[command], &[]);

    assert_eq!(plan.commands, 1);
    assert_eq!(plan.applied_commands, 1);
    assert_eq!(plan.failed_commands, 0);
    assert_eq!(plan.failed_missing_target_edges, 0);
    assert_eq!(plan.failed_closed_target_edges, 0);
    assert_eq!(plan.failed_missing_source_edges, 0);
    assert_eq!(plan.translated_face_records, 0);
    assert_eq!(plan.synthetic_side_edges, 1);
    assert_eq!(plan.exported_faces, 1);
    assert_eq!(plan.export_failed_faces, 0);
    assert_eq!(plan.export_non_triangular_faces, 0);
    assert_eq!(plan.export_left_ring_not_closed_faces, 0);
    assert_eq!(plan.export_missing_origin_faces, 0);
    assert_eq!(plan.export_face_record_left_mismatch_faces, 0);
    assert_eq!(plan.export_face_left_ring_mismatch_faces, 0);
    assert_eq!(plan.export_other_failed_faces, 0);
    assert!(plan.export_failed_face_indices.is_empty());
    assert_eq!(plan.exported_face_indices, faces);
    assert_eq!(plan.topology_edges_before_rewrite, 3);
    assert_eq!(plan.topology_edges_after_rewrite, 4);
    assert!(plan.ready_for_export);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_with_copied_edges_keeps_synthetic_from_side_idempotent()
{
    let faces = [[0, 1, 2], [2, 1, 3]];
    let face_sources = [
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
    ];
    let mut command = meshlib_rewrite_command([1, 2], [2, 1]);
    command.from_side_synthetic = true;
    command.synthetic_sides = 1;
    let cut_mesh = ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        faces: vec![[2, 1, 3]],
        cut_edges: vec![[2, 1]],
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![0],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let prepared_faces = [0];
    let vertex_map = [None, Some(1), Some(2), Some(3)];
    let copied_edges = ExactMeshlibCopiedEdgeTranslationInput {
        cut_mesh: &cut_mesh,
        prepared_faces: &prepared_faces,
        vertex_map: &vertex_map,
        contour_vertex_maps: vec![([2, 1], [1, 2])],
        contour_vertex_map_source_indices: vec![Some(0)],
        face_sources: &face_sources,
        incoming_operand: ExactBooleanOperand::Second,
        first_virtual_vertex: 4,
        append_prepared_faces: false,
        flip_orientation: false,
    };

    let plan = exact_meshlib_record_rewrite_apply_plan_with_copied_edges(
        &faces,
        &face_sources,
        &[command],
        &[],
        copied_edges,
    );

    assert_eq!(plan.applied_commands, 1);
    assert_eq!(plan.failed_commands, 0);
    assert_eq!(plan.synthetic_side_edges, 1);
    assert_eq!(plan.topology_edges_after_rewrite, 9);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_reports_missing_target_edge() {
    let faces = [[0, 1, 2]];
    let command = meshlib_rewrite_command([2, 3], [1, 2]);

    let plan = exact_meshlib_record_rewrite_apply_plan(&faces, &[command], &[]);

    assert_eq!(plan.applied_commands, 0);
    assert_eq!(plan.failed_commands, 1);
    assert_eq!(plan.failed_missing_target_edges, 1);
    assert_eq!(plan.failed_closed_target_edges, 0);
    assert_eq!(plan.failed_missing_source_edges, 0);
    assert_eq!(plan.failed_other_commands, 0);
    assert_eq!(
        plan.entries[0].error,
        Some("missing MeshLib rewrite target contour edge")
    );
    assert!(!plan.ready_for_export);
}

#[test]
fn output_topology_applies_meshlib_near_stitch_edge_update() {
    let faces = [[0, 1, 2], [1, 3, 4]];
    let mut topology = OutputFaceTopology::from_faces(&faces).unwrap();

    topology
        .apply_meshlib_near_stitch_edge_update([1, 0], [1, 3])
        .unwrap();

    assert_eq!(topology.export_faces().unwrap(), faces);
    assert_eq!(topology.not_lone_undirected_edge_count(), 6);
}

#[test]
fn output_topology_reports_missing_meshlib_near_stitch_edge() {
    let faces = [[0, 1, 2]];
    let mut topology = OutputFaceTopology::from_faces(&faces).unwrap();

    let error = topology
        .apply_meshlib_near_stitch_edge_update([1, 0], [7, 8])
        .unwrap_err();

    assert_eq!(error, "missing MeshLib near stitch next edge");
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_applies_near_stitch_update_command() {
    let faces = [[0, 1, 2], [1, 3, 4]];
    let update = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: None,
        endpoint: None,
        source_operand: None,
        previous_source_halfedge: None,
        next_source_halfedge: None,
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: None,
        strict_source_identity: false,
        previous_edge: [1, 0],
        next_edge: [1, 3],
    };

    let plan = exact_meshlib_record_rewrite_apply_plan(&faces, &[], &[update]);

    assert_eq!(plan.commands, 0);
    assert_eq!(plan.near_stitch_update_commands, 1);
    assert_eq!(plan.applied_near_stitch_updates, 1);
    assert_eq!(plan.failed_near_stitch_updates, 0);
    assert_eq!(plan.failed_near_stitch_start_updates, 0);
    assert_eq!(plan.failed_near_stitch_end_updates, 0);
    assert_eq!(plan.failed_missing_near_stitch_previous_edges, 0);
    assert_eq!(plan.failed_missing_near_stitch_next_edges, 0);
    assert_eq!(plan.failed_other_near_stitch_updates, 0);
    assert_eq!(
        plan.near_stitch_update_entries[0].status,
        ExactMeshlibNearStitchUpdateStatus::Applied
    );
    assert_eq!(plan.exported_face_indices, faces);
    assert!(plan.ready_for_export);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_reports_missing_near_stitch_edge() {
    let faces = [[0, 1, 2]];
    let update = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: None,
        endpoint: None,
        source_operand: None,
        previous_source_halfedge: None,
        next_source_halfedge: None,
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: None,
        strict_source_identity: false,
        previous_edge: [1, 0],
        next_edge: [7, 8],
    };

    let plan = exact_meshlib_record_rewrite_apply_plan(&faces, &[], &[update]);

    assert_eq!(plan.near_stitch_update_commands, 1);
    assert_eq!(plan.applied_near_stitch_updates, 0);
    assert_eq!(plan.failed_near_stitch_updates, 1);
    assert_eq!(plan.failed_near_stitch_start_updates, 0);
    assert_eq!(plan.failed_near_stitch_end_updates, 0);
    assert_eq!(plan.failed_missing_near_stitch_previous_edges, 0);
    assert_eq!(plan.failed_missing_near_stitch_next_edges, 1);
    assert_eq!(plan.failed_other_near_stitch_updates, 0);
    assert_eq!(
        plan.near_stitch_update_entries[0].status,
        ExactMeshlibNearStitchUpdateStatus::Failed
    );
    assert_eq!(
        plan.near_stitch_update_entries[0].error,
        Some("missing MeshLib near stitch next edge")
    );
    assert!(!plan.ready_for_export);
}

#[test]
fn exact_meshlib_record_rewrite_apply_plan_reports_specific_near_stitch_guard() {
    let faces = [[0, 1, 2]];
    let update = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: None,
        endpoint: None,
        source_operand: None,
        previous_source_halfedge: None,
        next_source_halfedge: None,
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: None,
        strict_source_identity: false,
        previous_edge: [0, 1],
        next_edge: [0, 1],
    };

    let plan = exact_meshlib_record_rewrite_apply_plan(&faces, &[], &[update]);

    assert_eq!(plan.near_stitch_update_commands, 1);
    assert_eq!(plan.applied_near_stitch_updates, 0);
    assert_eq!(plan.failed_near_stitch_updates, 1);
    assert_eq!(plan.failed_near_stitch_start_updates, 0);
    assert_eq!(plan.failed_near_stitch_end_updates, 0);
    assert_eq!(plan.failed_missing_near_stitch_previous_edges, 0);
    assert_eq!(plan.failed_missing_near_stitch_next_edges, 0);
    assert_eq!(plan.failed_near_stitch_origin_mismatches, 0);
    assert_eq!(plan.failed_near_stitch_previous_left_faces, 1);
    assert_eq!(plan.failed_near_stitch_next_right_faces, 0);
    assert_eq!(plan.failed_other_near_stitch_updates, 0);
    assert_eq!(
        plan.near_stitch_update_entries[0].error,
        Some("previous near stitch edge must not have a left face")
    );
    assert!(!plan.ready_for_export);
}

#[test]
fn exact_topology_splice_apply_plan_verifies_boundary_stitch() {
    let faces = [[0, 1, 2]];
    let splice_plan = exact_topology_splice_plan(&faces, &[stitched_edge([1, 2])]);

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &[]);

    assert_eq!(apply_plan.verified_boundary_edges, 1);
    assert_eq!(apply_plan.materialized_boundary_edges, 1);
    assert_eq!(apply_plan.exported_faces, 1);
    assert_eq!(apply_plan.export_failed_faces, 0);
    assert_eq!(apply_plan.exported_face_indices, faces);
    assert_eq!(apply_plan.exported_boundary_edges, 3);
    assert_eq!(apply_plan.exported_manifold_edges, 0);
    assert_eq!(apply_plan.exported_non_manifold_edges, 0);
    assert_eq!(apply_plan.topology_edges_before_materialization, 3);
    assert_eq!(apply_plan.topology_edges_after_materialization, 3);
    assert_eq!(apply_plan.deleted_synthetic_stitch_edges, 1);
    assert_eq!(apply_plan.duplicated_output_topology_edges, 0);
    assert_eq!(apply_plan.blocked_edges, 0);
    assert!(apply_plan.ready_for_mutation);
    assert_eq!(apply_plan.entries[0].directed_face_edge, Some([1, 2]));
    assert_eq!(
        apply_plan.entries[0].status,
        ExactTopologySpliceApplyStatus::VerifiedBoundaryStitch
    );
}

#[test]
fn exact_topology_splice_apply_plan_preserves_manifold_edges() {
    let faces = [[0, 1, 2], [2, 1, 3]];
    let splice_plan = exact_topology_splice_plan(&faces, &[stitched_edge([1, 2])]);

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &[]);

    assert_eq!(apply_plan.already_manifold_edges, 1);
    assert_eq!(apply_plan.verified_boundary_edges, 0);
    assert_eq!(apply_plan.exported_faces, 2);
    assert_eq!(apply_plan.export_failed_faces, 0);
    assert_eq!(apply_plan.exported_face_indices, faces);
    assert_eq!(apply_plan.exported_boundary_edges, 4);
    assert_eq!(apply_plan.exported_manifold_edges, 1);
    assert_eq!(apply_plan.exported_non_manifold_edges, 0);
    assert_eq!(apply_plan.topology_edges_before_materialization, 5);
    assert_eq!(apply_plan.topology_edges_after_materialization, 5);
    assert_eq!(apply_plan.deleted_synthetic_stitch_edges, 0);
    assert_eq!(apply_plan.duplicated_output_topology_edges, 0);
    assert!(apply_plan.ready_for_mutation);
    assert_eq!(
        apply_plan.entries[0].status,
        ExactTopologySpliceApplyStatus::AlreadyManifold
    );
}

#[test]
fn exact_topology_splice_apply_plan_verifies_ordered_boundary_stitch_path() {
    let faces = [[0, 1, 2], [2, 3, 0]];
    let splice_plan =
        exact_topology_splice_plan(&faces, &[stitched_edge([1, 2]), stitched_edge([2, 3])]);
    let stitch_paths = [ExactStitchPath {
        pair_indices: vec![0, 1],
        closed: false,
    }];

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &stitch_paths);

    assert_eq!(apply_plan.stitched_paths, 1);
    assert_eq!(apply_plan.verified_boundary_paths, 1);
    assert_eq!(apply_plan.blocked_paths, 0);
    assert_eq!(apply_plan.failed_paths, 0);
    assert_eq!(apply_plan.verified_boundary_edges, 2);
    assert_eq!(apply_plan.materialized_boundary_edges, 2);
    assert!(apply_plan.ready_for_mutation);
}

#[test]
fn exact_topology_splice_apply_plan_rejects_open_path_marked_closed() {
    let faces = [[0, 1, 2], [2, 3, 0]];
    let splice_plan =
        exact_topology_splice_plan(&faces, &[stitched_edge([1, 2]), stitched_edge([2, 3])]);
    let stitch_paths = [ExactStitchPath {
        pair_indices: vec![0, 1],
        closed: true,
    }];

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &stitch_paths);

    assert_eq!(apply_plan.stitched_paths, 1);
    assert_eq!(apply_plan.failed_paths, 1);
    assert_eq!(apply_plan.verified_boundary_paths, 0);
    assert_eq!(apply_plan.materialization_failed_edges, 2);
    assert_eq!(apply_plan.materialized_boundary_edges, 0);
    assert!(!apply_plan.ready_for_mutation);
}

#[test]
fn exact_topology_splice_apply_plan_blocks_non_manifold_edges() {
    let faces = [[0, 1, 2], [2, 1, 3], [1, 2, 4]];
    let splice_plan = exact_topology_splice_plan(&faces, &[stitched_edge([1, 2])]);

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &[]);

    assert_eq!(apply_plan.blocked_edges, 1);
    assert_eq!(apply_plan.exported_boundary_edges, 6);
    assert_eq!(apply_plan.exported_manifold_edges, 0);
    assert_eq!(apply_plan.exported_non_manifold_edges, 1);
    assert!(!apply_plan.ready_for_mutation);
    assert_eq!(
        apply_plan.entries[0].status,
        ExactTopologySpliceApplyStatus::BlockedNonManifold
    );
}

#[test]
fn exact_topology_splice_apply_plan_fails_same_direction_boundary_pair() {
    let faces = [[0, 1, 2]];
    let bad_source = ExactBooleanStitchedEdgeSource {
        output_edge: [1, 2],
        first_output_edge: Some([1, 2]),
        second_output_edge: Some([1, 2]),
        first_stitch_edge: Some([1, 2]),
        second_stitch_edge: Some([1, 2]),
        first_stitch_edge_synthetic: false,
        second_stitch_edge_synthetic: false,
        first_edge_index: 0,
        second_edge_index: 0,
        first_cut_edge: [1, 2],
        second_cut_edge: [1, 2],
    };
    let splice_plan = exact_topology_splice_plan(&faces, &[bad_source]);

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &[]);

    assert_eq!(apply_plan.failed_edges, 1);
    assert!(!apply_plan.ready_for_mutation);
    assert_eq!(
        apply_plan.entries[0].status,
        ExactTopologySpliceApplyStatus::FailedBoundaryStitch
    );
}

#[test]
fn exact_topology_splice_apply_plan_blocks_missing_directed_side() {
    let faces = [[0, 1, 2]];
    let missing_source = ExactBooleanStitchedEdgeSource {
        output_edge: [1, 2],
        first_output_edge: None,
        second_output_edge: Some([2, 1]),
        first_stitch_edge: None,
        second_stitch_edge: Some([2, 1]),
        first_stitch_edge_synthetic: false,
        second_stitch_edge_synthetic: false,
        first_edge_index: 0,
        second_edge_index: 0,
        first_cut_edge: [1, 2],
        second_cut_edge: [1, 2],
    };
    let splice_plan = exact_topology_splice_plan(&faces, &[missing_source]);

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &[]);

    assert_eq!(apply_plan.blocked_edges, 1);
    assert_eq!(apply_plan.failed_edges, 0);
    assert!(!apply_plan.ready_for_mutation);
    assert_eq!(
        apply_plan.entries[0].status,
        ExactTopologySpliceApplyStatus::BlockedMissingSide
    );
}

#[test]
fn exact_topology_splice_apply_plan_verifies_synthetic_missing_side() {
    let faces = [[0, 1, 2]];
    let synthetic_source = ExactBooleanStitchedEdgeSource {
        output_edge: [1, 2],
        first_output_edge: None,
        second_output_edge: Some([2, 1]),
        first_stitch_edge: Some([1, 2]),
        second_stitch_edge: Some([2, 1]),
        first_stitch_edge_synthetic: true,
        second_stitch_edge_synthetic: false,
        first_edge_index: 0,
        second_edge_index: 0,
        first_cut_edge: [1, 2],
        second_cut_edge: [1, 2],
    };
    let splice_plan = exact_topology_splice_plan(&faces, &[synthetic_source]);

    let apply_plan = exact_topology_splice_apply_plan(&faces, &splice_plan, &[]);

    assert_eq!(apply_plan.verified_boundary_edges, 1);
    assert_eq!(apply_plan.synthetic_side_edges, 1);
    assert_eq!(apply_plan.materialized_boundary_edges, 1);
    assert_eq!(apply_plan.exported_faces, 1);
    assert_eq!(apply_plan.export_failed_faces, 0);
    assert_eq!(apply_plan.exported_face_indices, faces);
    assert_eq!(apply_plan.exported_boundary_edges, 3);
    assert_eq!(apply_plan.exported_manifold_edges, 0);
    assert_eq!(apply_plan.exported_non_manifold_edges, 0);
    assert_eq!(apply_plan.topology_edges_before_materialization, 3);
    assert_eq!(apply_plan.topology_edges_after_materialization, 3);
    assert_eq!(apply_plan.deleted_synthetic_stitch_edges, 1);
    assert!(apply_plan.ready_for_mutation);
    assert_eq!(
        apply_plan.entries[0].status,
        ExactTopologySpliceApplyStatus::VerifiedBoundaryStitch
    );
}
