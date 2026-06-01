use super::*;

fn tetrahedron() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    (
        vec![
            [0.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 3.0],
        ],
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
    )
}

#[test]
fn exact_classify_components_selects_inside_faces() {
    let vertices = vec![
        [0.25, 0.25, 0.25],
        [0.75, 0.25, 0.25],
        [0.25, 0.75, 0.25],
        [5.0, 5.0, 5.0],
        [6.0, 5.0, 5.0],
        [5.0, 6.0, 5.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];
    let (other_vertices, other_faces) = tetrahedron();

    let result = exact_classify_components(
        &vertices,
        &faces,
        &other_vertices,
        &other_faces,
        &[],
        true,
        1e-9,
    )
    .unwrap();

    assert_eq!(result.components.len(), 2);
    assert_eq!(result.selected_faces, vec![0]);
    assert!(result.components[0].inside_other);
    assert!(!result.components[1].inside_other);
}

#[test]
fn exact_classify_components_respects_cut_edges() {
    let vertices = vec![
        [0.25, 0.25, 0.25],
        [1.25, 0.25, 0.25],
        [1.25, 1.25, 0.25],
        [0.25, 1.25, 0.25],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let (other_vertices, other_faces) = tetrahedron();

    let joined = exact_classify_components(
        &vertices,
        &faces,
        &other_vertices,
        &other_faces,
        &[],
        true,
        1e-9,
    )
    .unwrap();
    let split = exact_classify_components(
        &vertices,
        &faces,
        &other_vertices,
        &other_faces,
        &[[0, 2]],
        true,
        1e-9,
    )
    .unwrap();

    assert_eq!(joined.components.len(), 1);
    assert_eq!(split.components.len(), 2);
    assert_eq!(split.selected_faces, vec![0, 1]);
}

#[test]
fn exact_classify_components_with_cut_paths_selects_meshlib_right_part_for_first_operand() {
    let vertices = vec![
        [0.25, 0.25, 0.25],
        [1.25, 0.25, 0.25],
        [1.25, 1.25, 0.25],
        [0.25, 1.25, 0.25],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let (other_vertices, other_faces) = tetrahedron();

    let result = exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
        vertices: &vertices,
        faces_i64: &faces,
        other_vertices: &other_vertices,
        other_faces_i64: &other_faces,
        cut_edges: &[[0, 2]],
        cut_edge_paths: &[vec![[0, 2]]],
        need_inside: false,
        origin_is_first: true,
        epsilon: 1e-9,
    })
    .unwrap();

    assert_eq!(result.components.len(), 2);
    assert_eq!(result.selected_faces, vec![0]);
    assert!(result.used_cut_path_sides);
    assert!(result.cut_paths_consistent);
    assert_eq!(result.cut_path_overlap_components, 0);
}

#[test]
fn exact_classify_components_with_cut_paths_selects_meshlib_left_part_for_second_operand() {
    let vertices = vec![
        [0.25, 0.25, 0.25],
        [1.25, 0.25, 0.25],
        [1.25, 1.25, 0.25],
        [0.25, 1.25, 0.25],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let (other_vertices, other_faces) = tetrahedron();

    let result = exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
        vertices: &vertices,
        faces_i64: &faces,
        other_vertices: &other_vertices,
        other_faces_i64: &other_faces,
        cut_edges: &[[0, 2]],
        cut_edge_paths: &[vec![[0, 2]]],
        need_inside: false,
        origin_is_first: false,
        epsilon: 1e-9,
    })
    .unwrap();

    assert_eq!(result.components.len(), 2);
    assert_eq!(result.selected_faces, vec![1]);
    assert!(result.used_cut_path_sides);
    assert!(result.cut_paths_consistent);
    assert_eq!(result.cut_path_overlap_components, 0);
}

#[test]
fn exact_classify_components_with_cut_paths_uses_result_cut_edges_as_barriers() {
    let vertices = vec![
        [0.25, 0.25, 0.25],
        [1.25, 0.25, 0.25],
        [1.25, 1.25, 0.25],
        [0.25, 1.25, 0.25],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let (other_vertices, other_faces) = tetrahedron();

    let result = exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
        vertices: &vertices,
        faces_i64: &faces,
        other_vertices: &other_vertices,
        other_faces_i64: &other_faces,
        cut_edges: &[[0, 2]],
        cut_edge_paths: &[vec![[0, 1]]],
        need_inside: true,
        origin_is_first: true,
        epsilon: 1e-9,
    })
    .unwrap();

    assert_eq!(result.components.len(), 1);
    assert!(result.cut_paths_consistent);
    assert_eq!(result.cut_path_overlap_components, 0);
}

#[test]
fn orientation_normalized_cut_path_sides_handle_opposite_loop_directions() {
    let sides = vec![
        PathCutSideComponents {
            left_components: BTreeSet::from([1]),
            right_components: BTreeSet::from([0]),
        },
        PathCutSideComponents {
            left_components: BTreeSet::from([0]),
            right_components: BTreeSet::from([2]),
        },
    ];

    let fixed = merge_path_sides(&sides);
    let normalized = orientation_normalized_path_sides(&sides).unwrap();

    assert_eq!(fixed.overlap_count(), 1);
    assert_eq!(normalized.left_components, BTreeSet::from([0]));
    assert_eq!(normalized.right_components, BTreeSet::from([1, 2]));
    assert!(normalized.consistent());
}

#[test]
fn cut_path_sides_collect_all_directed_incident_faces() {
    let faces = vec![[0, 1, 2], [0, 1, 3], [1, 0, 4]];
    let directed_faces = directed_face_map(&faces);
    let face_to_component = vec![Some(0), Some(1), Some(2)];

    let sides = path_cut_side_components(&[vec![[0, 1]]], &directed_faces, &face_to_component);

    assert_eq!(sides[0].left_components, BTreeSet::from([0, 1]));
    assert_eq!(sides[0].right_components, BTreeSet::from([2]));
}

#[test]
fn exact_classify_components_with_cut_paths_reports_meshlib_not_dividable_case() {
    let vertices = vec![
        [0.25, 0.25, 0.25],
        [1.25, 0.25, 0.25],
        [1.25, 1.25, 0.25],
        [0.25, 1.25, 0.25],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let (other_vertices, other_faces) = tetrahedron();

    let result = exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
        vertices: &vertices,
        faces_i64: &faces,
        other_vertices: &other_vertices,
        other_faces_i64: &other_faces,
        cut_edges: &[[0, 2]],
        cut_edge_paths: &[vec![[0, 2], [2, 0]]],
        need_inside: false,
        origin_is_first: true,
        epsilon: 1e-9,
    })
    .unwrap();

    assert_eq!(result.components.len(), 2);
    assert!(!result.used_cut_path_sides);
    assert!(!result.cut_paths_consistent);
    assert_eq!(result.cut_left_components, 1);
    assert_eq!(result.cut_right_components, 1);
    assert_eq!(result.cut_path_overlap_components, 2);
}
