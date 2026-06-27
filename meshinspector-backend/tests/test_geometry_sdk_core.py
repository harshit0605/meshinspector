from __future__ import annotations

import base64
import struct

import numpy as np
import pytest
import trimesh

from geometry_sdk import (
    MeshDocument,
    default_sdk,
    extract_selected_faces_as_mesh as sdk_extract_selected_faces_as_mesh,
    expand_face_selection_to_components as sdk_expand_face_selection_to_components,
    graph_cut_select_region_auto_not_region as sdk_graph_cut_select_region_auto_not_region,
    select_camera_facing_faces as sdk_select_camera_facing_faces,
    select_face_by_ray as sdk_select_face_by_ray,
    select_faces_by_screen_brush as sdk_select_faces_by_screen_brush,
    select_faces_by_screen_polygon as sdk_select_faces_by_screen_polygon,
    select_faces_by_screen_rect as sdk_select_faces_by_screen_rect,
    select_inside_part_faces as sdk_select_inside_part_faces,
    select_not_smooth_faces as sdk_select_not_smooth_faces,
)
from geometry_sdk.accelerators import _rust_common
from geometry_sdk.accelerators import rust
from geometry_sdk.core import mesh as mesh_core
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.health import boundary_loops, compute_mesh_health
from geometry_sdk.core.mesh import (
    apply_meshlib_selection_modifier,
    boundary_edges,
    connected_face_components,
    extract_selected_faces_as_mesh,
    expand_face_selection_to_components,
    feature_object_descriptors,
    feature_pair_measurements,
    mesh_closest_surface_path_targets,
    mesh_geodesic_distance_field,
    mesh_geodesic_extreme_edges,
    mesh_geodesic_iso_region,
    mesh_surface_distance_seed_vertices,
    graph_cut_select_region,
    graph_cut_select_region_auto_not_region,
    mesh_geodesic_edge_point_path,
    mesh_cut_measure_edge_path_topology_cut,
    mesh_cut_measure_contours,
    mesh_geodesic_path,
    mesh_geodesic_polyline_path,
    mesh_geodesic_quadrangle_path,
    mesh_planar_triangle_strip_path,
    mesh_surface_edge_point_path,
    mesh_steepest_descent_path,
    mesh_triangle_strip_unfolded_path,
    mesh_from_ply,
    refine_feature_primitives,
    select_boundary_edges,
    select_boundary_faces,
    select_camera_facing_faces,
    select_crease_edges,
    select_face_by_ray,
    select_faces_by_area,
    select_faces_by_screen_brush,
    select_faces_by_screen_polygon,
    select_faces_by_screen_rect,
    select_inside_part_faces,
    select_largest_component_faces,
    select_not_smooth_faces,
    select_outer_layer_faces,
    select_overlapping_faces,
    select_overhang_faces,
    vertex_normals,
)
from geometry_sdk.testing.fixtures import closed_cube_with_flipped_top_triangle, crossing_triangles, cube, open_cube


OPAQUE_WHITE_PNG = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGP4DwQACfsD/fteaysAAAAASUVORK5CYII="
)


def test_cube_stats_are_deterministic() -> None:
    mesh = cube(size=2.0)
    stats = compute_mesh_stats(mesh)

    assert stats.vertex_count == 8
    assert stats.face_count == 12
    assert stats.boundary_edge_count == 0
    assert stats.connected_components == 1
    assert stats.bbox_size == (2.0, 2.0, 2.0)
    assert np.isclose(stats.surface_area_mm2, 24.0)
    assert np.isclose(stats.volume_mm3, 8.0)


def test_open_cube_reports_boundary_edges() -> None:
    mesh = open_cube(size=2.0)
    stats = compute_mesh_stats(mesh)
    health = compute_mesh_health(mesh)

    assert stats.boundary_edge_count == 4
    assert len(boundary_edges(mesh)) == 4
    assert len(boundary_loops(mesh)) == 1
    assert not health.is_closed
    assert health.holes_count == 1
    assert len(connected_face_components(mesh)) == 1


def test_expand_face_selection_to_components_matches_meshlib_component_selection() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [4.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [4, 5, 6]], dtype=np.int64),
        metadata={},
    )

    assert expand_face_selection_to_components(mesh, [0]) == [0, 1]
    assert sdk_expand_face_selection_to_components(mesh, [0]) == [0, 1]
    assert expand_face_selection_to_components(mesh, [2]) == [2]
    assert default_sdk.expand_face_selection_to_components(mesh, [0]) == [0, 1]


def test_extract_selected_faces_as_mesh_matches_meshlib_clone_region_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [1, 4, 3]], dtype=np.int64),
        metadata={"name": "source"},
    )

    result = extract_selected_faces_as_mesh(mesh, [2, 0, 2])

    np.testing.assert_allclose(
        result.vertices,
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
    )
    np.testing.assert_array_equal(result.faces, np.asarray([[0, 1, 2], [1, 3, 4]], dtype=np.int64))
    assert result.metadata["source_face_indices"] == [0, 2]
    assert result.metadata["source_vertex_indices"] == [0, 1, 2, 4, 3]
    assert result.metadata["meshlib_operation"] == "Mesh::cloneRegion"
    assert sdk_extract_selected_faces_as_mesh(mesh, [0]).face_count == 1
    assert default_sdk.extract_selected_faces_as_mesh(mesh, [0]).vertex_count == 3


def test_extract_selected_faces_as_mesh_remaps_meshlib_clone_region_visual_attributes() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [1, 4, 3]], dtype=np.int64),
        metadata={
            "name": "textured-source",
            "texture_files": ["matte.png", "gloss.png"],
            "texture_per_face": [0, 1, 0],
            "vertex_uvs": [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [2.0, 0.0]],
            "vertex_colors": [
                [10, 20, 30, 255],
                [40, 50, 60, 255],
                [70, 80, 90, 255],
                [100, 110, 120, 255],
                [130, 140, 150, 255],
            ],
            "face_colors": [[1, 2, 3, 255], [4, 5, 6, 255], [7, 8, 9, 255]],
        },
    )

    result = extract_selected_faces_as_mesh(mesh, [2, 0, 2])

    assert result.metadata["source_face_indices"] == [0, 2]
    assert result.metadata["source_vertex_indices"] == [0, 1, 2, 4, 3]
    assert result.metadata["texture_files"] == ["matte.png", "gloss.png"]
    assert result.metadata["texture_per_face"] == [0, 0]
    np.testing.assert_allclose(
        result.metadata["vertex_uvs"],
        [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [2.0, 0.0], [1.0, 1.0]],
    )
    assert result.metadata["vertex_colors"] == [
        [10, 20, 30, 255],
        [40, 50, 60, 255],
        [70, 80, 90, 255],
        [130, 140, 150, 255],
        [100, 110, 120, 255],
    ]
    assert result.metadata["face_colors"] == [[1, 2, 3, 255], [7, 8, 9, 255]]


def test_apply_meshlib_selection_modifier_matches_primary_ctrl_toggle_contract() -> None:
    assert apply_meshlib_selection_modifier([0, 2, 2], [2, 3, 3], "toggle", item_count=5) == [0, 3]
    assert apply_meshlib_selection_modifier([0, 2], [2, 3], "replace", item_count=5) == [2, 3]
    assert apply_meshlib_selection_modifier([0, 2], [2, 3], "add", item_count=5) == [0, 2, 3]
    assert apply_meshlib_selection_modifier([0, 2, 4], [2, 3], "subtract", item_count=5) == [0, 4]


def test_mesh_geodesic_path_matches_meshlib_edge_shortest_path_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    path = mesh_geodesic_path(mesh, start_vertex=0, end_vertex=3)

    assert path["vertex_indices"][0] == 0
    assert path["vertex_indices"][-1] == 3
    assert len(path["vertex_indices"]) == 3
    assert path["line_segments"] == 2
    assert path["length_mm"] == pytest.approx(2.0)
    np.testing.assert_allclose(path["edge_lengths"], [1.0, 1.0])
    assert path["meshlib_reference"] == "MR::buildShortestPath"
    assert default_sdk.mesh_geodesic_path(mesh, start_vertex=0, end_vertex=3)["length_mm"] == pytest.approx(2.0)


def test_mesh_cut_measure_contours_matches_meshlib_onemesh_contour_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    contours = mesh_cut_measure_contours(mesh, control_vertices=[0, 1, 3], close_path=True)

    assert contours["closed_path"] is True
    assert contours["contour_count"] == 1
    assert contours["cut_result_count"] == 1
    assert contours["pivot_indices"] == [0, 1, 2, 4]
    assert contours["path_vertex_indices"] == [0, 1, 3, 2, 0]
    assert contours["contours"] == [
        {
            "closed": True,
            "intersections": [
                {"primitive_type": "VertId", "primitive_id": 0, "coordinate": (0.0, 0.0, 0.0)},
                {"primitive_type": "VertId", "primitive_id": 1, "coordinate": (1.0, 0.0, 0.0)},
                {"primitive_type": "VertId", "primitive_id": 3, "coordinate": (1.0, 1.0, 0.0)},
                {"primitive_type": "VertId", "primitive_id": 2, "coordinate": (0.0, 1.0, 0.0)},
                {"primitive_type": "VertId", "primitive_id": 0, "coordinate": (0.0, 0.0, 0.0)},
            ],
        }
    ]
    assert contours["result_cut_vertex_indices"] == [[0, 1, 3, 2, 0]]
    assert contours["bad_face_indices"] == []
    assert contours["meshlib_reference"] == "MR::convertSurfacePathsToMeshContours / MR::cutMesh"
    assert default_sdk.mesh_cut_measure_contours(mesh, control_vertices=[0, 1, 3])["closed_path"] is False


def test_mesh_cut_measure_edge_path_topology_cut_splits_shared_edge_seam() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    result = mesh_cut_measure_edge_path_topology_cut(mesh, control_vertices=[1, 2])
    output_mesh = result["mesh"]

    assert isinstance(output_mesh, MeshDocument)
    assert output_mesh.vertex_count == 6
    assert output_mesh.face_count == 2
    np.testing.assert_array_equal(output_mesh.faces, np.asarray([[0, 1, 2], [5, 4, 3]], dtype=np.int64))
    assert result["source_path_vertex_indices"] == [1, 2]
    assert result["result_cut_vertex_indices"] == [[4, 5]]
    assert result["duplicate_vertex_map"] == [[1, 4], [2, 5]]
    assert result["cut_edge_pairs"] == [[1, 2]]
    assert result["result_cut_edge_pairs"] == [[4, 5]]
    assert result["bad_face_indices"] == []
    assert result["length_mm"] == pytest.approx(np.sqrt(2.0))
    assert output_mesh.metadata["rust_backed"] is True
    assert result["meshlib_reference"] == (
        "MR::convertSurfacePathsToMeshContours / MR::cutMesh edge-path seam subset"
    )
    assert default_sdk.mesh_cut_measure_edge_path_topology_cut(mesh, control_vertices=[1, 2])[
        "mesh"
    ].vertex_count == 6


def test_mesh_geodesic_quadrangle_path_matches_meshlib_reduce_path_crossing_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    path = mesh_geodesic_quadrangle_path(mesh, start_vertex=0, end_vertex=3)

    assert path["shared_edge"] == (1, 2)
    assert path["crossing_t"] == pytest.approx(0.5)
    assert path["crossing_point"] == pytest.approx((0.5, 0.5, 0.0))
    assert path["points"] == pytest.approx([(0.0, 0.0, 0.0), (0.5, 0.5, 0.0), (1.0, 1.0, 0.0)])
    assert path["graph_vertex_indices"] in ([0, 1, 3], [0, 2, 3])
    assert path["graph_length_mm"] == pytest.approx(2.0)
    assert path["length_mm"] == pytest.approx(np.sqrt(2.0))
    assert path["unfolded_quadrangle_convex"] is True
    assert path["meshlib_reference"] == "MR::shortestPathInQuadrangle / MR::reducePath"
    assert default_sdk.mesh_geodesic_quadrangle_path(mesh, start_vertex=0, end_vertex=3)["length_mm"] == pytest.approx(
        np.sqrt(2.0)
    )


