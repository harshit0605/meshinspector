use super::*;
use crate::math::{add, dot, scale};

fn cube() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let vertices = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 3, 2],
        [0, 2, 1],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    (vertices, faces)
}

// MeshLib-generated DifferenceAB output for two unit cubes offset by +1 on X.
// This pins the parity target while Rust exact-difference stitching catches up.
fn meshlib_cube_overlap_difference() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let vertices = vec![
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, -1.0],
        [1.0, 1e-9, 1.0],
        [0.0, -1.0, 1.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 1.0, -1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 1, 2],
        [0, 2, 3],
        [4, 5, 3],
        [4, 3, 2],
        [5, 6, 0],
        [5, 0, 3],
        [7, 2, 1],
        [8, 1, 0],
        [8, 0, 6],
        [8, 6, 9],
        [10, 9, 6],
        [11, 10, 6],
        [12, 6, 5],
        [6, 12, 11],
        [12, 5, 4],
        [13, 4, 2],
        [8, 14, 2],
        [8, 2, 7],
        [1, 8, 7],
        [14, 8, 12],
        [12, 9, 11],
        [9, 12, 8],
        [11, 9, 10],
        [14, 4, 13],
        [4, 14, 12],
        [13, 2, 14],
    ];
    (vertices, faces)
}

const MESHLIB_CUBE_OVERLAP_UNION_VERTICES: usize = 18;
const MESHLIB_CUBE_OVERLAP_UNION_FACES: usize = 32;
const MESHLIB_CUBE_OVERLAP_UNION_SELF_INTERSECTIONS: usize = 13;
const MESHLIB_CUBE_OVERLAP_INTERSECTION_VERTICES: usize = 12;
const MESHLIB_CUBE_OVERLAP_INTERSECTION_FACES: usize = 20;
const MESHLIB_CUBE_OVERLAP_INTERSECTION_SELF_INTERSECTIONS: usize = 0;
const MESHLIB_CUBE_OVERLAP_DIFFERENCE_VERTICES: usize = 15;
const MESHLIB_CUBE_OVERLAP_DIFFERENCE_FACES: usize = 26;
const MESHLIB_CUBE_OVERLAP_DIFFERENCE_SELF_INTERSECTIONS: usize = 11;

#[test]
fn cube_stats_match_python_fixture() {
    let (vertices, faces) = cube();
    let stats = mesh_stats(&vertices, &faces).unwrap();

    assert_eq!(stats.vertex_count, 8);
    assert_eq!(stats.face_count, 12);
    assert_eq!(stats.connected_components, 1);
    assert_eq!(stats.boundary_edge_count, 0);
    assert_eq!(stats.bbox_size, [2.0, 2.0, 2.0]);
    assert!((stats.surface_area_mm2 - 24.0).abs() < 1e-9);
    assert!((stats.volume_mm3 - 8.0).abs() < 1e-9);
}

#[test]
fn meshlib_cube_overlap_difference_fixture_matches_reference_envelope() {
    let (vertices, faces) = meshlib_cube_overlap_difference();
    let stats = mesh_stats(&vertices, &faces).unwrap();
    let health = mesh_health(&vertices, &faces, true, None, 1e-9).unwrap();

    assert_eq!(stats.vertex_count, MESHLIB_CUBE_OVERLAP_DIFFERENCE_VERTICES);
    assert_eq!(stats.face_count, MESHLIB_CUBE_OVERLAP_DIFFERENCE_FACES);
    assert_eq!(stats.connected_components, 1);
    assert_eq!(stats.boundary_edge_count, 0);
    assert_eq!(stats.bbox_min, [-1.0, -1.0, -1.0]);
    assert_eq!(stats.bbox_max, [1.0, 1.0, 1.0]);
    assert!((stats.surface_area_mm2 - 24.0).abs() < 1e-9);
    assert!((stats.volume_mm3 - 4.0).abs() < 1e-9);
    assert!(health.is_closed);
    assert_eq!(health.holes_count, 0);
    assert_eq!(health.boundary_edge_count, 0);
    assert_eq!(health.nonmanifold_edge_count, 0);
    assert_eq!(
        health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_DIFFERENCE_SELF_INTERSECTIONS)
    );
}

