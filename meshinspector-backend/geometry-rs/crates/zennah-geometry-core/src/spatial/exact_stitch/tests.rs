use super::*;

fn cut_mesh_with_paths(
    vertices: Vec<[f64; 3]>,
    cut_edges: Vec<[usize; 2]>,
    cut_edge_paths: Vec<Vec<[usize; 2]>>,
    cut_edge_path_closed: Vec<bool>,
) -> ExactCutMeshResult {
    ExactCutMeshResult {
        vertices,
        faces: Vec::new(),
        cut_edges,
        cut_edge_paths,
        cut_edge_path_closed,
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: Vec::new(),
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    }
}

#[test]
fn exact_stitch_plan_pairs_reversed_matching_edges() {
    let first_vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let second_vertices = vec![[1.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
    let plan = exact_stitch_plan_by_edges(
        &first_vertices,
        &[[0, 1]],
        &second_vertices,
        &[[0, 1]],
        1e-9,
    );

    assert!(plan.compatible);
    assert_eq!(plan.pairs.len(), 1);
    assert_eq!(
        plan.paths,
        vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }]
    );
    assert!(plan.pairs[0].second_reversed);
    assert!(plan.unmatched_first_edges.is_empty());
    assert!(plan.unmatched_second_edges.is_empty());
}

#[test]
fn exact_stitch_plan_reports_unmatched_edges() {
    let first_vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let second_vertices = vec![[0.0, 1.0, 0.0], [1.0, 1.0, 0.0]];
    let plan = exact_stitch_plan_by_edges(
        &first_vertices,
        &[[0, 1]],
        &second_vertices,
        &[[0, 1]],
        1e-9,
    );

    assert!(!plan.compatible);
    assert_eq!(plan.unmatched_first_edges, vec![0]);
    assert_eq!(plan.unmatched_second_edges, vec![0]);
    assert!(plan.paths.is_empty());
}

#[test]
fn exact_stitch_plan_accepts_meshlib_style_quantized_cut_endpoints() {
    let first_vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let second_vertices = vec![[3.0e-9, 0.0, 0.0], [1.0 + 3.0e-9, 0.0, 0.0]];
    let plan = exact_stitch_plan_by_edges(
        &first_vertices,
        &[[0, 1]],
        &second_vertices,
        &[[0, 1]],
        1e-9,
    );

    assert!(plan.compatible);
    assert_eq!(plan.pairs.len(), 1);
    assert!(plan.unmatched_first_edges.is_empty());
    assert!(plan.unmatched_second_edges.is_empty());
}

#[test]
fn exact_stitch_plan_prefers_meshlib_result_cut_path_grouping() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let first = cut_mesh_with_paths(
        vertices.clone(),
        vec![[0, 1], [1, 2]],
        vec![vec![[0, 1]], vec![[1, 2]]],
        vec![false, false],
    );
    let second = cut_mesh_with_paths(
        vertices,
        vec![[0, 1], [1, 2]],
        vec![vec![[0, 1]], vec![[1, 2]]],
        vec![false, false],
    );

    let plan = exact_stitch_plan_from_cut_meshes(&first, &second, 1e-9);

    assert!(plan.compatible);
    assert_eq!(plan.pairs.len(), 2);
    assert_eq!(plan.paths.len(), 2);
    assert_eq!(plan.paths[0].pair_indices, vec![0]);
    assert_eq!(plan.paths[1].pair_indices, vec![1]);
}

#[test]
fn exact_stitch_plan_keeps_cut_path_segments_for_closed_edge_components() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let first = cut_mesh_with_paths(
        vertices.clone(),
        vec![[0, 1], [1, 2], [2, 3], [0, 3]],
        vec![vec![[0, 1]], vec![[1, 2]], vec![[2, 3]], vec![[3, 0]]],
        vec![false, false, false, false],
    );
    let second = cut_mesh_with_paths(
        vertices,
        vec![[0, 1], [1, 2], [2, 3], [0, 3]],
        vec![vec![[0, 1]], vec![[1, 2]], vec![[2, 3]], vec![[3, 0]]],
        vec![false, false, false, false],
    );

    let plan = exact_stitch_plan_from_cut_meshes(&first, &second, 1e-9);

    assert!(plan.compatible);
    assert_eq!(plan.pairs.len(), 4);
    assert_eq!(plan.paths.len(), 4);
    assert!(plan.paths.iter().all(|path| !path.closed));
    assert_eq!(
        plan.paths
            .iter()
            .map(|path| path.pair_indices.clone())
            .collect::<Vec<_>>(),
        vec![vec![0], vec![1], vec![2], vec![3]]
    );
}

