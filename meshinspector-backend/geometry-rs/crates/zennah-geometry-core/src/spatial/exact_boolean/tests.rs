use super::super::exact_boolean_assembly::assemble_classified_boolean_with_stitch;
use super::super::exact_classify::ExactMeshPartClassification;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_fill_apply::ExactCutHoleFillResult;
use super::super::exact_stitch::{ExactStitchEdgePair, ExactStitchPath, ExactStitchPlan};
use super::*;

fn cut_mesh(vertices: Vec<[f64; 3]>, faces: Vec<[i64; 3]>) -> ExactCutMeshResult {
    ExactCutMeshResult {
        source_face_for_faces: (0..faces.len()).collect(),
        vertices,
        faces,
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        skipped_source_faces: Vec::new(),
    }
}

fn cut_fill_with_paths(
    cut_edge_paths: Vec<Vec<[usize; 2]>>,
    cut_edge_path_closed: Vec<bool>,
) -> ExactCutHoleFillResult {
    let cut_edges = cut_edge_paths.iter().flatten().copied().collect();
    ExactCutHoleFillResult {
        mesh: ExactCutMeshResult {
            source_face_for_faces: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            cut_edges,
            cut_edge_paths,
            cut_edge_path_closed,
            skipped_source_faces: Vec::new(),
        },
        fill_plans: Vec::new(),
        added_face_ranges: Vec::new(),
    }
}

fn tetrahedron(offset: [f64; 3]) -> ExactCutMeshResult {
    scaled_tetrahedron(offset, 1.0)
}

fn scaled_tetrahedron(offset: [f64; 3], scale: f64) -> ExactCutMeshResult {
    let vertices = vec![
        [offset[0], offset[1], offset[2]],
        [scale + offset[0], offset[1], offset[2]],
        [offset[0], scale + offset[1], offset[2]],
        [offset[0], offset[1], scale + offset[2]],
    ];
    let faces = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
    cut_mesh(vertices, faces)
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

fn assert_exported_mesh_matches_output(result: &ExactBooleanPipelineResult) {
    let exported_stats = result
        .diagnostics
        .topology_splice_exported_mesh_stats
        .as_ref()
        .unwrap();
    let output_stats = &result.diagnostics.output_mesh_stats;
    assert!(!result.diagnostics.topology_splice_export_changed_faces);
    assert_eq!(exported_stats.face_count, output_stats.face_count);
    assert_eq!(exported_stats.vertex_count, output_stats.vertex_count);
    assert_close(
        exported_stats.surface_area_mm2,
        output_stats.surface_area_mm2,
    );
    assert_close(exported_stats.volume_mm3, output_stats.volume_mm3);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_exported_mesh_health
            .as_ref()
            .unwrap()
            .boundary_edge_count,
        result.diagnostics.output_mesh_health.boundary_edge_count
    );
}

#[test]
fn exact_assemble_boolean_union_keeps_outside_parts() {
    let first = tetrahedron([0.0, 0.0, 0.0]);
    let second = tetrahedron([3.0, 0.0, 0.0]);

    let result =
        exact_assemble_boolean_from_cut_meshes(&first, &second, ExactBooleanOperation::Union, 1e-9)
            .unwrap();

    assert_eq!(result.selected_first_faces, vec![0, 1, 2, 3]);
    assert_eq!(result.selected_second_faces, vec![0, 1, 2, 3]);
    assert_eq!(result.faces.len(), 8);
    assert_eq!(result.face_sources.len(), 8);
    assert!(result.stitched_edge_sources.is_empty());
    assert_eq!(
        result.face_sources[0],
        ExactBooleanOutputFaceSource {
            operand: ExactBooleanOperand::First,
            cut_face: 0,
            source_face: 0,
        }
    );
    assert_eq!(
        result.face_sources[4],
        ExactBooleanOutputFaceSource {
            operand: ExactBooleanOperand::Second,
            cut_face: 0,
            source_face: 0,
        }
    );
    assert!(!result.flipped_first);
    assert!(!result.flipped_second);
}