def test_mesh_planar_triangle_strip_path_matches_meshlib_funnel_crossing_contract() -> None:
    path = mesh_planar_triangle_strip_path(
        start=(0.0, 0.0),
        portals=[
            (0.0, 1.0, 1.0, 0.0),
            (1.0, 1.0, 1.0, 0.0),
        ],
        end=(2.0, 1.0),
    )

    assert path["crossing_positions"] == pytest.approx([2.0 / 3.0, 0.5])
    np.testing.assert_allclose(path["crossing_points"], [(2.0 / 3.0, 1.0 / 3.0), (1.0, 0.5)])
    np.testing.assert_allclose(
        path["points"],
        [(0.0, 0.0), (2.0 / 3.0, 1.0 / 3.0), (1.0, 0.5), (2.0, 1.0)],
    )
    assert path["length_mm"] == pytest.approx(np.sqrt(5.0))
    assert path["meshlib_reference"] == "MR::PathInPlanarTriangleStrip / MR::reducePath"
    sdk_path = default_sdk.mesh_planar_triangle_strip_path(
        start=(0.0, 0.0),
        portals=np.asarray(
            [
                (0.0, 1.0, 1.0, 0.0),
                (1.0, 1.0, 1.0, 0.0),
            ],
            dtype=np.float64,
        ),
        end=(2.0, 1.0),
    )
    assert sdk_path["crossing_positions"] == pytest.approx([2.0 / 3.0, 0.5])


def test_mesh_triangle_strip_unfolded_path_matches_meshlib_unfolder_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [1, 4, 3]], dtype=np.int64),
        metadata={},
    )

    path = mesh_triangle_strip_unfolded_path(
        mesh,
        start_face_index=0,
        crossed_edges=[(1, 2), (1, 3)],
        end_face_index=2,
        start_point=(0.0, 0.0, 0.0),
        end_point=(2.0, 1.0, 0.0),
    )

    assert path["strip_face_indices"] == [0, 1, 2]
    assert path["crossed_edges"] == [(1, 2), (1, 3)]
    assert path["oriented_edges"] == [(2, 1), (3, 1)]
    assert path["crossing_positions"] == pytest.approx([2.0 / 3.0, 0.5])
    np.testing.assert_allclose(path["crossing_points"], [(2.0 / 3.0, 1.0 / 3.0, 0.0), (1.0, 0.5, 0.0)])
    np.testing.assert_allclose(
        path["points"],
        [(0.0, 0.0, 0.0), (2.0 / 3.0, 1.0 / 3.0, 0.0), (1.0, 0.5, 0.0), (2.0, 1.0, 0.0)],
    )
    assert path["length_mm"] == pytest.approx(np.sqrt(5.0))
    assert path["planar_length_mm"] == pytest.approx(np.sqrt(5.0))
    assert path["meshlib_reference"] == "MR::TriangleStripUnfolder / MR::reducePath"
    assert default_sdk.mesh_triangle_strip_unfolded_path(
        mesh,
        start_face_index=0,
        crossed_edges=np.asarray([(1, 2), (1, 3)], dtype=np.int64),
        end_face_index=2,
        start_point=(0.0, 0.0, 0.0),
        end_point=(2.0, 1.0, 0.0),
    )["length_mm"] == pytest.approx(np.sqrt(5.0))


def test_mesh_surface_edge_point_path_matches_meshlib_surface_path_length_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    path = mesh_surface_edge_point_path(
        mesh,
        edges=[(0, 1), (1, 3), (2, 3)],
        positions=[0.5, 0.5, 0.5],
    )

    assert path["edges"] == [(0, 1), (1, 3), (2, 3)]
    assert path["positions"] == [0.5, 0.5, 0.5]
    np.testing.assert_allclose(path["points"], [(0.5, 0.0, 0.0), (1.0, 0.5, 0.0), (0.5, 1.0, 0.0)])
    assert path["segment_lengths"] == pytest.approx([np.sqrt(0.5), np.sqrt(0.5)])
    assert path["length_mm"] == pytest.approx(np.sqrt(2.0))
    assert path["meshlib_reference"] == "MR::surfacePathLength / MR::surfacePathToContour3f"
    assert default_sdk.mesh_surface_edge_point_path(
        mesh,
        edges=np.asarray([(0, 1), (1, 3), (2, 3)], dtype=np.int64),
        positions=np.asarray([0.5, 0.5, 0.5], dtype=np.float64),
    )["length_mm"] == pytest.approx(np.sqrt(2.0))


def test_mesh_geodesic_edge_point_path_matches_meshlib_geodesic_path_length_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    path = mesh_geodesic_edge_point_path(
        mesh,
        start_point=(0.0, 0.0, 0.0),
        edges=[(1, 2)],
        positions=[0.5],
        end_point=(1.0, 1.0, 0.0),
    )

    assert path["start_point"] == (0.0, 0.0, 0.0)
    assert path["end_point"] == (1.0, 1.0, 0.0)
    assert path["edges"] == [(1, 2)]
    assert path["positions"] == [0.5]
    assert path["mid_points"] == [(0.5, 0.5, 0.0)]
    np.testing.assert_allclose(path["points"], [(0.0, 0.0, 0.0), (0.5, 0.5, 0.0), (1.0, 1.0, 0.0)])
    assert path["segment_lengths"] == pytest.approx([np.sqrt(0.5), np.sqrt(0.5)])
    assert path["length_mm"] == pytest.approx(np.sqrt(2.0))
    assert path["meshlib_reference"] == "MR::geodesicPathLength / MR::geodesicPathToContour3f"
    assert default_sdk.mesh_geodesic_edge_point_path(
        mesh,
        start_point=(0.0, 0.0, 0.0),
        edges=np.asarray([(1, 2)], dtype=np.int64),
        positions=np.asarray([0.5], dtype=np.float64),
        end_point=(1.0, 1.0, 0.0),
    )["length_mm"] == pytest.approx(np.sqrt(2.0))


def test_mesh_steepest_descent_triangle_step_matches_meshlib_triangle_exit_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2]], dtype=np.int64),
        metadata={},
    )
    scalars = np.asarray([0.0, 1.0, 0.0], dtype=np.float64)

    step = mesh_core.mesh_steepest_descent_triangle_step(
        mesh,
        vertex_scalars=scalars,
        face_index=0,
        start_barycentric=(0.5, 0.25, 0.25),
    )

    assert step["face_index"] == 0
    assert step["start_barycentric"] == pytest.approx((0.5, 0.25, 0.25))
    assert step["start_point"] == pytest.approx((0.25, 0.25, 0.0))
    assert step["start_value"] == pytest.approx(0.25)
    assert step["gradient"] == pytest.approx((1.0, 0.0, 0.0))
    assert step["gradient_norm"] == pytest.approx(1.0)
    assert step["crossed_edge"] == (2, 0)
    assert step["edge_position"] == pytest.approx(0.75)
    assert step["crossing_point"] == pytest.approx((0.0, 0.25, 0.0))
    assert step["kind"] == "edge"
    assert step["meshlib_reference"] == "MR::findSteepestDescentPoint(MeshTriPoint)"

    assert default_sdk.mesh_steepest_descent_triangle_step(
        mesh,
        vertex_scalars=scalars,
        face_index=0,
        start_barycentric=(0.5, 0.25, 0.25),
    )["crossing_point"] == pytest.approx((0.0, 0.25, 0.0))

    import geometry_sdk

    assert geometry_sdk.mesh_steepest_descent_triangle_step(
        mesh,
        vertex_scalars=scalars,
        face_index=0,
        start_barycentric=(0.5, 0.25, 0.25),
    )["crossed_edge"] == (2, 0)


def test_mesh_steepest_descent_edge_step_matches_meshlib_edgepoint_vertex_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    scalars = np.asarray([0.0, 1.0, 1.0, 1.0], dtype=np.float64)

    step = mesh_core.mesh_steepest_descent_edge_step(
        mesh,
        vertex_scalars=scalars,
        edge=(1, 2),
        edge_position=0.5,
    )

    assert step["start_edge"] == (1, 2)
    assert step["edge_position"] == pytest.approx(0.5)
    assert step["start_point"] == pytest.approx((0.5, 0.5, 0.0))
    assert step["start_value"] == pytest.approx(1.0)
    assert step["crossed_edge"] == (0, 1)
    assert step["crossing_point"] == pytest.approx((0.0, 0.0, 0.0))
    assert step["crossing_edge_position"] == pytest.approx(0.0)
    assert step["kind"] == "vertex"
    assert step["side"] == "left"
    assert step["meshlib_reference"] == "MR::findSteepestDescentPoint(MeshEdgePoint)"

    assert default_sdk.mesh_steepest_descent_edge_step(
        mesh,
        vertex_scalars=scalars,
        edge=(1, 2),
        edge_position=0.5,
    )["crossing_point"] == pytest.approx((0.0, 0.0, 0.0))

    import geometry_sdk

    assert geometry_sdk.mesh_steepest_descent_edge_step(
        mesh,
        vertex_scalars=scalars,
        edge=(1, 2),
        edge_position=0.5,
    )["crossed_edge"] == (0, 1)


def test_mesh_steepest_descent_vertex_step_matches_meshlib_vertid_triangle_exit_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2]], dtype=np.int64),
        metadata={},
    )
    scalars = np.asarray([1.0, 0.0, 0.0], dtype=np.float64)

    step = mesh_core.mesh_steepest_descent_vertex_step(
        mesh,
        vertex_scalars=scalars,
        vertex_index=0,
    )

    assert step["start_vertex"] == 0
    assert step["start_point"] == pytest.approx((0.0, 0.0, 0.0))
    assert step["start_value"] == pytest.approx(1.0)
    assert step["crossed_edge"] == (1, 2)
    assert step["edge_position"] == pytest.approx(0.5)
    assert step["crossing_point"] == pytest.approx((0.5, 0.5, 0.0))
    assert step["kind"] == "edge"
    assert step["source"] == "face"
    assert step["gradient_norm"] == pytest.approx(np.sqrt(2.0))
    assert step["meshlib_reference"] == "MR::findSteepestDescentPoint(VertId)"

    assert default_sdk.mesh_steepest_descent_vertex_step(
        mesh,
        vertex_scalars=scalars,
        vertex_index=0,
    )["crossing_point"] == pytest.approx((0.5, 0.5, 0.0))

    import geometry_sdk

    assert geometry_sdk.mesh_steepest_descent_vertex_step(
        mesh,
        vertex_scalars=scalars,
        vertex_index=0,
    )["crossed_edge"] == (1, 2)


def test_mesh_steepest_descent_path_matches_meshlib_descent_path_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    scalars = np.asarray([0.0, 1.0, 1.0, 2.0], dtype=np.float64)

    path = mesh_steepest_descent_path(
        mesh,
        vertex_scalars=scalars,
        face_index=1,
        start_barycentric=(0.25, 0.25, 0.5),
        max_steps=8,
    )

    assert path["start_face_index"] == 1
    assert path["start_barycentric"] == pytest.approx((0.25, 0.25, 0.5))
    assert path["start_point"] == pytest.approx((0.75, 0.75, 0.0))
    assert path["start_value"] == pytest.approx(1.5)
    assert path["edges"] == [(2, 1), (0, 1)]
    assert path["positions"][0] == pytest.approx(0.5)
    assert path["positions"][1] == pytest.approx(0.0)
    assert path["points"][0] == pytest.approx((0.5, 0.5, 0.0))
    assert path["points"][1] == pytest.approx((0.0, 0.0, 0.0))
    assert path["reached_vertex"] == 0
    assert path["stopped_reason"] == "local_minimum"
    assert path["steps"] == 2
    assert path["length_mm"] == pytest.approx(1.5 * np.sqrt(0.5))
    assert path["meshlib_reference"] == "MR::computeSteepestDescentPath"

    assert default_sdk.mesh_steepest_descent_path(
        mesh,
        vertex_scalars=scalars,
        face_index=1,
        start_barycentric=(0.25, 0.25, 0.5),
    )["reached_vertex"] == 0

    import geometry_sdk

    assert geometry_sdk.mesh_steepest_descent_path(
        mesh,
        vertex_scalars=scalars,
        face_index=1,
        start_barycentric=(0.25, 0.25, 0.5),
    )["edges"] == [(2, 1), (0, 1)]