#[test]
fn exact_boolean_cube_overlap_promotes_paired_coplanar_candidate_to_meshlib_envelope() {
    let (source_vertices, source_faces) = cube();
    let target_vertices = source_vertices
        .iter()
        .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
        .collect::<Vec<_>>();

    let result = exact_boolean_from_meshes(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &source_faces,
        ExactBooleanOperation::Union,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.diagnostics.parity_ready);
    assert!(result.diagnostics.stitch_compatible);
    assert_eq!(result.diagnostics.stitch_unmatched_first_edges, 0);
    assert_eq!(result.diagnostics.stitch_unmatched_second_edges, 0);
    assert_eq!(result.diagnostics.stitch_cut_path_length_mismatches, 0);
    assert!(result.diagnostics.first_prepare_part_dividable);
    assert!(result.diagnostics.second_prepare_part_dividable);
    assert_eq!(result.diagnostics.first_cut_path_overlap_components, 0);
    assert_eq!(result.diagnostics.second_cut_path_overlap_components, 0);
    assert!(result.diagnostics.first_skipped_source_faces.is_empty());
    assert!(result.diagnostics.second_skipped_source_faces.is_empty());
    assert!(result.diagnostics.requires_topology_splice);
    assert!(result.diagnostics.coplanar_overlap_pairs > 0);
    assert!(result.diagnostics.coplanar_overlap_region_edges > 0);
    assert!(result.diagnostics.coplanar_overlap_area > 0.0);
    assert_eq!(
        result.diagnostics.coplanar_overlap_contours,
        result.diagnostics.coplanar_overlap_pairs
    );
    assert_eq!(
        result.diagnostics.coplanar_overlap_contour_edges,
        result.diagnostics.coplanar_overlap_region_edges
    );
    assert!(result.diagnostics.coplanar_cut_trial_contours > 0);
    assert!(result.diagnostics.coplanar_cut_trial_contour_edges > 0);
    assert!(result.diagnostics.coplanar_cut_trial_first_cut_edges > 0);
    assert!(result.diagnostics.coplanar_cut_trial_second_cut_edges > 0);
    assert_eq!(result.diagnostics.paired_coplanar_cut_trial_contours, 2);
    assert_eq!(
        result.diagnostics.paired_coplanar_cut_trial_contour_edges,
        16
    );
    assert_eq!(
        result.diagnostics.paired_coplanar_cut_trial_first_cut_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_cut_trial_second_cut_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_stitch_cut_path_length_mismatches,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_stitch_unmatched_first_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_stitch_unmatched_second_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_duplicate_first_path_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_duplicate_second_path_edges,
        0
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_stitch_compatible
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_prepare_part_dividable
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_prepare_part_dividable
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_cut_path_side_components,
        [1, 1]
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_cut_path_side_components,
        [1, 1]
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_cut_path_overlap_components,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_cut_path_overlap_components,
        0
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_result_cut_paths_complete
    );
    assert!(result.diagnostics.paired_coplanar_candidate_output_faces > 0);
    assert!(result.diagnostics.paired_coplanar_candidate_output_volume > 0.0);
    assert_eq!(
        result.diagnostics.paired_coplanar_candidate_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_nonmanifold_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_duplicate_output_faces,
        0
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_preserves_active_volume
    );
    assert!((result.diagnostics.paired_coplanar_candidate_output_volume - 12.0).abs() < 1e-6);
    assert!(
        (result
            .diagnostics
            .paired_coplanar_candidate_active_volume_delta
            .abs())
            < 1e-6
    );
    assert!(result.diagnostics.coplanar_cut_trial_accepted);
    assert!(result
        .diagnostics
        .coplanar_cut_trial_first_skipped_faces
        .is_empty());
    assert!(result
        .diagnostics
        .coplanar_cut_trial_second_skipped_faces
        .is_empty());
    assert_eq!(result.diagnostics.stitched_output_edges, 16);
    assert_eq!(result.diagnostics.stitched_output_edges_with_two_faces, 16);
    assert_eq!(result.diagnostics.stitched_output_edges_needing_splice, 0);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_mapped_contour_edges, 8);
    assert_eq!(
        [
            result.diagnostics.meshlib_topology_base_faces,
            result.diagnostics.meshlib_topology_incoming_faces,
            result.assembly.prepare_first_faces.len(),
            result.assembly.prepare_second_faces.len(),
            result.diagnostics.meshlib_topology_selected_first_faces,
            result.diagnostics.meshlib_topology_selected_second_faces,
            result.diagnostics.meshlib_topology_first_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_second_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_duplicate_first_source_faces,
            result
                .diagnostics
                .meshlib_topology_duplicate_second_source_faces,
        ],
        [20, 20, 20, 20, 30, 14, 8, 4, 20, 4]
    );
    assert_eq!(
        (
            result.diagnostics.meshlib_topology_raw_selected_faces,
            result
                .diagnostics
                .meshlib_topology_same_oriented_overlap_faces,
            result.diagnostics.meshlib_topology_boundary_misses,
            result
                .diagnostics
                .meshlib_topology_coplanar_selection_delta_faces,
        ),
        ([20, 20], [13, 12], [[0, 0], [9, 9]], [10, -6])
    );
    assert_eq!(result.diagnostics.meshlib_topology_missing_base_edges, 0);
    assert_eq!(
        result.diagnostics.meshlib_topology_missing_incoming_edges,
        8
    );
    assert_eq!(result.diagnostics.meshlib_topology_direction_mismatches, 0);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_mapped_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_missing_stitch_contour_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_synthetic_stitch_contour_edges,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_direction_mismatches,
        0
    );
    assert!(!result.diagnostics.meshlib_topology_stitch_metadata_ready);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_unmaterialized_stitch_contour_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_synthetic_stitch_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_direction_mismatches,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_ready
    );
    assert_eq!(
        result.diagnostics.meshlib_topology_record_rewrite_commands,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_blocked_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_synthetic_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_direction_mismatches,
        0
    );
    assert!(result.diagnostics.meshlib_topology_record_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_open_stitch_paths, 5);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_updates,
        10
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_blocked_updates,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_ready
    );
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_near_stitch_update_commands,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_applied,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_start,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_end,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_previous_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_next_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_origin_mismatches,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_previous_left_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_next_right_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_other,
        ],
        [10, 8, 2, 1, 1, 0, 0, 0, 1, 1, 0]
    );
    let near_stitch_failed_details = &result
        .diagnostics
        .meshlib_topology_near_stitch_failed_details;
    assert_eq!(near_stitch_failed_details.len(), 2);
    assert_eq!(
        near_stitch_failed_details.len(),
        result
            .diagnostics
            .meshlib_topology_near_stitch_updates_failed
    );
    assert!(near_stitch_failed_details
        .iter()
        .all(|detail| detail.endpoint.is_some()
            && detail.candidate_diagnostics.is_some()
            && !detail.error.is_empty()));
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_record_rewrite_applied_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_closed_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_sources,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_other_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_prepared_synthetic_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_translated_face_records,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_synthetic_sides,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_exported_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_failed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_non_triangular_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_left_ring_not_closed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_missing_origin_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_other_failed_faces,
        ],
        [16, 0, 0, 0, 0, 0, 8, 8, 8, 44, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        (
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_changed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_ready,
        ),
        (true, false)
    );
    let prepared_base_rewrite = result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite;
    assert_eq!(
        [
            prepared_base_rewrite.prepared_faces,
            prepared_base_rewrite.prepared_vertices,
            prepared_base_rewrite.virtual_vertices,
            prepared_base_rewrite.prepared_face_sources,
            prepared_base_rewrite.applied_commands,
            prepared_base_rewrite.failed_commands,
            prepared_base_rewrite.near_stitch_updates_applied,
            prepared_base_rewrite.near_stitch_updates_failed,
            prepared_base_rewrite.exported_faces,
            prepared_base_rewrite.export_failed_faces,
        ],
        [20, 24, 0, 20, 16, 0, 0, 0, 40, 0]
    );
    assert!(prepared_base_rewrite.ready_for_export);
    assert_eq!(
        (
            prepared_base_rewrite.record_rewrite_near_stitch_target_left_closures,
            prepared_base_rewrite.record_rewrite_near_stitch_target_right_closures,
        ),
        (8, 0)
    );
    assert_eq!(
        [
            prepared_base_rewrite.record_failed_missing_targets,
            prepared_base_rewrite.record_failed_closed_targets,
            prepared_base_rewrite.record_failed_missing_sources,
            prepared_base_rewrite.record_failed_other_commands,
            prepared_base_rewrite.translated_copied_edge_records,
            prepared_base_rewrite.translated_copied_face_records,
            prepared_base_rewrite.failed_copied_edge_records,
            prepared_base_rewrite.refreshed_face_records,
            prepared_base_rewrite.near_stitch_failed_start,
            prepared_base_rewrite.near_stitch_failed_end,
            prepared_base_rewrite.near_stitch_missing_previous_edges,
            prepared_base_rewrite.near_stitch_missing_next_edges,
            prepared_base_rewrite.near_stitch_origin_mismatches,
            prepared_base_rewrite.near_stitch_previous_left_faces,
            prepared_base_rewrite.near_stitch_next_right_faces,
            prepared_base_rewrite.near_stitch_failed_other,
            prepared_base_rewrite.export_non_triangular_faces,
            prepared_base_rewrite.export_left_ring_not_closed_faces,
            prepared_base_rewrite.export_missing_origin_faces,
            prepared_base_rewrite.export_face_record_left_mismatch_faces,
            prepared_base_rewrite.export_face_left_ring_mismatch_faces,
            prepared_base_rewrite.export_other_failed_faces,
        ],
        [0, 0, 0, 0, 44, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        prepared_base_rewrite.near_stitch_failed_details.len(),
        prepared_base_rewrite.near_stitch_updates_failed
    );
    assert!(prepared_base_rewrite.near_stitch_failed_details.is_empty());
    let rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_stats
        .as_ref()
        .expect("rewrite export stats");
    assert_eq!(rewrite_stats.vertex_count, 24);
    assert_eq!(rewrite_stats.face_count, 44);
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_health
        .is_some());
    let packed_rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_stats
        .as_ref()
        .expect("packed rewrite export stats");
    assert_eq!(packed_rewrite_stats.vertex_count, 24);
    assert_eq!(packed_rewrite_stats.face_count, 44);
    assert_ne!(
        packed_rewrite_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_UNION_VERTICES
    );
    assert_ne!(
        packed_rewrite_stats.face_count,
        MESHLIB_CUBE_OVERLAP_UNION_FACES
    );
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_health
        .is_some());
    assert!(result.diagnostics.topology_splice_ready);
    assert_eq!(result.diagnostics.topology_splice_non_manifold_edges, 0);
    assert!(result.diagnostics.topology_splice_apply_ready);
    assert_eq!(
        result.diagnostics.topology_splice_verified_boundary_edges,
        0
    );
    assert_eq!(result.diagnostics.topology_splice_blocked_edges, 0);
    assert_eq!(result.diagnostics.topology_splice_failed_edges, 0);
    assert!(!result.assembly.stitched_edge_paths.is_empty());
    assert!(result.topology_splice_apply_plan.stitched_paths > 0);
    assert_eq!(result.topology_splice_apply_plan.verified_boundary_paths, 5);
    assert_eq!(result.topology_splice_apply_plan.blocked_paths, 0);
    assert_eq!(result.topology_splice_apply_plan.failed_paths, 0);
    assert_eq!(result.diagnostics.topology_splice_synthetic_side_edges, 0);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_materialized_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_materialization_failed_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_duplicate_output_face_groups,
        0
    );
    assert_eq!(result.diagnostics.topology_splice_duplicate_output_faces, 0);
    assert!(result.diagnostics.first_cut_edges >= 3);
    assert!(result.diagnostics.second_cut_edges >= 3);
    assert!(result.diagnostics.result_cut_paths_complete);
    assert_eq!(
        result.diagnostics.result_cut_mapped_paths,
        result.diagnostics.result_cut_paths
    );
    assert_eq!(
        result.diagnostics.result_cut_mapped_path_edges,
        result.diagnostics.result_cut_path_edges
    );
    assert_eq!(
        result.diagnostics.result_cut_mapped_closed_paths,
        result.diagnostics.result_cut_closed_paths
    );
    assert!(result.diagnostics.output_mesh_health.is_closed);
    assert_eq!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(0)
    );
    assert_ne!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_UNION_SELF_INTERSECTIONS),
        "the promoted union candidate is envelope-ready but not MeshLib topology-parity-ready"
    );
    assert!(
        result
            .diagnostics
            .output_mesh_health
            .self_intersections_available
    );
    assert_eq!(result.diagnostics.output_mesh_stats.vertex_count, 24);
    assert_eq!(result.diagnostics.output_mesh_stats.face_count, 44);
    assert_ne!(
        result.diagnostics.output_mesh_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_UNION_VERTICES
    );
    assert_ne!(
        result.diagnostics.output_mesh_stats.face_count,
        MESHLIB_CUBE_OVERLAP_UNION_FACES
    );
    assert_eq!(result.diagnostics.output_mesh_stats.connected_components, 1);
    assert_eq!(result.diagnostics.output_mesh_health.boundary_edge_count, 0);
    assert_eq!(
        result.diagnostics.output_mesh_health.nonmanifold_edge_count,
        0
    );
    assert_eq!(result.topology_splice_apply_plan.exported_boundary_edges, 0);
    assert!(!result.diagnostics.topology_splice_export_changed_faces);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.topology_splice_exported_faces, 44);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_before_materialization,
        66
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_after_materialization,
        66
    );
    assert_eq!(
        result.diagnostics.topology_splice_deleted_synthetic_edges,
        0
    );
    assert!(
        (result.diagnostics.output_mesh_stats.volume_mm3 - 12.0).abs() < 1e-3,
        "the stored MeshLib cube-overlap union volume is 12 mm3"
    );
    assert!(
        (result.diagnostics.output_mesh_stats.surface_area_mm2 - 32.0).abs() < 1e-6,
        "the MeshLib-style cube-overlap union envelope is a 3x2x2 box"
    );
}

