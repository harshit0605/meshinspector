from __future__ import annotations

import math

import numpy as np
import pytest

from geometry_sdk import GeometrySDK, MeshDocument
from geometry_sdk.point_cloud import (
    MultiwayICPRegistrationResult,
    PointCloudDocument,
    multiway_all_object_combined_icp,
    multiway_all_object_point_to_plane_icp,
    multiway_all_object_point_to_point_icp,
    multiway_aabb_cascade_combined_icp,
    multiway_aabb_cascade_point_to_plane_icp,
    multiway_aabb_cascade_point_to_point_icp,
    multiway_combined_icp,
    multiway_sequential_cascade_combined_icp,
    multiway_sequential_cascade_point_to_plane_icp,
    multiway_sequential_cascade_point_to_point_icp,
    point_cloud_triangulate_filled_candidate_mesh,
    point_cloud_local_fan_triangles,
    multiway_point_to_plane_icp,
    multiway_point_to_point_icp,
    pairwise_point_to_plane_icp,
    pairwise_point_to_point_icp,
    point_cloud_local_neighbor_fan,
    point_cloud_local_triangulation_repetitions,
    point_cloud_extract_selected_points_as_object,
    point_cloud_n_closest_neighbors,
    point_cloud_nearest_projections,
    point_cloud_pick_by_ray,
    point_cloud_select_by_screen_brush,
    point_cloud_select_by_screen_polygon,
    point_cloud_select_by_screen_rect,
    point_cloud_neighbors_in_radius,
    point_cloud_project_to_mesh,
    point_cloud_two_closest_points,
    point_cloud_triangulate_candidate_mesh,
    point_cloud_triangulate_cleaned_candidate_mesh,
    point_cloud_triangulate_topology_candidate_mesh,
    point_cloud_grid_sample,
    load_point_cloud_ply,
    save_point_cloud_ply,
    point_cloud_uniform_sample,
)


def transformed_reference_points() -> tuple[PointCloudDocument, PointCloudDocument, float, np.ndarray]:
    angle = 0.15
    cos = math.cos(angle)
    sin = math.sin(angle)
    translation = np.array([0.03, -0.02, 0.04], dtype=np.float64)
    reference = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.1],
            [0.2, 0.9, -0.2],
            [-0.1, 0.2, 1.1],
            [0.7, 0.4, 0.8],
        ],
        dtype=np.float64,
    )
    rotation = np.array(
        [
            [cos, -sin, 0.0],
            [sin, cos, 0.0],
            [0.0, 0.0, 1.0],
        ],
        dtype=np.float64,
    )
    floating = reference @ rotation.T + translation
    return PointCloudDocument(floating), PointCloudDocument(reference), angle, translation


def test_point_cloud_ply_io_routes_through_rust_and_preserves_metadata(tmp_path) -> None:
    source = (
        b"ply\nformat ascii 1.0\ncomment MeshInspector.com\n"
        b"element vertex 2\n"
        b"property float x\nproperty float y\nproperty float z\n"
        b"property float nx\nproperty float ny\nproperty float nz\n"
        b"property uchar red\nproperty uchar green\nproperty uchar blue\n"
        b"end_header\n"
        b"0.0 1.0 2.0 0.0 0.0 1.0 255 128 0\n"
        b"3.0 4.0 5.0 0.0 1.0 0.0 4 5 6\n"
    )
    path = tmp_path / "points.ply"
    path.write_bytes(source)

    cloud = load_point_cloud_ply(path)

    assert cloud.metadata["source"] == "rust_point_cloud_from_ply"
    np.testing.assert_allclose(cloud.points, np.array([[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]]))
    assert cloud.metadata["normals"] == [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0]]
    assert cloud.metadata["point_colors"] == [[255, 128, 0], [4, 5, 6]]

    output_path = tmp_path / "roundtrip.ply"
    save_point_cloud_ply(cloud, output_path)
    reloaded = load_point_cloud_ply(output_path)

    np.testing.assert_allclose(reloaded.points, cloud.points)
    assert reloaded.metadata["normals"] == cloud.metadata["normals"]
    assert reloaded.metadata["point_colors"] == cloud.metadata["point_colors"]


def test_pairwise_point_to_point_icp_recovers_meshlib_style_rigid_transform() -> None:
    floating, reference, angle, _translation = transformed_reference_points()

    result = pairwise_point_to_point_icp(floating, reference, max_iterations=25, tolerance=1e-12)

    assert result.method == "point_to_point"
    assert result.mode == "rigid"
    assert result.active_pair_count == floating.point_count
    assert result.mean_square_distance < 1e-12
    expected_inverse = np.array(
        [
            [math.cos(angle), math.sin(angle), 0.0],
            [-math.sin(angle), math.cos(angle), 0.0],
            [0.0, 0.0, 1.0],
        ],
        dtype=np.float64,
    )
    np.testing.assert_allclose(result.rotation, expected_inverse, atol=1e-8)
    np.testing.assert_allclose(result.apply(floating).points, reference.points, atol=1e-8)


def test_pairwise_point_to_point_icp_supports_translation_only_mode() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.2, 0.1, 1.0],
            ],
            dtype=np.float64,
        )
    )
    translation = np.array([0.25, -0.1, 0.05], dtype=np.float64)
    floating = PointCloudDocument(reference.points + translation)

    result = pairwise_point_to_point_icp(
        floating,
        reference,
        max_iterations=10,
        tolerance=1e-12,
        mode="translation",
    )

    assert result.mode == "translation"
    np.testing.assert_allclose(result.rotation, np.eye(3), atol=1e-12)
    np.testing.assert_allclose(result.translation, -translation, atol=1e-10)
    np.testing.assert_allclose(result.apply(floating).points, reference.points, atol=1e-10)


def test_geometry_sdk_exposes_pairwise_point_cloud_icp() -> None:
    floating, reference, _angle, _translation = transformed_reference_points()

    result = GeometrySDK().pairwise_point_to_point_icp(floating, reference, max_iterations=25)

    assert result.mean_square_distance < 1e-12
    np.testing.assert_allclose(result.apply(floating).points, reference.points, atol=1e-8)