def test_mesh_fast_marching_surface_path_matches_meshlib_vertex_endpoint_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    fast_marching_path = getattr(mesh_core, "mesh_fast_marching_surface_path", None)

    assert callable(fast_marching_path)
    path = fast_marching_path(mesh, start_vertex=0, end_vertex=3)

    assert path["start_vertex"] == 0
    assert path["end_vertex"] == 3
    assert path["start_face_index"] == 0
    assert path["start_barycentric"] == pytest.approx((1.0, 0.0, 0.0))
    assert path["surface_distances_mm"] == pytest.approx([1.7071067811865475, 1.0, 1.0, 0.0])
    assert path["edges"] == [(1, 2), (3, 2)]
    assert path["positions"] == pytest.approx([0.5, 0.0], abs=1e-9)
    np.testing.assert_allclose(path["points"], [(0.5, 0.5, 0.0), (1.0, 1.0, 0.0)], atol=1e-9)
    assert path["length_mm"] == pytest.approx(np.sqrt(2.0))
    assert path["reached_vertex"] == 3
    assert path["stopped_reason"] == "end_reached"
    assert path["meshlib_reference"] == "MR::computeFastMarchingPath"
    assert default_sdk.mesh_fast_marching_surface_path(mesh, start_vertex=0, end_vertex=3)["steps"] == 2


def test_mesh_fast_marching_surface_path_tri_points_stops_in_end_triangle_like_meshlib() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    tri_point_path = getattr(mesh_core, "mesh_fast_marching_surface_path_tri_points", None)

    assert callable(tri_point_path)
    path = tri_point_path(
        mesh,
        start_face_index=0,
        start_barycentric=(0.5, 0.25, 0.25),
        end_face_index=1,
        end_barycentric=(0.25, 0.25, 0.5),
    )

    assert path["start_face_index"] == 0
    assert path["end_face_index"] == 1
    assert path["start_point"] == pytest.approx((0.25, 0.25, 0.0))
    assert path["end_point"] == pytest.approx((0.75, 0.75, 0.0))
    assert path["edges"] == [(1, 2)]
    assert path["positions"] == pytest.approx([0.5], abs=1e-9)
    np.testing.assert_allclose(path["points"], [(0.5, 0.5, 0.0)], atol=1e-9)
    assert path["reached_face_index"] == 1
    assert path["stopped_reason"] == "end_triangle_reached"
    assert path["steps"] == 1
    assert path["length_mm"] == pytest.approx(np.sqrt(0.5))
    assert path["meshlib_reference"] == "MR::computeFastMarchingPath"
    assert (
        default_sdk.mesh_fast_marching_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.5, 0.25, 0.25),
            end_face_index=1,
            end_barycentric=(0.25, 0.25, 0.5),
        )["steps"]
        == 1
    )


def test_mesh_surface_path_tri_points_reduces_single_crossing_like_meshlib_compute_surface_path() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    surface_path = getattr(mesh_core, "mesh_surface_path_tri_points", None)

    assert callable(surface_path)
    path = surface_path(
        mesh,
        start_face_index=0,
        start_barycentric=(0.8, 0.1, 0.1),
        end_face_index=1,
        end_barycentric=(0.1, 0.3, 0.6),
        max_geodesic_iters=5,
    )

    assert path["start_point"] == pytest.approx((0.1, 0.1, 0.0))
    assert path["end_point"] == pytest.approx((0.9, 0.7, 0.0))
    assert path["edges"] == [(1, 2)]
    assert path["positions"] == pytest.approx([31.0 / 70.0], abs=1e-9)
    np.testing.assert_allclose(path["points"], [(39.0 / 70.0, 31.0 / 70.0, 0.0)], atol=1e-9)
    assert path["reached_face_index"] == 1
    assert path["reduce_iterations"] == 1
    assert path["steps"] == 1
    assert path["segment_lengths"] == pytest.approx([4.0 / 7.0, 3.0 / 7.0], abs=1e-9)
    assert path["length_mm"] == pytest.approx(1.0)
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.8, 0.1, 0.1),
            end_face_index=1,
            end_barycentric=(0.1, 0.3, 0.6),
            max_geodesic_iters=5,
        )["length_mm"]
        == pytest.approx(1.0)
    )


def test_mesh_surface_path_tri_points_reduces_unfolded_triangle_strip_like_meshlib_compute_surface_path() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 2.0, 0.0],
                [1.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [2, 3, 4], [4, 3, 5]], dtype=np.int64),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.8, 0.1, 0.1),
        end_face_index=3,
        end_barycentric=(0.1, 0.1, 0.8),
        max_steps=8,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.8, 0.1, 0.1),
        end_face_index=3,
        end_barycentric=(0.1, 0.1, 0.8),
        max_geodesic_iters=5,
    )

    assert approximate["edges"] == [(1, 2), (3, 2), (3, 4)]
    assert path["length_mm"] < approximate["length_mm"]
    assert path["edges"] == [(2, 1), (2, 3), (4, 3)]
    assert path["approximate_edges"] == [(1, 2), (3, 2), (3, 4)]
    assert path["reached_face_index"] == 3
    assert path["reduce_iterations"] == 1
    assert path["steps"] == 3
    assert path["positions"] == pytest.approx([9.0 / 26.0, 0.5, 17.0 / 26.0], abs=1e-9)
    np.testing.assert_allclose(
        path["points"],
        [
            (9.0 / 26.0, 17.0 / 26.0, 0.0),
            (0.5, 1.0, 0.0),
            (17.0 / 26.0, 35.0 / 26.0, 0.0),
        ],
        atol=1e-9,
    )
    assert path["length_mm"] == pytest.approx(np.sqrt(3.88))
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.8, 0.1, 0.1),
            end_face_index=3,
            end_barycentric=(0.1, 0.1, 0.8),
            max_geodesic_iters=5,
        )["positions"]
        == pytest.approx([9.0 / 26.0, 0.5, 17.0 / 26.0], abs=1e-9)
    )


def test_mesh_surface_path_tri_points_collapses_strip_vertex_run_like_meshlib_reduce_path() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 1.0, 0.0],
                [0.0, 2.0, 0.0],
                [1.0, 2.0, 0.0],
                [2.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray(
            [
                [0, 1, 3],
                [4, 3, 1],
                [1, 2, 4],
                [5, 4, 2],
                [3, 4, 6],
                [7, 6, 4],
                [4, 5, 7],
                [8, 7, 5],
            ],
            dtype=np.int64,
        ),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.8, 0.1, 0.1),
        end_face_index=7,
        end_barycentric=(0.8, 0.1, 0.1),
        max_steps=18,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.8, 0.1, 0.1),
        end_face_index=7,
        end_barycentric=(0.8, 0.1, 0.1),
        max_geodesic_iters=5,
    )
    one_iter_path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.8, 0.1, 0.1),
        end_face_index=7,
        end_barycentric=(0.8, 0.1, 0.1),
        max_geodesic_iters=1,
    )

    assert approximate["edges"] == [(1, 3), (4, 3), (4, 6), (4, 7), (5, 7)]
    assert one_iter_path["edges"] == [(3, 1), (3, 4), (6, 4), (7, 4), (7, 5)]
    assert one_iter_path["reduce_iterations"] == 1
    assert one_iter_path["steps"] == 5
    assert path["edges"] == [(3, 1), (7, 4), (7, 5)]
    assert path["reached_face_index"] == 7
    assert path["reduce_iterations"] == 2
    assert path["steps"] == 3
    assert path["positions"] == pytest.approx([0.5, 1.0, 0.5], abs=1e-9)
    np.testing.assert_allclose(path["points"][1], (1.0, 1.0, 0.0), atol=1e-9)
    assert path["length_mm"] == pytest.approx(1.8 * np.sqrt(2.0))
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.8, 0.1, 0.1),
            end_face_index=7,
            end_barycentric=(0.8, 0.1, 0.1),
            max_geodesic_iters=5,
        )["edges"]
        == [(3, 1), (7, 4), (7, 5)]
    )


def test_mesh_surface_path_tri_points_avoids_adjacent_face_vertex_like_meshlib_reduce_path() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]], dtype=np.int64),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.1, 0.6, 0.3),
        end_face_index=1,
        end_barycentric=(0.4, 0.1, 0.5),
        max_steps=8,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.1, 0.6, 0.3),
        end_face_index=1,
        end_barycentric=(0.4, 0.1, 0.5),
        max_geodesic_iters=5,
    )

    assert approximate["edges"] == [(0, 1), (0, 1)]
    assert approximate["positions"][1] == pytest.approx(0.0)
    assert path["length_mm"] < approximate["length_mm"]
    assert path["edges"] == [(0, 2)]
    assert path["approximate_edges"] == [(0, 1), (0, 1)]
    assert path["reached_face_index"] == 1
    assert path["reduce_iterations"] == 2
    assert path["steps"] == 1
    assert path["positions"] == pytest.approx([21.0 / 110.0], abs=1e-9)
    np.testing.assert_allclose(path["points"], [(0.0, 21.0 / 110.0, 0.0)], atol=1e-9)
    assert path["length_mm"] == pytest.approx(np.sqrt(1.25))
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.1, 0.6, 0.3),
            end_face_index=1,
            end_barycentric=(0.4, 0.1, 0.5),
            max_geodesic_iters=5,
        )["edges"]
        == [(0, 2)]
    )


def test_mesh_surface_path_tri_points_avoids_non_adjacent_vertex_fan_like_meshlib_reduce_path() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]], dtype=np.int64),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.1, 0.6, 0.3),
        end_face_index=2,
        end_barycentric=(0.4, 0.5, 0.1),
        max_steps=8,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.1, 0.6, 0.3),
        end_face_index=2,
        end_barycentric=(0.4, 0.5, 0.1),
        max_geodesic_iters=5,
    )

    assert approximate["edges"] == [(0, 1), (0, 1)]
    assert approximate["positions"][1] == pytest.approx(0.0)
    assert path["length_mm"] < approximate["length_mm"]
    assert path["edges"] == [(0, 2), (0, 3)]
    assert path["approximate_edges"] == [(0, 1), (0, 1)]
    assert path["reached_face_index"] == 2
    assert path["reduce_iterations"] == 2
    assert path["steps"] == 2
    assert path["positions"] == pytest.approx([9.0 / 110.0, 9.0 / 40.0], abs=1e-9)
    np.testing.assert_allclose(
        path["points"],
        [(0.0, 9.0 / 110.0, 0.0), (-9.0 / 40.0, 0.0, 0.0)],
        atol=1e-9,
    )
    assert path["length_mm"] == pytest.approx(np.sqrt(1.37))
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.1, 0.6, 0.3),
            end_face_index=2,
            end_barycentric=(0.4, 0.5, 0.1),
            max_geodesic_iters=5,
        )["edges"]
        == [(0, 2), (0, 3)]
    )


def test_mesh_surface_path_tri_points_removes_repeated_edge_vertex_detour_like_meshlib_reduce_path() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 1.0, 0.0],
                [0.0, 2.0, 0.0],
                [1.0, 2.0, 0.0],
                [2.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray(
            [
                [0, 1, 3],
                [3, 1, 4],
                [1, 2, 4],
                [4, 2, 5],
                [3, 4, 6],
                [6, 4, 7],
                [4, 5, 7],
                [7, 5, 8],
            ],
            dtype=np.int64,
        ),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.05, 0.1, 0.85),
        end_face_index=2,
        end_barycentric=(0.1, 0.05, 0.85),
        max_steps=16,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.05, 0.1, 0.85),
        end_face_index=2,
        end_barycentric=(0.1, 0.05, 0.85),
        max_geodesic_iters=5,
    )

    assert approximate["edges"] == [(1, 3), (4, 3), (4, 3)]
    assert approximate["positions"][2] == pytest.approx(0.0)
    assert path["length_mm"] < approximate["length_mm"]
    assert path["edges"] == [(3, 1), (4, 1)]
    assert path["approximate_edges"] == [(1, 3), (4, 3), (4, 3)]
    assert path["reached_face_index"] == 2
    assert path["reduce_iterations"] == 2
    assert path["steps"] == 2
    assert path["positions"] == pytest.approx([0.15, 0.15], abs=1e-9)
    np.testing.assert_allclose(path["points"], [(0.15, 0.85, 0.0), (1.0, 0.85, 0.0)], atol=1e-9)
    assert path["length_mm"] == pytest.approx(0.95)
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.05, 0.1, 0.85),
            end_face_index=2,
            end_barycentric=(0.1, 0.05, 0.85),
            max_geodesic_iters=5,
        )["edges"]
        == [(3, 1), (4, 1)]
    )