#[test]
fn exact_assemble_boolean_intersection_drops_disjoint_parts() {
    let first = tetrahedron([0.0, 0.0, 0.0]);
    let second = tetrahedron([3.0, 0.0, 0.0]);

    let result = exact_assemble_boolean_from_cut_meshes(
        &first,
        &second,
        ExactBooleanOperation::Intersection,
        1e-9,
    )
    .unwrap();

    assert!(result.selected_first_faces.is_empty());
    assert!(result.selected_second_faces.is_empty());
    assert!(result.faces.is_empty());
}

#[test]
fn exact_assemble_boolean_difference_flips_subtracted_inside_part() {
    let first = tetrahedron([0.0, 0.0, 0.0]);
    let second = tetrahedron([3.0, 0.0, 0.0]);
    let first_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0, 1, 2, 3],
        used_cut_path_sides: false,
        cut_paths_consistent: true,
        cut_left_components: 0,
        cut_right_components: 0,
        cut_path_overlap_components: 0,
    };
    let second_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0, 1, 2, 3],
        used_cut_path_sides: false,
        cut_paths_consistent: true,
        cut_left_components: 0,
        cut_right_components: 0,
        cut_path_overlap_components: 0,
    };

    let result = assemble_classified_boolean(
        &first,
        &second,
        Some(&first_classification),
        Some(&second_classification),
        ExactBooleanOperation::DifferenceAB,
    );

    assert_eq!(result.selected_first_faces, vec![0, 1, 2, 3]);
    assert_eq!(result.selected_second_faces, vec![0, 1, 2, 3]);
    assert!(!result.flipped_first);
    assert!(result.flipped_second);
    assert_eq!(result.faces[4][1], 6);
    assert_eq!(result.faces[4][2], 5);
    assert_eq!(
        result.face_sources[4],
        ExactBooleanOutputFaceSource {
            operand: ExactBooleanOperand::Second,
            cut_face: 0,
            source_face: 0,
        }
    );
}

#[test]
fn exact_boolean_from_meshes_cuts_fills_and_assembles_union() {
    let first = tetrahedron([0.0, 0.0, 0.0]);
    let second = tetrahedron([3.0, 0.0, 0.0]);

    let result = exact_boolean_from_meshes(
        &first.vertices,
        &first.faces,
        &second.vertices,
        &second.faces,
        ExactBooleanOperation::Union,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.first_cut.fill_plans.is_empty());
    assert!(result.second_cut.fill_plans.is_empty());
    assert!(result.stitch_plan.compatible);
    assert!(result.diagnostics.parity_ready);
    assert!(!result.diagnostics.requires_topology_splice);
    assert_eq!(result.diagnostics.output_faces, result.assembly.faces.len());
    assert_eq!(result.diagnostics.output_mesh_stats.vertex_count, 8);
    assert_eq!(result.diagnostics.output_mesh_stats.face_count, 8);
    assert_eq!(result.diagnostics.output_mesh_stats.connected_components, 2);
    assert_eq!(result.diagnostics.output_mesh_stats.boundary_edge_count, 0);
    assert_close(result.diagnostics.output_mesh_stats.volume_mm3, 1.0 / 3.0);
    assert!(result.diagnostics.output_mesh_health.is_closed);
    assert_eq!(
        result.diagnostics.output_mesh_health.nonmanifold_edge_count,
        0
    );
    assert_exported_mesh_matches_output(&result);
    assert_eq!(result.assembly.selected_first_faces, vec![0, 1, 2, 3]);
    assert_eq!(result.assembly.selected_second_faces, vec![0, 1, 2, 3]);
    assert_eq!(result.assembly.faces.len(), 8);
    assert_eq!(result.assembly.face_sources.len(), 8);
}