def test_multiway_point_to_point_icp_fixes_last_object_like_meshlib() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.1],
                [0.2, 0.9, -0.2],
                [-0.1, 0.2, 1.1],
                [0.7, 0.4, 0.8],
            ],
            dtype=np.float64,
        )
    )
    first_offset = np.array([0.3, -0.06, 0.04], dtype=np.float64)
    second_offset = np.array([0.1, -0.02, 0.01], dtype=np.float64)
    objects = (
        PointCloudDocument(reference.points + first_offset),
        PointCloudDocument(reference.points + second_offset),
        reference,
    )

    result = multiway_point_to_point_icp(
        objects,
        max_iterations=60,
        tolerance=1e-18,
        mode="translation",
    )
    sdk_result = GeometrySDK().multiway_point_to_point_icp(
        objects,
        max_iterations=60,
        tolerance=1e-18,
        mode="translation",
    )

    assert isinstance(result, MultiwayICPRegistrationResult)
    assert result.method == "point_to_point"
    assert result.mode == "translation"
    assert result.fixed_object_index == 2
    assert result.active_pair_count == reference.point_count * 6
    assert result.mean_square_distance < 1e-18
    assert len(result.transforms) == 3
    np.testing.assert_allclose(result.transforms[0].translation, -first_offset, atol=1e-8)
    np.testing.assert_allclose(result.transforms[1].translation, -second_offset, atol=1e-8)
    np.testing.assert_allclose(result.transforms[2].translation, np.zeros(3), atol=1e-12)
    for transform, cloud in zip(result.transforms, objects, strict=True):
        np.testing.assert_allclose(transform.apply(cloud).points, reference.points, atol=1e-8)
    np.testing.assert_allclose(
        sdk_result.transforms[0].translation,
        result.transforms[0].translation,
        atol=1e-12,
    )


def test_multiway_point_to_plane_icp_fixes_last_object_like_meshlib() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [1.0, 1.0, -5.0],
                [14.0, 1.0, 1.0],
                [1.0, 14.0, 2.0],
                [-11.0, 2.0, 3.0],
                [1.0, -11.0, 4.0],
                [1.0, 2.0, 8.0],
                [2.0, 1.0, -5.0],
                [15.0, 1.5, 1.0],
                [1.5, 15.0, 2.0],
                [-11.0, 2.5, 3.1],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [0.0, 0.0, -1.0],
            [1.0, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.0, 0.1, 1.0],
            [0.1, -1.1, 1.1],
            [0.1, 0.1, 1.0],
            [0.1, 0.0, -1.0],
            [1.1, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.1, 0.1, 1.1],
        ],
        dtype=np.float64,
    )
    first_offset = np.array([0.03, -0.02, 0.01], dtype=np.float64)
    second_offset = np.array([0.01, -0.005, 0.003], dtype=np.float64)
    objects = (
        PointCloudDocument(reference.points + first_offset),
        PointCloudDocument(reference.points + second_offset),
        reference,
    )
    object_normals = (normals, normals, normals)

    result = multiway_point_to_plane_icp(
        objects,
        object_normals,
        max_iterations=60,
        tolerance=1e-18,
        mode="translation",
    )
    sdk_result = GeometrySDK().multiway_point_to_plane_icp(
        objects,
        object_normals,
        max_iterations=60,
        tolerance=1e-18,
        mode="translation",
    )

    assert isinstance(result, MultiwayICPRegistrationResult)
    assert result.method == "point_to_plane"
    assert result.mode == "translation"
    assert result.fixed_object_index == 2
    assert result.active_pair_count == reference.point_count * 6
    assert result.mean_square_distance < 1e-18
    np.testing.assert_allclose(result.transforms[0].translation, -first_offset, atol=1e-8)
    np.testing.assert_allclose(result.transforms[1].translation, -second_offset, atol=1e-8)
    np.testing.assert_allclose(result.transforms[2].translation, np.zeros(3), atol=1e-12)
    for transform, cloud in zip(result.transforms, objects, strict=True):
        np.testing.assert_allclose(transform.apply(cloud).points, reference.points, atol=1e-8)
    np.testing.assert_allclose(
        sdk_result.transforms[1].translation,
        result.transforms[1].translation,
        atol=1e-12,
    )


def test_multiway_combined_icp_uses_meshlib_point_then_plane_schedule() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [1.0, 1.0, -5.0],
                [14.0, 1.0, 1.0],
                [1.0, 14.0, 2.0],
                [-11.0, 2.0, 3.0],
                [1.0, -11.0, 4.0],
                [1.0, 2.0, 8.0],
                [2.0, 1.0, -5.0],
                [15.0, 1.5, 1.0],
                [1.5, 15.0, 2.0],
                [-11.0, 2.5, 3.1],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [0.0, 0.0, -1.0],
            [1.0, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.0, 0.1, 1.0],
            [0.1, -1.1, 1.1],
            [0.1, 0.1, 1.0],
            [0.1, 0.0, -1.0],
            [1.1, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.1, 0.1, 1.1],
        ],
        dtype=np.float64,
    )
    first_offset = np.array([0.03, -0.02, 0.01], dtype=np.float64)
    second_offset = np.array([0.01, -0.005, 0.003], dtype=np.float64)
    objects = (
        PointCloudDocument(reference.points + first_offset),
        PointCloudDocument(reference.points + second_offset),
        reference,
    )
    object_normals = (normals, normals, normals)

    result = multiway_combined_icp(
        objects,
        object_normals,
        max_iterations=60,
        tolerance=1e-18,
        mode="translation",
    )
    sdk_result = GeometrySDK().multiway_combined_icp(
        objects,
        object_normals,
        max_iterations=60,
        tolerance=1e-18,
        mode="translation",
    )

    assert isinstance(result, MultiwayICPRegistrationResult)
    assert result.method == "combined"
    assert result.mode == "translation"
    assert result.fixed_object_index == 2
    assert result.active_pair_count == reference.point_count * 6
    assert result.iterations >= 2
    assert result.mean_square_distance < 1e-18
    np.testing.assert_allclose(result.transforms[0].translation, -first_offset, atol=1e-8)
    np.testing.assert_allclose(result.transforms[1].translation, -second_offset, atol=1e-8)
    np.testing.assert_allclose(result.transforms[2].translation, np.zeros(3), atol=1e-12)
    for transform, cloud in zip(result.transforms, objects, strict=True):
        np.testing.assert_allclose(transform.apply(cloud).points, reference.points, atol=1e-8)
    np.testing.assert_allclose(
        sdk_result.transforms[0].translation,
        result.transforms[0].translation,
        atol=1e-12,
    )