def test_mesh_surface_path_tri_points_removes_duplicate_nonvertex_location_like_meshlib_reduce_path() -> None:
    vertices = []
    for y in range(7):
        for x in range(7):
            vertices.append([float(x), float(y), 0.0])
    faces = []

    def vertex_id(x: int, y: int) -> int:
        return y * 7 + x

    for y in range(6):
        for x in range(6):
            faces.append([vertex_id(x, y), vertex_id(x + 1, y), vertex_id(x, y + 1)])
            faces.append([vertex_id(x + 1, y + 1), vertex_id(x, y + 1), vertex_id(x + 1, y)])
    mesh = MeshDocument(
        vertices=np.asarray(vertices, dtype=np.float64),
        faces=np.asarray(faces, dtype=np.int64),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.46671207298740064, 0.48702168304170673, 0.04626624397089257),
        end_face_index=12,
        end_barycentric=(0.7272053095059872, 0.0827160816645162, 0.19007860882949656),
        max_steps=80,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.46671207298740064, 0.48702168304170673, 0.04626624397089257),
        end_face_index=12,
        end_barycentric=(0.7272053095059872, 0.0827160816645162, 0.19007860882949656),
        max_geodesic_iters=5,
    )

    assert approximate["edges"] == [(1, 7), (7, 1)]
    assert path["length_mm"] < approximate["length_mm"]
    assert path["edges"] == [(7, 1), (7, 8)]
    assert path["reached_face_index"] == 12
    assert path["reduce_iterations"] == 2
    assert path["steps"] == 2
    assert path["positions"] == pytest.approx([0.23185930365948093, 0.14990354056320485], abs=1e-12)
    np.testing.assert_allclose(
        path["points"],
        [(0.23185930365948093, 0.7681406963405191, 0.0), (0.14990354056320485, 1.0, 0.0)],
        atol=1e-12,
    )
    assert path["length_mm"] == pytest.approx(1.2131651764324607)
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.46671207298740064, 0.48702168304170673, 0.04626624397089257),
            end_face_index=12,
            end_barycentric=(0.7272053095059872, 0.0827160816645162, 0.19007860882949656),
            max_geodesic_iters=5,
        )["edges"]
        == [(7, 1), (7, 8)]
    )


def test_mesh_surface_path_tri_points_removes_same_triangle_nonvertex_detour_like_meshlib_reduce_path() -> None:
    vertices = []
    for y in range(8):
        for x in range(8):
            vertices.append([float(x), float(y), 0.0])
    faces = []

    def vertex_id(x: int, y: int) -> int:
        return y * 8 + x

    for y in range(7):
        for x in range(7):
            faces.append([vertex_id(x, y), vertex_id(x + 1, y), vertex_id(x, y + 1)])
            faces.append([vertex_id(x + 1, y + 1), vertex_id(x, y + 1), vertex_id(x + 1, y)])
    mesh = MeshDocument(
        vertices=np.asarray(vertices, dtype=np.float64),
        faces=np.asarray(faces, dtype=np.int64),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=2,
        start_barycentric=(0.2675321287343475, 0.523820181254268, 0.2086476900113846),
        end_face_index=0,
        end_barycentric=(0.42935248132098075, 0.41316891553417373, 0.1574786031448455),
        max_steps=80,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=2,
        start_barycentric=(0.2675321287343475, 0.523820181254268, 0.2086476900113846),
        end_face_index=0,
        end_barycentric=(0.42935248132098075, 0.41316891553417373, 0.1574786031448455),
        max_geodesic_iters=5,
    )

    assert approximate["edges"] == [(1, 2), (1, 2)]
    assert path["length_mm"] < approximate["length_mm"]
    assert path["edges"] == [(1, 9), (1, 8)]
    assert path["reached_face_index"] == 0
    assert path["reduce_iterations"] == 2
    assert path["steps"] == 2
    assert path["positions"] == pytest.approx([0.18451464196622006, 0.1763882171520069], abs=1e-12)
    np.testing.assert_allclose(
        path["points"],
        [(1.0, 0.18451464196622006, 0.0), (0.8236117828479931, 0.1763882171520069, 0.0)],
        atol=1e-12,
    )
    assert path["length_mm"] == pytest.approx(1.1118293526870044)
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=2,
            start_barycentric=(0.2675321287343475, 0.523820181254268, 0.2086476900113846),
            end_face_index=0,
            end_barycentric=(0.42935248132098075, 0.41316891553417373, 0.1574786031448455),
            max_geodesic_iters=5,
        )["edges"]
        == [(1, 9), (1, 8)]
    )


def test_mesh_surface_path_tri_points_collapses_repeated_location_strip_vertex_run_like_meshlib_reduce_path() -> None:
    vertices = []
    for y in range(9):
        for x in range(9):
            vertices.append([float(x), float(y), 0.0])
    faces = []

    def vertex_id(x: int, y: int) -> int:
        return y * 9 + x

    for y in range(8):
        for x in range(8):
            faces.append([vertex_id(x, y), vertex_id(x + 1, y), vertex_id(x, y + 1)])
            faces.append([vertex_id(x + 1, y + 1), vertex_id(x, y + 1), vertex_id(x + 1, y)])
    mesh = MeshDocument(
        vertices=np.asarray(vertices, dtype=np.float64),
        faces=np.asarray(faces, dtype=np.int64),
        metadata={},
    )
    approximate = mesh_core.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.27924020234514274, 0.3661046568847575, 0.35465514077009963),
        end_face_index=66,
        end_barycentric=(0.04181337750965728, 0.8199993556651197, 0.13818726682522295),
        max_steps=120,
    )
    path = mesh_core.mesh_surface_path_tri_points(
        mesh,
        start_face_index=0,
        start_barycentric=(0.27924020234514274, 0.3661046568847575, 0.35465514077009963),
        end_face_index=66,
        end_barycentric=(0.04181337750965728, 0.8199993556651197, 0.13818726682522295),
        max_geodesic_iters=5,
    )

    assert approximate["edges"] == [
        (1, 9),
        (10, 9),
        (10, 18),
        (10, 19),
        (11, 19),
        (20, 19),
        (20, 28),
        (29, 28),
        (29, 37),
        (29, 38),
        (38, 29),
    ]
    assert path["length_mm"] < approximate["length_mm"]
    assert path["edges"] == [
        (9, 1),
        (10, 11),
        (19, 11),
        (19, 20),
        (28, 20),
        (28, 29),
        (37, 29),
        (37, 38),
    ]
    assert path["reached_face_index"] == 66
    assert path["reduce_iterations"] == 2
    assert path["steps"] == 8
    assert path["positions"][0] == pytest.approx(0.5044751236295062, abs=1e-12)
    assert path["positions"][1] == pytest.approx(0.0, abs=1e-12)
    assert path["positions"][7] == pytest.approx(0.78389141814473, abs=1e-12)
    np.testing.assert_allclose(path["points"][1], (1.0, 1.0, 0.0), atol=1e-12)
    assert path["length_mm"] == pytest.approx(4.148145908127404)
    assert path["meshlib_reference"] == "MR::computeSurfacePath / MR::reducePath"
    assert (
        default_sdk.mesh_surface_path_tri_points(
            mesh,
            start_face_index=0,
            start_barycentric=(0.27924020234514274, 0.3661046568847575, 0.35465514077009963),
            end_face_index=66,
            end_barycentric=(0.04181337750965728, 0.8199993556651197, 0.13818726682522295),
            max_geodesic_iters=5,
        )["edges"]
        == [
            (9, 1),
            (10, 11),
            (19, 11),
            (19, 20),
            (28, 20),
            (28, 29),
            (37, 29),
            (37, 38),
        ]
    )


def test_mesh_geodesic_polyline_path_exposes_control_vertex_surface_path() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    path = mesh_geodesic_polyline_path(mesh, control_vertices=[0, 1, 3])

    assert path["control_vertex_indices"] == [0, 1, 3]
    assert path["control_vertex_offsets"] == [0, 1, 2]
    assert path["leg_vertex_offsets"] == [0, 1]
    assert path["vertex_indices"] == [0, 1, 3]
    assert path["line_segments"] == 2
    assert path["closed_path"] is False
    assert path["length_mm"] == pytest.approx(2.0)
    np.testing.assert_allclose(path["point_normals"], [[0.0, 0.0, 1.0]] * 3)
    np.testing.assert_allclose(path["leg_lengths"], [1.0, 1.0])
    assert path["meshlib_reference"] == "MR::buildShortestPath control polyline"
    assert default_sdk.mesh_geodesic_polyline_path(mesh, control_vertices=[0, 1, 3])["length_mm"] == pytest.approx(2.0)

    closed = mesh_geodesic_polyline_path(mesh, control_vertices=[0, 1, 3], close_path=True)

    assert closed["closed_path"] is True
    assert closed["control_vertex_indices"] == [0, 1, 3, 0]
    assert closed["vertex_indices"][0] == 0
    assert closed["vertex_indices"][-1] == 0
    assert closed["line_segments"] == 4
    assert closed["length_mm"] == pytest.approx(4.0)


def test_feature_pair_measurements_expose_meshlib_center_distance_and_angle() -> None:
    features = [
        {
            "feature_id": "plane_xy",
            "kind": "plane",
            "center": (0.0, 0.0, 0.0),
            "normal": (0.0, 0.0, 1.0),
        },
        {
            "feature_id": "axis_z",
            "kind": "line",
            "center": (0.0, 0.0, 0.0),
            "direction": (0.0, 0.0, 1.0),
            "length": 4.0,
        },
        {
            "feature_id": "sphere",
            "kind": "sphere",
            "center": (3.0, 0.0, 0.0),
            "radius": 1.0,
        },
    ]

    measurements = feature_pair_measurements(features, [("plane_xy", "axis_z"), (1, 2)])

    assert measurements[0]["first_feature_id"] == "plane_xy"
    assert measurements[0]["second_feature_id"] == "axis_z"
    assert measurements[0]["center_distance"]["status"] == "ok"
    assert measurements[0]["center_distance"]["distance_mm"] == pytest.approx(0.0)
    assert measurements[0]["angle"]["status"] == "ok"
    assert measurements[0]["angle"]["angle_degrees"] == pytest.approx(90.0)
    assert measurements[0]["angle"]["is_surface_normal_a"] is True
    assert measurements[0]["angle"]["is_surface_normal_b"] is False
    assert measurements[0]["distance"]["status"] == "ok"
    assert measurements[0]["distance"]["distance_mm"] == pytest.approx(-2.0)
    assert measurements[0]["distance"]["closest_point_a"] == (0.0, 0.0, 0.0)
    assert measurements[0]["distance"]["closest_point_b"] == (0.0, 0.0, -2.0)
    assert measurements[0]["intersections"] == [
        {
            "kind": "point",
            "center": (0.0, 0.0, 0.0),
            "direction": None,
            "radius_mm": 0.0,
            "length_mm": 0.0,
            "start_point": None,
            "end_point": None,
            "meshlib_primitive": "MR::Features::Primitives::Sphere(point)",
        }
    ]
    assert measurements[0]["meshlib_reference"] == "MR::Features::MeasureResult"
    assert measurements[1]["distance"]["status"] == "ok"
    assert measurements[1]["distance"]["distance_mm"] == pytest.approx(2.0)
    assert measurements[1]["distance"]["closest_point_a"] == (0.0, 0.0, 0.0)
    assert measurements[1]["distance"]["closest_point_b"] == (2.0, 0.0, 0.0)
    assert measurements[1]["center_distance"]["distance_mm"] == pytest.approx(3.0)
    assert measurements[1]["center_distance"]["closest_point_a"] == (0.0, 0.0, 0.0)
    assert measurements[1]["center_distance"]["closest_point_b"] == (3.0, 0.0, 0.0)
    assert measurements[1]["intersections"] == []
    assert default_sdk.feature_pair_measurements(features, [("plane_xy", "axis_z")])[0]["angle"]["angle_degrees"] == pytest.approx(90.0)