#[test]
fn exact_boolean_from_meshes_handles_contained_trivial_operations() {
    let outer = scaled_tetrahedron([0.0, 0.0, 0.0], 4.0);
    let inner = tetrahedron([0.5, 0.5, 0.5]);

    let union = exact_boolean_from_meshes(
        &outer.vertices,
        &outer.faces,
        &inner.vertices,
        &inner.faces,
        ExactBooleanOperation::Union,
        8,
        1e-9,
    )
    .unwrap();
    assert!(union.stitch_plan.compatible);
    assert!(union.diagnostics.parity_ready);
    assert_eq!(union.assembly.selected_first_faces, vec![0, 1, 2, 3]);
    assert!(union.assembly.selected_second_faces.is_empty());
    assert_eq!(union.assembly.faces.len(), 4);

    let intersection = exact_boolean_from_meshes(
        &outer.vertices,
        &outer.faces,
        &inner.vertices,
        &inner.faces,
        ExactBooleanOperation::Intersection,
        8,
        1e-9,
    )
    .unwrap();
    assert!(intersection.diagnostics.parity_ready);
    assert!(intersection.assembly.selected_first_faces.is_empty());
    assert_eq!(
        intersection.assembly.selected_second_faces,
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        intersection.assembly.face_sources[0].operand,
        ExactBooleanOperand::Second
    );

    let difference = exact_boolean_from_meshes(
        &outer.vertices,
        &outer.faces,
        &inner.vertices,
        &inner.faces,
        ExactBooleanOperation::DifferenceAB,
        8,
        1e-9,
    )
    .unwrap();
    assert!(difference.diagnostics.parity_ready);
    assert_eq!(difference.assembly.selected_first_faces, vec![0, 1, 2, 3]);
    assert_eq!(difference.assembly.selected_second_faces, vec![0, 1, 2, 3]);
    assert!(difference.assembly.flipped_second);
    assert_eq!(difference.assembly.faces.len(), 8);
}

#[test]
fn exact_boolean_from_meshes_cuts_intersecting_solids_and_exports_closed_topology() {
    let first = scaled_tetrahedron([0.0, 0.0, 0.0], 2.0);
    let second = scaled_tetrahedron([0.5, 0.5, 0.5], 2.0);

    let result = exact_boolean_from_meshes(
        &first.vertices,
        &first.faces,
        &second.vertices,
        &second.faces,
        ExactBooleanOperation::Union,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.diagnostics.first_cut_edges > 0);
    assert!(result.diagnostics.second_cut_edges > 0);
    assert!(!result.diagnostics.possible_missing_cut_intersections);
    assert!(result.stitch_plan.compatible);
    assert_eq!(
        result.diagnostics.stitched_output_edges,
        result.stitch_plan.pairs.len()
    );
    assert!(result.diagnostics.topology_splice_ready);
    assert!(result.diagnostics.topology_splice_apply_ready);
    assert!(result.diagnostics.first_prepare_part_dividable);
    assert!(result.diagnostics.second_prepare_part_dividable);
    assert_eq!(result.diagnostics.first_cut_path_side_components, [1, 1]);
    assert_eq!(result.diagnostics.second_cut_path_side_components, [1, 1]);
    assert!(result.diagnostics.result_cut_paths_complete);
    assert_eq!(
        result.diagnostics.result_cut_mapped_paths,
        result.diagnostics.result_cut_paths
    );
    assert_eq!(
        result.diagnostics.result_cut_mapped_path_edges,
        result.diagnostics.result_cut_path_edges
    );
    assert_eq!(result.diagnostics.topology_splice_missing_edges, 0);
    assert_eq!(result.diagnostics.topology_splice_non_manifold_edges, 0);
    assert_eq!(result.diagnostics.topology_splice_blocked_edges, 0);
    assert_eq!(result.diagnostics.topology_splice_failed_edges, 0);
    assert_eq!(result.diagnostics.stitched_output_edges_needing_splice, 0);
    assert_eq!(
        result.diagnostics.topology_splice_verified_boundary_edges,
        result.diagnostics.stitched_output_edges_needing_splice
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_materialization_failed_edges,
        0
    );
    assert_eq!(
        result.diagnostics.topology_splice_exported_faces,
        result.assembly.faces.len()
    );
    assert_eq!(result.diagnostics.topology_splice_export_failed_faces, 0);
    assert_exported_mesh_matches_output(&result);
    assert_eq!(
        result.diagnostics.topology_splice_duplicated_output_edges,
        0
    );
    assert!(
        result.assembly.vertices.len()
            < result.first_cut.mesh.vertices.len() + result.second_cut.mesh.vertices.len()
    );
    assert!(
        result.diagnostics.first_vertices_mixed_against_second
            || result.diagnostics.second_vertices_mixed_against_first
    );
    assert!(result.diagnostics.requires_topology_splice);
    assert_eq!(result.diagnostics.output_mesh_health.boundary_edge_count, 0);
    assert!(result.diagnostics.parity_ready);
}