def test_multiway_all_object_icp_exposes_meshlib_global_system_modes() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [1.0, 1.0, -5.0],
                [14.0, 1.0, 1.0],
                [1.0, 14.0, 2.0],
                [-11.0, 2.0, 3.0],
                [1.0, -11.0, 4.0],
                [1.0, 2.0, 8.0],
                [2.0, 1.0, -5.0],
                [15.0, 1.5, 1.0],
                [1.5, 15.0, 2.0],
                [-11.0, 2.5, 3.1],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [0.0, 0.0, -1.0],
            [1.0, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.0, 0.1, 1.0],
            [0.1, -1.1, 1.1],
            [0.1, 0.1, 1.0],
            [0.1, 0.0, -1.0],
            [1.1, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.1, 0.1, 1.1],
        ],
        dtype=np.float64,
    )
    first_offset = np.array([0.03, -0.02, 0.01], dtype=np.float64)
    second_offset = np.array([0.01, -0.005, 0.003], dtype=np.float64)
    objects = (
        PointCloudDocument(reference.points + first_offset),
        PointCloudDocument(reference.points + second_offset),
        reference,
    )
    object_normals = (normals, normals, normals)

    point_result = multiway_all_object_point_to_point_icp(
        objects,
        max_iterations=60,
        tolerance=1e-18,
    )
    plane_result = multiway_all_object_point_to_plane_icp(
        objects,
        object_normals,
        max_iterations=60,
        tolerance=1e-18,
    )
    combined_result = GeometrySDK().multiway_all_object_combined_icp(
        objects,
        object_normals,
        max_iterations=60,
        tolerance=1e-18,
    )

    assert point_result.method == "point_to_point_all_object"
    assert plane_result.method == "point_to_plane_all_object"
    assert combined_result.method == "combined_all_object"
    for result in (point_result, plane_result, combined_result):
        assert isinstance(result, MultiwayICPRegistrationResult)
        assert result.fixed_object_index == 2
        assert result.active_pair_count == reference.point_count * 6
        assert result.mean_square_distance < 1e-16
        np.testing.assert_allclose(result.transforms[0].translation, -first_offset, atol=1e-7)
        np.testing.assert_allclose(result.transforms[1].translation, -second_offset, atol=1e-7)
        np.testing.assert_allclose(result.transforms[2].translation, np.zeros(3), atol=1e-12)


def test_multiway_sequential_cascade_icp_exposes_meshlib_max_group_size_modes() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [1.0, 1.0, -5.0],
                [14.0, 1.0, 1.0],
                [1.0, 14.0, 2.0],
                [-11.0, 2.0, 3.0],
                [1.0, -11.0, 4.0],
                [1.0, 2.0, 8.0],
                [2.0, 1.0, -5.0],
                [15.0, 1.5, 1.0],
                [1.5, 15.0, 2.0],
                [-11.0, 2.5, 3.1],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [0.0, 0.0, -1.0],
            [1.0, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.0, 0.1, 1.0],
            [0.1, -1.1, 1.1],
            [0.1, 0.1, 1.0],
            [0.1, 0.0, -1.0],
            [1.1, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.1, 0.1, 1.1],
        ],
        dtype=np.float64,
    )
    offsets = (
        np.array([0.03, -0.02, 0.01], dtype=np.float64),
        np.array([0.01, -0.005, 0.003], dtype=np.float64),
        np.array([-0.02, 0.015, -0.004], dtype=np.float64),
        np.zeros(3, dtype=np.float64),
    )
    objects = tuple(PointCloudDocument(reference.points + offset) for offset in offsets)
    object_normals = (normals, normals, normals, normals)

    point_result = multiway_sequential_cascade_point_to_point_icp(
        objects,
        max_group_size=2,
        max_iterations=60,
        tolerance=1e-18,
    )
    plane_result = multiway_sequential_cascade_point_to_plane_icp(
        objects,
        object_normals,
        max_group_size=2,
        max_iterations=60,
        tolerance=1e-18,
    )
    combined_result = GeometrySDK().multiway_sequential_cascade_combined_icp(
        objects,
        object_normals,
        max_group_size=2,
        max_iterations=60,
        tolerance=1e-18,
    )

    assert point_result.method == "point_to_point_sequential_cascade"
    assert plane_result.method == "point_to_plane_sequential_cascade"
    assert combined_result.method == "combined_sequential_cascade"
    for result in (point_result, plane_result, combined_result):
        assert isinstance(result, MultiwayICPRegistrationResult)
        assert result.fixed_object_index == 3
        assert result.active_pair_count >= reference.point_count * 8
        assert result.mean_square_distance < 1e-16

    for result in (point_result,):
        for transform, cloud, _offset in zip(result.transforms, objects, offsets, strict=True):
            np.testing.assert_allclose(transform.apply(cloud).points, reference.points, atol=1e-7)


