use super::*;

#[test]
fn optimal_split_steps_match_meshlib_max_polygon_subdivisions_sampling() {
    assert_eq!(optimal_split_steps(0, 6, 30, 20), vec![1, 2, 3, 4, 5]);
    assert_eq!(
        optimal_split_steps(0, 29, 30, 8),
        vec![1, 2, 5, 11, 17, 23, 27, 28]
    );
    assert_eq!(
        optimal_split_steps(27, 26, 30, 8),
        vec![28, 29, 2, 8, 14, 20, 24, 25]
    );
}

#[test]
fn multiple_edges_resolve_mode_none_disables_existing_edge_avoidance() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.1, 0.0],
        [1.5, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.5, 1.0],
    ];
    let boundary_loop = vec![0, 1, 2, 3, 4];
    let existing_faces = vec![[1, 3, 5]];

    let none_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &existing_faces,
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::None,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );
    let simple_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &existing_faces,
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );

    assert!(triangulation_uses_edge(&none_patch, 1, 3));
    assert!(!triangulation_uses_edge(&simple_patch, 1, 3));
}

#[test]
fn multiple_edges_resolve_mode_strong_keeps_meshlib_duplicate_edge_guard() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.1, 0.0],
        [1.5, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.5, 1.0],
    ];
    let boundary_loop = vec![0, 1, 2, 3, 4];
    let existing_faces = vec![[1, 3, 5]];

    let strong_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &existing_faces,
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Strong,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );

    assert_eq!(strong_patch.len(), 3);
    assert!(!triangulation_uses_edge(&strong_patch, 1, 3));
}

#[test]
fn make_degenerate_band_duplicates_boundary_and_adds_two_band_faces_per_edge() {
    let vertices = open_cube_vertices();
    let faces = open_cube_faces();

    let repaired = service_fill_holes_with_fill_params(
        &vertices,
        &faces,
        None,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMultipleEdgesResolveMode::Simple,
        true,
        false,
        true,
        FillHoleMetricMode::Circumscribed,
    )
    .unwrap();

    assert_eq!(repaired.report.added_vertices, 4);
    assert_eq!(repaired.report.added_faces, 10);
    assert_eq!(repaired.vertices.len(), vertices.len() + 4);
    assert_eq!(repaired.faces.len(), faces.len() + 10);
}

#[test]
fn service_fill_holes_reports_meshlib_out_new_faces_indices() {
    let vertices = open_cube_vertices();
    let faces = open_cube_faces();

    let repaired = service_fill_holes_with_fill_params(
        &vertices,
        &faces,
        None,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMultipleEdgesResolveMode::Simple,
        false,
        false,
        true,
        FillHoleMetricMode::Circumscribed,
    )
    .unwrap();

    assert_eq!(repaired.report.added_faces, 2);
    assert_eq!(
        repaired.report.new_face_indices,
        vec![faces.len(), faces.len() + 1]
    );
}

#[test]
fn stop_before_bad_triangulation_skips_bad_patch_like_meshlib() {
    let vertices = sliver_open_box_vertices();
    let faces = sliver_open_box_faces();

    let repaired = service_fill_holes_with_fill_params(
        &vertices,
        &faces,
        None,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMultipleEdgesResolveMode::Simple,
        false,
        true,
        true,
        FillHoleMetricMode::Circumscribed,
    )
    .unwrap();

    assert_eq!(repaired.report.input_holes, 1);
    assert_eq!(repaired.report.filled_holes, 0);
    assert_eq!(repaired.report.skipped_holes, 1);
    assert_eq!(repaired.report.added_vertices, 0);
    assert_eq!(repaired.report.added_faces, 0);
    assert_eq!(repaired.vertices.len(), vertices.len());
    assert_eq!(repaired.faces.len(), faces.len());
}

#[test]
fn min_area_metric_selects_meshlib_minimum_double_area_diagonal() {
    let vertices = metric_choice_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];

    let default_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );
    let min_area_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::MinArea,
        true,
    );

    assert!(triangulation_uses_edge(&default_patch, 0, 2));
    assert!(triangulation_uses_edge(&min_area_patch, 1, 3));
}

#[test]
fn edge_length_metric_selects_meshlib_shortest_generated_edge() {
    let vertices = edge_length_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];

    let default_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );
    let edge_length_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::EdgeLength,
        true,
    );

    assert!(triangulation_uses_edge(&default_patch, 1, 3));
    assert!(triangulation_uses_edge(&edge_length_patch, 0, 2));
}

