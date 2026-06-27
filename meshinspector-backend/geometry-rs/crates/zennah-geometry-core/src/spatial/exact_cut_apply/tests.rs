use super::super::exact_cut::{
    ExactCutPathSegment, ExactCutPoint, ExactCutPreplan, ExactCutPrimitive,
};
use super::super::exact_one_mesh::{
    ExactOneMeshContour, ExactOneMeshIntersection, ExactOneMeshPrimitive,
};
use super::*;

#[test]
fn exact_cut_mesh_records_original_face_source_events_without_cuts() {
    let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let faces = vec![[0, 1, 2]];
    let contours = Vec::new();

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.source_face_for_faces, vec![0]);
    assert_eq!(
        result.cut_face_source_events,
        vec![ExactCutFaceSourceEvent {
            kind: ExactCutFaceSourceEventKind::Original,
            source_face: 0,
        }]
    );
}

#[test]
fn exact_cut_mesh_preserves_collapsed_contour_segments_as_metadata() {
    let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let faces = vec![[0, 1, 2]];
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.0, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.0, 0.0, 0.0],
            },
        ],
        closed: true,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert!(result.cut_edges.is_empty());
    assert_eq!(result.collapsed_cut_segment_paths, vec![vec![[3, 3]]]);
    assert_eq!(
        result.collapsed_cut_segment_path_source_faces,
        vec![vec![Some(0)]]
    );
}

#[test]
fn exact_cut_mesh_splits_two_boundary_chords_on_one_face() {
    let vertices = vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 4.0, 0.0]];
    let faces = vec![[0, 1, 2]];
    let contours = vec![
        ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [1.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([2, 0]),
                    coordinate: [0.0, 2.0, 0.0],
                },
            ],
            closed: false,
        },
        ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [2.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                    coordinate: [2.0, 2.0, 0.0],
                },
            ],
            closed: false,
        },
    ];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert!(result.skipped_source_faces.is_empty());
    assert_eq!(result.cut_edges.len(), 2);
    assert_eq!(result.cut_edge_paths.len(), 2);
    assert_eq!(result.cut_edge_paths[0].len(), 1);
    assert_eq!(result.cut_edge_paths[1].len(), 1);
    assert_eq!(result.cut_edge_path_closed, vec![false, false]);
    assert_eq!(result.faces.len(), 5);
    assert!(result
        .cut_face_source_events
        .iter()
        .all(|event| event.kind == ExactCutFaceSourceEventKind::Split));
}

#[test]
fn exact_cut_mesh_splits_shared_edge_contour_on_adjacent_faces() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, -1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3]];
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [0.5, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.5, 0.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert!(result.skipped_source_faces.is_empty());
    assert_eq!(result.cut_edges.len(), 1);
    assert_eq!(result.cut_edge_paths, vec![vec![[4, 5]]]);
    assert_eq!(result.cut_edge_path_closed, vec![false]);
    assert_eq!(result.faces.len(), 6);
    assert_eq!(
        result
            .source_face_for_faces
            .iter()
            .filter(|source| **source == 0)
            .count(),
        3
    );
    assert_eq!(
        result
            .source_face_for_faces
            .iter()
            .filter(|source| **source == 1)
            .count(),
        3
    );
    assert!(result
        .cut_face_source_events
        .iter()
        .all(|event| event.kind == ExactCutFaceSourceEventKind::Split));
}

#[test]
fn exact_cut_mesh_preserves_same_edge_cut_piece_in_output_faces() {
    let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let faces = vec![[0, 1, 2]];
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [0.5, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.5, 0.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert!(result.skipped_source_faces.is_empty());
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.cut_edge_paths, vec![vec![[3, 4]]]);
    assert_eq!(result.cut_edge_path_closed, vec![false]);
    assert!(result
        .faces
        .iter()
        .any(|face| triangle_has_edge(*face, [3, 4])));
}

#[test]
fn exact_cut_mesh_splits_single_segment_even_if_face_is_bad_candidate() {
    let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let faces = vec![[0, 1, 2]];
    let preplan = ExactCutPreplan {
        vertices_after_preplan: vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        cut_points: vec![
            ExactCutPoint {
                contour_index: 0,
                intersection_index: 0,
                original_primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                primitive: ExactCutPrimitive::Edge([0, 1]),
                coordinate: [1.0, 0.0, 0.0],
                vertex_index: 3,
                inserted_vertex: true,
            },
            ExactCutPoint {
                contour_index: 0,
                intersection_index: 1,
                original_primitive: ExactOneMeshPrimitive::Edge([2, 0]),
                primitive: ExactCutPrimitive::Edge([2, 0]),
                coordinate: [0.0, 1.0, 0.0],
                vertex_index: 4,
                inserted_vertex: true,
            },
        ],
        contour_points: vec![vec![0, 1]],
        contour_closed: vec![false],
        path_segments: vec![ExactCutPathSegment {
            contour_index: 0,
            from_point: 0,
            to_point: 1,
            source_faces: vec![0],
        }],
        collapsed_segments: Vec::new(),
        edge_splits: Vec::new(),
        removed_face_candidates: vec![0],
        bad_face_candidates: vec![0],
    };

    let result = exact_cut_mesh_from_preplan(&vertices, &faces, &preplan, 1e-9).unwrap();

    assert!(result.skipped_source_faces.is_empty());
    assert_eq!(result.cut_edges, vec![[3, 4]]);
}

#[test]
fn exact_cut_mesh_does_not_mark_surviving_open_piece_as_closed_loop() {
    let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let faces = vec![[0, 1, 2]];
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [0.5, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
        ],
        closed: true,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.cut_edge_paths.len(), 1);
    assert_eq!(result.cut_edge_paths[0].len(), 1);
    assert_eq!(result.cut_edge_path_closed, vec![false]);
}

fn triangle_has_edge(face: [i64; 3], edge: [usize; 2]) -> bool {
    (0..3).any(|index| {
        let candidate = [face[index] as usize, face[(index + 1) % 3] as usize];
        candidate == edge || candidate == [edge[1], edge[0]]
    })
}