#[test]
fn exact_boolean_cube_overlap_intersection_promotes_paired_coplanar_candidate() {
    let (source_vertices, source_faces) = cube();
    let target_vertices = source_vertices
        .iter()
        .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
        .collect::<Vec<_>>();

    let result = exact_boolean_from_meshes(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &source_faces,
        ExactBooleanOperation::Intersection,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.diagnostics.parity_ready);
    assert!(result.diagnostics.stitch_compatible);
    assert_eq!(result.diagnostics.stitch_unmatched_first_edges, 0);
    assert_eq!(result.diagnostics.stitch_unmatched_second_edges, 0);
    assert!(result.diagnostics.first_prepare_part_dividable);
    assert!(result.diagnostics.second_prepare_part_dividable);
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_preserves_active_volume
    );
    assert_eq!(
        result.diagnostics.paired_coplanar_candidate_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_nonmanifold_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_duplicate_output_faces,
        0
    );
    assert!(result.diagnostics.output_mesh_health.is_closed);
    assert_eq!(result.diagnostics.output_mesh_health.boundary_edge_count, 0);
    assert_eq!(
        result.diagnostics.output_mesh_health.nonmanifold_edge_count,
        0
    );
    assert_eq!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(3)
    );
    assert_ne!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_INTERSECTION_SELF_INTERSECTIONS),
        "the promoted intersection candidate is envelope-ready but not MeshLib topology-parity-ready"
    );
    assert!(
        result
            .diagnostics
            .output_mesh_health
            .self_intersections_available
    );
    assert_eq!(result.diagnostics.output_mesh_stats.connected_components, 1);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_mapped_contour_edges, 8);
    assert_eq!(result.diagnostics.meshlib_topology_missing_base_edges, 8);
    assert_eq!(
        result.diagnostics.meshlib_topology_missing_incoming_edges,
        0
    );
    assert_eq!(result.diagnostics.meshlib_topology_direction_mismatches, 0);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_mapped_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_missing_stitch_contour_edges,
        0
    );
    assert_eq!(
        [
            result.diagnostics.meshlib_topology_base_faces,
            result.diagnostics.meshlib_topology_incoming_faces,
            result.assembly.prepare_first_faces.len(),
            result.assembly.prepare_second_faces.len(),
            result.diagnostics.meshlib_topology_selected_first_faces,
            result.diagnostics.meshlib_topology_selected_second_faces,
            result.diagnostics.meshlib_topology_first_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_second_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_duplicate_first_source_faces,
            result
                .diagnostics
                .meshlib_topology_duplicate_second_source_faces,
        ],
        [16, 16, 16, 16, 22, 6, 6, 2, 12, 4]
    );
    assert_eq!(
        (
            result.diagnostics.meshlib_topology_raw_selected_faces,
            result
                .diagnostics
                .meshlib_topology_same_oriented_overlap_faces,
            result.diagnostics.meshlib_topology_boundary_misses,
            result
                .diagnostics
                .meshlib_topology_coplanar_selection_delta_faces,
        ),
        ([16, 16], [13, 12], [[0, 0], [9, 9]], [6, -10])
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_synthetic_stitch_contour_edges,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_direction_mismatches,
        0
    );
    assert!(!result.diagnostics.meshlib_topology_stitch_metadata_ready);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_unmaterialized_stitch_contour_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_synthetic_stitch_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_direction_mismatches,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_ready
    );
    assert_eq!(
        result.diagnostics.meshlib_topology_record_rewrite_commands,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_blocked_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_synthetic_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_direction_mismatches,
        0
    );
    assert!(result.diagnostics.meshlib_topology_record_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_open_stitch_paths, 5);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_updates,
        10
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_blocked_updates,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_ready
    );
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_near_stitch_update_commands,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_applied,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_start,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_end,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_previous_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_next_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_origin_mismatches,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_previous_left_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_next_right_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_other,
        ],
        [10, 1, 9, 5, 4, 0, 0, 0, 9, 0, 0]
    );
    let near_stitch_failed_details = &result
        .diagnostics
        .meshlib_topology_near_stitch_failed_details;
    assert_eq!(near_stitch_failed_details.len(), 9);
    assert_eq!(
        near_stitch_failed_details.len(),
        result
            .diagnostics
            .meshlib_topology_near_stitch_updates_failed
    );
    assert!(near_stitch_failed_details
        .iter()
        .all(|detail| detail.endpoint.is_some()
            && detail.candidate_diagnostics.is_some()
            && !detail.error.is_empty()));
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_record_rewrite_applied_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_closed_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_sources,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_other_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_prepared_synthetic_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_translated_face_records,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_synthetic_sides,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_exported_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_failed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_non_triangular_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_left_ring_not_closed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_missing_origin_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_other_failed_faces,
        ],
        [16, 0, 0, 0, 0, 0, 0, 13, 8, 28, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        (
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_changed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_ready,
        ),
        (true, false)
    );
    let prepared_base_rewrite = result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite;
    assert_eq!(
        [
            prepared_base_rewrite.prepared_faces,
            prepared_base_rewrite.prepared_vertices,
            prepared_base_rewrite.virtual_vertices,
            prepared_base_rewrite.prepared_face_sources,
            prepared_base_rewrite.applied_commands,
            prepared_base_rewrite.failed_commands,
            prepared_base_rewrite.near_stitch_updates_applied,
            prepared_base_rewrite.near_stitch_updates_failed,
            prepared_base_rewrite.exported_faces,
            prepared_base_rewrite.export_failed_faces,
        ],
        [16, 24, 8, 16, 16, 0, 0, 0, 32, 0]
    );
    assert!(prepared_base_rewrite.ready_for_export);
    assert_eq!(
        (
            prepared_base_rewrite.record_rewrite_near_stitch_target_left_closures,
            prepared_base_rewrite.record_rewrite_near_stitch_target_right_closures,
        ),
        (16, 0)
    );
    assert_eq!(
        [
            prepared_base_rewrite.record_failed_missing_targets,
            prepared_base_rewrite.record_failed_closed_targets,
            prepared_base_rewrite.record_failed_missing_sources,
            prepared_base_rewrite.record_failed_other_commands,
            prepared_base_rewrite.translated_copied_edge_records,
            prepared_base_rewrite.translated_copied_face_records,
            prepared_base_rewrite.failed_copied_edge_records,
            prepared_base_rewrite.refreshed_face_records,
            prepared_base_rewrite.near_stitch_failed_start,
            prepared_base_rewrite.near_stitch_failed_end,
            prepared_base_rewrite.near_stitch_missing_previous_edges,
            prepared_base_rewrite.near_stitch_missing_next_edges,
            prepared_base_rewrite.near_stitch_origin_mismatches,
            prepared_base_rewrite.near_stitch_previous_left_faces,
            prepared_base_rewrite.near_stitch_next_right_faces,
            prepared_base_rewrite.near_stitch_failed_other,
            prepared_base_rewrite.export_non_triangular_faces,
            prepared_base_rewrite.export_left_ring_not_closed_faces,
            prepared_base_rewrite.export_missing_origin_faces,
            prepared_base_rewrite.export_face_record_left_mismatch_faces,
            prepared_base_rewrite.export_face_left_ring_mismatch_faces,
            prepared_base_rewrite.export_other_failed_faces,
        ],
        [0, 0, 0, 0, 32, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        prepared_base_rewrite.near_stitch_failed_details.len(),
        prepared_base_rewrite.near_stitch_updates_failed
    );
    assert!(prepared_base_rewrite.near_stitch_failed_details.is_empty());
    let rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_stats
        .as_ref()
        .expect("rewrite export stats");
    assert_eq!(rewrite_stats.vertex_count, 16);
    assert_eq!(rewrite_stats.face_count, 28);
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_health
        .is_some());
    let packed_rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_stats
        .as_ref()
        .expect("packed rewrite export stats");
    assert_eq!(packed_rewrite_stats.vertex_count, 16);
    assert_eq!(packed_rewrite_stats.face_count, 28);
    assert_ne!(
        packed_rewrite_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_VERTICES
    );
    assert_ne!(
        packed_rewrite_stats.face_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_FACES
    );
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_health
        .is_some());
    assert_eq!(result.diagnostics.output_mesh_stats.vertex_count, 16);
    assert_eq!(result.diagnostics.output_mesh_stats.face_count, 28);
    assert_ne!(
        result.diagnostics.output_mesh_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_VERTICES
    );
    assert_ne!(
        result.diagnostics.output_mesh_stats.face_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_FACES
    );
    assert!(!result.diagnostics.topology_splice_export_changed_faces);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.topology_splice_exported_faces, 28);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_before_materialization,
        42
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_after_materialization,
        42
    );
    assert_eq!(
        result.diagnostics.topology_splice_deleted_synthetic_edges,
        0
    );
    assert!(
        (result.diagnostics.output_mesh_stats.volume_mm3 - 4.0).abs() < 1e-6,
        "the stored MeshLib cube-overlap intersection volume is 4 mm3"
    );
    assert!(
        (result.diagnostics.output_mesh_stats.surface_area_mm2 - 16.0).abs() < 1e-6,
        "the MeshLib-style cube-overlap intersection envelope is a 1x2x2 box"
    );
}

#[test]
fn exact_boolean_cube_overlap_difference_tracks_meshlib_envelope_gap() {
    let (source_vertices, source_faces) = cube();
    let target_vertices = source_vertices
        .iter()
        .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
        .collect::<Vec<_>>();
    let (reference_vertices, reference_faces) = meshlib_cube_overlap_difference();
    let reference_stats = mesh_stats(&reference_vertices, &reference_faces).unwrap();
    let reference_health =
        mesh_health(&reference_vertices, &reference_faces, true, None, 1e-9).unwrap();

    let result = exact_boolean_from_meshes(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &source_faces,
        ExactBooleanOperation::DifferenceAB,
        8,
        1e-9,
    )
    .unwrap();

    assert!(!result.diagnostics.parity_ready);
    assert!(!result.diagnostics.stitch_compatible);
    assert_eq!(result.diagnostics.stitch_unmatched_first_edges, 4);
    assert_eq!(result.diagnostics.stitch_unmatched_second_edges, 4);
    assert_eq!(result.diagnostics.stitch_cut_path_length_mismatches, 8);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert!(!result.diagnostics.first_prepare_part_dividable);
    assert!(!result.diagnostics.second_prepare_part_dividable);
    assert!(result.diagnostics.result_cut_paths_complete);
    assert_eq!(
        [
            result.diagnostics.meshlib_topology_base_faces,
            result.diagnostics.meshlib_topology_incoming_faces,
            result.assembly.prepare_first_faces.len(),
            result.assembly.prepare_second_faces.len(),
            result.assembly.selected_first_faces.len(),
            result.assembly.selected_second_faces.len(),
        ],
        [22, 14, 22, 14, 22, 14]
    );
    let prepared_base_rewrite = result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite;
    assert_eq!(
        [
            prepared_base_rewrite.prepared_faces,
            prepared_base_rewrite.prepared_vertices,
            prepared_base_rewrite.virtual_vertices,
            prepared_base_rewrite.prepared_face_sources,
            prepared_base_rewrite.applied_commands,
            prepared_base_rewrite.failed_commands,
            prepared_base_rewrite.near_stitch_updates_applied,
            prepared_base_rewrite.near_stitch_updates_failed,
            prepared_base_rewrite.exported_faces,
            prepared_base_rewrite.export_failed_faces,
        ],
        [22, 20, 0, 22, 20, 0, 0, 0, 32, 4]
    );
    assert!(!prepared_base_rewrite.ready_for_export);
    assert_eq!(
        (
            prepared_base_rewrite.record_rewrite_near_stitch_target_left_closures,
            prepared_base_rewrite.record_rewrite_near_stitch_target_right_closures,
        ),
        (15, 0)
    );
    assert_eq!(
        [
            prepared_base_rewrite.record_failed_missing_targets,
            prepared_base_rewrite.record_failed_closed_targets,
            prepared_base_rewrite.record_failed_missing_sources,
            prepared_base_rewrite.record_failed_other_commands,
            prepared_base_rewrite.translated_copied_edge_records,
            prepared_base_rewrite.translated_copied_face_records,
            prepared_base_rewrite.failed_copied_edge_records,
            prepared_base_rewrite.refreshed_face_records,
            prepared_base_rewrite.near_stitch_failed_start,
            prepared_base_rewrite.near_stitch_failed_end,
            prepared_base_rewrite.near_stitch_missing_previous_edges,
            prepared_base_rewrite.near_stitch_missing_next_edges,
            prepared_base_rewrite.near_stitch_origin_mismatches,
            prepared_base_rewrite.near_stitch_previous_left_faces,
            prepared_base_rewrite.near_stitch_next_right_faces,
            prepared_base_rewrite.near_stitch_failed_other,
            prepared_base_rewrite.export_non_triangular_faces,
            prepared_base_rewrite.export_left_ring_not_closed_faces,
            prepared_base_rewrite.export_missing_origin_faces,
            prepared_base_rewrite.export_face_record_left_mismatch_faces,
            prepared_base_rewrite.export_face_left_ring_mismatch_faces,
            prepared_base_rewrite.export_other_failed_faces,
        ],
        [0, 0, 0, 0, 22, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0]
    );
    assert_eq!(
        (
            prepared_base_rewrite.near_stitch_skipped_previous_left_source_edges,
            prepared_base_rewrite.near_stitch_skipped_next_right_source_edges,
        ),
        (1, 0)
    );
    assert_eq!(
        (
            prepared_base_rewrite.near_stitch_previous_left_copied_source_edges,
            prepared_base_rewrite.near_stitch_next_right_copied_source_edges,
        ),
        (0, 0)
    );
    assert_eq!(
        (
            result.diagnostics.meshlib_topology_raw_selected_faces,
            result
                .diagnostics
                .meshlib_topology_same_oriented_overlap_faces,
            result.diagnostics.meshlib_topology_boundary_misses,
            result
                .diagnostics
                .meshlib_topology_coplanar_selection_delta_faces,
        ),
        ([22, 14], [13, 12], [[20, 22], [20, 22]], [0, 0])
    );
    assert_eq!(result.diagnostics.output_mesh_stats.connected_components, 1);
    assert_eq!(result.diagnostics.output_mesh_stats.vertex_count, 20);
    assert_eq!(result.diagnostics.output_mesh_stats.face_count, 36);
    assert_ne!(
        result.diagnostics.output_mesh_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_DIFFERENCE_VERTICES
    );
    assert_ne!(
        result.diagnostics.output_mesh_stats.face_count,
        MESHLIB_CUBE_OVERLAP_DIFFERENCE_FACES
    );
    assert_eq!(result.diagnostics.output_mesh_health.boundary_edge_count, 8);
    assert_eq!(
        result.diagnostics.output_mesh_health.nonmanifold_edge_count,
        8
    );
    assert!(!result.diagnostics.output_mesh_health.is_closed);
    assert!(
        result
            .diagnostics
            .output_mesh_health
            .self_intersections_available
    );
    assert!(result
        .diagnostics
        .output_mesh_health
        .self_intersections
        .is_some());
    assert!(reference_health.is_closed);
    assert_eq!(reference_health.boundary_edge_count, 0);
    assert_eq!(reference_health.nonmanifold_edge_count, 0);
    assert_eq!(
        reference_health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_DIFFERENCE_SELF_INTERSECTIONS)
    );
    assert!(
        (result.diagnostics.output_mesh_stats.volume_mm3 - reference_stats.volume_mm3).abs() < 1e-6,
        "the active fallback preserves the stored MeshLib cube-overlap difference volume"
    );
    assert!(
        (result.diagnostics.output_mesh_stats.surface_area_mm2 - reference_stats.surface_area_mm2)
            .abs()
            < 1e-6,
        "MeshLib keeps the 2x2x2 source envelope for this coplanar difference"
    );
    for axis in 0..3 {
        assert!(
            (result.diagnostics.output_mesh_stats.bbox_min[axis] - reference_stats.bbox_min[axis])
                .abs()
                < 1e-6
        );
        assert!(
            (result.diagnostics.output_mesh_stats.bbox_max[axis] - reference_stats.bbox_max[axis])
                .abs()
                < 1e-6
        );
    }
    assert_ne!(
        result.assembly.faces.len(),
        reference_faces.len(),
        "face-count parity should only land after MeshLib-style difference stitching is implemented"
    );
    assert!(!result.diagnostics.topology_splice_export_changed_faces);
    assert_eq!(result.diagnostics.topology_splice_exported_faces, 36);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_before_materialization,
        64
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_after_materialization,
        64
    );
    assert_eq!(
        result.diagnostics.topology_splice_deleted_synthetic_edges,
        8
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_stitch_compatible
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_prepare_part_dividable
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_prepare_part_dividable
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_preserves_active_volume
    );
    assert_eq!(
        result.diagnostics.paired_coplanar_candidate_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_nonmanifold_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_duplicate_output_faces,
        0
    );
    assert!(
        !result
            .diagnostics
            .paired_coplanar_candidate_result_cut_paths_complete
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_self_intersections_available
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_self_intersections,
        Some(2)
    );
    assert_ne!(
        result
            .diagnostics
            .paired_coplanar_candidate_self_intersections,
        reference_health.self_intersections
    );
    assert!((result.diagnostics.paired_coplanar_candidate_output_volume - 4.0).abs() < 1e-6);
    assert!(
        (result.diagnostics.paired_coplanar_candidate_output_area - 16.0).abs() < 1e-6,
        "the closed paired candidate is the mathematical slab, not MeshLib's coplanar envelope"
    );
}

