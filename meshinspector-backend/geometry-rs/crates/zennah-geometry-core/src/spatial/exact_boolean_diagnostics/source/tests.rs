use super::*;
use crate::spatial::exact_cut_apply::ExactCutFaceSourceEvent;
use crate::spatial::exact_stitch::{ExactStitchEdgePair, ExactStitchPath};

#[test]
fn stitch_result_cut_source_inventory_follows_stitch_pair_order() {
    let first = cut_mesh(
        vec![[0, 1], [1, 2], [2, 3]],
        vec![vec![[0, 1], [1, 2], [2, 3]]],
        vec![vec![Some(10), Some(10), Some(20)]],
    );
    let second = cut_mesh(
        vec![[0, 1], [1, 2], [2, 3]],
        vec![vec![[0, 1], [1, 2], [2, 3]]],
        vec![vec![Some(30), Some(40), Some(40)]],
    );
    let stitch_plan = ExactStitchPlan {
        pairs: vec![
            stitch_pair(0, [0, 1], 0, [0, 1]),
            stitch_pair(1, [1, 2], 1, [1, 2]),
            stitch_pair(2, [2, 3], 2, [2, 3]),
        ],
        paths: vec![
            ExactStitchPath {
                pair_indices: vec![0, 1],
                closed: false,
            },
            ExactStitchPath {
                pair_indices: vec![2],
                closed: false,
            },
        ],
        unmatched_first_edges: Vec::new(),
        unmatched_second_edges: Vec::new(),
        compatible: true,
    };

    let inventory = stitch_result_cut_source_inventory(&first, &second, &stitch_plan);

    assert_eq!(inventory.path_lengths, vec![2, 1]);
    assert_eq!(
        inventory.first_path_source_faces,
        vec![vec![10, 10], vec![20]]
    );
    assert_eq!(
        inventory.second_path_source_faces,
        vec![vec![30, 40], vec![40]]
    );
    assert_eq!(
        inventory.first_path_source_face_runs,
        vec![vec![[10, 2]], vec![[20, 1]]]
    );
    assert_eq!(
        inventory.second_path_source_face_runs,
        vec![vec![[30, 1], [40, 1]], vec![[40, 1]]]
    );
    assert_eq!(inventory.first_source_faces, vec![10, 10, 20]);
    assert_eq!(inventory.second_source_faces, vec![30, 40, 40]);
    assert_eq!(inventory.first_source_face_runs, vec![[10, 2], [20, 1]]);
    assert_eq!(inventory.second_source_face_runs, vec![[30, 1], [40, 2]]);
    assert_eq!(inventory.missing_source_records, [0, 0]);
    assert_eq!(inventory.edge_grouped_path_lengths, vec![3]);
    assert_eq!(inventory.edge_grouped_closed_paths, 0);
    assert_eq!(
        inventory.first_edge_grouped_path_source_faces,
        vec![vec![10, 10, 20]]
    );
    assert_eq!(
        inventory.second_edge_grouped_path_source_faces,
        vec![vec![30, 40, 40]]
    );
    assert_eq!(
        inventory.first_edge_grouped_path_source_face_runs,
        vec![vec![[10, 2], [20, 1]]]
    );
    assert_eq!(
        inventory.second_edge_grouped_path_source_face_runs,
        vec![vec![[30, 1], [40, 2]]]
    );
    assert_eq!(inventory.first_edge_grouped_source_faces, vec![10, 10, 20]);
    assert_eq!(inventory.second_edge_grouped_source_faces, vec![30, 40, 40]);
    assert_eq!(
        inventory.first_edge_grouped_source_face_runs,
        vec![[10, 2], [20, 1]]
    );
    assert_eq!(
        inventory.second_edge_grouped_source_face_runs,
        vec![[30, 1], [40, 2]]
    );
    assert_eq!(inventory.edge_grouped_missing_source_records, [0, 0]);
}

#[test]
fn cut_path_inventory_tracks_meshlib_removed_face_owner_candidates() {
    let cut = ExactCutMeshResult {
        vertices: Vec::new(),
        faces: vec![[0, 1, 4], [2, 1, 4], [2, 3, 4], [0, 3, 4]],
        cut_edges: Vec::new(),
        cut_edge_paths: vec![vec![[0, 1], [1, 2], [2, 3], [3, 0]]],
        cut_edge_path_closed: vec![true],
        cut_edge_path_source_faces: vec![vec![Some(10), Some(20), Some(30), Some(40)]],
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![10, 92, 30, 93],
        cut_face_source_events: Vec::<ExactCutFaceSourceEvent>::new(),
        skipped_source_faces: Vec::new(),
    };

    let inventory = cut_path_inventory(&cut);

    assert_eq!(
        inventory.closed_path_meshlib_removed_face_owner_candidates,
        vec![vec![92, 30, 30, 40]]
    );
    assert_eq!(
        inventory.closed_path_meshlib_removed_face_owner_candidate_runs,
        vec![vec![[92, 1], [30, 2], [40, 1]]]
    );
}

fn cut_mesh(
    cut_edges: Vec<[usize; 2]>,
    cut_edge_paths: Vec<Vec<[usize; 2]>>,
    cut_edge_path_source_faces: Vec<Vec<Option<usize>>>,
) -> ExactCutMeshResult {
    ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
        ],
        faces: Vec::new(),
        cut_edges,
        cut_edge_paths,
        cut_edge_path_closed: Vec::new(),
        cut_edge_path_source_faces,
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: Vec::new(),
        cut_face_source_events: Vec::<ExactCutFaceSourceEvent>::new(),
        skipped_source_faces: Vec::new(),
    }
}

fn stitch_pair(
    first_edge_index: usize,
    first_edge: [usize; 2],
    second_edge_index: usize,
    second_edge: [usize; 2],
) -> ExactStitchEdgePair {
    ExactStitchEdgePair {
        first_edge_index,
        second_edge_index,
        first_edge,
        second_edge,
        second_reversed: false,
    }
}