def test_feature_pair_measurements_match_meshlib_parallel_cylinder_center_distance_fallback() -> None:
    features = [
        {
            "feature_id": "a",
            "kind": "cylinder",
            "center": (0.0, 0.0, 0.0),
            "direction": (0.0, 0.0, 1.0),
            "radius_mm": 0.5,
            "length_mm": 2.0,
        },
        {
            "feature_id": "b",
            "kind": "cylinder",
            "center": (1.0, 0.0, 4.0),
            "direction": (0.0, 0.0, 1.0),
            "radius_mm": 0.5,
            "length_mm": 2.0,
        },
    ]

    measurement = feature_pair_measurements(features, [("a", "b")])[0]

    assert measurement["distance"]["status"] == "not_implemented"
    assert measurement["center_distance"]["status"] == "ok"
    assert measurement["center_distance"]["distance_mm"] == pytest.approx(np.sqrt(17.0))
    assert measurement["center_distance"]["closest_point_a"] == (0.0, 0.0, 4.0)
    assert measurement["center_distance"]["closest_point_b"] == (1.0, 0.0, 8.0)


def test_feature_object_descriptors_match_meshlib_primitive_to_object_contract() -> None:
    features = [
        {
            "feature_id": "point_from_sphere",
            "kind": "sphere",
            "center": (1.0, 2.0, 3.0),
            "radius_mm": 0.0,
        },
        {
            "feature_id": "plane_xy",
            "kind": "plane",
            "center": (0.0, 0.0, 0.0),
            "normal": (0.0, 0.0, 1.0),
        },
        {
            "feature_id": "cylinder_z",
            "kind": "cylinder",
            "center": (0.0, 0.0, 0.0),
            "direction": (0.0, 0.0, 1.0),
            "radius_mm": 2.0,
            "length_mm": 5.0,
        },
        {
            "feature_id": "cone_z",
            "kind": "cone",
            "center": (0.0, 0.0, 0.0),
            "direction": (0.0, 0.0, 1.0),
            "radius_mm": 2.0,
            "length_mm": 10.0,
        },
    ]

    descriptors = feature_object_descriptors(features, infinite_extent_mm=25.0)

    assert descriptors[0]["object_type"] == "PointObject"
    assert descriptors[0]["source_kind"] == "sphere"
    assert descriptors[0]["shared_properties"] == [
        {
            "name": "Point",
            "kind": "position",
            "scalar_value": None,
            "vector_value": (1.0, 2.0, 3.0),
        }
    ]
    assert descriptors[1]["object_type"] == "PlaneObject"
    assert descriptors[1]["shared_properties"][0]["name"] == "Center"
    assert descriptors[1]["shared_properties"][1]["name"] == "Normal"
    assert descriptors[1]["shared_properties"][2]["name"] == "Size"
    assert descriptors[1]["shared_properties"][2]["scalar_value"] == pytest.approx(25.0)
    assert descriptors[1]["shared_properties"][3]["name"] == "SizeX"
    assert descriptors[1]["shared_properties"][4]["name"] == "SizeY"
    assert descriptors[2]["object_type"] == "CylinderObject"
    assert [property["name"] for property in descriptors[2]["shared_properties"]] == [
        "Radius",
        "Length",
        "Center",
        "Main axis",
    ]
    assert descriptors[3]["object_type"] == "ConeObject"
    assert [property["name"] for property in descriptors[3]["shared_properties"]] == [
        "Angle",
        "Height",
        "Center",
        "Main axis",
    ]
    assert descriptors[3]["shared_properties"][0]["scalar_value"] == pytest.approx(np.arctan(2.0 / 10.0))
    assert descriptors[3]["shared_properties"][1]["scalar_value"] == pytest.approx(10.0)
    assert descriptors[3]["shared_properties"][2]["vector_value"] == pytest.approx((0.0, 0.0, 0.0))
    assert descriptors[2]["meshlib_reference"] == "MR::Features::primitiveToObject"
    assert default_sdk.feature_object_descriptors(features, infinite_extent_mm=25.0)[1]["object_type"] == "PlaneObject"


def test_refine_feature_primitives_matches_meshlib_plane_refine_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    features = [
        {
            "feature_id": "plane_xy",
            "kind": "plane",
            "center": (0.25, 0.25, 0.9),
            "normal": (0.0, 0.0, 1.0),
        }
    ]

    refinements = refine_feature_primitives(
        mesh,
        features,
        distance_limit_mm=0.2,
        normal_tolerance_degrees=30.0,
        max_iterations=4,
    )

    assert refinements[0]["feature_id"] == "plane_xy"
    assert refinements[0]["kind"] == "plane"
    assert refinements[0]["meshlib_reference"] == "MR::refineFeatureObject"
    assert refinements[0]["selected_vertex_indices"] == [0, 1, 2, 3]
    assert refinements[0]["selected_count"] == 4
    assert refinements[0]["iterations"] == 2
    assert refinements[0]["converged"] is True
    assert refinements[0]["primitive"]["center"] == pytest.approx((0.5, 0.5, 1.0))
    assert refinements[0]["primitive"]["direction"] == pytest.approx((0.0, 0.0, 1.0))
    assert refinements[0]["primitive"]["length_mm"] == pytest.approx(np.sqrt(2.0))
    assert default_sdk.refine_feature_primitives(mesh, features, distance_limit_mm=0.2)[0]["selected_count"] == 4


def test_refine_feature_primitives_uses_meshlib_cylinder_approximation() -> None:
    radius = 1.5
    length = 10.0
    center = np.asarray([1.0, 2.0, 3.0], dtype=np.float64)
    angle_count = 32
    height_count = 10
    arch_size = np.pi / 1.5
    vertices: list[np.ndarray] = []
    for height_index in range(height_count):
        z = -0.5 + height_index / (height_count - 1)
        for angle_index in range(angle_count):
            angle = arch_size * angle_index / (angle_count - 1)
            vertices.append(
                center
                + np.asarray(
                    [
                        radius * np.cos(angle),
                        radius * np.sin(angle),
                        length * z,
                    ],
                    dtype=np.float64,
                )
            )
    faces: list[tuple[int, int, int]] = []
    for height_index in range(height_count - 1):
        row = height_index * angle_count
        next_row = (height_index + 1) * angle_count
        for angle_index in range(angle_count - 1):
            a = row + angle_index
            b = row + angle_index + 1
            c = next_row + angle_index
            d = next_row + angle_index + 1
            faces.append((a, c, b))
            faces.append((b, c, d))
    mesh = MeshDocument(
        vertices=np.asarray(vertices, dtype=np.float64),
        faces=np.asarray(faces, dtype=np.int64),
        metadata={},
    )
    features = [
        {
            "feature_id": "partial_cylinder",
            "kind": "cylinder",
            "center": tuple(center),
            "direction": (0.0, 0.0, 1.0),
            "radius_mm": radius,
            "length_mm": length,
        }
    ]

    refinement = refine_feature_primitives(
        mesh,
        features,
        distance_limit_mm=0.05,
        normal_tolerance_degrees=180.0,
        max_iterations=2,
    )[0]

    assert refinement["selected_count"] == len(vertices)
    assert refinement["meshlib_reference"] == "MR::refineFeatureObject"
    primitive = refinement["primitive"]
    assert primitive["kind"] == "cylinder"
    assert primitive["center"] == pytest.approx(tuple(center), abs=0.1)
    assert primitive["radius_mm"] == pytest.approx(radius, abs=0.1)
    assert primitive["length_mm"] == pytest.approx(length, abs=0.1)
    assert abs(primitive["direction"][2]) > 0.9
    assert default_sdk.refine_feature_primitives(
        mesh,
        features,
        distance_limit_mm=0.05,
        normal_tolerance_degrees=180.0,
        max_iterations=2,
    )[0]["primitive"]["radius_mm"] == pytest.approx(radius, abs=0.1)


def test_refine_feature_primitives_uses_meshlib_cone_approximation() -> None:
    height = 10.0
    base_radius = 2.0
    apex = np.asarray([1.0, 2.0, 3.0], dtype=np.float64)
    angle_count = 14
    height_count = 6
    arch_size = np.pi / 1.5
    vertices: list[np.ndarray] = []
    for height_index in range(height_count):
        z_fraction = 0.1 + 0.9 * height_index / (height_count - 1)
        radius = base_radius * z_fraction
        z = height * z_fraction
        for angle_index in range(angle_count):
            angle = arch_size * angle_index / (angle_count - 1)
            vertices.append(
                apex
                + np.asarray(
                    [
                        radius * np.cos(angle),
                        radius * np.sin(angle),
                        z,
                    ],
                    dtype=np.float64,
                )
            )
    faces: list[tuple[int, int, int]] = []
    for height_index in range(height_count - 1):
        row = height_index * angle_count
        next_row = (height_index + 1) * angle_count
        for angle_index in range(angle_count - 1):
            a = row + angle_index
            b = row + angle_index + 1
            c = next_row + angle_index
            d = next_row + angle_index + 1
            faces.append((a, c, b))
            faces.append((b, c, d))
    mesh = MeshDocument(
        vertices=np.asarray(vertices, dtype=np.float64),
        faces=np.asarray(faces, dtype=np.int64),
        metadata={},
    )
    features = [
        {
            "feature_id": "partial_cone",
            "kind": "cone",
            "center": tuple(apex),
            "direction": (0.0, 0.0, 1.0),
            "radius_mm": base_radius,
            "length_mm": height,
        }
    ]

    refinement = refine_feature_primitives(
        mesh,
        features,
        distance_limit_mm=0.05,
        normal_tolerance_degrees=180.0,
        max_iterations=1,
    )[0]

    assert refinement["selected_count"] == len(vertices)
    assert refinement["meshlib_reference"] == "MR::refineFeatureObject"
    primitive = refinement["primitive"]
    assert primitive["kind"] == "cone"
    assert primitive["center"] == pytest.approx(tuple(apex), abs=0.05)
    assert primitive["radius_mm"] == pytest.approx(base_radius, abs=0.05)
    assert primitive["length_mm"] == pytest.approx(height, abs=0.05)
    assert primitive["direction"] == pytest.approx((0.0, 0.0, 1.0), abs=0.05)


def test_mesh_geodesic_distance_field_matches_meshlib_surface_distance_seed_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    field = mesh_geodesic_distance_field(mesh, seed_vertices=[0])

    assert field["seed_vertices"] == [0]
    assert field["reachable_vertex_count"] == 4
    assert field["distances_mm"] == pytest.approx([0.0, 1.0, 1.0, 1.7071067811865475])
    assert field["predecessor_vertices"][0] is None
    assert field["max_distance_mm"] == pytest.approx(1.7071067811865475)
    assert field["meshlib_reference"] == "MR::computeSurfaceDistances / SurfaceDistanceBuilder"
    assert default_sdk.mesh_geodesic_distance_field(mesh, seed_vertices=[0])["max_distance_mm"] == pytest.approx(
        1.7071067811865475
    )


def test_mesh_geodesic_distance_field_uses_meshlib_triangle_front_update() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2]], dtype=np.int64),
        metadata={},
    )

    field = mesh_geodesic_distance_field(mesh, seed_vertices=[0, 1])

    assert field["distances_mm"] == pytest.approx([0.0, 0.0, 1.0])
    assert field["max_distance_mm"] == pytest.approx(1.0)
    assert field["meshlib_reference"] == "MR::computeSurfaceDistances / SurfaceDistanceBuilder"