#[test]
fn core_mesh_helpers_match_python_contract() {
    let (vertices, faces) = cube();

    let (bbox_min, bbox_max) = mesh_bounds(&vertices);
    assert_eq!(bbox_min, [-1.0, -1.0, -1.0]);
    assert_eq!(bbox_max, [1.0, 1.0, 1.0]);
    assert_eq!(
        safe_normalize_vectors(&[[3.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
        vec![[1.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
    );
    assert_eq!(
        normalize_axis_vector([0.0, 2.0, 0.0]).unwrap(),
        [0.0, 1.0, 0.0]
    );
    assert!(normalize_axis_vector([0.0, 0.0, 0.0]).is_err());

    assert_eq!(
        face_normals_for_mesh(&vertices, &faces).unwrap().len(),
        faces.len()
    );
    assert_eq!(
        vertex_normals_for_mesh(&vertices, &faces).unwrap().len(),
        vertices.len()
    );
    assert!((mesh_surface_area(&vertices, &faces).unwrap() - 24.0).abs() < 1e-9);
    assert!((mesh_signed_volume(&vertices, &faces).unwrap() - 8.0).abs() < 1e-9);
    assert!((mesh_volume(&vertices, &faces).unwrap() - 8.0).abs() < 1e-9);
    assert_eq!(boundary_edges_for_mesh(&vertices, &faces).unwrap().len(), 0);
    assert_eq!(
        face_adjacency_for_mesh(&vertices, &faces).unwrap().len(),
        faces.len()
    );
    let mut components = connected_face_components_for_mesh(&vertices, &faces).unwrap();
    assert_eq!(components.len(), 1);
    components[0].sort_unstable();
    assert_eq!(components[0], (0..faces.len() as i64).collect::<Vec<_>>());
    assert_eq!(
        vertex_neighbors_for_mesh(&vertices, &faces).unwrap().len(),
        vertices.len()
    );
}

#[test]
fn open_cube_boundary_loops_match_python_fixture() {
    let (vertices, mut faces) = cube();
    faces.truncate(10);

    let loops = boundary_loops(&vertices, &faces).unwrap();
    let health = mesh_health(&vertices, &faces, true, Some(50_000), 1e-8).unwrap();

    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].len(), 4);
    assert!(!health.is_closed);
    assert_eq!(health.holes_count, 1);
    assert_eq!(health.boundary_edge_count, 4);
    assert_eq!(health.nonmanifold_edge_count, 0);
    assert_eq!(health.self_intersections, Some(0));
    assert!(health.self_intersections_available);
}

#[test]
fn service_fill_holes_uses_triangulated_patch_like_meshlib_fillhole() {
    let (vertices, mut faces) = cube();
    faces.truncate(10);

    let repaired = service_fill_holes(&vertices, &faces, None).unwrap();
    let health = mesh_health(
        &repaired.vertices,
        &repaired.faces,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();

    assert_eq!(repaired.vertices.len(), vertices.len());
    assert_eq!(repaired.report.input_holes, 1);
    assert_eq!(repaired.report.filled_holes, 1);
    assert_eq!(repaired.report.added_vertices, 0);
    assert_eq!(repaired.report.added_faces, 2);
    assert!(health.is_closed);
}

#[test]
fn health_can_skip_self_intersection_budget() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let health = mesh_health(&vertices, &faces, true, Some(1), 1e-8).unwrap();

    assert_eq!(health.self_intersections, None);
    assert!(!health.self_intersections_available);
}

#[test]
fn service_mesh_health_matches_current_meshlib_payload_contract() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let health = service_mesh_health(&vertices, &faces, 1, 1e-8).unwrap();

    assert!(!health.is_closed);
    assert_eq!(health.self_intersections, 2);
    assert_eq!(health.self_intersection_faces, vec![0]);
    assert_eq!(health.holes_count, 2);
    assert_eq!(health.degenerate_faces, 0);
    assert_eq!(health.health_score, 56);
}

#[test]
fn summarize_thickness_matches_python_behavior() {
    let values = vec![2.0_f32, f32::NAN, -1.0, 0.25, f32::INFINITY, 0.75];

    let summary = summarize_thickness(&values, 0.6);

    assert_eq!(summary.min_mm, Some(0.25));
    assert!((summary.avg_mm.unwrap() - 1.0).abs() < 1e-9);
    assert_eq!(summary.max_mm, Some(2.0));
    assert_eq!(summary.valid_vertex_count, 3);
    assert_eq!(summary.violation_count, 1);
}

#[test]
fn summarize_thickness_handles_no_valid_values() {
    let values = vec![f32::NAN, 0.0, -1.0];

    let summary = summarize_thickness(&values, 0.6);

    assert_eq!(summary.min_mm, None);
    assert_eq!(summary.avg_mm, None);
    assert_eq!(summary.max_mm, None);
    assert_eq!(summary.valid_vertex_count, 0);
    assert_eq!(summary.violation_count, 0);
}

#[test]
fn material_weight_conversions_match_python_contract() {
    assert_eq!(material_density_g_cm3("gold_18k"), 15.58);
    assert_eq!(material_density_g_cm3("unknown"), 15.58);
    assert!((mm3_to_grams(1000.0, "gold_18k") - 15.58).abs() < 1e-12);
    assert!((grams_to_mm3(15.58, "gold_18k") - 1000.0).abs() < 1e-12);

    let table = material_weight_table(1000.0);
    assert_eq!(table.len(), 7);
    assert_eq!(table[0].0, "gold_24k");
    assert_eq!(table[2].0, "gold_18k");
    assert_eq!(table[2].1.volume_mm3, 1000.0);
    assert_eq!(table[2].1.weight_g, 15.58);
    assert!(table[6].1.weight_g > table[5].1.weight_g);
}

#[test]
fn sdf_value_transforms_match_voxel_ops_contract() {
    let values = vec![-2.0_f32, -0.25, 0.0, 1.5];
    let offset = sdf_offset_values(&values, 0.5).unwrap();
    assert_eq!(offset, vec![-2.5, -0.75, -0.5, 1.0]);

    let shell = sdf_shell_values(&values, 1.0).unwrap();
    assert_eq!(shell, vec![1.0, -0.25, 0.0, 1.5]);
    assert!(sdf_offset_values(&values, f64::NAN).is_err());
    assert!(sdf_shell_values(&values, 0.0).is_err());
}

#[test]
fn occupied_sdf_surface_extraction_matches_python_contract() {
    let values = vec![
        -1.0_f32, -1.0, //
        -1.0, -1.0, //
        -1.0, -1.0, //
        -1.0, -1.0,
    ];

    let mesh =
        extract_surface_mesh_from_sdf_cells(&values, [0.0, 0.0, 0.0], [2, 2, 2], 1.0, 0.0).unwrap();

    assert_eq!(mesh.vertices.len(), 8);
    assert_eq!(mesh.faces.len(), 12);
    assert_eq!(mesh.vertices[0], [0.0, 0.0, 0.0]);
    assert_eq!(mesh.faces[0], [0, 1, 2]);
    assert_eq!(mesh.faces[1], [0, 2, 3]);

    let empty =
        extract_surface_mesh_from_sdf_cells(&[1.0; 8], [0.0, 0.0, 0.0], [2, 2, 2], 1.0, 0.0)
            .unwrap();
    assert!(empty.vertices.is_empty());
    assert!(empty.faces.is_empty());
}

#[test]
fn ring_size_helpers_match_python_module_contract() {
    assert_eq!(ring_diameter_for_size(5.0), 15.67);
    assert!(
        (ring_diameter_for_size(5.25) - ((40.0 + 5.25 * 2.55) / std::f64::consts::PI)).abs()
            < 1e-12
    );
    assert_eq!(closest_ring_size(Some(15.6)), Some(5.0));
    assert_eq!(closest_ring_size(None), None);
}

#[test]
fn empty_ring_measurement_matches_python_module_contract() {
    let measurement = measure_ring(&[], None).unwrap();

    assert_eq!(measurement.ring_axis, [0.0, 1.0, 0.0]);
    assert_eq!(measurement.ring_axis_confidence, 0.0);
    assert_eq!(measurement.inner_diameter_mm, None);
    assert_eq!(measurement.bbox_mm, [0.0, 0.0, 0.0]);
    assert!(measurement.needs_axis_confirmation);
}

#[test]
fn nearest_distances_to_indices_matches_vertex_targets() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 3.0, 0.0],
        [4.0, 0.0, 0.0],
    ];

    let distances = nearest_distances_to_indices(&vertices, &[0, 2]).unwrap();

    assert_eq!(distances, vec![0.0, 2.0, 0.0, 4.0]);
}