def test_multiway_aabb_cascade_icp_exposes_meshlib_aabb_tree_based_mode() -> None:
    spatial_reference = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.2],
                [0.2, 0.9, -0.2],
                [-0.1, 0.2, 1.1],
                [0.7, 0.4, 0.8],
            ],
            dtype=np.float64,
        )
    )
    spatial_offsets = (
        np.array([-5.1, 0.0, 0.0], dtype=np.float64),
        np.array([0.2, 0.0, 0.0], dtype=np.float64),
        np.array([-5.0, 0.0, 0.0], dtype=np.float64),
        np.zeros(3, dtype=np.float64),
    )
    spatial_objects = tuple(
        PointCloudDocument(spatial_reference.points + offset) for offset in spatial_offsets
    )
    point_result = multiway_aabb_cascade_point_to_point_icp(
        spatial_objects,
        max_group_size=2,
        max_iterations=80,
        tolerance=1e-18,
    )

    reference = PointCloudDocument(
        np.array(
            [
                [1.0, 1.0, -5.0],
                [14.0, 1.0, 1.0],
                [1.0, 14.0, 2.0],
                [-11.0, 2.0, 3.0],
                [1.0, -11.0, 4.0],
                [1.0, 2.0, 8.0],
                [2.0, 1.0, -5.0],
                [15.0, 1.5, 1.0],
                [1.5, 15.0, 2.0],
                [-11.0, 2.5, 3.1],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [0.0, 0.0, -1.0],
            [1.0, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.0, 0.1, 1.0],
            [0.1, -1.1, 1.1],
            [0.1, 0.1, 1.0],
            [0.1, 0.0, -1.0],
            [1.1, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.1, 0.1, 1.1],
        ],
        dtype=np.float64,
    )
    offsets = (
        np.array([0.03, -0.02, 0.01], dtype=np.float64),
        np.array([0.01, -0.005, 0.003], dtype=np.float64),
        np.array([-0.02, 0.015, -0.004], dtype=np.float64),
        np.zeros(3, dtype=np.float64),
    )
    objects = tuple(PointCloudDocument(reference.points + offset) for offset in offsets)
    object_normals = (normals, normals, normals, normals)

    plane_result = multiway_aabb_cascade_point_to_plane_icp(
        objects,
        object_normals,
        max_group_size=2,
        max_iterations=80,
        tolerance=1e-18,
    )
    combined_result = multiway_aabb_cascade_combined_icp(
        objects,
        object_normals,
        max_group_size=2,
        max_iterations=80,
        tolerance=1e-18,
    )

    assert point_result.method == "point_to_point_aabb_cascade"
    assert plane_result.method == "point_to_plane_aabb_cascade"
    assert combined_result.method == "combined_aabb_cascade"

    assert isinstance(point_result, MultiwayICPRegistrationResult)
    assert point_result.fixed_object_index == 3
    assert point_result.active_pair_count >= spatial_reference.point_count * 8
    assert point_result.mean_square_distance < 1e-16
    for transform, cloud, _offset in zip(
        point_result.transforms,
        spatial_objects,
        spatial_offsets,
        strict=True,
    ):
        np.testing.assert_allclose(transform.apply(cloud).points, spatial_reference.points, atol=1e-7)

    for result in (plane_result, combined_result):
        assert isinstance(result, MultiwayICPRegistrationResult)
        assert result.fixed_object_index == 3
        assert result.active_pair_count >= reference.point_count * 8
        assert result.mean_square_distance < 1e-16


def meshlib_style_point_to_plane_fixture() -> tuple[PointCloudDocument, PointCloudDocument, np.ndarray, float, np.ndarray]:
    reference_points = np.array(
        [
            [1.0, 1.0, -5.0],
            [14.0, 1.0, 1.0],
            [1.0, 14.0, 2.0],
            [-11.0, 2.0, 3.0],
            [1.0, -11.0, 4.0],
            [1.0, 2.0, 8.0],
            [2.0, 1.0, -5.0],
            [15.0, 1.5, 1.0],
            [1.5, 15.0, 2.0],
            [-11.0, 2.5, 3.1],
        ],
        dtype=np.float64,
    )
    reference_normals = np.array(
        [
            [0.0, 0.0, -1.0],
            [1.0, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.0, 0.1, 1.0],
            [0.1, -1.1, 1.1],
            [0.1, 0.1, 1.0],
            [0.1, 0.0, -1.0],
            [1.1, 0.1, 1.0],
            [0.1, 1.0, 1.2],
            [-1.1, 0.1, 1.1],
        ],
        dtype=np.float64,
    )
    angle = 0.005
    rotation = np.array(
        [
            [math.cos(angle), -math.sin(angle), 0.0],
            [math.sin(angle), math.cos(angle), 0.0],
            [0.0, 0.0, 1.0],
        ],
        dtype=np.float64,
    )
    translation = np.array([0.003, -0.002, 0.0015], dtype=np.float64)
    floating_points = reference_points @ rotation.T + translation
    return PointCloudDocument(floating_points), PointCloudDocument(reference_points), reference_normals, angle, translation


def test_pairwise_point_to_plane_icp_recovers_meshlib_style_small_rigid_transform() -> None:
    floating, reference, reference_normals, angle, _translation = meshlib_style_point_to_plane_fixture()

    result = pairwise_point_to_plane_icp(
        floating,
        reference,
        reference_normals,
        max_iterations=25,
        tolerance=1e-18,
    )

    assert result.method == "point_to_plane"
    assert result.mode == "rigid"
    assert result.active_pair_count == floating.point_count
    assert result.mean_square_distance < 1e-16
    expected_inverse = np.array(
        [
            [math.cos(angle), math.sin(angle), 0.0],
            [-math.sin(angle), math.cos(angle), 0.0],
            [0.0, 0.0, 1.0],
        ],
        dtype=np.float64,
    )
    np.testing.assert_allclose(result.rotation, expected_inverse, atol=1e-7)
    np.testing.assert_allclose(result.apply(floating).points, reference.points, atol=1e-7)


def test_geometry_sdk_exposes_pairwise_point_to_plane_icp() -> None:
    floating, reference, reference_normals, _angle, _translation = meshlib_style_point_to_plane_fixture()

    result = GeometrySDK().pairwise_point_to_plane_icp(
        floating,
        reference,
        reference_normals,
        max_iterations=25,
        tolerance=1e-18,
    )

    assert result.method == "point_to_plane"
    assert result.mean_square_distance < 1e-16
    np.testing.assert_allclose(result.apply(floating).points, reference.points, atol=1e-7)


def test_pairwise_point_to_plane_icp_distance_filter_rejects_meshlib_style_far_pairs() -> None:
    _floating, reference, reference_normals, _angle, _translation = meshlib_style_point_to_plane_fixture()
    translation = np.array([0.02, -0.015, 0.01], dtype=np.float64)
    floating = PointCloudDocument(np.vstack([reference.points + translation, np.array([[50.0, -40.0, 30.0]])]))

    result = pairwise_point_to_plane_icp(
        floating,
        reference,
        reference_normals,
        max_iterations=10,
        tolerance=1e-12,
        mode="translation",
        max_pair_distance=0.1,
    )

    assert result.active_pair_count == reference.point_count
    np.testing.assert_allclose(result.translation, -translation, atol=1e-10)


def test_pairwise_point_to_plane_icp_cos_threshold_rejects_meshlib_style_opposed_normals() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.2],
                [0.0, 1.0, -0.1],
                [0.3, -0.2, 1.0],
            ],
            dtype=np.float64,
        )
    )
    reference_normals = np.array(
        [
            [0.0, 0.0, 1.0],
            [0.1, 0.0, 1.0],
            [0.0, 0.2, 1.0],
            [0.2, -0.1, 1.0],
        ],
        dtype=np.float64,
    )
    reference_normals /= np.linalg.norm(reference_normals, axis=1, keepdims=True)
    floating_normals = reference_normals.copy()
    floating_normals[1] *= -1.0
    translation = np.array([0.02, -0.015, 0.01], dtype=np.float64)
    floating = PointCloudDocument(reference.points + translation)

    result = GeometrySDK().pairwise_point_to_plane_icp(
        floating,
        reference,
        reference_normals,
        floating_normals=floating_normals,
        cos_threshold=0.7,
        max_iterations=10,
        tolerance=1e-12,
        mode="translation",
    )

    assert result.active_pair_count == reference.point_count - 1
    np.testing.assert_allclose(result.translation, -translation, atol=1e-10)