#[test]
fn exact_stitch_cut_path_plan_retains_complete_paths_when_later_path_unmatched() {
    let first_vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
    ];
    let second_vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 1.0, 0.0],
        [3.0, 1.0, 0.0],
    ];
    let first = cut_mesh_with_paths(
        first_vertices,
        vec![[0, 1], [2, 3]],
        vec![vec![[0, 1]], vec![[2, 3]]],
        vec![false, false],
    );
    let second = cut_mesh_with_paths(
        second_vertices,
        vec![[0, 1], [2, 3]],
        vec![vec![[0, 1]], vec![[2, 3]]],
        vec![false, false],
    );

    let plan = exact_stitch_plan_by_cut_paths(&first, &second, 1e-9).unwrap();

    assert!(!plan.compatible);
    assert_eq!(plan.pairs.len(), 1);
    assert_eq!(plan.paths.len(), 1);
    assert_eq!(plan.paths[0].pair_indices, vec![0]);
    assert_eq!(plan.unmatched_first_edges, vec![1]);
    assert_eq!(plan.unmatched_second_edges, vec![1]);
}

#[test]
fn exact_stitch_cut_path_plan_retains_pairs_when_path_lengths_differ() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
    ];
    let first = cut_mesh_with_paths(
        vertices.clone(),
        vec![[0, 1], [1, 2]],
        vec![vec![[0, 1], [1, 2]]],
        vec![false],
    );
    let second = cut_mesh_with_paths(
        vertices,
        vec![[0, 1], [1, 2], [2, 3]],
        vec![vec![[0, 1], [1, 2], [2, 3]]],
        vec![false],
    );

    let plan = exact_stitch_plan_by_cut_paths(&first, &second, 1e-9).unwrap();

    assert!(!plan.compatible);
    assert_eq!(plan.pairs.len(), 2);
    assert_eq!(plan.paths.len(), 1);
    assert_eq!(plan.paths[0].pair_indices, vec![0, 1]);
    assert!(!plan.paths[0].closed);
    assert!(plan.unmatched_first_edges.is_empty());
    assert_eq!(plan.unmatched_second_edges, vec![2]);
}

#[test]
fn exact_stitch_cut_path_plan_splits_segments_around_unmatched_edges() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
    ];
    let first = cut_mesh_with_paths(
        vertices.clone(),
        vec![[0, 1], [1, 2], [2, 3]],
        vec![vec![[0, 1], [1, 2], [2, 3]]],
        vec![false],
    );
    let second = cut_mesh_with_paths(
        vertices,
        vec![[0, 1], [2, 3]],
        vec![vec![[0, 1], [2, 3]]],
        vec![false],
    );

    let plan = exact_stitch_plan_by_cut_paths(&first, &second, 1e-9).unwrap();

    assert!(!plan.compatible);
    assert_eq!(plan.pairs.len(), 2);
    assert_eq!(plan.paths.len(), 2);
    assert_eq!(plan.paths[0].pair_indices, vec![0]);
    assert_eq!(plan.paths[1].pair_indices, vec![1]);
    assert_eq!(plan.unmatched_first_edges, vec![1]);
}