#[test]
fn exact_boolean_intersection_uses_meshlib_left_hole_connect_order() {
    let first = scaled_tetrahedron([0.0, 0.0, 0.0], 2.0);
    let second = scaled_tetrahedron([0.5, 0.5, 0.5], 2.0);

    let result = exact_boolean_from_meshes(
        &first.vertices,
        &first.faces,
        &second.vertices,
        &second.faces,
        ExactBooleanOperation::Intersection,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.diagnostics.requires_topology_splice);
    assert!(result.diagnostics.parity_ready);
    assert!(!result.assembly.selected_first_faces.is_empty());
    assert!(!result.assembly.selected_second_faces.is_empty());
    assert_eq!(
        result.assembly.face_sources[0].operand,
        ExactBooleanOperand::Second
    );
}

#[test]
fn meshlib_result_cut_path_summary_follows_optional_out_cut_contract() {
    let first = cut_fill_with_paths(vec![vec![[0, 1]], vec![[1, 2], [2, 0]]], vec![false, true]);
    let second = cut_fill_with_paths(vec![vec![[10, 11], [11, 12], [12, 10]]], vec![true]);

    assert_eq!(
        meshlib_result_cut_path_summary(ExactBooleanOperation::Union, &first, &second),
        MeshlibResultCutPathSummary {
            paths: 2,
            path_edges: 3,
            closed_paths: 1,
        }
    );
    assert_eq!(
        meshlib_result_cut_path_summary(ExactBooleanOperation::Intersection, &first, &second),
        MeshlibResultCutPathSummary {
            paths: 1,
            path_edges: 3,
            closed_paths: 1,
        }
    );
    assert_eq!(
        meshlib_result_cut_path_summary(ExactBooleanOperation::InsideA, &first, &second).paths,
        2
    );
    assert_eq!(
        meshlib_result_cut_path_summary(ExactBooleanOperation::OutsideB, &first, &second).paths,
        1
    );
}

#[test]
fn meshlib_result_cut_mapping_uses_stitch_counterpart_for_excluded_source_edge() {
    let first = ExactCutMeshResult {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1]],
        cut_edge_paths: vec![vec![[0, 1]]],
        cut_edge_path_closed: vec![false],
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let second = ExactCutMeshResult {
        vertices: first.vertices.clone(),
        faces: first.faces.clone(),
        cut_edges: vec![[0, 1]],
        cut_edge_paths: vec![vec![[1, 0]]],
        cut_edge_path_closed: vec![false],
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let first_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: Vec::new(),
        used_cut_path_sides: true,
        cut_paths_consistent: true,
        cut_left_components: 0,
        cut_right_components: 1,
        cut_path_overlap_components: 0,
    };
    let second_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0],
        used_cut_path_sides: true,
        cut_paths_consistent: true,
        cut_left_components: 1,
        cut_right_components: 0,
        cut_path_overlap_components: 0,
    };
    let stitch_plan = ExactStitchPlan {
        pairs: vec![ExactStitchEdgePair {
            first_edge_index: 0,
            second_edge_index: 0,
            first_edge: [0, 1],
            second_edge: [1, 0],
            second_reversed: true,
        }],
        paths: vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }],
        unmatched_first_edges: Vec::new(),
        unmatched_second_edges: Vec::new(),
        compatible: true,
    };

    let result = assemble_classified_boolean_with_stitch(
        &first,
        &second,
        Some(&first_classification),
        Some(&second_classification),
        Some(&stitch_plan),
        ExactBooleanOperation::Union,
    );

    assert!(result.result_cut_paths_complete);
    assert_eq!(result.result_cut_paths, vec![vec![[0, 1]]]);
    assert_eq!(result.result_cut_path_closed, vec![false]);
    assert!(result.stitched_edge_sources[0].first_stitch_edge_synthetic);
}