#[test]
fn protected_hollow_scale_field_preserves_selected_regions() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [16.0, 0.0, 0.0],
    ];
    let regions = vec!["head".to_string(), "outer_band".to_string()];
    let scales = protected_hollow_scale_field(
        &vertices,
        &regions,
        &[0, 1, 3],
        &[1, 2, 3],
        &["head".to_string()],
        1.0,
    )
    .unwrap();

    assert_eq!(scales.len(), vertices.len());
    assert!((scales[1] - 0.18).abs() < 1e-6);
    assert!(scales[0] < 1.0);
    assert_eq!(scales[3], 1.0);
}

#[test]
fn hollow_preview_offsets_vertices_inward() {
    let (vertices, faces) = cube();
    let regions = vec!["head".to_string()];

    let displaced = weighted_inner_offset_vertices(
        &vertices,
        &faces,
        &regions,
        &[0, 1],
        &[0],
        &["head".to_string()],
        0.5,
    )
    .unwrap();

    assert_eq!(displaced.len(), vertices.len());
    assert_ne!(displaced[0], vertices[0]);
}

#[test]
fn adaptive_hollow_to_weight_hits_midpoint_target() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.5,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let midpoint_shell = voxel_shell_mesh(&vertices, &faces, 0.8, options).unwrap();
    let target_weight_g = mm3_to_grams(
        mesh_volume(&midpoint_shell.vertices, &midpoint_shell.faces).unwrap(),
        "silver_925",
    );

    let result = adaptive_hollow_to_weight(
        &vertices,
        &faces,
        target_weight_g,
        "silver_925",
        0.01,
        0.4,
        1.2,
        1,
        options,
    )
    .unwrap();

    assert_eq!(result.iterations, 1);
    assert_eq!(result.wall_thickness_mm, Some(0.8));
    assert!(result.warning.is_none());
    assert!((result.achieved_weight_g - target_weight_g).abs() < 0.01);
    assert!(!result.faces.is_empty());
}

#[test]
fn protected_hollow_mesh_builds_closed_shell() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.5,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };

    let shell = protected_hollow_mesh(
        &vertices,
        &faces,
        &["head".to_string()],
        &[0, 2],
        &[0, 1],
        &["head".to_string()],
        0.8,
        options,
    )
    .unwrap();
    let health = mesh_health(&shell.vertices, &shell.faces, true, Some(50_000), 1e-8).unwrap();

    assert!(!shell.faces.is_empty());
    assert!(health.is_closed);
    assert!(
        mesh_volume(&shell.vertices, &shell.faces).unwrap()
            < mesh_volume(&vertices, &faces).unwrap()
    );
}

#[test]
fn global_thicken_mesh_uses_service_offset_contract() {
    let (vertices, faces) = cube();
    let thickened = global_thicken_mesh(&vertices, &faces, 1.0).unwrap();
    let reference = voxel_offset_mesh(
        &vertices,
        &faces,
        0.5,
        VoxelMeshOptions {
            voxel_size: 0.25,
            padding_mm: None,
            extractor: VoxelMeshExtractor::Marching,
            refine: false,
        },
    )
    .unwrap();

    assert_eq!(thickened.vertices, reference.vertices);
    assert_eq!(thickened.faces, reference.faces);
    assert!(mesh_volume(&thickened.vertices, &thickened.faces).unwrap() > 8.0);
}

#[test]
fn service_hollow_mesh_uses_meshlib_service_shell_contract() {
    let (vertices, faces) = cube();
    let shell = service_hollow_mesh(&vertices, &faces, 1.0).unwrap();
    let reference = voxel_shell_mesh(
        &vertices,
        &faces,
        1.0,
        VoxelMeshOptions {
            voxel_size: 0.25,
            padding_mm: None,
            extractor: VoxelMeshExtractor::Marching,
            refine: false,
        },
    )
    .unwrap();

    assert_eq!(service_hollow_voxel_size(&vertices, 1.0).unwrap(), 0.25);
    assert_eq!(shell.vertices, reference.vertices);
    assert_eq!(shell.faces, reference.faces);
    assert!(mesh_volume(&shell.vertices, &shell.faces).unwrap() < 8.0);
}

#[test]
fn adaptive_protected_hollow_to_weight_hits_midpoint_target() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.5,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let region_ids = vec!["head".to_string()];
    let vertex_offsets = vec![0, 2];
    let vertex_indices = vec![0, 1];
    let protect_region_ids = vec!["head".to_string()];
    let midpoint_shell = protected_hollow_mesh(
        &vertices,
        &faces,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &protect_region_ids,
        0.8,
        options,
    )
    .unwrap();
    let target_weight_g = mm3_to_grams(
        mesh_volume(&midpoint_shell.vertices, &midpoint_shell.faces).unwrap(),
        "silver_925",
    );

    let result = adaptive_protected_hollow_to_weight(
        &vertices,
        &faces,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &protect_region_ids,
        target_weight_g,
        "silver_925",
        0.01,
        0.4,
        1.2,
        1,
        options,
    )
    .unwrap();

    assert_eq!(result.iterations, 1);
    assert_eq!(result.wall_thickness_mm, Some(0.8));
    assert!(result.warning.is_none());
    assert!((result.achieved_weight_g - target_weight_g).abs() < 0.01);
    assert!(!result.faces.is_empty());
}

#[test]
fn drain_hole_planning_returns_opposing_plans() {
    let vertices = vec![
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
    ];
    let plans = plan_drain_holes(
        &vertices,
        &["inner_band".to_string()],
        &[0, 4],
        &[0, 1, 2, 3],
        [0.0, 1.0, 0.0],
        0.8,
        1.0,
    )
    .unwrap();

    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].radius_mm, 0.5);
    assert_eq!(plans[0].length_mm, 4.0);
    assert!(dot(plans[0].direction, plans[1].direction) < -0.95);
}

#[test]
fn drain_hole_cutter_mesh_counts_match_python_contract() {
    let cutter = drain_hole_cutter_mesh(
        DrainHolePlan {
            center_mm: [0.0, 0.0, 0.0],
            direction: [1.0, 0.0, 0.0],
            radius_mm: 0.5,
            length_mm: 4.0,
        },
        16,
    )
    .unwrap();

    assert_eq!(cutter.vertices.len(), 34);
    assert_eq!(cutter.faces.len(), 64);

    let cutters = drain_hole_cutters_mesh(
        &[
            DrainHolePlan {
                center_mm: [0.0, 0.0, 0.0],
                direction: [1.0, 0.0, 0.0],
                radius_mm: 0.5,
                length_mm: 4.0,
            },
            DrainHolePlan {
                center_mm: [0.0, 0.0, 0.0],
                direction: [-1.0, 0.0, 0.0],
                radius_mm: 0.5,
                length_mm: 4.0,
            },
        ],
        12,
    )
    .unwrap();

    assert_eq!(cutters.vertices.len(), 52);
    assert_eq!(cutters.faces.len(), 96);
}

#[test]
fn compare_summary_matches_cube_surface_distances() {
    let source_vertices = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let (target_vertices, target_faces) = cube();
    let target_vertices: Vec<[f64; 3]> = target_vertices
        .into_iter()
        .map(|vertex| scale(vertex, 2.0))
        .collect();

    let distances =
        nearest_surface_distances(&source_vertices, &target_vertices, &target_faces).unwrap();
    let summary = compare_summary(&source_vertices, &target_vertices, &target_faces).unwrap();

    assert!(distances
        .iter()
        .all(|distance| (*distance - 1.0).abs() < 1e-6));
    assert_eq!(summary.min_mm, Some(1.0));
    assert_eq!(summary.max_mm, Some(1.0));
    assert_eq!(summary.mean_mm, Some(1.0));
}

#[test]
fn nearest_vertex_distances_match_python_behavior() {
    let source_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let target_vertices = vec![[0.0, 1.0, 0.0], [5.0, 0.0, 0.0]];

    let distances = nearest_vertex_distances(&source_vertices, &target_vertices);

    assert_eq!(distances.len(), 2);
    assert!((distances[0] - 1.0).abs() < 1e-6);
    assert!((distances[1] - 2.236_068).abs() < 1e-5);
}