#[test]
fn edge_length_metric_smooth_bd_counts_meshlib_boundary_edges() {
    let vertices = edge_length_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];
    let existing_faces = vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let sharp_weight = triangulate_hole_loop_weight_with_fill_params_for_tests(
        &vertices,
        &existing_faces,
        &boundary_loop,
        FillHoleMetricMode::EdgeLength,
        false,
    );
    let smooth_weight = triangulate_hole_loop_weight_with_fill_params_for_tests(
        &vertices,
        &existing_faces,
        &boundary_loop,
        FillHoleMetricMode::EdgeLength,
        true,
    );

    assert!(smooth_weight > sharp_weight);
}

#[test]
fn universal_metric_selects_meshlib_smoothest_dihedral_patch() {
    let vertices = universal_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];
    let existing_faces = vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let circumscribed_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &existing_faces,
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );
    let universal_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &existing_faces,
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Universal,
        true,
    );

    assert!(triangulation_uses_edge(&circumscribed_patch, 0, 2));
    assert!(triangulation_uses_edge(&universal_patch, 1, 3));
}

#[test]
fn max_dihedral_angle_metric_uses_meshlib_max_combiner() {
    let vertices = max_dihedral_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3, 4];

    let max_dihedral_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::MaxDihedralAngle,
        false,
    );

    assert!(triangulation_uses_edge(&max_dihedral_patch, 0, 2));
    assert!(triangulation_uses_edge(&max_dihedral_patch, 0, 3));
    assert!(!triangulation_uses_edge(&max_dihedral_patch, 2, 4));
}

#[test]
fn parallel_plane_metric_minimizes_meshlib_normal_projection() {
    let vertices = parallel_plane_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3, 4];

    let circumscribed_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        false,
    );
    let parallel_plane_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::ParallelPlane,
        false,
    );

    assert!(triangulation_uses_edge(&circumscribed_patch, 0, 2));
    assert!(triangulation_uses_edge(&parallel_plane_patch, 1, 3));
    assert!(!triangulation_uses_edge(&parallel_plane_patch, 0, 2));
}

#[test]
fn complex_fill_metric_uses_meshlib_aspect_area_and_edge_penalty() {
    let vertices = complex_fill_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];

    let circumscribed_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        false,
    );
    let complex_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::ComplexFill,
        false,
    );

    assert!(triangulation_uses_edge(&circumscribed_patch, 0, 2));
    assert!(triangulation_uses_edge(&complex_patch, 1, 3));
    assert!(!triangulation_uses_edge(&complex_patch, 0, 2));
}

#[test]
fn min_tri_angle_metric_selects_meshlib_largest_minimum_angle_diagonal() {
    let vertices = min_tri_angle_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];

    let default_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );
    let min_tri_angle_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::MinTriAngle,
        true,
    );

    assert!(triangulation_uses_edge(&default_patch, 0, 2));
    assert!(triangulation_uses_edge(&min_tri_angle_patch, 1, 3));
}

#[test]
fn plane_metric_rejects_triangles_against_meshlib_left_ring_normal() {
    let vertices = plane_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];

    let default_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Circumscribed,
        true,
    );
    let plane_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Plane,
        true,
    );

    assert!(triangulation_uses_edge(&default_patch, 0, 2));
    assert!(triangulation_uses_edge(&plane_patch, 1, 3));
}

#[test]
fn plane_normalized_metric_multiplies_meshlib_plane_score_by_aspect_ratio() {
    let vertices = plane_normalized_metric_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];

    let plane_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::Plane,
        true,
    );
    let plane_normalized_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::PlaneNormalized,
        true,
    );

    assert!(triangulation_uses_edge(&plane_patch, 0, 2));
    assert!(triangulation_uses_edge(&plane_normalized_patch, 1, 3));
}

#[test]
fn meshlib_stitch_fill_metric_modes_are_selectable_rust_modes() {
    let vertices = metric_choice_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];
    for mode in [
        FillHoleMetricMode::ComplexStitch,
        FillHoleMetricMode::EdgeLengthStitch,
        FillHoleMetricMode::VerticalStitch,
        FillHoleMetricMode::VerticalStitchEdgeBased,
    ] {
        let patch = triangulate_hole_loop_with_multiple_edges_resolve_mode(
            &vertices,
            &[],
            &boundary_loop,
            FillHoleMultipleEdgesResolveMode::Simple,
            DEFAULT_MAX_POLYGON_SUBDIVISIONS,
            mode,
            true,
        );
        assert_eq!(patch.len(), 2);
    }
}