#[test]
fn meshlib_result_cut_mapping_retains_segments_around_missing_source_edge() {
    let first = ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [10.0, 0.0, 0.0],
        ],
        faces: vec![[0, 1, 2], [3, 4, 5]],
        cut_edges: vec![[0, 1], [1, 6], [3, 4]],
        cut_edge_paths: vec![vec![[0, 1], [1, 6], [3, 4]]],
        cut_edge_path_closed: vec![true],
        source_face_for_faces: vec![0, 1],
        skipped_source_faces: Vec::new(),
    };
    let second = ExactCutMeshResult {
        vertices: Vec::new(),
        faces: Vec::new(),
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: Vec::new(),
        skipped_source_faces: Vec::new(),
    };
    let first_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0, 1],
        used_cut_path_sides: true,
        cut_paths_consistent: false,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 1,
    };

    let result = assemble_classified_boolean_with_stitch(
        &first,
        &second,
        Some(&first_classification),
        None,
        None,
        ExactBooleanOperation::Union,
    );

    assert!(!result.result_cut_paths_complete);
    assert_eq!(result.result_cut_paths, vec![vec![[0, 1]], vec![[3, 4]]]);
    assert_eq!(result.result_cut_path_closed, vec![false, false]);
}

#[test]
fn meshlib_result_cut_mapping_uses_coincident_selected_vertex_fallback() {
    let first = ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        faces: vec![[0, 2, 3]],
        cut_edges: vec![[0, 1]],
        cut_edge_paths: vec![vec![[0, 1]]],
        cut_edge_path_closed: vec![false],
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let second = ExactCutMeshResult {
        vertices: vec![[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 1.0]],
        faces: vec![[0, 1, 2]],
        cut_edges: Vec::new(),
        cut_edge_paths: Vec::new(),
        cut_edge_path_closed: Vec::new(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let first_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0],
        used_cut_path_sides: true,
        cut_paths_consistent: true,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 0,
    };
    let second_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0],
        used_cut_path_sides: true,
        cut_paths_consistent: true,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 0,
    };

    let result = assemble_classified_boolean_with_stitch(
        &first,
        &second,
        Some(&first_classification),
        Some(&second_classification),
        None,
        ExactBooleanOperation::Union,
    );

    assert!(result.result_cut_paths_complete);
    assert_eq!(result.result_cut_paths, vec![vec![[0, 3]]]);
    assert_eq!(result.result_cut_path_closed, vec![false]);
}

#[test]
fn assembly_retains_complete_stitch_paths_from_incompatible_partial_plan() {
    let first = ExactCutMeshResult {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1], [1, 2]],
        cut_edge_paths: vec![vec![[0, 1]], vec![[1, 2]]],
        cut_edge_path_closed: vec![false, false],
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let second = ExactCutMeshResult {
        vertices: first.vertices.clone(),
        faces: first.faces.clone(),
        cut_edges: first.cut_edges.clone(),
        cut_edge_paths: first.cut_edge_paths.clone(),
        cut_edge_path_closed: first.cut_edge_path_closed.clone(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let first_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: Vec::new(),
        used_cut_path_sides: true,
        cut_paths_consistent: false,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 1,
    };
    let second_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0],
        used_cut_path_sides: true,
        cut_paths_consistent: false,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 1,
    };
    let stitch_plan = ExactStitchPlan {
        pairs: vec![ExactStitchEdgePair {
            first_edge_index: 0,
            second_edge_index: 0,
            first_edge: [0, 1],
            second_edge: [0, 1],
            second_reversed: false,
        }],
        paths: vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }],
        unmatched_first_edges: vec![1],
        unmatched_second_edges: vec![1],
        compatible: false,
    };

    let result = assemble_classified_boolean_with_stitch(
        &first,
        &second,
        Some(&first_classification),
        Some(&second_classification),
        Some(&stitch_plan),
        ExactBooleanOperation::Union,
    );

    assert_eq!(result.stitched_edge_sources.len(), 1);
    assert_eq!(
        result.stitched_edge_paths,
        vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }]
    );
}