#[test]
fn signed_compare_summary_uses_unsigned_fallback_for_open_target() {
    let source_vertices = vec![[0.0, 0.0, 0.0]];
    let (target_vertices, mut target_faces) = cube();
    target_faces.truncate(10);

    let distances = signed_surface_distances(
        &source_vertices,
        &target_vertices,
        &target_faces,
        0.5,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();
    let summary = signed_compare_summary(
        &source_vertices,
        &target_vertices,
        &target_faces,
        0.5,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();

    assert_eq!(distances.len(), 1);
    assert!(distances[0] >= 0.0);
    assert_eq!(summary.min_mm, Some(f64::from(distances[0])));
}

#[test]
fn version_compare_summary_matches_service_contract_shape() {
    let (source_vertices, source_faces) = cube();
    let target_vertices: Vec<[f64; 3]> = source_vertices
        .iter()
        .map(|vertex| scale(*vertex, 2.0))
        .collect();
    let target_faces = source_faces.clone();

    let summary = version_compare_summary(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &target_faces,
        SignedCompareOptions {
            winding_threshold: 0.5,
            reject_self_intersections: true,
            max_self_intersection_faces: Some(50_000),
            epsilon: 1e-8,
        },
    )
    .unwrap();

    assert!((summary.volume_delta_mm3 + 56.0).abs() < 1e-12);
    assert_eq!(summary.bbox_delta_mm, [-2.0, -2.0, -2.0]);
    assert_eq!(summary.min_signed_distance_mm, Some(-1.0));
    assert_eq!(summary.max_signed_distance_mm, Some(-1.0));
    assert_eq!(summary.mean_signed_distance_mm, Some(-1.0));
}

#[test]
fn version_compare_distances_filter_service_outliers() {
    let (source_vertices, _) = cube();
    let (target_vertices, target_faces) = cube();
    let far_target_vertices: Vec<[f64; 3]> = target_vertices
        .iter()
        .map(|vertex| add(*vertex, [100.0, 0.0, 0.0]))
        .collect();

    let distances = version_compare_distances(
        &source_vertices,
        &far_target_vertices,
        &target_faces,
        SignedCompareOptions {
            winding_threshold: 0.5,
            reject_self_intersections: true,
            max_self_intersection_faces: Some(50_000),
            epsilon: 1e-8,
        },
    )
    .unwrap();

    assert_eq!(distances.len(), source_vertices.len());
    assert!(distances.iter().all(|distance| distance.is_nan()));
}

#[test]
fn service_compare_distances_follow_meshlib_reference_mesh_direction() {
    let (source_vertices, source_faces) = cube();
    let other_vertices = vec![
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.0, 0.5, -0.5],
        [0.0, 0.0, 0.75],
    ];
    let other_faces = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
    let options = SignedCompareOptions {
        winding_threshold: 0.5,
        reject_self_intersections: true,
        max_self_intersection_faces: Some(50_000),
        epsilon: 1e-8,
    };

    let service_distances =
        service_compare_distances(&source_vertices, &source_faces, &other_vertices, options)
            .unwrap();
    let expected =
        version_compare_distances(&other_vertices, &source_vertices, &source_faces, options)
            .unwrap();
    let summary = service_compare_summary(
        &source_vertices,
        &source_faces,
        &other_vertices,
        &other_faces,
        options,
    )
    .unwrap();

    assert_eq!(service_distances.len(), other_vertices.len());
    assert_eq!(service_distances, expected);
    assert!(summary.volume_delta_mm3 > 0.0);
    assert_eq!(summary.bbox_delta_mm, [1.0, 1.0, 0.75]);
    assert_eq!(
        summary.min_signed_distance_mm,
        summarize_distances(&service_distances, true).min_mm
    );
}

#[test]
fn cube_has_no_self_intersections() {
    let (vertices, faces) = cube();
    let intersections = self_intersecting_faces(&vertices, &faces, 1e-8).unwrap();

    assert!(intersections.is_empty());
}

#[test]
fn crossing_triangles_report_both_faces() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let intersections = self_intersecting_faces(&vertices, &faces, 1e-8).unwrap();

    assert_eq!(intersections, vec![0, 1]);
}

#[test]
fn point_mesh_distances_match_cube_fixture() {
    let (vertices, faces) = cube();
    let points = vec![[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]];

    let distances = point_mesh_distances(&points, &vertices, &faces).unwrap();

    assert_eq!(distances.len(), 2);
    assert!((distances[0] - 1.0).abs() < 1e-9);
    assert!((distances[1] - 1.0).abs() < 1e-9);
}

#[test]
fn closest_points_report_face_ids_for_multiple_queries() {
    let (vertices, faces) = cube();
    let points = vec![[2.0, 0.0, 0.0], [0.0, 0.0, 2.0], [0.25, 0.25, 0.25]];

    let hits = closest_points_on_mesh(&points, &vertices, &faces).unwrap();

    assert_eq!(hits.closest_points.len(), 3);
    assert_eq!(hits.distances.len(), 3);
    assert_eq!(hits.face_indices.len(), 3);
    assert!(hits.face_indices.iter().all(|face_id| *face_id >= 0));
    assert!((hits.distances[0] - 1.0).abs() < 1e-9);
    assert!((hits.distances[1] - 1.0).abs() < 1e-9);
    assert!((hits.distances[2] - 0.75).abs() < 1e-9);
}

#[test]
fn ray_hits_cube_front_face() {
    let (vertices, faces) = cube();

    let hit = first_ray_hit(
        &vertices,
        &faces,
        [0.0, 0.0, 3.0],
        [0.0, 0.0, -1.0],
        1e-8,
        &[],
    )
    .unwrap()
    .unwrap();

    assert!((hit.distance - 2.0).abs() < 1e-9);
    assert_eq!(hit.point, [0.0, 0.0, 1.0]);
}

#[test]
fn ray_hit_skips_ignored_nearest_faces() {
    let (vertices, faces) = cube();

    let hit = first_ray_hit(
        &vertices,
        &faces,
        [0.0, 0.0, 3.0],
        [0.0, 0.0, -1.0],
        1e-8,
        &[2, 3],
    )
    .unwrap()
    .unwrap();

    assert!((hit.distance - 4.0).abs() < 1e-9);
    assert_eq!(hit.point, [0.0, 0.0, -1.0]);
}

#[test]
fn batched_ray_hits_reuse_tree_and_report_misses() {
    let (vertices, faces) = cube();
    let origins = vec![[0.0, 0.0, 3.0], [4.0, 4.0, 4.0]];
    let directions = vec![[0.0, 0.0, -1.0], [1.0, 0.0, 0.0]];

    let hits = first_ray_hits(&vertices, &faces, &origins, &directions, 1e-8, &[]).unwrap();

    assert_eq!(hits.face_indices.len(), 2);
    assert!(hits.face_indices[0] >= 0);
    assert!((hits.distances[0] - 2.0).abs() < 1e-9);
    assert_eq!(hits.points[0], [0.0, 0.0, 1.0]);
    assert_eq!(hits.face_indices[1], -1);
    assert!(hits.distances[1].is_infinite());
    assert!(hits.points[1].iter().all(|value| value.is_nan()));
}

#[test]
fn winding_numbers_classify_cube_points() {
    let (vertices, faces) = cube();
    let points = vec![[0.0, 0.0, 0.0], [3.0, 0.0, 0.0]];

    let values = winding_numbers(&points, &vertices, &faces).unwrap();

    assert!((values[0].abs() - 1.0).abs() < 1e-9);
    assert!(values[1].abs() < 1e-9);
}

#[test]
fn signed_point_mesh_distances_classify_cube_points() {
    let (vertices, faces) = cube();
    let points = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]];

    let distances = signed_point_mesh_distances(&points, &vertices, &faces, 0.5).unwrap();

    assert!((distances[0] + 1.0).abs() < 1e-9);
    assert!((distances[1] - 1.0).abs() < 1e-9);
}

#[test]
fn ray_thickness_matches_cube_fixture_shape() {
    let (vertices, faces) = cube();

    let thickness = ray_thickness_at_vertices(&vertices, &faces, 1e-5).unwrap();

    assert_eq!(thickness.len(), vertices.len());
    assert!(thickness.iter().all(|value| value.is_finite()));
    assert!(thickness.iter().all(|value| *value > 0.0));
}

#[test]
fn service_thickness_combines_insphere_and_ray_like_meshlib_service() {
    let (vertices, faces) = cube();
    let options = InSphereThicknessOptions {
        max_radius: 0.5,
        ..InSphereThicknessOptions::default()
    };

    let ray = ray_thickness_at_vertices(&vertices, &faces, options.epsilon).unwrap();
    let insphere = insphere_thickness_at_vertices(&vertices, &faces, options).unwrap();
    let combined = service_thickness_at_vertices(&vertices, &faces, options).unwrap();

    assert_eq!(combined.len(), vertices.len());
    for ((combined_value, insphere_value), ray_value) in combined.iter().zip(&insphere).zip(&ray) {
        assert!(combined_value.is_finite());
        assert!(*combined_value > 0.0);
        assert!(*combined_value <= *insphere_value + 1e-6);
        assert!(*combined_value as f64 <= *ray_value + 1e-6);
        assert!(*combined_value <= 1.0 + 1e-6);
    }
}

#[test]
fn sdf_grid_values_classify_cube_center() {
    let (vertices, faces) = cube();

    let values =
        sdf_grid_values(&vertices, &faces, [-2.0, -2.0, -2.0], [5, 5, 5], 1.0, 0.5).unwrap();

    assert_eq!(values.len(), 125);
    assert!(values[2 * 25 + 2 * 5 + 2] < 0.0);
    assert!(values[0] > 0.0);
}

#[test]
fn sdf_grid_helpers_match_python_contract() {
    let values = vec![-1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    let cells = sdf_cell_values(&values, [2, 2, 2]).unwrap();
    let occupancy = sdf_occupancy(&values, [2, 2, 2], 0.0).unwrap();
    let volume = estimate_sdf_volume(&values, [2, 2, 2], 0.5, 0.0).unwrap();
    let samples = sample_sdf_values_batch(
        &values,
        [0.0, 0.0, 0.0],
        [2, 2, 2],
        1.0,
        &[[0.0, 0.0, 0.0], [0.5, 0.5, 0.5]],
    )
    .unwrap();
    let gradients =
        sample_sdf_gradients_batch(&values, [0.0, 0.0, 0.0], [2, 2, 2], 1.0, &[[0.5; 3]]).unwrap();

    assert_eq!(cells, vec![0.75]);
    assert_eq!(occupancy, vec![0]);
    assert_eq!(volume, 0.0);
    assert_eq!(samples[0], -1.0);
    assert!((samples[1] - 0.75).abs() < 1e-6);
    assert_eq!(gradients.len(), 1);
    assert!(gradients[0].iter().all(|value| value.is_finite()));
}

#[test]
fn sdf_boolean_values_match_field_operations() {
    let left = vec![-1.0, 0.25, 2.0];
    let right = vec![0.5, -0.75, 1.0];

    let union = sdf_boolean_values(&left, &right, SdfBooleanOperation::Union).unwrap();
    let intersection =
        sdf_boolean_values(&left, &right, SdfBooleanOperation::Intersection).unwrap();
    let difference = sdf_boolean_values(&left, &right, SdfBooleanOperation::Difference).unwrap();

    assert_eq!(union, vec![-1.0, -0.75, 1.0]);
    assert_eq!(intersection, vec![0.5, 0.25, 2.0]);
    assert_eq!(difference, vec![-0.5, 0.75, 2.0]);
}

#[test]
fn sdf_boolean_values_reject_mismatched_lengths() {
    let error = sdf_boolean_values(&[0.0], &[0.0, 1.0], SdfBooleanOperation::Union).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::MismatchedSdfValueCount { left: 1, right: 2 }
    ));
}