def test_pairwise_point_to_plane_icp_mutual_closest_rejects_meshlib_style_non_reciprocal_pairs() -> None:
    _floating, reference, reference_normals, _angle, _translation = meshlib_style_point_to_plane_fixture()
    translation = np.array([0.02, -0.015, 0.01], dtype=np.float64)
    non_reciprocal = reference.points[0] + np.array([0.05, 0.0, 0.0], dtype=np.float64)
    floating = PointCloudDocument(np.vstack([reference.points + translation, non_reciprocal]))

    result = pairwise_point_to_plane_icp(
        floating,
        reference,
        reference_normals,
        max_iterations=10,
        tolerance=1e-12,
        mode="translation",
        mutual_closest=True,
    )

    assert result.active_pair_count == reference.point_count
    np.testing.assert_allclose(result.translation, -translation, atol=1e-10)


def test_point_cloud_grid_sample_keeps_meshlib_style_voxel_center_representatives() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.55, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.55, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    sampled, indices = point_cloud_grid_sample(cloud, voxel_size=1.0, return_indices=True)

    assert indices.tolist() == [1, 2]
    np.testing.assert_allclose(sampled.points, cloud.points[indices])


def test_geometry_sdk_exposes_point_cloud_grid_sample() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.4, 0.0, 0.0],
                [1.2, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    sampled = GeometrySDK().point_cloud_grid_sample(cloud, voxel_size=1.0)

    assert sampled.point_count == 2


def test_point_cloud_uniform_sample_uses_meshlib_style_lexicographical_order() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.6, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [1.1, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    sampled, indices = point_cloud_uniform_sample(cloud, distance=1.0, return_indices=True)

    assert indices.tolist() == [1, 2]
    np.testing.assert_allclose(sampled.points, cloud.points[indices])


def test_point_cloud_uniform_sample_keeps_exact_distance_boundary_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    sampled, indices = point_cloud_uniform_sample(cloud, distance=1.0, return_indices=True)

    assert indices.tolist() == [0, 1]
    np.testing.assert_allclose(sampled.points, cloud.points)


def test_point_cloud_uniform_sample_uses_normals_to_preserve_curvature_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.4, 0.0, 0.0],
                [0.8, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        dtype=np.float64,
    )

    sampled, indices = point_cloud_uniform_sample(
        cloud,
        distance=1.0,
        normals=normals,
        min_normal_dot=0.5,
        return_indices=True,
    )

    assert indices.tolist() == [0, 1, 2]
    np.testing.assert_allclose(sampled.points, cloud.points)


def test_geometry_sdk_exposes_point_cloud_uniform_sample() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.6, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [1.1, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    sampled = GeometrySDK().point_cloud_uniform_sample(cloud, distance=1.0)

    assert sampled.point_count == 2


def test_point_cloud_screen_selectors_match_meshlib_viewport_area_contract() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [-0.8, -0.2, 0.0],
                [-0.4, 0.4, 0.0],
                [0.3, 0.0, 0.0],
                [1.4, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    view_projection = np.eye(4, dtype=np.float64)
    polygon = [[-0.95, -0.35], [-0.25, -0.35], [-0.25, 0.55], [-0.95, 0.55]]

    assert point_cloud_select_by_screen_polygon(cloud, view_projection, polygon).tolist() == [0, 1]
    assert GeometrySDK().point_cloud_select_by_screen_polygon(cloud, view_projection, polygon).tolist() == [0, 1]
    assert point_cloud_select_by_screen_rect(cloud, view_projection, [-1.0, -0.5], [-0.5, 0.1]).tolist() == [0]

    brush_path = [[-0.9, -0.4], [-0.9, 0.0]]
    assert point_cloud_select_by_screen_brush(cloud, view_projection, brush_path, radius_px=0.12).tolist() == [0]
    assert point_cloud_select_by_screen_brush(cloud, view_projection, brush_path, radius_px=0.05).tolist() == []

    normals = np.array(
        [
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        dtype=np.float64,
    )
    assert point_cloud_select_by_screen_polygon(
        cloud,
        view_projection,
        polygon,
        normals=normals,
        include_backfaces=True,
    ).tolist() == [0, 1]
    assert point_cloud_select_by_screen_polygon(
        cloud,
        view_projection,
        polygon,
        normals=normals,
        include_backfaces=False,
    ).tolist() == []
    assert point_cloud_select_by_screen_polygon(
        cloud,
        view_projection,
        polygon,
        include_backfaces=False,
    ).tolist() == [0, 1]


def test_point_cloud_pick_by_ray_matches_meshlib_frontmost_point_pick_contract() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.04, 0.0, 1.0],
                [0.02, 0.0, 2.0],
                [0.2, 0.0, 0.5],
                [0.0, 0.0, -1.0],
            ],
            dtype=np.float64,
        )
    )

    selected = point_cloud_pick_by_ray(
        cloud,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        max_distance_to_ray=0.05,
        max_depth=10.0,
    )

    assert selected.tolist() == [0]
    assert GeometrySDK().point_cloud_pick_by_ray(
        cloud,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        max_distance_to_ray=0.01,
        max_depth=10.0,
    ).tolist() == []

    normals = np.array(
        [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ],
        dtype=np.float64,
    )
    assert point_cloud_pick_by_ray(
        cloud,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        max_distance_to_ray=0.05,
        max_depth=10.0,
        normals=normals,
        include_backfaces=False,
    ).tolist() == [1]