#[test]
fn assembly_reuses_contour_vertices_from_incompatible_nonconflicting_stitch_pairs() {
    let first = ExactCutMeshResult {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1], [1, 2]],
        cut_edge_paths: vec![vec![[0, 1]], vec![[1, 2]]],
        cut_edge_path_closed: vec![false, false],
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let second = ExactCutMeshResult {
        vertices: vec![[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        faces: vec![[1, 0, 2]],
        cut_edges: vec![[1, 0], [0, 2]],
        cut_edge_paths: vec![vec![[1, 0]], vec![[0, 2]]],
        cut_edge_path_closed: vec![false, false],
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let first_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0],
        used_cut_path_sides: true,
        cut_paths_consistent: false,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 1,
    };
    let second_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0],
        used_cut_path_sides: true,
        cut_paths_consistent: false,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 1,
    };
    let stitch_plan = ExactStitchPlan {
        pairs: vec![ExactStitchEdgePair {
            first_edge_index: 0,
            second_edge_index: 0,
            first_edge: [0, 1],
            second_edge: [1, 0],
            second_reversed: false,
        }],
        paths: vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }],
        unmatched_first_edges: vec![1],
        unmatched_second_edges: vec![1],
        compatible: false,
    };

    let result = assemble_classified_boolean_with_stitch(
        &first,
        &second,
        Some(&first_classification),
        Some(&second_classification),
        Some(&stitch_plan),
        ExactBooleanOperation::Union,
    );

    assert_eq!(result.vertices.len(), 4);
    assert_eq!(result.faces, vec![[0, 1, 2], [0, 1, 3]]);
    assert_eq!(result.stitched_edge_sources.len(), 1);
    assert_eq!(
        result.stitched_edge_sources[0].first_output_edge,
        Some([0, 1])
    );
    assert_eq!(
        result.stitched_edge_sources[0].second_output_edge,
        Some([0, 1])
    );
}

#[test]
fn assembly_splits_stitch_paths_around_unmapped_partial_pairs() {
    let first = ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        ],
        faces: vec![[0, 1, 2]],
        cut_edges: vec![[0, 1], [3, 4], [1, 2]],
        cut_edge_paths: vec![vec![[0, 1], [3, 4], [1, 2]]],
        cut_edge_path_closed: vec![false],
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let second = ExactCutMeshResult {
        vertices: first.vertices.clone(),
        faces: first.faces.clone(),
        cut_edges: first.cut_edges.clone(),
        cut_edge_paths: first.cut_edge_paths.clone(),
        cut_edge_path_closed: first.cut_edge_path_closed.clone(),
        source_face_for_faces: vec![0],
        skipped_source_faces: Vec::new(),
    };
    let first_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: Vec::new(),
        used_cut_path_sides: true,
        cut_paths_consistent: false,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 1,
    };
    let second_classification = ExactMeshPartClassification {
        components: Vec::new(),
        selected_faces: vec![0],
        used_cut_path_sides: true,
        cut_paths_consistent: false,
        cut_left_components: 1,
        cut_right_components: 1,
        cut_path_overlap_components: 1,
    };
    let stitch_plan = ExactStitchPlan {
        pairs: vec![
            ExactStitchEdgePair {
                first_edge_index: 0,
                second_edge_index: 0,
                first_edge: [0, 1],
                second_edge: [0, 1],
                second_reversed: false,
            },
            ExactStitchEdgePair {
                first_edge_index: 1,
                second_edge_index: 1,
                first_edge: [3, 4],
                second_edge: [3, 4],
                second_reversed: false,
            },
            ExactStitchEdgePair {
                first_edge_index: 2,
                second_edge_index: 2,
                first_edge: [1, 2],
                second_edge: [1, 2],
                second_reversed: false,
            },
        ],
        paths: vec![ExactStitchPath {
            pair_indices: vec![0, 1, 2],
            closed: false,
        }],
        unmatched_first_edges: Vec::new(),
        unmatched_second_edges: Vec::new(),
        compatible: false,
    };

    let result = assemble_classified_boolean_with_stitch(
        &first,
        &second,
        Some(&first_classification),
        Some(&second_classification),
        Some(&stitch_plan),
        ExactBooleanOperation::Union,
    );

    assert_eq!(result.stitched_edge_sources.len(), 2);
    assert_eq!(
        result.stitched_edge_paths,
        vec![
            ExactStitchPath {
                pair_indices: vec![0],
                closed: false,
            },
            ExactStitchPath {
                pair_indices: vec![1],
                closed: false,
            },
        ]
    );
}