#[test]
fn sdf_boolean_marching_tetrahedra_matches_staged_field_extraction() {
    let (vertices, faces) = cube();
    let left = sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let right: Vec<f32> = left.iter().map(|value| *value - 0.25).collect();
    let staged_values = sdf_boolean_values(&left, &right, SdfBooleanOperation::Union).unwrap();
    let staged =
        marching_tetrahedra(&staged_values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();

    let resident = sdf_boolean_marching_tetrahedra(
        &left,
        &right,
        SdfBooleanOperation::Union,
        [-1.5, -1.5, -1.5],
        [7, 7, 7],
        0.5,
        0.0,
    )
    .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn sdf_offset_marching_tetrahedra_matches_staged_field_extraction() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let staged_values: Vec<f32> = values.iter().map(|value| *value - 0.25).collect();
    let staged =
        marching_tetrahedra(&staged_values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();

    let resident =
        sdf_offset_marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.25, 0.0)
            .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn sdf_shell_marching_tetrahedra_matches_staged_field_extraction() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-2.0, -2.0, -2.0], [9, 9, 9], 0.5, 0.5).unwrap();
    let staged_values: Vec<f32> = values
        .iter()
        .map(|value| (*value as f64).max(-(*value as f64 + 0.75)) as f32)
        .collect();
    let staged =
        marching_tetrahedra(&staged_values, [-2.0, -2.0, -2.0], [9, 9, 9], 0.5, 0.0).unwrap();

    let resident =
        sdf_shell_marching_tetrahedra(&values, [-2.0, -2.0, -2.0], [9, 9, 9], 0.5, 0.75, 0.0)
            .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn project_vertices_to_sdf_moves_points_toward_cube_surface() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let query = vec![[1.25, 0.0, 0.0], [0.0, 0.0, 1.25]];

    let projected =
        project_vertices_to_sdf(&query, &values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0, 3)
            .unwrap();

    assert!((projected[0][0] - 1.0).abs() < 1e-5);
    assert!(projected[0][1].abs() < 1e-5);
    assert!(projected[0][2].abs() < 1e-5);
    assert!((projected[1][2] - 1.0).abs() < 1e-5);
}

#[test]
fn project_vertices_to_sdf_rejects_mismatched_grid_values() {
    let error = project_vertices_to_sdf(&[], &[0.0], [0.0; 3], [2, 2, 2], 1.0, 0.0, 1).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::SdfValueCountDoesNotMatchShape {
            values: 1,
            shape: [2, 2, 2]
        }
    ));
}

#[test]
fn refine_vertices_with_sdf_matches_staged_smoothing_and_projection() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let moved: Vec<[f64; 3]> = vertices
        .iter()
        .map(|vertex| [vertex[0] * 0.92, vertex[1] * 0.92, vertex[2] * 0.92])
        .collect();
    let smoothed = laplacian_smooth_vertices(&moved, &faces, 1, 0.2).unwrap();
    let staged = project_vertices_to_sdf(
        &smoothed,
        &values,
        [-1.5, -1.5, -1.5],
        [7, 7, 7],
        0.5,
        0.0,
        3,
    )
    .unwrap();

    let resident = refine_vertices_with_sdf(
        &moved,
        &faces,
        &values,
        [-1.5, -1.5, -1.5],
        [7, 7, 7],
        0.5,
        0.0,
        1,
        0.2,
        3,
    )
    .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn laplacian_smooth_vertices_matches_one_ring_average() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 3, 2]];

    let smoothed = laplacian_smooth_vertices(&vertices, &faces, 1, 0.5).unwrap();

    assert_eq!(smoothed.len(), vertices.len());
    assert_eq!(smoothed[0], [0.5, 0.5, 0.0]);
    assert_eq!(smoothed[3], [1.5, 1.5, 0.0]);
}

#[test]
fn laplacian_smooth_vertices_zero_iterations_is_noop() {
    let (vertices, faces) = cube();

    let smoothed = laplacian_smooth_vertices(&vertices, &faces, 0, 1.0).unwrap();

    assert_eq!(smoothed, vertices);
}

#[test]
fn weighted_laplacian_smooth_vertices_scales_by_weight() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 3, 2]];
    let weights = vec![1.0, 0.0, 0.5, 1.0];

    let smoothed =
        weighted_laplacian_smooth_vertices(&vertices, &faces, &weights, 1, 0.5, 0.02).unwrap();

    assert_eq!(smoothed[0], [0.5, 0.5, 0.0]);
    assert_eq!(smoothed[1], vertices[1]);
    assert!((smoothed[2][0] - 1.0 / 3.0).abs() < 1e-12);
    assert!((smoothed[2][1] - 5.0 / 3.0).abs() < 1e-12);
    assert_eq!(smoothed[2][2], 0.0);
}

#[test]
fn weighted_laplacian_smooth_vertices_rejects_mismatched_weights() {
    let (vertices, faces) = cube();

    let error =
        weighted_laplacian_smooth_vertices(&vertices, &faces, &[1.0], 1, 0.5, 0.02).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::WeightCountDoesNotMatchVertices {
            weights: 1,
            vertices: 8
        }
    ));
}

#[test]
fn taubin_smooth_vertices_alternates_laplacian_passes() {
    let (vertices, faces) = cube();

    let smoothed = taubin_smooth_vertices(&vertices, &faces, 2, 0.25, -0.5).unwrap();
    let laplacian_only = laplacian_smooth_vertices(&vertices, &faces, 2, 0.25).unwrap();

    assert_eq!(smoothed.len(), vertices.len());
    assert_ne!(smoothed, vertices);
    assert_ne!(smoothed, laplacian_only);
}

#[test]
fn falloff_weights_match_gaussian_seed_distances() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
    ];

    let weights = falloff_weights(&vertices, &[0], 1.0, 3.0).unwrap();

    assert_eq!(weights[0], 1.0);
    assert!((weights[1] - (-0.5_f32).exp()).abs() < 1e-6);
    assert!((weights[2] - (-2.0_f32).exp()).abs() < 1e-6);
    assert_eq!(weights[3], 0.0);
}

#[test]
fn falloff_weights_reject_empty_seeds() {
    let (vertices, _) = cube();

    let error = falloff_weights(&vertices, &[], 1.0, 3.0).unwrap_err();

    assert!(matches!(error, GeometryError::EmptySeedIndices));
}

#[test]
fn smooth_vertices_with_falloff_matches_weighted_pipeline() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 3, 2]];
    let weights = falloff_weights(&vertices, &[0], 2.0, 3.0).unwrap();

    let resident = smooth_vertices_with_falloff(
        &vertices,
        &faces,
        &[0],
        SmoothFalloffOptions {
            falloff_mm: 2.0,
            iterations: 2,
            strength: 0.35,
            active_threshold: 0.02,
            cutoff_multiplier: 3.0,
        },
    )
    .unwrap();
    let staged =
        weighted_laplacian_smooth_vertices(&vertices, &faces, &weights, 2, 0.35, 0.02).unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn outward_directions_flips_center_facing_normals() {
    let vertices = vec![[0.0, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 1.0, -1.0]];
    let faces = vec![[0, 1, 2]];

    let directions = outward_directions(&vertices, &faces).unwrap();

    assert_eq!(directions, vec![[0.0, 0.0, -1.0]; 3]);
}

#[test]
fn local_offset_vertices_matches_staged_displacement() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];
    let directions = outward_directions(&vertices, &faces).unwrap();
    let weights = falloff_weights(&vertices, &[0], 2.0, 3.0).unwrap();

    let displaced = local_offset_vertices(&vertices, &faces, &[0], 2.0, 0.25, 3.0).unwrap();

    for (index, vertex) in vertices.iter().enumerate() {
        let expected = add(
            *vertex,
            scale(directions[index], 0.25 * weights[index] as f64),
        );
        assert_eq!(displaced[index], expected);
    }
}

#[test]
fn local_thicken_to_minimum_vertices_uses_deficit_field() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];
    let thickness = vec![0.25, 0.75, 1.5, f32::NAN];
    let min_target = 1.0;
    let deficit_scale = 0.75;
    let directions = outward_directions(&vertices, &faces).unwrap();
    let weights = falloff_weights(&vertices, &[0], 2.0, 3.0).unwrap();

    let displaced = local_thicken_to_minimum_vertices(
        &vertices,
        &faces,
        &[0],
        &thickness,
        min_target,
        2.0,
        deficit_scale,
    )
    .unwrap();

    for (index, vertex) in vertices.iter().enumerate() {
        let safe_thickness = if thickness[index].is_finite() {
            thickness[index].max(0.0) as f64
        } else {
            0.0
        };
        let deficit = (min_target - safe_thickness).clamp(0.0, min_target);
        let expected = add(
            *vertex,
            scale(
                directions[index],
                deficit * weights[index] as f64 * deficit_scale,
            ),
        );
        assert_eq!(displaced[index], expected);
    }
}

#[test]
fn apply_brush_strokes_matches_sequential_pipeline() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];
    let first = local_offset_vertices(&vertices, &faces, &[0], 2.0, 0.25, 3.0).unwrap();
    let second = local_offset_vertices(&first, &faces, &[3], 1.5, -0.1, 3.0).unwrap();
    let expected = smooth_vertices_with_falloff(
        &second,
        &faces,
        &[0, 3],
        SmoothFalloffOptions {
            falloff_mm: 2.0,
            iterations: 1,
            strength: 0.25,
            active_threshold: 0.02,
            cutoff_multiplier: 3.0,
        },
    )
    .unwrap();

    let composed = apply_brush_strokes(
        &vertices,
        &faces,
        &[0, 1, 2],
        &[0, 1, 2, 4],
        &[0, 3, 0, 3],
        &[0, 0, 0],
        &[0, 0, 0, 0],
        &[],
        &[0, 0, 0, 0],
        &[],
        &[0.25, 0.1, 0.0],
        &[2.0, 1.5, 2.0],
        &[1, 1, 1],
        &[0.5, 0.5, 0.25],
        3.0,
    )
    .unwrap();

    assert_eq!(composed, expected);
}