def test_mesh_closest_surface_path_targets_match_meshlib_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [4.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [4, 5, 6]], dtype=np.int64),
        metadata={},
    )

    targets = mesh_closest_surface_path_targets(mesh, start_vertices=[3, 2, 6], end_vertices=[0, 1])

    assert targets["start_vertices"] == [2, 3, 6]
    assert targets["end_vertices"] == [0, 1]
    assert targets["target_vertices"] == [0, 1, None]
    assert targets["target_distances_mm"][:2] == pytest.approx([1.0, 1.0])
    assert targets["target_distances_mm"][2] is None
    assert targets["predecessor_vertices"][3] == 1
    assert targets["meshlib_reference"] == "MR::computeClosestSurfacePathTargets"
    assert default_sdk.mesh_closest_surface_path_targets(mesh, start_vertices=[3], end_vertices=[0])[
        "target_vertices"
    ] == [0]


def test_mesh_geodesic_iso_region_exposes_surface_distance_cut_select_foundation() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    region = mesh_geodesic_iso_region(mesh, seed_vertices=[0], iso_value_mm=0.5)

    assert region["selected_vertex_indices"] == [0]
    assert region["selected_face_indices"] == []
    assert region["crossing_face_indices"] == [0]
    assert region["boundary_edges"] == [(0, 1), (0, 2)]
    assert len(region["iso_segments"]) == 1
    np.testing.assert_allclose(region["iso_segments"][0], [[0.5, 0.0, 0.0], [0.0, 0.5, 0.0]])
    np.testing.assert_allclose(region["clipped_vertices"], [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.5, 0.0]])
    assert region["clipped_faces"] == [(0, 1, 2)]
    assert region["clipped_source_face_indices"] == [0]
    assert region["clipped_source_vertex_indices"] == [0, None, None]
    assert region["meshlib_reference"] == "MR::computeClosestSurfacePathTargets surface-distance iso"


def test_mesh_geodesic_extreme_edges_match_meshlib_ridge_and_gorge_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    ridge = mesh_geodesic_extreme_edges(mesh, scalars=[0.0, 1.0, 1.0, 0.0], extreme_type="ridge")
    gorge = mesh_geodesic_extreme_edges(mesh, scalars=[1.0, 0.0, 0.0, 1.0], extreme_type="gorge")

    assert ridge["edge_indices"] == [(1, 2)]
    assert ridge["extreme_type"] == "ridge"
    assert ridge["meshlib_reference"] == "MR::findExtremeEdges"
    assert gorge["edge_indices"] == [(1, 2)]
    assert default_sdk.mesh_geodesic_extreme_edges(
        mesh,
        scalars=[0.0, 1.0, 1.0, 0.0],
        extreme_type="ridge",
    )["edge_indices"] == [(1, 2)]


def test_mesh_surface_distance_seed_vertices_exposes_official_source_modes() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )

    sources = mesh_surface_distance_seed_vertices(mesh, seed_vertices=[0], seed_edges=[(1, 3)], seed_face_ids=[0])

    assert sources["seed_vertices"] == [0, 1, 2, 3]
    assert sources["selected_edges"] == [(1, 3)]
    assert sources["selected_face_indices"] == [0]
    assert sources["selected_face_boundary_edges"] == [(0, 1), (0, 2), (1, 2)]
    assert sources["meshlib_reference"] == "Surface Distance selected edges / selected triangles boundary"


def test_select_largest_component_faces_matches_meshlib_surface_area_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [4.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [4, 5, 6]], dtype=np.int64),
        metadata={},
    )

    assert select_largest_component_faces(mesh) == [0, 1]
    assert default_sdk.select_largest_component_faces(mesh, min_area_mm2=1.1) == []


def test_select_boundary_faces_and_edges_match_meshlib_boundary_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [4.5, 1.0, 0.0],
                [4.5, 0.5, 1.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray(
            [
                [0, 1, 2],
                [2, 1, 3],
                [4, 6, 5],
                [4, 5, 7],
                [5, 6, 7],
                [6, 4, 7],
            ],
            dtype=np.int64,
        ),
        metadata={},
    )

    assert select_boundary_faces(mesh) == [0, 1]
    assert select_boundary_edges(mesh) == [(0, 1), (0, 2), (1, 3), (2, 3)]
    assert default_sdk.select_boundary_faces(mesh) == [0, 1]
    assert default_sdk.select_boundary_edges(mesh) == [(0, 1), (0, 2), (1, 3), (2, 3)]


def test_select_degenerate_faces_matches_meshlib_aspect_ratio_and_boundary_filter() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 0.001, 0.0],
                [0.5, 0.4, 1.0],
                [3.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                [3.5, 0.001, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray(
            [
                [0, 1, 2],
                [0, 3, 1],
                [1, 3, 2],
                [2, 3, 0],
                [4, 5, 6],
            ],
            dtype=np.int64,
        ),
        metadata={},
    )

    assert default_sdk.select_degenerate_faces(mesh, min_aspect_ratio=100.0) == [0, 4]
    assert default_sdk.select_degenerate_faces(mesh, min_aspect_ratio=100.0, boundary_only=True) == [4]


def test_select_short_edges_matches_meshlib_critical_length_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.05, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
        metadata={},
    )

    assert default_sdk.select_short_edges(mesh, max_edge_length_mm=0.05) == [(0, 1)]


def test_select_faces_by_area_matches_meshlib_area_threshold_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [3.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [3.0, 2.0, 0.0],
                [7.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [7.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5], [6, 7, 8]], dtype=np.int64),
        metadata={},
    )

    assert select_faces_by_area(mesh, area=1.0, scalar_type="absolute", compare_type="less") == [0]
    assert default_sdk.select_faces_by_area(mesh, area=50.0, scalar_type="percentage", compare_type="greater") == [2]


def test_select_crease_edges_matches_meshlib_find_crease_edges_contract() -> None:
    selected = select_crease_edges(cube(size=2.0), angle_from_planar_radians=0.3)

    assert len(selected) == 12
    assert (0, 2) not in selected
    assert (0, 1) in selected
    assert default_sdk.select_crease_edges(cube(size=2.0), angle_from_planar_radians=0.3) == selected


def test_select_overhang_faces_matches_meshlib_layer_basement_and_normal_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 2.0],
                [1.0, 0.0, 2.0],
                [0.0, 1.0, 2.0],
                [3.0, 0.0, 2.0],
                [4.0, 0.0, 2.0],
                [3.0, 1.0, 2.0],
                [6.0, 0.0, 0.0],
                [7.0, 0.0, 0.0],
                [6.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 2, 1], [3, 4, 5], [6, 8, 7]], dtype=np.int64),
        metadata={},
    )

    assert select_overhang_faces(mesh, axis=(0.0, 0.0, 1.0), layer_height_mm=0.5, max_overhang_distance_mm=0.5) == [0]
    assert default_sdk.select_overhang_faces(
        mesh,
        axis=(0.0, 0.0, 1.0),
        layer_height_mm=0.5,
        max_overhang_distance_mm=0.5,
    ) == [0]


def test_select_outer_layer_faces_matches_meshlib_double_layer_seed_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )

    assert select_outer_layer_faces(mesh, epsilon=1e-8) == [0]
    assert default_sdk.select_outer_layer_faces(mesh, epsilon=1e-8) == [0]


def test_select_overlapping_faces_matches_meshlib_opposite_close_triangle_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 5e-6],
                [1.0, 0.0, 5e-6],
                [0.0, 1.0, 5e-6],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 5, 4]], dtype=np.int64),
        metadata={},
    )

    assert select_overlapping_faces(mesh, max_dist_sq=1e-10, max_normal_dot=-0.99, min_area_fraction=1e-5) == [0, 1]
    assert default_sdk.select_overlapping_faces(mesh) == [0, 1]


def test_select_overlapping_faces_rejects_same_orientation_and_far_triangles_like_meshlib() -> None:
    same_orientation = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 5e-6],
                [1.0, 0.0, 5e-6],
                [0.0, 1.0, 5e-6],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    assert default_sdk.select_overlapping_faces(same_orientation) == []

    far = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1e-4],
                [1.0, 0.0, 1e-4],
                [0.0, 1.0, 1e-4],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 5, 4]], dtype=np.int64),
        metadata={},
    )
    assert default_sdk.select_overlapping_faces(far) == []


def test_graph_cut_select_region_matches_meshlib_source_sink_edge_length_cut_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [5.0, 5.0, 0.0],
                [0.0, 1.0, 0.0],
                [10.0, 1.0, 0.0],
                [5.0, 5.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [1, 0, 3], [0, 3, 4], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )

    assert graph_cut_select_region(mesh, source_face_ids=[0], sink_face_ids=[3], boundary_weight=1.0) == [0, 1]
    assert default_sdk.graph_cut_select_region(mesh, source_face_ids=[0], sink_face_ids=[3]) == [0, 1]


def test_graph_cut_select_region_auto_not_region_matches_meshinspector_uncertainty_workflow() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [5.0, 5.0, 0.0],
                [0.0, 1.0, 0.0],
                [10.0, 1.0, 0.0],
                [5.0, 5.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [1, 0, 3], [0, 3, 4], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )

    expected = [0, 1]
    assert (
        graph_cut_select_region_auto_not_region(
            mesh,
            source_face_ids=[0],
            uncertainty_distance_mm=12.0,
            boundary_weight=1.0,
        )
        == expected
    )
    assert (
        sdk_graph_cut_select_region_auto_not_region(
            mesh,
            source_face_ids=[0],
            uncertainty_distance_mm=12.0,
        )
        == expected
    )
    assert (
        default_sdk.graph_cut_select_region_auto_not_region(
            mesh,
            source_face_ids=[0],
            uncertainty_distance_mm=12.0,
        )
        == expected
    )


def test_graph_cut_select_region_matches_meshinspector_curvature_preference() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, -1.0],
                [2.0, 1.0, 0.0],
                [2.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [1, 0, 3], [3, 0, 4], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )

    assert graph_cut_select_region(mesh, source_face_ids=[0], sink_face_ids=[3]) == [0, 1]
    assert (
        graph_cut_select_region(
            mesh,
            source_face_ids=[0],
            sink_face_ids=[3],
            curvature_preference="convex",
        )
        == [0]
    )
    assert (
        default_sdk.graph_cut_select_region(
            mesh,
            source_face_ids=[0],
            sink_face_ids=[3],
            curvature_preference="concave",
        )
        == [0, 1, 2]
    )


def test_select_faces_by_screen_polygon_matches_meshlib_lasso_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    view_projection = np.eye(4, dtype=np.float64).reshape(-1).tolist()
    polygon = [[-1.0, -1.0], [-0.05, -1.0], [-0.05, 1.0], [-1.0, 1.0]]

    assert select_faces_by_screen_polygon(mesh, view_projection, polygon) == [0]
    assert sdk_select_faces_by_screen_polygon(mesh, view_projection, polygon) == [0]
    assert default_sdk.select_faces_by_screen_polygon(mesh, view_projection, polygon) == [0]

    backface = MeshDocument(vertices=mesh.vertices[:3].copy(), faces=np.asarray([[0, 2, 1]], dtype=np.int64), metadata={})
    assert select_faces_by_screen_polygon(backface, view_projection, polygon, include_backfaces=True) == [0]
    assert select_faces_by_screen_polygon(backface, view_projection, polygon, include_backfaces=False) == []


def test_select_faces_by_screen_rect_matches_meshlib_rect_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    view_projection = np.eye(4, dtype=np.float64).reshape(-1).tolist()

    assert select_faces_by_screen_rect(mesh, view_projection, [-1.0, -1.0], [-0.05, 1.0]) == [0]
    assert sdk_select_faces_by_screen_rect(mesh, view_projection, [-1.0, -1.0], [-0.05, 1.0]) == [0]
    assert default_sdk.select_faces_by_screen_rect(mesh, view_projection, [-1.0, -1.0], [-0.05, 1.0]) == [0]
    assert select_faces_by_screen_rect(mesh, view_projection, [-0.05, -1.0], [1.0, 1.0]) == [1]

    backface = MeshDocument(vertices=mesh.vertices[:3].copy(), faces=np.asarray([[0, 2, 1]], dtype=np.int64), metadata={})
    assert select_faces_by_screen_rect(backface, view_projection, [-1.0, -1.0], [-0.05, 1.0], include_backfaces=True) == [0]
    assert select_faces_by_screen_rect(backface, view_projection, [-1.0, -1.0], [-0.05, 1.0], include_backfaces=False) == []