#[test]
fn exact_stitch_plan_uses_edge_fallback_when_mismatched_paths_hide_pairs() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
    ];
    let first = cut_mesh_with_paths(
        vertices.clone(),
        vec![[0, 1], [1, 2], [2, 3]],
        vec![vec![[0, 1], [1, 2]], vec![[2, 3]]],
        vec![false, false],
    );
    let second = cut_mesh_with_paths(
        vertices,
        vec![[0, 1], [1, 2], [2, 3]],
        vec![vec![[0, 1], [1, 2], [2, 3]], vec![]],
        vec![false, false],
    );

    let plan = exact_stitch_plan_from_cut_meshes(&first, &second, 1e-9);

    assert!(plan.compatible);
    assert_eq!(plan.pairs.len(), 3);
    assert!(plan.unmatched_first_edges.is_empty());
    assert!(plan.unmatched_second_edges.is_empty());
}

#[test]
fn exact_stitch_plan_by_cut_paths_ignores_non_result_cut_edges() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let first = cut_mesh_with_paths(
        vertices.clone(),
        vec![[0, 1]],
        vec![vec![[0, 1]]],
        vec![false],
    );
    let second = cut_mesh_with_paths(
        vertices,
        vec![[0, 1], [1, 2]],
        vec![vec![[1, 0]]],
        vec![false],
    );

    let plan = exact_stitch_plan_from_cut_meshes(&first, &second, 1e-9);

    assert!(plan.compatible);
    assert_eq!(plan.pairs.len(), 1);
    assert!(plan.unmatched_first_edges.is_empty());
    assert!(plan.unmatched_second_edges.is_empty());
}

#[test]
fn exact_stitch_plan_keeps_directed_closed_cut_path_edges() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let first = cut_mesh_with_paths(
        vertices.clone(),
        vec![[0, 1], [1, 2], [2, 3], [0, 3]],
        vec![vec![[0, 1], [1, 2], [2, 3], [3, 0]]],
        vec![true],
    );
    let second = cut_mesh_with_paths(
        vertices,
        vec![[0, 1], [1, 2], [2, 3], [0, 3]],
        vec![vec![[1, 0], [2, 1], [3, 2], [0, 3]]],
        vec![true],
    );

    let plan = exact_stitch_plan_from_cut_meshes(&first, &second, 1e-9);

    assert!(plan.compatible);
    assert_eq!(plan.paths.len(), 1);
    assert!(plan.paths[0].closed);
    assert_eq!(plan.paths[0].pair_indices, vec![0, 1, 2, 3]);
    assert_eq!(plan.pairs[3].first_edge, [3, 0]);
    assert!(plan.pairs.iter().all(|pair| pair.second_reversed));
}

#[test]
fn exact_stitch_plan_groups_open_edge_path_in_order() {
    let first_vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let second_vertices = vec![[2.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
    let plan = exact_stitch_plan_by_edges(
        &first_vertices,
        &[[1, 2], [0, 1]],
        &second_vertices,
        &[[0, 1], [1, 2]],
        1e-9,
    );

    assert!(plan.compatible);
    assert_eq!(plan.paths.len(), 1);
    assert_eq!(plan.paths[0].pair_indices, vec![1, 0]);
    assert!(!plan.paths[0].closed);
}

#[test]
fn exact_stitch_plan_groups_closed_contour_path() {
    let first_vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let second_vertices = vec![
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let plan = exact_stitch_plan_by_edges(
        &first_vertices,
        &[[0, 1], [1, 2], [2, 3], [3, 0]],
        &second_vertices,
        &[[1, 0], [0, 3], [3, 2], [2, 1]],
        1e-9,
    );

    assert!(plan.compatible);
    assert_eq!(plan.paths.len(), 1);
    assert_eq!(plan.paths[0].pair_indices, vec![0, 1, 2, 3]);
    assert!(plan.paths[0].closed);
}

#[test]
fn exact_stitch_vertex_map_reuses_reversed_second_endpoints() {
    let plan = ExactStitchPlan {
        pairs: vec![ExactStitchEdgePair {
            first_edge_index: 0,
            second_edge_index: 0,
            first_edge: [10, 11],
            second_edge: [2, 1],
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

    let map = exact_stitch_vertex_map(&plan, 3);

    assert!(map.conflicts.is_empty());
    assert_eq!(map.second_to_first[1], Some(10));
    assert_eq!(map.second_to_first[2], Some(11));
}