#[test]
fn apply_brush_strokes_respects_masks_and_protected_vertices() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];

    let composed = apply_brush_strokes(
        &vertices,
        &faces,
        &[0],
        &[0, 1],
        &[0],
        &[1],
        &[0, 2],
        &[0, 2],
        &[0, 1],
        &[2],
        &[0.25],
        &[2.0],
        &[1],
        &[0.5],
        3.0,
    )
    .unwrap();

    assert_ne!(composed[0], vertices[0]);
    assert_eq!(composed[1], vertices[1]);
    assert_eq!(composed[2], vertices[2]);
    assert_eq!(composed[3], vertices[3]);
}

#[test]
fn region_brush_masks_respects_allowed_operations_and_overrides() {
    let region_ids = vec![
        "inner_band".to_string(),
        "outer_band".to_string(),
        "head".to_string(),
    ];
    let vertex_offsets = vec![0, 3, 6, 9];
    let vertex_indices = vec![4, 1, 2, 8, 7, 6, 11, 10, 9];
    let allowed_offsets = vec![0, 1, 2, 4];
    let allowed_operations = vec![1, 0, 0, 2];

    let (editable, protected) = region_brush_masks(
        1,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &allowed_offsets,
        &allowed_operations,
        &[],
        &["head".to_string()],
        false,
        true,
        true,
    )
    .unwrap();

    assert_eq!(editable, vec![1, 2, 4]);
    assert_eq!(protected, vec![6, 7, 8, 9, 10, 11]);
}

#[test]
fn region_brush_masks_rejects_unknown_region_id() {
    let region_ids = vec!["inner_band".to_string()];
    let error = region_brush_masks(
        0,
        &region_ids,
        &[0, 1],
        &[0],
        &[0, 1],
        &[0],
        &["missing".to_string()],
        &[],
        true,
        false,
        true,
    )
    .unwrap_err();

    assert!(matches!(error, GeometryError::UnknownRegionIds { ids } if ids == vec!["missing"]));
}

#[test]
fn apply_brush_strokes_rejects_bad_seed_offsets() {
    let (vertices, faces) = cube();
    let error = apply_brush_strokes(
        &vertices,
        &faces,
        &[0],
        &[0],
        &[0],
        &[0],
        &[0, 0],
        &[],
        &[0, 0],
        &[],
        &[0.1],
        &[1.0],
        &[1],
        &[0.5],
        3.0,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GeometryError::BrushSeedOffsetCountMismatch {
            offsets: 1,
            operations: 1
        }
    ));
}

#[test]
fn marching_tetrahedra_extracts_cube_surface() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();

    let mesh = marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();

    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.faces.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|point| point.iter().all(|value| value.is_finite())));
}

#[test]
fn finalized_marching_tetrahedra_repairs_and_orients_cube_surface() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();

    let raw = marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();
    let finalized =
        finalized_marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();
    let stats = mesh_stats(&finalized.vertices, &finalized.faces).unwrap();
    let health = mesh_health(
        &finalized.vertices,
        &finalized.faces,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();

    assert!(finalized.vertices.len() < raw.vertices.len());
    assert_eq!(finalized.faces.len(), raw.faces.len());
    assert_eq!(stats.boundary_edge_count, 0);
    assert!(health.is_closed);
    assert!(stats.volume_mm3 > 0.0);
}

fn shifted_scaled_tetrahedron(offset: [f64; 3], scale: f64) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let vertices = vec![
        [offset[0], offset[1], offset[2]],
        [scale + offset[0], offset[1], offset[2]],
        [offset[0], scale + offset[1], offset[2]],
        [offset[0], offset[1], scale + offset[2]],
    ];
    let faces = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
    (vertices, faces)
}

#[test]
fn exact_intersection_pipeline_reports_mixed_tetra_surface_contacts() {
    let (first_vertices, first_faces) = shifted_scaled_tetrahedron([0.0, 0.0, 0.0], 2.0);
    let (second_vertices, second_faces) = shifted_scaled_tetrahedron([0.5, 0.5, 0.5], 2.0);

    let intersections = exact_mesh_intersections(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!intersections.is_empty());

    let contours = exact_intersection_contours(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!contours.is_empty());

    let one_mesh_contours = exact_one_mesh_intersection_contours(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!one_mesh_contours.first.is_empty());
    assert_eq!(
        one_mesh_contours.first.len(),
        one_mesh_contours.second.len()
    );

    let cut_meshes = exact_mesh_pair_cut_meshes(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!cut_meshes.first.cut_edges.is_empty());
    assert!(!cut_meshes.second.cut_edges.is_empty());
}

#[test]
fn marching_tetrahedra_rejects_mismatched_grid_values() {
    let error = marching_tetrahedra(&[1.0], [0.0; 3], [2, 2, 2], 1.0, 0.0).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::SdfValueCountDoesNotMatchShape {
            values: 1,
            shape: [2, 2, 2]
        }
    ));
}

#[test]
fn orient_faces_consistently_flips_shared_same_direction_edges() {
    let faces = vec![[0, 1, 2], [1, 2, 3], [4, 5, 6]];

    let result = orient_faces_consistently(&faces).unwrap();

    assert_eq!(result.faces, vec![[0, 1, 2], [1, 3, 2], [4, 5, 6]]);
    assert_eq!(result.component_offsets, vec![0, 2, 3]);
    assert_eq!(result.component_faces, vec![0, 1, 2]);
}

#[test]
fn orient_faces_consistently_rejects_negative_indices() {
    let error = orient_faces_consistently(&[[0, -1, 2]]).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::NegativeFaceIndex {
            face: 0,
            vertex: -1
        }
    ));
}

fn exact_cut_triangle() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    (
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        vec![[0, 1, 2]],
    )
}

#[test]
fn exact_cut_mesh_splits_vertex_to_opposite_edge_triangle() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [0.0, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 4);
    assert_eq!(result.faces.len(), 2);
    assert_eq!(result.cut_edges, vec![[0, 3]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_edge_to_edge_triangle() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.0, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([2, 0]),
                coordinate: [0.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 3);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_interior_face_point_to_edge_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.4, 0.4, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 4);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_edge_point_to_interior_face_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.4, 0.4, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 4);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_interior_face_point_to_interior_face_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.4, 0.4, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.8, 0.6, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 5);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_two_boundary_spokes_to_shared_interior_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.0, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.5, 0.5, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 6);
    assert_eq!(result.faces.len(), 5);
    assert_eq!(result.cut_edges, vec![[3, 4], [4, 5]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_mesh_pair_cut_meshes_return_operand_results() {
    let first_vertices = vec![[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]];
    let first_faces = vec![[0, 1, 2]];
    let second_vertices = vec![[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [3.0, 0.0, 0.0]];
    let second_faces = vec![[0, 1, 2]];

    let result = exact_mesh_pair_cut_meshes(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.first.vertices.len() >= first_vertices.len());
    assert!(result.second.vertices.len() >= second_vertices.len());
    assert!(!result.first.cut_edges.is_empty() || !result.first.skipped_source_faces.is_empty());
    assert!(!result.second.cut_edges.is_empty() || !result.second.skipped_source_faces.is_empty());
}

#[test]
fn exact_planar_hole_fill_plan_triangulates_loop_and_preserves_source_face() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 2.0, 0.0],
    ];

    let plan = exact_planar_hole_fill_plan(&vertices, &[0, 1, 2, 3], 1e-9).unwrap();
    let execution = execute_exact_planar_hole_fill_plan(&plan, 42);

    assert_eq!(plan.boundary_loop, vec![0, 1, 2, 3]);
    assert_eq!(plan.num_tris, 2);
    assert_eq!(execution.faces.len(), 2);
    assert_eq!(execution.source_face_for_faces, vec![42, 42]);
}

#[test]
fn exact_planar_hole_fill_plan_rejects_degenerate_loop() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];

    assert!(exact_planar_hole_fill_plan(&vertices, &[0, 1, 2], 1e-9).is_none());
}

fn exact_cut_mesh_with_square_cut_hole() -> ExactCutMeshResult {
    ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [2.0, -1.0, 0.0],
            [2.0, 2.0, 0.0],
            [-1.0, 2.0, 0.0],
        ],
        faces: vec![[4, 0, 1], [5, 1, 2], [6, 2, 3], [7, 3, 0]],
        cut_edges: vec![[0, 1], [1, 2], [2, 3], [0, 3]],
        cut_edge_paths: vec![vec![[0, 1], [1, 2], [2, 3], [3, 0]]],
        cut_edge_path_closed: vec![true],
        source_face_for_faces: vec![100, 101, 102, 103],
        skipped_source_faces: Vec::new(),
    }
}

#[test]
fn exact_cut_hole_fill_plans_discover_cut_boundary_loop() {
    let cut_mesh = exact_cut_mesh_with_square_cut_hole();

    let plans = exact_cut_hole_fill_plans(&cut_mesh, 1e-9).unwrap();

    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].representative_edge, [1, 0]);
    assert_eq!(plans[0].boundary_loop, vec![1, 0, 3, 2]);
    assert_eq!(
        plans[0].boundary_edges,
        vec![[1, 0], [0, 3], [3, 2], [2, 1]]
    );
    assert_eq!(plans[0].source_face, 100);
    assert_eq!(plans[0].fill_plan.num_tris, 2);
}

#[test]
fn exact_fill_cut_holes_appends_plan_faces_with_source_mapping() {
    let cut_mesh = exact_cut_mesh_with_square_cut_hole();

    let result = exact_fill_cut_holes(&cut_mesh, 1e-9).unwrap();

    assert_eq!(result.fill_plans.len(), 1);
    assert_eq!(result.added_face_ranges, vec![[cut_mesh.faces.len(), 6]]);
    assert_eq!(result.mesh.vertices, cut_mesh.vertices);
    assert_eq!(result.mesh.faces.len(), cut_mesh.faces.len() + 2);
    assert_eq!(
        &result.mesh.source_face_for_faces[cut_mesh.source_face_for_faces.len()..],
        &[100, 100]
    );
    assert_eq!(result.mesh.cut_edges, cut_mesh.cut_edges);
}