def test_point_cloud_extract_selected_points_as_object_matches_meshlib_clone_region_contract() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        ),
        metadata={
            "normals": np.array(
                [
                    [0.0, 0.0, 1.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, -1.0, 0.0],
                ],
                dtype=np.float64,
            ),
            "point_colors": np.array(
                [
                    [255, 0, 0],
                    [0, 255, 0],
                    [0, 0, 255],
                    [255, 255, 0],
                ],
                dtype=np.uint8,
            ),
        },
    )

    extracted = point_cloud_extract_selected_points_as_object(cloud, [3, 1, 3])

    np.testing.assert_allclose(extracted.points, [[1.0, 0.0, 0.0], [3.0, 0.0, 0.0]])
    assert extracted.metadata["source_point_indices"] == [1, 3]
    assert extracted.metadata["meshlib_operation"] == "ObjectPoints::cloneRegion"
    np.testing.assert_allclose(extracted.metadata["normals"], [[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]])
    np.testing.assert_array_equal(extracted.metadata["point_colors"], [[0, 255, 0], [255, 255, 0]])
    assert GeometrySDK().point_cloud_extract_selected_points_as_object(cloud, [0]).point_count == 1


def test_point_cloud_nearest_projections_matches_meshlib_style_result_payload() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    queries = np.array(
        [
            [0.8, 0.0, 0.0],
            [0.0, 1.4, 0.0],
        ],
        dtype=np.float64,
    )

    result = point_cloud_nearest_projections(queries, reference)

    assert result.vertex_indices.tolist() == [1, 2]
    np.testing.assert_allclose(result.points, reference.points[result.vertex_indices])
    np.testing.assert_allclose(result.squared_distances, [0.04, 0.36], atol=1e-12)
    np.testing.assert_allclose(result.distances, [0.2, 0.6], atol=1e-12)


def test_point_cloud_nearest_projections_respects_meshlib_style_upper_limit() -> None:
    reference = PointCloudDocument(np.array([[0.0, 0.0, 0.0]], dtype=np.float64))
    queries = np.array([[1.0, 0.0, 0.0]], dtype=np.float64)

    result = point_cloud_nearest_projections(queries, reference, up_dist_limit_sq=1.0)

    assert result.vertex_indices.tolist() == [-1]
    np.testing.assert_allclose(result.points, [[0.0, 0.0, 0.0]])
    np.testing.assert_allclose(result.squared_distances, [1.0])


def test_point_cloud_nearest_projections_can_skip_same_index_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.3, 0.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    result = point_cloud_nearest_projections(cloud, cloud, skip_same_index=True)

    assert result.vertex_indices.tolist() == [1, 0, 1]


def test_point_cloud_nearest_projections_supports_meshlib_style_low_limit_early_return() -> None:
    reference = PointCloudDocument(
        np.array(
            [
                [0.2, 0.0, 0.0],
                [0.05, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    queries = np.array([[0.0, 0.0, 0.0]], dtype=np.float64)

    result = point_cloud_nearest_projections(queries, reference, lo_dist_limit_sq=0.05)

    assert result.vertex_indices.tolist() == [0]
    np.testing.assert_allclose(result.squared_distances, [0.04], atol=1e-12)


def test_geometry_sdk_exposes_point_cloud_nearest_projections() -> None:
    reference = PointCloudDocument(np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))
    queries = np.array([[0.9, 0.0, 0.0]], dtype=np.float64)

    result = GeometrySDK().point_cloud_nearest_projections(queries, reference)

    assert result.vertex_indices.tolist() == [1]


def test_point_cloud_project_to_mesh_matches_meshlib_style_projection_payload() -> None:
    mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )

    result = point_cloud_project_to_mesh(np.array([[0.25, 0.25, 1.0]], dtype=np.float64), mesh)

    np.testing.assert_allclose(result.points, [[0.25, 0.25, 0.0]])
    np.testing.assert_allclose(result.squared_distances, [1.0])
    np.testing.assert_allclose(result.distances, [1.0])
    assert result.face_indices.tolist() == [0]
    assert result.vertex_indices.tolist() == [0]
    np.testing.assert_allclose(result.normals, [[0.0, 0.0, 1.0]])
    assert result.boundary_flags.tolist() == [False]


def test_point_cloud_project_to_mesh_respects_meshlib_style_strict_upper_limit() -> None:
    mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )

    result = point_cloud_project_to_mesh(
        np.array([[0.25, 0.25, 1.0]], dtype=np.float64),
        mesh,
        up_dist_limit_sq=1.0,
    )

    assert result.face_indices.tolist() == [-1]
    assert result.vertex_indices.tolist() == [-1]
    np.testing.assert_allclose(result.points, [[0.0, 0.0, 0.0]])
    np.testing.assert_allclose(result.squared_distances, [1.0])


def test_point_cloud_project_to_mesh_returns_meshlib_style_pseudonormals() -> None:
    mesh = MeshDocument(
        vertices=np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2], [0, 3, 1], [0, 2, 3]], dtype=np.int64),
    )

    result = point_cloud_project_to_mesh(
        np.array([[0.5, -0.2, -0.2], [-0.2, -0.2, -0.2]], dtype=np.float64),
        mesh,
    )

    np.testing.assert_allclose(result.points, [[0.5, 0.0, 0.0], [0.0, 0.0, 0.0]])
    np.testing.assert_allclose(
        result.normals,
        [
            [0.0, 1.0 / np.sqrt(2.0), 1.0 / np.sqrt(2.0)],
            [1.0 / np.sqrt(3.0), 1.0 / np.sqrt(3.0), 1.0 / np.sqrt(3.0)],
        ],
    )


def test_point_cloud_project_to_mesh_uses_meshlib_style_face_region_mask() -> None:
    mesh = MeshDocument(
        vertices=np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 2.0],
                [1.0, 0.0, 2.0],
                [0.0, 1.0, 2.0],
            ],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
    )

    result = GeometrySDK().point_cloud_project_to_mesh(
        np.array([[0.25, 0.25, 0.1]], dtype=np.float64),
        mesh,
        face_mask=np.array([False, True], dtype=np.bool_),
    )

    np.testing.assert_allclose(result.points, [[0.25, 0.25, 2.0]])
    np.testing.assert_allclose(result.squared_distances, [3.61])
    assert result.face_indices.tolist() == [1]
    assert result.vertex_indices.tolist() == [3]