def test_select_faces_by_screen_brush_matches_meshlib_near_polygon_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    view_projection = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]
    brush_path = [[-0.9, -0.7], [-0.9, 0.7]]

    assert select_faces_by_screen_brush(mesh, view_projection, brush_path, radius_px=0.12) == [0]
    assert sdk_select_faces_by_screen_brush(mesh, view_projection, brush_path, radius_px=0.12) == [0]
    assert default_sdk.select_faces_by_screen_brush(mesh, view_projection, brush_path, radius_px=0.12) == [0]
    assert select_faces_by_screen_brush(mesh, view_projection, brush_path, radius_px=0.05) == []

    backface = MeshDocument(vertices=mesh.vertices[:3], faces=np.asarray([[0, 2, 1]], dtype=np.int64), metadata={})
    assert select_faces_by_screen_brush(backface, view_projection, brush_path, radius_px=0.12, include_backfaces=True) == [0]
    assert select_faces_by_screen_brush(backface, view_projection, brush_path, radius_px=0.12, include_backfaces=False) == []


def test_select_face_by_ray_matches_meshlib_pick_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )

    assert select_face_by_ray(mesh, [-0.5, -0.5, 1.0], [0.0, 0.0, -1.0]) == [0]
    assert sdk_select_face_by_ray(mesh, [-0.5, -0.5, 1.0], [0.0, 0.0, -1.0]) == [0]
    assert default_sdk.select_face_by_ray(mesh, [-0.5, -0.5, 1.0], [0.0, 0.0, -1.0]) == [0]
    assert select_face_by_ray(mesh, [-0.5, -0.5, 1.0], [0.0, 0.0, -1.0], ignore_faces=[0]) == []
    assert select_face_by_ray(mesh, [0.5, -0.5, 1.0], [0.0, 0.0, -1.0]) == [1]


def test_select_inside_part_faces_matches_meshlib_winding_self_intersection_contract() -> None:
    outer = cube(size=4.0)
    inner = cube(size=1.0)
    mesh = MeshDocument(
        vertices=np.vstack([outer.vertices, inner.vertices]),
        faces=np.vstack([outer.faces, inner.faces + outer.vertex_count]),
        metadata={},
    )

    expected = list(range(12, 24))
    assert select_inside_part_faces(mesh) == expected
    assert sdk_select_inside_part_faces(mesh) == expected
    assert default_sdk.select_inside_part_faces(mesh) == expected


def test_select_camera_facing_faces_matches_meshinspector_view_direction_contract() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, -1.0, -1.0],
                [0.0, 1.0, -1.0],
                [0.0, 1.0, 1.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 5, 4], [6, 7, 8]], dtype=np.int64),
        metadata={},
    )

    assert select_camera_facing_faces(mesh, camera_direction=[0.0, 0.0, -1.0]) == [0]
    assert sdk_select_camera_facing_faces(mesh, camera_direction=[0.0, 0.0, 1.0]) == [1]
    assert default_sdk.select_camera_facing_faces(mesh, camera_direction=[0.0, 0.0, -1.0], min_dot=0.5) == [0]
    with pytest.raises(ValueError, match="camera_direction"):
        select_camera_facing_faces(mesh, camera_direction=[0.0, 0.0, 0.0])


def test_select_not_smooth_faces_matches_meshlib_neighbor_angle_contract() -> None:
    mesh = closed_cube_with_flipped_top_triangle()

    expected = [2, 3]
    assert select_not_smooth_faces(mesh, min_angle_radians=0.3) == expected
    assert sdk_select_not_smooth_faces(mesh, min_angle_radians=0.3) == expected
    assert default_sdk.select_not_smooth_faces(mesh, min_angle_radians=0.3) == expected


def test_select_faces_by_screen_polygon_samples_large_triangles_like_meshlib() -> None:
    mesh = MeshDocument(
        vertices=np.asarray([[-0.9, -0.9, 0.0], [0.9, -0.9, 0.0], [0.0, 0.9, 0.0]], dtype=np.float64),
        faces=np.asarray([[0, 1, 2]], dtype=np.int64),
        metadata={},
    )
    view_projection = np.eye(4, dtype=np.float64).reshape(-1).tolist()
    polygon = [[-0.1, -0.1], [0.1, -0.1], [0.1, 0.1], [-0.1, 0.1]]

    assert select_faces_by_screen_polygon(mesh, view_projection, polygon) == [0]


def test_vertex_normals_match_vertex_count() -> None:
    mesh = cube(size=1.0)
    normals = vertex_normals(mesh)

    assert normals.shape == mesh.vertices.shape
    assert np.all(np.isfinite(normals))