#[test]
fn vertical_stitch_metric_uses_meshlib_caller_supplied_up_dir() {
    let vertices = vertical_stitch_up_dir_boundary_vertices();
    let boundary_loop = vec![0, 1, 2, 3];

    let default_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode_and_metric_up_dir(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::VerticalStitch,
        true,
        None,
    );
    let x_axis_patch = triangulate_hole_loop_with_multiple_edges_resolve_mode_and_metric_up_dir(
        &vertices,
        &[],
        &boundary_loop,
        FillHoleMultipleEdgesResolveMode::Simple,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMetricMode::VerticalStitch,
        true,
        Some([1.0, 0.0, 0.0]),
    );

    assert!(triangulation_uses_edge(&default_patch, 1, 3));
    assert!(triangulation_uses_edge(&x_axis_patch, 0, 2));
}

fn open_cube_vertices() -> Vec<[f64; 3]> {
    vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ]
}

fn metric_choice_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [2.919047, 0.461774, 0.673408],
        [0.324723, 2.717115, -1.489469],
        [0.468535, 2.068132, -1.758068],
    ]
}

fn edge_length_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [3.424361, 0.515909, -0.317714],
        [1.257992, 2.191716, -0.380263],
        [0.567597, 1.422257, -0.093612],
    ]
}

fn universal_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [1.452682, 0.165345, -1.239369],
        [4.198573, 1.677396, 0.146909],
        [4.623103, 0.400114, 1.902811],
        [0.207095, 3.185662, 0.391847],
        [-1.122086, 3.021953, -4.709435],
    ]
}

fn max_dihedral_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [1.981194, -0.367531, -0.728251],
        [0.532394, 2.162295, -2.433038],
        [-1.572133, 2.289641, -1.347618],
        [-2.881760, -1.364960, -0.166524],
        [0.960472, -1.567687, 0.778782],
    ]
}

fn parallel_plane_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [1.609692, -0.087843, 2.770195],
        [0.203712, 2.505018, 3.249312],
        [-1.320781, 1.566465, 4.577460],
        [-1.064627, -0.429701, -4.678661],
        [1.896156, -3.739819, -4.482766],
    ]
}

fn complex_fill_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [1.830766, 0.505110, -2.595065],
        [-0.265999, 0.890404, -0.237632],
        [-3.207773, -0.167293, -3.545285],
        [0.317084, -2.684187, 3.511188],
    ]
}

fn min_tri_angle_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [2.055912, 0.314945, 0.665642],
        [0.827621, 0.340184, -0.500982],
        [-0.451904, 3.298288, 0.762371],
    ]
}

fn plane_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [2.737994, -1.424968, -1.349824],
        [1.281238, 3.077649, 1.060197],
        [1.176539, 0.804286, -0.468469],
    ]
}

fn plane_normalized_metric_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [4.905726, -0.087403, 0.875388],
        [3.806988, 1.467058, -1.687848],
        [0.205486, 1.048638, -0.471093],
    ]
}

fn vertical_stitch_up_dir_boundary_vertices() -> Vec<[f64; 3]> {
    vec![
        [2.346601, -3.341016, 0.902265],
        [-0.108446, 1.041179, 2.760621],
        [-2.236316, 2.356664, -1.339711],
        [-2.055715, 1.851914, -3.062926],
    ]
}

fn sliver_open_box_vertices() -> Vec<[f64; 3]> {
    let length = 1.0e12;
    let width = 1.0e-8;
    vec![
        [0.0, 0.0, 0.0],
        [length, 0.0, 0.0],
        [length, width, 0.0],
        [0.0, width, 0.0],
        [0.0, 0.0, 1.0],
        [length, 0.0, 1.0],
        [length, width, 1.0],
        [0.0, width, 1.0],
    ]
}

fn open_cube_faces() -> Vec<[i64; 3]> {
    vec![
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
    ]
}

fn sliver_open_box_faces() -> Vec<[i64; 3]> {
    vec![
        [0, 2, 1],
        [0, 3, 2],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ]
}

fn triangulation_uses_edge(faces: &[[i64; 3]], a: i64, b: i64) -> bool {
    faces.iter().any(|face| {
        [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
            .into_iter()
            .any(|(u, v)| (u == a && v == b) || (u == b && v == a))
    })
}