def test_point_cloud_project_to_mesh_rejects_face_region_mask_length_mismatch() -> None:
    mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )

    with pytest.raises(ValueError, match="face_mask length"):
        point_cloud_project_to_mesh(
            np.array([[0.25, 0.25, 1.0]], dtype=np.float64),
            mesh,
            face_mask=np.array([True, False], dtype=np.bool_),
        )


def test_point_cloud_project_to_mesh_applies_meshlib_style_rigid_reference_transform() -> None:
    mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    mesh_transform = np.eye(4, dtype=np.float64)
    mesh_transform[0, 3] = 10.0

    result = point_cloud_project_to_mesh(
        np.array([[10.25, 0.25, 1.0]], dtype=np.float64),
        mesh,
        mesh_transform=mesh_transform,
    )

    np.testing.assert_allclose(result.points, [[0.25, 0.25, 0.0]])
    np.testing.assert_allclose(result.squared_distances, [1.0])
    assert result.face_indices.tolist() == [0]

    point_transform = np.eye(4, dtype=np.float64)
    point_transform[0, 3] = 10.0
    sdk_result = GeometrySDK().point_cloud_project_to_mesh(
        PointCloudDocument(np.array([[0.25, 0.25, 1.0]], dtype=np.float64)),
        mesh,
        point_transform=point_transform,
        mesh_transform=mesh_transform,
    )

    np.testing.assert_allclose(sdk_result.points, [[0.25, 0.25, 0.0]])
    np.testing.assert_allclose(sdk_result.squared_distances, [1.0])


def test_geometry_sdk_exposes_point_cloud_project_to_mesh_boundary_hits() -> None:
    mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )

    result = GeometrySDK().point_cloud_project_to_mesh(
        PointCloudDocument(np.array([[0.5, 0.0, 1.0]], dtype=np.float64)),
        mesh,
    )

    np.testing.assert_allclose(result.points, [[0.5, 0.0, 0.0]])
    assert result.boundary_flags.tolist() == [True]


def test_point_cloud_n_closest_neighbors_matches_meshlib_style_rows() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.3, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [4.5, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    neighbors = point_cloud_n_closest_neighbors(cloud, num_neighbors=2)

    assert neighbors.tolist() == [
        [1, 2],
        [0, 2],
        [1, 0],
        [2, 1],
    ]


def test_point_cloud_n_closest_neighbors_fills_missing_with_invalid_id_like_meshlib() -> None:
    cloud = PointCloudDocument(np.array([[0.0, 0.0, 0.0]], dtype=np.float64))

    neighbors = point_cloud_n_closest_neighbors(cloud, num_neighbors=2)

    assert neighbors.tolist() == [[-1, -1]]


def test_point_cloud_two_closest_points_matches_meshlib_style_sorted_pair() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [2.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.2, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    result = point_cloud_two_closest_points(cloud)

    assert result.vertex_indices.tolist() == [1, 2]
    assert result.squared_distance == pytest.approx(0.04)


def test_geometry_sdk_exposes_point_cloud_neighbor_queries() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.4, 0.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    neighbors = GeometrySDK().point_cloud_n_closest_neighbors(cloud, num_neighbors=1)
    closest_pair = GeometrySDK().point_cloud_two_closest_points(cloud)

    assert neighbors.tolist() == [[1], [0], [1]]
    assert closest_pair.vertex_indices.tolist() == [0, 1]


def test_point_cloud_neighbors_in_radius_matches_meshlib_style_ball_membership() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.5, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.1, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    neighbors = point_cloud_neighbors_in_radius(cloud, center_index=0, radius=1.0)

    assert neighbors.tolist() == [1, 2]


def test_point_cloud_neighbors_in_radius_filters_crossing_normals_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.5, 0.0, 0.0],
                [0.8, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        dtype=np.float64,
    )

    neighbors = point_cloud_neighbors_in_radius(
        cloud,
        center_index=0,
        radius=1.0,
        normals=normals,
    )

    assert neighbors.tolist() == [2]


def test_point_cloud_neighbors_in_radius_keeps_untrusted_normals_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.5, 0.0, 0.0],
                [0.8, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.array(
        [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        dtype=np.float64,
    )

    neighbors = point_cloud_neighbors_in_radius(
        cloud,
        center_index=0,
        radius=1.0,
        normals=normals,
        untrusted_indices=np.array([1], dtype=np.int64),
    )

    assert neighbors.tolist() == [1, 2]


def test_geometry_sdk_exposes_point_cloud_radius_neighbors() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.4, 0.0, 0.0],
                [1.4, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    neighbors = GeometrySDK().point_cloud_neighbors_in_radius(cloud, center_index=1, radius=1.0)

    assert neighbors.tolist() == [0, 2]


def test_point_cloud_local_neighbor_fan_orders_projected_neighbors_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    fan = point_cloud_local_neighbor_fan(
        cloud,
        center_index=0,
        radius=1.1,
        boundary_angle=3.2,
        normals=normals,
    )

    assert fan.neighbors.tolist() == [2, 1, 4, 3]
    assert fan.boundary_neighbor == -1
    assert fan.actual_radius == pytest.approx(1.1)
    assert fan.removed_count == 0


def test_point_cloud_local_fan_triangles_use_meshlib_next_curr_order() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    fan = point_cloud_local_fan_triangles(
        cloud,
        center_index=0,
        radius=1.1,
        boundary_angle=3.2,
        normals=normals,
    )

    assert fan.triangles.tolist() == [[0, 1, 2], [0, 4, 1], [0, 3, 4], [0, 2, 3]]
    assert fan.boundary_neighbor == -1
    assert fan.removed_count == 0


def test_point_cloud_local_triangulation_repetitions_match_meshlib_buckets() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    repetitions = point_cloud_local_triangulation_repetitions(
        cloud,
        radius=1.5,
        boundary_angle=3.0,
        normals=normals,
    )

    assert repetitions.repetition_counts.tolist() == [0, 0, 0, 1]
    assert repetitions.repeated_3.tolist() == [[0, 1, 2]]
    assert repetitions.repeated_2.tolist() == []


def test_point_cloud_triangulate_candidate_mesh_returns_meshlib_rep3_then_rep2_faces() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    mesh = point_cloud_triangulate_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        normals=normals,
    )

    assert mesh.vertices.tolist() == cloud.points.tolist()
    assert mesh.faces.tolist() == [[0, 1, 2]]
    assert mesh.metadata["source"] == "point_cloud_triangulate_candidate_mesh"
    assert mesh.metadata["repetition_counts"] == [0, 0, 0, 1]
    assert mesh.metadata["repeated_3_count"] == 1
    assert mesh.metadata["repeated_2_count"] == 0


def test_point_cloud_triangulate_cleaned_candidate_mesh_removes_meshlib_bad_triangles_stage() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    mesh = point_cloud_triangulate_cleaned_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        normals=normals,
    )

    assert mesh.faces.tolist() == [[0, 1, 2]]
    assert mesh.metadata["source"] == "point_cloud_triangulate_cleaned_candidate_mesh"
    assert mesh.metadata["input_face_count"] == 1
    assert mesh.metadata["removed_hole_complicating_face_count"] == 0
    assert mesh.metadata["output_repeated_boundary_vertex_count"] == 0