def test_mesh_ply_import_exposes_meshlib_uv_and_color_metadata() -> None:
    mesh = mesh_from_ply(
        b"ply\n"
        b"format ascii 1.0\n"
        b"comment TextureFile jewel_surface.jpg\n"
        b"element vertex 3\n"
        b"property float x\n"
        b"property float y\n"
        b"property float z\n"
        b"property float texture_u\n"
        b"property float texture_v\n"
        b"property float u\n"
        b"property float v\n"
        b"property uchar r\n"
        b"property uchar g\n"
        b"property uchar b\n"
        b"element face 1\n"
        b"property list uchar int vertex_indices\n"
        b"end_header\n"
        b"0 0 0 9.1 9.2 0.1 0.2 10 20 30\n"
        b"1 0 0 9.3 9.4 0.3 0.4 40 50 60\n"
        b"0 1 0 9.5 9.6 0.5 0.6 70 80 90\n"
        b"3 0 1 2\n"
    )

    np.testing.assert_allclose(mesh.vertices, [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
    assert mesh.faces.tolist() == [[0, 1, 2]]
    assert mesh.metadata["texture_files"] == ["jewel_surface.jpg"]
    np.testing.assert_allclose(mesh.metadata["vertex_uvs"], [[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]])
    assert mesh.metadata["vertex_colors"] == [[10, 20, 30, 255], [40, 50, 60, 255], [70, 80, 90, 255]]


def test_mesh_ply_import_exposes_binary_meshlib_metadata() -> None:
    source = _binary_meshlib_ply_fixture()

    mesh = mesh_from_ply(source)

    np.testing.assert_allclose(mesh.vertices, [[0.125, 0.0, 0.0], [1.25, 0.0, 0.0], [0.0, 1.5, 0.0]])
    assert mesh.faces.tolist() == [[0, 1, 2]]
    assert mesh.metadata["texture_files"] == ["binary_surface.png"]
    np.testing.assert_allclose(mesh.metadata["vertex_uvs"], [[0.125, 0.25], [0.375, 0.5], [0.625, 0.75]])
    assert mesh.metadata["vertex_colors"] == [[10, 20, 30, 255], [40, 50, 60, 255], [70, 80, 90, 255]]
    assert mesh.metadata["face_colors"] == [[1, 2, 3, 255]]
    np.testing.assert_allclose(mesh.metadata["tri_corner_uvs"], [[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]])


def test_save_mesh_preserves_meshlib_vertex_uvs_through_ply_and_glb_preview(tmp_path) -> None:
    mesh = MeshDocument(
        vertices=np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.asarray([[0, 1, 2]], dtype=np.int64),
        metadata={
            "texture_files": ["jewel_surface.png"],
            "vertex_uvs": [[0.125, 0.25], [0.375, 0.5], [0.625, 0.75]],
        },
    )

    ply_path = default_sdk.save_mesh(mesh, tmp_path / "textured_jewel.ply", file_type="ply")
    reloaded = default_sdk.load_mesh(ply_path)

    assert reloaded.metadata["texture_files"] == ["jewel_surface.png"]
    np.testing.assert_allclose(reloaded.metadata["vertex_uvs"], mesh.metadata["vertex_uvs"])

    glb_path = default_sdk.save_mesh(reloaded, tmp_path / "textured_jewel.glb", file_type="glb")
    preview = trimesh.load(str(glb_path), force="mesh")

    assert isinstance(preview.visual, trimesh.visual.texture.TextureVisuals)
    np.testing.assert_allclose(preview.visual.uv, mesh.metadata["vertex_uvs"], atol=1e-6)


def test_save_mesh_preserves_meshlib_tri_corner_uvs_in_ply_and_flattens_preview_uvs(tmp_path) -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
        metadata={
            "texture_files": ["checker.png"],
            "tri_corner_uvs": [
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            ],
        },
    )

    ply_path = default_sdk.save_mesh(mesh, tmp_path / "corner_textured.ply", file_type="ply")
    reloaded = default_sdk.load_mesh(ply_path)

    assert reloaded.vertex_count == 4
    assert reloaded.face_count == 2
    assert reloaded.metadata["texture_files"] == ["checker.png"]
    np.testing.assert_allclose(reloaded.metadata["tri_corner_uvs"], mesh.metadata["tri_corner_uvs"])

    glb_path = default_sdk.save_mesh(reloaded, tmp_path / "corner_textured.glb", file_type="glb")
    preview = trimesh.load(str(glb_path), force="mesh")

    assert preview.vertices.shape[0] == 6
    assert isinstance(preview.visual, trimesh.visual.texture.TextureVisuals)
    np.testing.assert_allclose(
        preview.visual.uv,
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        atol=1e-6,
    )


def test_mesh_ply_import_packs_polygon_texcoord_lists_like_meshlib() -> None:
    mesh = mesh_from_ply(
        b"ply\n"
        b"format ascii 1.0\n"
        b"element vertex 4\n"
        b"property float x\n"
        b"property float y\n"
        b"property float z\n"
        b"element face 1\n"
        b"property list uchar int vertex_indices\n"
        b"property list uchar float texcoord\n"
        b"end_header\n"
        b"0 0 0\n"
        b"1 0 0\n"
        b"1 1 0\n"
        b"0 1 0\n"
        b"4 0 1 2 3 8 0.0 0.0 1.0 0.0 1.0 1.0 0.0 1.0\n"
    )

    assert mesh.faces.tolist() == [[0, 1, 2], [0, 2, 3]]
    np.testing.assert_allclose(
        mesh.metadata["tri_corner_uvs"],
        [
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [[0.0, 1.0], [0.0, 0.0], [0.0, 0.0]],
        ],
    )


def test_mesh_ply_import_keeps_polygon_face_colors_per_meshlib_source_face_row() -> None:
    mesh = mesh_from_ply(
        b"ply\n"
        b"format ascii 1.0\n"
        b"element vertex 4\n"
        b"property float x\n"
        b"property float y\n"
        b"property float z\n"
        b"element face 1\n"
        b"property list uchar int vertex_indices\n"
        b"property uchar red\n"
        b"property uchar green\n"
        b"property uchar blue\n"
        b"end_header\n"
        b"0 0 0\n"
        b"1 0 0\n"
        b"1 1 0\n"
        b"0 1 0\n"
        b"4 0 1 2 3 7 8 9\n"
    )

    assert mesh.faces.tolist() == [[0, 1, 2], [0, 2, 3]]
    assert mesh.metadata["face_colors"] == [[7, 8, 9, 255]]


def test_default_sdk_load_mesh_routes_ply_uploads_through_rust_meshlib_parser(tmp_path) -> None:
    source_path = tmp_path / "jewel_binary.ply"
    source_path.write_bytes(_binary_meshlib_ply_fixture())

    mesh = default_sdk.load_mesh(source_path)

    np.testing.assert_allclose(mesh.vertices, [[0.125, 0.0, 0.0], [1.25, 0.0, 0.0], [0.0, 1.5, 0.0]])
    assert mesh.faces.tolist() == [[0, 1, 2]]
    assert mesh.metadata["source"] == "rust_mesh_from_ply"
    assert mesh.metadata["meshlib_reference"] == "MR::loadPly"
    assert mesh.metadata["source_path"] == str(source_path)
    assert mesh.metadata["texture_files"] == ["binary_surface.png"]
    np.testing.assert_allclose(mesh.metadata["vertex_uvs"], [[0.125, 0.25], [0.375, 0.5], [0.625, 0.75]])
    assert mesh.metadata["vertex_colors"] == [[10, 20, 30, 255], [40, 50, 60, 255], [70, 80, 90, 255]]


def test_mesh_obj_import_triangulates_meshlib_negative_index_quad() -> None:
    assert hasattr(mesh_core, "mesh_from_obj")
    mesh = mesh_core.mesh_from_obj(
        b"o relative_quad\n"
        b"v 0 0 0\n"
        b"v 1 0 0\n"
        b"v 1 1 0\n"
        b"v 0 1 0\n"
        b"f -4 -3 -2 -1\n"
    )

    np.testing.assert_allclose(
        mesh.vertices,
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
    )
    assert mesh.faces.tolist() == [[0, 1, 2], [0, 2, 3]]
    assert mesh.metadata["source"] == "rust_mesh_from_obj"
    assert mesh.metadata["meshlib_reference"] == "MR::MeshLoad::fromSceneObjFile"
    assert mesh.metadata["object_names"] == ["relative_quad"]


def test_default_sdk_load_mesh_routes_obj_uploads_through_rust_meshlib_parser(tmp_path) -> None:
    source_path = tmp_path / "relative_quad.obj"
    source_path.write_text(
        "v 0 0 0\n"
        "v 1 0 0\n"
        "v 1 1 0\n"
        "v 0 1 0\n"
        "f -4 -3 -2 -1\n",
        encoding="utf-8",
    )

    mesh = default_sdk.load_mesh(source_path)

    assert mesh.faces.tolist() == [[0, 1, 2], [0, 2, 3]]
    assert mesh.metadata["source"] == "rust_mesh_from_obj"
    assert mesh.metadata["meshlib_reference"] == "MR::MeshLoad::fromSceneObjFile"
    assert mesh.metadata["source_path"] == str(source_path)


def test_mesh_obj_import_loads_meshlib_mtl_diffuse_texture_metadata(tmp_path) -> None:
    (tmp_path / "jewel.mtl").write_text(
        "newmtl polished_gold\n"
        "Kd 0.2 0.4 0.6\n"
        "map_Kd -clamp on albedo.png\n",
        encoding="utf-8",
    )
    mesh = mesh_core.mesh_from_obj(
        b"mtllib jewel.mtl\n"
        b"usemtl polished_gold\n"
        b"v 0 0 0\n"
        b"v 1 0 0\n"
        b"v 1 1 0\n"
        b"v 0 1 0\n"
        b"f -4 -3 -2 -1\n",
        material_dir=tmp_path,
    )

    assert mesh.faces.tolist() == [[0, 1, 2], [0, 2, 3]]
    assert mesh.metadata["diffuse_color"] == [51, 102, 153, 255]
    assert mesh.metadata["texture_files"] == ["albedo.png"]
    assert mesh.metadata["texture_per_face"] == [0, 0]
    assert mesh.metadata["material_names"] == ["polished_gold"]


def test_default_sdk_load_mesh_routes_obj_mtl_metadata_through_rust_parser(tmp_path) -> None:
    source_path = tmp_path / "textured_quad.obj"
    (tmp_path / "jewel.mtl").write_text(
        "newmtl polished_gold\n"
        "Kd 0.2 0.4 0.6\n"
        "map_Kd albedo.png\n",
        encoding="utf-8",
    )
    source_path.write_text(
        "mtllib jewel.mtl\n"
        "usemtl polished_gold\n"
        "v 0 0 0\n"
        "v 1 0 0\n"
        "v 1 1 0\n"
        "v 0 1 0\n"
        "f -4 -3 -2 -1\n",
        encoding="utf-8",
    )

    mesh = default_sdk.load_mesh(source_path)

    assert mesh.metadata["source"] == "rust_mesh_from_obj"
    assert mesh.metadata["source_path"] == str(source_path)
    assert mesh.metadata["diffuse_color"] == [51, 102, 153, 255]
    assert mesh.metadata["texture_files"] == ["albedo.png"]
    assert mesh.metadata["texture_per_face"] == [0, 0]


def test_default_sdk_load_mesh_loads_meshlib_obj_map_kd_texture_image(tmp_path) -> None:
    source_path = tmp_path / "textured_quad.obj"
    (tmp_path / "albedo.png").write_bytes(OPAQUE_WHITE_PNG)
    (tmp_path / "jewel.mtl").write_text(
        "newmtl polished_gold\n"
        "Kd 0.2 0.4 0.6\n"
        "map_Kd -clamp on albedo.png\n",
        encoding="utf-8",
    )
    source_path.write_text(
        "mtllib jewel.mtl\n"
        "usemtl polished_gold\n"
        "v 0 0 0\n"
        "v 1 0 0\n"
        "v 1 1 0\n"
        "v 0 1 0\n"
        "f -4 -3 -2 -1\n",
        encoding="utf-8",
    )

    mesh = default_sdk.load_mesh(source_path)

    assert mesh.metadata["texture_files"] == ["albedo.png"]
    assert mesh.metadata["texture_per_face"] == [0, 0]
    assert mesh.metadata["texture_images"] == [
        {
            "file": "albedo.png",
            "resolved_path": str(tmp_path / "albedo.png"),
            "width": 1,
            "height": 1,
            "filter": "Linear",
            "wrap": "Clamp",
            "pixels_rgba": [[255, 255, 255, 255]],
        }
    ]


def test_default_sdk_load_mesh_routes_obj_vt_uvs_into_glb_preview(tmp_path) -> None:
    source_path = tmp_path / "textured_quad.obj"
    (tmp_path / "albedo.png").write_bytes(OPAQUE_WHITE_PNG)
    (tmp_path / "jewel.mtl").write_text(
        "newmtl polished_gold\n"
        "map_Kd albedo.png\n",
        encoding="utf-8",
    )
    source_path.write_text(
        "mtllib jewel.mtl\n"
        "usemtl polished_gold\n"
        "v 0 0 0\n"
        "v 1 0 0\n"
        "v 1 1 0\n"
        "v 0 1 0\n"
        "vt 0.0 0.0\n"
        "vt 1.0 0.0\n"
        "vt 1.0 1.0\n"
        "vt 0.0 1.0\n"
        "f 1/1 2/2 3/3 4/4\n",
        encoding="utf-8",
    )

    mesh = default_sdk.load_mesh(source_path)

    assert mesh.metadata["texture_files"] == ["albedo.png"]
    assert mesh.metadata["texture_per_face"] == [0, 0]
    np.testing.assert_allclose(
        mesh.metadata["tri_corner_uvs"],
        [
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ],
    )

    glb_path = default_sdk.save_mesh(mesh, tmp_path / "textured_quad.glb", file_type="glb")
    preview = trimesh.load(str(glb_path), force="mesh")

    assert isinstance(preview.visual, trimesh.visual.texture.TextureVisuals)
    np.testing.assert_allclose(
        preview.visual.uv,
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        atol=1e-6,
    )


def test_default_sdk_load_mesh_exposes_meshlib_ply_normals_and_edges(tmp_path) -> None:
    source_path = tmp_path / "wireframe_normals.ply"
    source_path.write_text(
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property double nx\n"
        "property double ny\n"
        "property double nz\n"
        "element face 1\n"
        "property list uchar int vertex_indices\n"
        "element edge 2\n"
        "property short vertex1\n"
        "property uint vertex2\n"
        "end_header\n"
        "0 0 0 0.0 0.0 1.0\n"
        "1 0 0 0.0 0.0 1.0\n"
        "0 1 0 0.0 0.0 1.0\n"
        "3 0 1 2\n"
        "0 1\n"
        "1 2\n",
        encoding="utf-8",
    )

    mesh = default_sdk.load_mesh(source_path)

    np.testing.assert_allclose(mesh.metadata["vertex_normals_ply"], [[0.0, 0.0, 1.0]] * 3)
    assert mesh.metadata["edges"] == [[0, 1], [1, 2]]


def test_default_sdk_load_mesh_loads_first_existing_texture_like_meshlib_texturefile(tmp_path) -> None:
    source_path = tmp_path / "textured_jewel.ply"
    (tmp_path / "jewel_surface.png").write_bytes(OPAQUE_WHITE_PNG)
    (tmp_path / "ignored_surface.png").write_bytes(OPAQUE_WHITE_PNG)
    source_path.write_text(
        "ply\n"
        "format ascii 1.0\n"
        "comment TextureFile missing_surface.png\n"
        "comment TextureFile jewel_surface.png\n"
        "comment TextureFile ignored_surface.png\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element face 1\n"
        "property list uchar int vertex_indices\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1 0\n"
        "3 0 1 2\n",
        encoding="utf-8",
    )

    mesh = default_sdk.load_mesh(source_path)

    assert mesh.metadata["texture_files"] == ["missing_surface.png", "jewel_surface.png", "ignored_surface.png"]
    assert mesh.metadata["texture_images"] == [
        {
            "file": "jewel_surface.png",
            "resolved_path": str(tmp_path / "jewel_surface.png"),
            "width": 1,
            "height": 1,
            "filter": "Linear",
            "wrap": "Clamp",
            "pixels_rgba": [[255, 255, 255, 255]],
        }
    ]


def test_default_sdk_load_mesh_trims_meshlib_texturefile_comment_trailing_spaces(tmp_path) -> None:
    source_path = tmp_path / "textured_jewel_trailing_space.ply"
    (tmp_path / "jewel_surface.png").write_bytes(OPAQUE_WHITE_PNG)
    source_path.write_text(
        "ply\n"
        "format ascii 1.0\n"
        "comment TextureFile jewel_surface.png   \n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element face 1\n"
        "property list uchar int vertex_indices\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1 0\n"
        "3 0 1 2\n",
        encoding="utf-8",
    )

    mesh = default_sdk.load_mesh(source_path)

    assert mesh.metadata["texture_files"] == ["jewel_surface.png"]
    assert mesh.metadata["texture_images"] == [
        {
            "file": "jewel_surface.png",
            "resolved_path": str(tmp_path / "jewel_surface.png"),
            "width": 1,
            "height": 1,
            "filter": "Linear",
            "wrap": "Clamp",
            "pixels_rgba": [[255, 255, 255, 255]],
        }
    ]


def _binary_meshlib_ply_fixture() -> bytes:
    source = bytearray(
        b"ply\n"
        b"format binary_little_endian 1.0\n"
        b"comment TextureFile binary_surface.png\n"
        b"element vertex 3\n"
        b"property double x\n"
        b"property double y\n"
        b"property double z\n"
        b"property float s\n"
        b"property float t\n"
        b"property uchar red\n"
        b"property uchar green\n"
        b"property uchar blue\n"
        b"element face 1\n"
        b"property list uchar int vertex_indices\n"
        b"property uchar red\n"
        b"property uchar green\n"
        b"property uchar blue\n"
        b"property list uchar float texcoord\n"
        b"end_header\n"
    )
    for point, uv, color in [
        ((0.125, 0.0, 0.0), (0.125, 0.25), (10, 20, 30)),
        ((1.25, 0.0, 0.0), (0.375, 0.5), (40, 50, 60)),
        ((0.0, 1.5, 0.0), (0.625, 0.75), (70, 80, 90)),
    ]:
        source.extend(struct.pack("<dddffBBB", *point, *uv, *color))
    source.extend(struct.pack("<BiiiBBBBffffff", 3, 0, 1, 2, 1, 2, 3, 6, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0))
    return bytes(source)


def test_health_reports_self_intersections_for_crossing_faces() -> None:
    health = compute_mesh_health(crossing_triangles())

    assert health.self_intersections_available
    assert health.self_intersections == 2


def test_health_can_skip_self_intersections_for_large_mesh_budget() -> None:
    health = compute_mesh_health(crossing_triangles(), max_self_intersection_faces=1)

    assert not health.self_intersections_available
    assert health.self_intersections is None
    assert not health.is_closed


def test_health_module_is_rust_owned(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = open_cube(size=2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    loops = boundary_loops(mesh)
    health = compute_mesh_health(mesh)

    assert len(loops) == 1
    assert health.boundary_edge_count == 4

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel mesh_health is required"):
        compute_mesh_health(mesh)