def test_point_cloud_triangulate_topology_candidate_mesh_applies_meshbuilder_edge_filter() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    mesh = point_cloud_triangulate_topology_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        normals=normals,
    )

    assert mesh.faces.tolist() == [[0, 1, 2]]
    assert mesh.metadata["source"] == "point_cloud_triangulate_topology_candidate_mesh"
    assert mesh.metadata["candidate_face_count"] == 1
    assert mesh.metadata["topology_skipped_face_count"] == 0
    assert mesh.metadata["topology_degenerate_face_count"] == 0
    assert mesh.metadata["topology_nonmanifold_edge_face_count"] == 0
    assert mesh.metadata["topology_nonmanifold_vertex_face_count"] == 0
    assert mesh.metadata["topology_unsafe_retry_face_count"] == 0
    assert mesh.metadata["removed_hole_complicating_face_count"] == 0


def test_point_cloud_triangulate_filled_candidate_mesh_uses_meshlib_perimeter_threshold() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    default_threshold = point_cloud_triangulate_filled_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        normals=normals,
    )
    explicit_threshold = point_cloud_triangulate_filled_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        crit_hole_length=4.0,
        normals=normals,
    )

    assert default_threshold.metadata["source"] == "point_cloud_triangulate_filled_candidate_mesh"
    assert default_threshold.face_count == 1
    assert default_threshold.metadata["input_hole_count"] == 1
    assert default_threshold.metadata["filled_hole_count"] == 0
    assert default_threshold.metadata["skipped_hole_count"] == 1
    assert default_threshold.metadata["topology_unsafe_retry_face_count"] == 0
    assert explicit_threshold.face_count == 2
    assert explicit_threshold.metadata["filled_hole_count"] == 1
    assert explicit_threshold.metadata["added_fill_face_count"] == 1


def test_point_cloud_local_neighbor_fan_marks_neighbor_before_large_boundary_gap() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    fan = point_cloud_local_neighbor_fan(
        cloud,
        center_index=0,
        radius=1.1,
        boundary_angle=3.0,
        normals=normals,
    )

    assert fan.neighbors.tolist() == [2, 1]
    assert fan.boundary_neighbor == 1


def test_point_cloud_local_neighbor_fan_supports_num_neighbor_mode_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.5, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    fan = point_cloud_local_neighbor_fan(
        cloud,
        center_index=0,
        radius=0.0,
        num_neighbors=2,
        boundary_angle=math.tau,
    )

    assert fan.neighbors.tolist() == [1, 2]
    assert fan.actual_radius == pytest.approx(1.0)


def test_geometry_sdk_exposes_point_cloud_local_neighbor_fan() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    fan = GeometrySDK().point_cloud_local_neighbor_fan(
        cloud,
        center_index=0,
        radius=1.1,
        boundary_angle=3.0,
    )

    assert fan.neighbors.tolist() == [2, 1]
    assert fan.boundary_neighbor == 1


def test_geometry_sdk_exposes_point_cloud_local_fan_triangles_and_repetitions() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    sdk = GeometrySDK()
    fan = sdk.point_cloud_local_fan_triangles(
        cloud,
        center_index=0,
        radius=1.1,
        boundary_angle=3.0,
        normals=normals,
    )
    repetitions = sdk.point_cloud_local_triangulation_repetitions(
        cloud,
        radius=1.5,
        boundary_angle=3.0,
        normals=normals,
    )

    assert fan.triangles.tolist() == [[0, 1, 2]]
    assert repetitions.repeated_3.tolist() == [[0, 1, 2]]


def test_geometry_sdk_exposes_point_cloud_triangulate_candidate_mesh() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    mesh = GeometrySDK().point_cloud_triangulate_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        normals=normals,
    )

    assert mesh.vertex_count == 3
    assert mesh.face_count == 1
    assert mesh.faces.tolist() == [[0, 1, 2]]


def test_geometry_sdk_exposes_point_cloud_triangulate_cleaned_candidate_mesh() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    mesh = GeometrySDK().point_cloud_triangulate_cleaned_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        normals=normals,
    )

    assert mesh.vertex_count == 3
    assert mesh.face_count == 1
    assert mesh.metadata["removed_hole_complicating_face_count"] == 0


def test_geometry_sdk_exposes_point_cloud_triangulate_topology_candidate_mesh() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    mesh = GeometrySDK().point_cloud_triangulate_topology_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        normals=normals,
    )

    assert mesh.vertex_count == 3
    assert mesh.face_count == 1
    assert mesh.metadata["topology_skipped_face_count"] == 0


def test_geometry_sdk_exposes_point_cloud_triangulate_filled_candidate_mesh() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    normals = np.tile(np.array([[0.0, 0.0, 1.0]], dtype=np.float64), (cloud.point_count, 1))

    mesh = GeometrySDK().point_cloud_triangulate_filled_candidate_mesh(
        cloud,
        radius=1.5,
        num_neighbors=0,
        boundary_angle=3.0,
        crit_hole_length=4.0,
        normals=normals,
    )

    assert mesh.vertex_count == 3
    assert mesh.face_count == 2
    assert mesh.metadata["filled_hole_count"] == 1


def test_point_cloud_local_neighbor_fan_optimizer_removes_center_coincident_neighbor_like_meshlib() -> None:
    cloud = PointCloudDocument(
        np.array(
            [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        )
    )

    fan = point_cloud_local_neighbor_fan(
        cloud,
        center_index=0,
        radius=1.1,
        boundary_angle=math.tau,
        max_removes=1,
    )

    assert fan.neighbors.tolist() == [2, 3]
    assert fan.removed_count == 0
