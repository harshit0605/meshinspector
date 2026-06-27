from __future__ import annotations

import os
import struct

import numpy as np
import pytest
from PIL import Image

from geometry_sdk import GeometrySDK, RegionEntry
from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.accelerators import rust
from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.deform.thicken import global_thicken
from geometry_sdk.repair.basic import orient_faces_outward
from geometry_sdk.testing.fixtures import box, cube, pendant, ring
from geometry_sdk.testing.openvdb import synthetic_openvdb_single_dense_leaf
from geometry_sdk.voxel.extract import extract_surface_mesh
from geometry_sdk.voxel.marching import (
    _orient_faces_consistently,
    extract_boolean_marching_tetrahedra,
    extract_marching_tetrahedra,
    extract_offset_marching_tetrahedra,
    extract_shell_marching_tetrahedra,
)
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh, voxel_offset_mesh, voxel_partial_offset_mesh, voxel_shell_mesh, voxel_thicken_mesh, voxel_weighted_shell_mesh
from geometry_sdk.voxel.ops import (
    sdf_difference,
    sdf_intersection,
    sdf_offset,
    sdf_shell,
    sdf_union,
    voxel_binary_operation,
    voxel_binary_values,
    voxel_binary_iso_value,
)
from geometry_sdk.voxel.refine import laplacian_smooth_vertices, project_vertices_to_sdf, refine_sdf_mesh
from geometry_sdk.voxel.raw import load_raw_voxels, load_raw_voxels_auto, load_tiff_voxels_dir, voxel_default_iso_value
from geometry_sdk.voxel.rendering import voxel_volume_render_data, voxel_volume_render_lut, voxel_volume_render_ray
from geometry_sdk.voxel.active_box import voxel_active_box
from geometry_sdk.voxel.conversion import (
    voxel_move_mesh_to_max_deriv,
    voxel_to_mesh_dual,
    voxel_to_mesh_dual_vdb_payload,
    voxel_to_mesh_simple,
    voxel_to_mesh_smart,
)
from geometry_sdk.voxel.path import voxel_path, voxel_path_build_four
from geometry_sdk.voxel.segmentation import voxel_mask_to_mesh, voxel_segmentation, voxel_segmentation_mesh
from geometry_sdk.voxel.slice import voxel_slice
from geometry_sdk.voxel.line_graph import voxel_line_graph
from geometry_sdk.voxel.sdf import estimate_sdf_volume, sample_aligned_sdf_grids, sample_sdf_grid, sample_sdf_values
from geometry_sdk.types import MeshDocument, SDFGrid, VoxelVolume


def test_sdf_grid_samples_negative_inside_and_positive_outside_cube() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=1.0, padding_mm=1.0)

    assert grid.shape == (5, 5, 5)
    assert grid.values[2, 2, 2] < 0.0
    assert grid.values[0, 0, 0] > 0.0


def test_sdf_grid_coordinate_helpers_match_meshlib_dense_grid_order() -> None:
    grid = SDFGrid(
        origin=(-1.0, 2.0, 4.0),
        voxel_size_mm=0.5,
        shape=(2, 2, 2),
        values=np.zeros((2, 2, 2), dtype=np.float32),
    )

    points = grid.points()

    np.testing.assert_allclose(
        points,
        [
            [-1.0, 2.0, 4.0],
            [-1.0, 2.0, 4.5],
            [-1.0, 2.5, 4.0],
            [-1.0, 2.5, 4.5],
            [-0.5, 2.0, 4.0],
            [-0.5, 2.0, 4.5],
            [-0.5, 2.5, 4.0],
            [-0.5, 2.5, 4.5],
        ],
    )
    np.testing.assert_allclose(
        grid.point_to_grid(np.array([[-1.0, 2.0, 4.0], [-0.25, 3.0, 5.5]], dtype=np.float64)),
        [[0.0, 0.0, 0.0], [1.5, 2.0, 3.0]],
    )


def test_voxel_value_range_crosses_rust_boundary() -> None:
    values = np.asarray([-3.0, 2.5, 0.0, 9.0], dtype=np.float32)

    assert _rust_voxel.voxel_value_range(values) == pytest.approx((-3.0, 9.0))


def test_sdf_volume_estimate_is_close_for_cube_at_half_mm_resolution() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    volume = estimate_sdf_volume(grid)

    assert np.isclose(volume, 8.0, atol=4.0)


def test_sdf_grid_handles_ring_fixture() -> None:
    grid = sample_sdf_grid(ring(radial_segments=16, tube_segments=8), voxel_size_mm=2.0, padding_mm=2.0)

    assert grid.values.shape == grid.shape
    assert np.any(grid.values < 0.0)
    assert np.any(grid.values > 0.0)


def test_rust_accelerated_sdf_grid_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    source = cube(size=2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    python_grid = sample_sdf_grid(source, voxel_size_mm=1.0, padding_mm=1.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_grid = sample_sdf_grid(source, voxel_size_mm=1.0, padding_mm=1.0)

    assert rust_grid.origin == python_grid.origin
    assert rust_grid.shape == python_grid.shape
    assert np.allclose(rust_grid.values, python_grid.values, atol=1e-6)


def test_engine_exposes_sdf_sampling() -> None:
    sdk = GeometrySDK()
    grid = sdk.sample_sdf_grid(cube(size=2.0), voxel_size_mm=1.0, padding_mm=1.0)

    assert grid.values.shape == (5, 5, 5)


def test_sdf_boolean_operations_on_aligned_grids() -> None:
    outer = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=1.0)
    expanded = sdf_offset(outer, 0.75)
    union = sdf_union(outer, expanded)
    intersection = sdf_intersection(outer, expanded)
    difference = sdf_difference(expanded, outer)

    assert np.allclose(union.values, expanded.values)
    assert np.allclose(intersection.values, outer.values)
    assert np.any(difference.values < 0.0)
    assert np.any(difference.values > 0.0)


def test_voxel_binary_operations_match_meshlib_binary_operations_plugin_scalar_contract() -> None:
    left = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.array(
            [1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0],
            dtype=np.float32,
        ).reshape(2, 2, 2),
    )
    right = SDFGrid(
        origin=left.origin,
        voxel_size_mm=left.voxel_size_mm,
        shape=left.shape,
        values=np.array(
            [0.5, -0.5, 1.5, -1.5, 2.0, -2.0, 4.0, -4.0],
            dtype=np.float32,
        ).reshape(2, 2, 2),
    )

    max_grid = voxel_binary_values(left, right, operation="max")
    min_grid = voxel_binary_values(left, right, operation="min")
    sum_grid, sum_iso = voxel_binary_operation(left, right, operation="sum", left_iso_value=1.0, right_iso_value=2.0)
    multiply_grid, multiply_iso = voxel_binary_operation(
        left,
        right,
        operation="multiply",
        left_iso_value=1.0,
        right_iso_value=2.0,
    )
    divide_grid, divide_iso = voxel_binary_operation(
        right,
        left,
        operation="divide",
        left_iso_value=1.0,
        right_iso_value=0.0,
    )

    assert np.allclose(max_grid.values, np.maximum(left.values, right.values))
    assert np.allclose(min_grid.values, np.minimum(left.values, right.values))
    assert np.allclose(sum_grid.values, left.values + right.values)
    assert np.allclose(multiply_grid.values, left.values * right.values)
    assert np.allclose(divide_grid.values, right.values / left.values)
    assert sum_iso == pytest.approx(3.0)
    assert multiply_iso == pytest.approx(2.0)
    assert divide_iso == pytest.approx(1.0)
    assert voxel_binary_iso_value(1.0, 2.0, operation="max") == pytest.approx(2.0)


def test_load_raw_voxels_matches_meshlib_uint16_normalization_contract(tmp_path) -> None:
    path = tmp_path / "explicit.raw"
    path.write_bytes(np.array([0, 32768, 65535, 16384], dtype="<u2").tobytes())

    volume = load_raw_voxels(
        path,
        dimensions=(2, 2, 1),
        voxel_size=(0.5, 1.0, 2.0),
        scalar_type="uint16",
    )

    assert volume.dimensions == (2, 2, 1)
    assert volume.values.shape == (2, 2, 1)
    assert volume.voxel_size == pytest.approx((0.5, 1.0, 2.0))
    assert volume.scalar_type == "uint16"
    assert volume.metadata["source"] == "MeshLib VoxelsLoad::fromRaw"
    assert volume.metadata["default_iso_value"] == pytest.approx(85.0 / 256.0)
    assert volume.metadata["default_iso_value_source"] == "MR::ObjectVoxels::histogram().getBinMinMax(bins.size() / 3).first"
    np.testing.assert_allclose(
        volume.values.reshape(-1),
        [0.0, 32768.0 / 65535.0, 1.0, 16384.0 / 65535.0],
        atol=1e-6,
    )
    assert volume.min_value == pytest.approx(0.0)
    assert volume.max_value == pytest.approx(1.0)


def test_load_raw_voxels_matches_meshlib_float32_4_fourth_channel_contract(tmp_path) -> None:
    path = tmp_path / "rgba.raw"
    path.write_bytes(struct.pack("<ffffffff", 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0))

    volume = load_raw_voxels(
        path,
        dimensions=(2, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        scalar_type="float32_4",
    )

    # float32_4 takes the 4th channel of each 16-byte voxel, unnormalized (like
    # float32). voxel0=(1,2,3,4)->4.0, voxel1=(5,6,7,8)->8.0. (Previously a stray
    # `/ 0.0` in the kernel made every value +Infinity; this asserts the real
    # finite contract.)
    np.testing.assert_allclose(volume.values.reshape(-1), [4.0, 8.0], atol=1e-6)
    assert volume.min_value == pytest.approx(4.0)
    assert volume.max_value == pytest.approx(8.0)
    assert np.all(np.isfinite(volume.values.reshape(-1)))


def test_load_raw_voxels_auto_matches_meshlib_filename_parameter_parser(tmp_path) -> None:
    path = tmp_path / "w2_h1_s1_x500_F_sample.raw"
    path.write_bytes(struct.pack("<ff", 0.0, 1.25))

    volume = load_raw_voxels_auto(path)

    assert volume.dimensions == (2, 1, 1)
    assert volume.voxel_size == pytest.approx((0.5, 0.5, 0.5))
    assert volume.scalar_type == "float32"
    assert volume.grid_level_set is False
    np.testing.assert_allclose(volume.values.reshape(-1), [0.0, 1.25])


def test_engine_exposes_meshlib_raw_voxel_loader(tmp_path) -> None:
    path = tmp_path / "explicit.raw"
    path.write_bytes(np.array([0, 255], dtype=np.uint8).tobytes())

    volume = GeometrySDK().load_raw_voxels(
        path,
        dimensions=(2, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        scalar_type="uint8",
    )

    np.testing.assert_allclose(volume.values.reshape(-1), [0.0, 1.0])


def test_load_tiff_voxels_dir_matches_meshlib_sorted_slice_stack_contract(tmp_path) -> None:
    Image.fromarray(np.array([[10.0, 11.0]], dtype=np.float32), mode="F").save(tmp_path / "slice_10.tiff")
    Image.fromarray(np.array([[2.0, 3.0]], dtype=np.float32), mode="F").save(tmp_path / "slice_02.tiff")

    volume = load_tiff_voxels_dir(tmp_path, voxel_size=(0.5, 0.25, 2.0))

    assert volume.dimensions == (2, 1, 2)
    assert volume.voxel_size == pytest.approx((0.5, 0.25, 2.0))
    assert volume.scalar_type == "tiff"
    assert volume.metadata["source"] == "MeshLib VoxelsLoad::loadTiffDir"
    assert volume.metadata["default_iso_value"] == pytest.approx(2.0 + 85.0 * ((11.0 - 2.0) / 256.0))
    assert volume.metadata["default_iso_value_source"] == "MR::ObjectVoxels::histogram().getBinMinMax(bins.size() / 3).first"
    np.testing.assert_allclose(volume.values.reshape(-1), [2.0, 3.0, 10.0, 11.0])
    assert volume.min_value == pytest.approx(2.0)
    assert volume.max_value == pytest.approx(11.0)
    assert volume.metadata["source_files"][0].endswith("slice_02.tiff")
    assert volume.metadata["source_files"][1].endswith("slice_10.tiff")


def test_load_tiff_voxels_dir_matches_meshlib_rgb_and_level_set_contract(tmp_path) -> None:
    Image.fromarray(np.array([[[100, 0, 0]]], dtype=np.uint8), mode="RGB").save(tmp_path / "slice_01.tiff")
    Image.fromarray(np.array([[[0, 100, 0]]], dtype=np.uint8), mode="RGB").save(tmp_path / "slice_02.tiff")

    volume = GeometrySDK().load_tiff_voxels_dir(
        tmp_path,
        voxel_size=(1.0, 1.0, 1.0),
        grid_level_set=True,
    )

    assert volume.dimensions == (1, 1, 2)
    np.testing.assert_allclose(volume.values.reshape(-1), [29.9, 58.7], atol=1e-5)
    assert volume.grid_level_set is True


def test_voxel_default_iso_value_matches_meshlib_object_voxels_histogram_contract() -> None:
    iso_value = voxel_default_iso_value(np.asarray([-10.0, 0.0, 10.0, 20.0], dtype=np.float32))

    assert iso_value == pytest.approx(-10.0 + 85.0 * ((20.0 - -10.0) / 256.0))
    assert voxel_default_iso_value(np.asarray([42.0, 42.0, 42.0], dtype=np.float32)) == pytest.approx(42.0)


def test_voxel_path_matches_meshlib_difference_and_exponent_metric_contracts() -> None:
    values = np.asarray(
        [
            0.0,
            0.0,
            0.0,
            0.0,
            10.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        dtype=np.float32,
    ).reshape(3, 3, 1)

    difference_path = voxel_path(
        values,
        shape=(3, 3, 1),
        start=(0, 1, 0),
        finish=(2, 1, 0),
        metric="difference",
    )
    exponent_path = GeometrySDK().voxel_path(
        values,
        shape=(3, 3, 1),
        start=(0, 1, 0),
        finish=(2, 1, 0),
        metric="exponent",
    )

    assert difference_path.coordinates[0] == (0, 1, 0)
    assert difference_path.coordinates[-1] == (2, 1, 0)
    assert len(difference_path.voxel_indices) == 5
    assert (1, 1, 0) not in difference_path.coordinates
    assert difference_path.total_metric == pytest.approx(0.0)
    assert exponent_path.coordinates == [(0, 1, 0), (1, 1, 0), (2, 1, 0)]
    assert exponent_path.total_metric < 1.001


def test_voxel_path_uses_meshlib_x_fastest_order_for_direct_3d_arrays() -> None:
    values = np.zeros((3, 2, 1), dtype=np.float32)
    values[1, 0, 0] = 10.0

    difference_path = GeometrySDK().voxel_path(
        values,
        start=(0, 0, 0),
        finish=(2, 0, 0),
        metric="difference",
    )
    exponent_path = voxel_path(
        values,
        start=(0, 0, 0),
        finish=(2, 0, 0),
        metric="exponent",
    )

    assert len(difference_path.coordinates) == 5
    assert (1, 0, 0) not in difference_path.coordinates
    assert difference_path.total_metric == pytest.approx(0.0)
    assert exponent_path.coordinates == [(0, 0, 0), (1, 0, 0), (2, 0, 0)]
    assert exponent_path.total_metric < 1.001


def test_voxel_path_build_four_matches_meshlib_quarter_seed_contract() -> None:
    values = np.zeros((5, 5, 5), dtype=np.float32)

    result = voxel_path_build_four(
        values,
        start=(0, 2, 2),
        finish=(4, 2, 2),
        metric="difference",
    )
    sdk_result = GeometrySDK().voxel_path_build_four(
        values,
        start=(0, 2, 2),
        finish=(4, 2, 2),
        metric="difference",
    )

    assert [entry["quarters_mask"] for entry in result] == [1, 2, 4, 8]
    assert len(sdk_result) == 4
    for entry in sdk_result:
        assert entry["path"].coordinates[0] == (0, 2, 2)
        assert entry["path"].coordinates[-1] == (4, 2, 2)
        assert entry["path"].total_metric == pytest.approx(0.0)
    assert (2, 0, 0) in sdk_result[0]["path"].coordinates
    assert (2, 0, 2) in sdk_result[1]["path"].coordinates
    assert (2, 2, 0) in sdk_result[2]["path"].coordinates
    assert (2, 2, 2) in sdk_result[3]["path"].coordinates


def test_voxel_slice_matches_meshlib_save_slice_texture_order_contract() -> None:
    values = np.zeros((2, 3, 4), dtype=np.float32)
    for x in range(2):
        for y in range(3):
            for z in range(4):
                values[x, y, z] = x + 10 * y + 100 * z

    xy = voxel_slice(values, plane="xy", slice_index=2, min_value=200.0, max_value=221.0)
    yz = GeometrySDK().voxel_slice(values, plane="yz", slice_index=1, min_value=1.0, max_value=321.0)
    zx = voxel_slice(values, plane="zx", slice_index=2, min_value=20.0, max_value=321.0)

    assert xy.width == 2
    assert xy.height == 3
    np.testing.assert_allclose(xy.values, [200.0, 201.0, 210.0, 211.0, 220.0, 221.0])
    assert xy.coordinates[0] == (0, 0, 2)
    assert xy.coordinates[-1] == (1, 2, 2)
    np.testing.assert_allclose(xy.normalized_values, [0.0, 1.0 / 21.0, 10.0 / 21.0, 11.0 / 21.0, 20.0 / 21.0, 1.0])

    assert yz.width == 3
    assert yz.height == 4
    np.testing.assert_allclose(yz.values, [1.0, 11.0, 21.0, 101.0, 111.0, 121.0, 201.0, 211.0, 221.0, 301.0, 311.0, 321.0])
    assert yz.coordinates[0] == (1, 0, 0)
    assert yz.coordinates[-1] == (1, 2, 3)

    assert zx.width == 4
    assert zx.height == 2
    np.testing.assert_allclose(zx.values, [20.0, 120.0, 220.0, 320.0, 21.0, 121.0, 221.0, 321.0])
    assert zx.coordinates[0] == (0, 2, 0)
    assert zx.coordinates[-1] == (1, 2, 3)


def test_voxel_line_graph_matches_meshinspector_axis_probe_contract() -> None:
    values = np.zeros((3, 2, 2), dtype=np.float32)
    for x in range(3):
        for y in range(2):
            for z in range(2):
                values[x, y, z] = x + 10 * y + 100 * z

    x_line = voxel_line_graph(values, axis="x", fixed_coordinate=(0, 1, 1))
    y_line = GeometrySDK().voxel_line_graph(values, axis="y", fixed_coordinate=(2, 0, 1))
    z_line = voxel_line_graph(values, axis="z", fixed_coordinate=(2, 1, 0))

    assert x_line.axis == 0
    assert x_line.positions == [0, 1, 2]
    assert x_line.coordinates == [(0, 1, 1), (1, 1, 1), (2, 1, 1)]
    assert x_line.voxel_indices == [9, 10, 11]
    np.testing.assert_allclose(x_line.values, [110.0, 111.0, 112.0])

    assert y_line.axis == 1
    assert y_line.positions == [0, 1]
    assert y_line.coordinates == [(2, 0, 1), (2, 1, 1)]
    assert y_line.voxel_indices == [8, 11]
    np.testing.assert_allclose(y_line.values, [102.0, 112.0])

    assert z_line.axis == 2
    assert z_line.positions == [0, 1]
    assert z_line.coordinates == [(2, 1, 0), (2, 1, 1)]
    assert z_line.voxel_indices == [5, 11]
    np.testing.assert_allclose(z_line.values, [12.0, 112.0])


def test_voxel_active_box_matches_meshlib_max_excluded_bounds_contract() -> None:
    values = np.zeros((4, 3, 2), dtype=np.float32)
    for x in range(4):
        for y in range(3):
            for z in range(2):
                values[x, y, z] = x + 10 * y + 100 * z

    active_box = voxel_active_box(values, min_corner=(1, 1, 0), dimensions=(2, 2, 2))
    sdk_active_box = GeometrySDK().voxel_active_box(values, min_corner=(1, 1, 0), dimensions=(2, 2, 2))

    assert active_box.min_corner == (1, 1, 0)
    assert active_box.dimensions == (2, 2, 2)
    assert active_box.coordinates == [
        (1, 1, 0),
        (2, 1, 0),
        (1, 2, 0),
        (2, 2, 0),
        (1, 1, 1),
        (2, 1, 1),
        (1, 2, 1),
        (2, 2, 1),
    ]
    assert active_box.source_indices == [5, 6, 9, 10, 17, 18, 21, 22]
    np.testing.assert_allclose(active_box.values, [11.0, 12.0, 21.0, 22.0, 111.0, 112.0, 121.0, 122.0])
    np.testing.assert_allclose(sdk_active_box.values, active_box.values)


def test_voxel_segmentation_matches_meshlib_graph_cut_and_boundary_seed_contracts() -> None:
    line = np.array([0.0, 0.0, 0.0, 10.0, 10.0], dtype=np.float32).reshape(5, 1, 1)

    graph_cut = voxel_segmentation(
        line,
        inside_seeds=[(4, 0, 0)],
        outside_seeds=[(0, 0, 0)],
        exponent_modifier=2.0,
        voxels_expansion=4,
        include_boundary_outside=False,
    )

    assert graph_cut.min_corner == (0, 0, 0)
    assert graph_cut.dimensions == (5, 1, 1)
    assert graph_cut.selected_coordinates == [(3, 0, 0), (4, 0, 0)]
    assert graph_cut.source_indices == [3, 4]
    assert graph_cut.part_indices == [3, 4]
    np.testing.assert_allclose(graph_cut.selected_values, [10.0, 10.0])

    volume = np.zeros((5, 3, 3), dtype=np.float32)
    volume[3, 1, 1] = 10.0
    volume[4, 1, 1] = 10.0

    boundary_seeded = GeometrySDK().voxel_segmentation(
        volume,
        inside_seeds=[(4, 1, 1)],
        outside_seeds=[],
        exponent_modifier=2.0,
        voxels_expansion=4,
        include_boundary_outside=True,
    )

    assert boundary_seeded.min_corner == (0, 0, 0)
    assert boundary_seeded.dimensions == (5, 3, 3)
    assert boundary_seeded.selected_coordinates == [(3, 1, 1), (4, 1, 1)]
    assert boundary_seeded.source_indices == [23, 24]
    assert boundary_seeded.part_indices == [23, 24]


def test_voxel_segmentation_mesh_matches_meshlib_simple_mask_iso_shift_contract() -> None:
    volume = np.zeros((5, 5, 5), dtype=np.float32)
    volume[2, 2, 2] = 10.0

    mesh = voxel_segmentation_mesh(
        volume,
        inside_seeds=[(2, 2, 2)],
        outside_seeds=[],
        exponent_modifier=2.0,
        voxels_expansion=1,
        include_boundary_outside=True,
        voxel_size=(0.5, 1.0, 2.0),
    )
    sdk_mesh = GeometrySDK().voxel_segmentation_mesh(
        volume,
        inside_seeds=[(2, 2, 2)],
        outside_seeds=[],
        exponent_modifier=2.0,
        voxels_expansion=1,
        include_boundary_outside=True,
        voxel_size=(0.5, 1.0, 2.0),
    )

    assert mesh.vertex_count > 0
    assert mesh.face_count > 0
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [0.75, 1.5, 3.0])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [1.25, 2.5, 5.0])
    assert mesh.metadata["source"] == "voxel_segmentation_mesh"
    assert mesh.metadata["segmentation"]["min_corner"] == (1, 1, 1)
    assert mesh.metadata["segmentation"]["dimensions"] == (3, 3, 3)
    assert mesh.metadata["segmentation"]["selected_coordinates"] == [(2, 2, 2)]
    np.testing.assert_allclose(sdk_mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(sdk_mesh.faces, mesh.faces)


def test_voxel_mask_to_mesh_matches_meshlib_smooth_mask_meshing_contract() -> None:
    volume = np.zeros((5, 5, 5), dtype=np.float32)
    volume[2, 2, 2] = 10.0

    mesh = voxel_mask_to_mesh(
        volume,
        mask_coordinates=[(2, 2, 2)],
        voxel_size=(0.5, 1.0, 2.0),
        mask_expansion=1,
        smooth_band_radius=3,
    )
    sdk_mesh = GeometrySDK().voxel_mask_to_mesh(
        volume,
        mask_coordinates=[(2, 2, 2)],
        voxel_size=(0.5, 1.0, 2.0),
        mask_expansion=1,
        smooth_band_radius=3,
    )

    assert mesh.vertex_count > 0
    assert mesh.face_count > 0
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [0.75, 1.5, 3.0])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [1.25, 2.5, 5.0])
    assert mesh.metadata["source"] == "voxel_mask_to_mesh"
    assert mesh.metadata["voxel_size"] == (0.5, 1.0, 2.0)
    assert mesh.metadata["mask"]["min_corner"] == (1, 1, 1)
    assert mesh.metadata["mask"]["dimensions"] == (3, 3, 3)
    assert mesh.metadata["mask"]["source_indices"] == [62]
    assert mesh.metadata["mask"]["part_indices"] == [13]
    assert mesh.metadata["mask"]["selected_coordinates"] == [(2, 2, 2)]
    np.testing.assert_allclose(sdk_mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(sdk_mesh.faces, mesh.faces)


def test_voxel_to_mesh_simple_matches_meshlib_dense_volume_iso_contract() -> None:
    volume = np.zeros((5, 5, 5), dtype=np.float32)
    volume[2, 2, 2] = 10.0

    mesh = voxel_to_mesh_simple(
        volume,
        iso_value=5.0,
        voxel_size=(0.5, 1.0, 2.0),
    )
    sdk_mesh = GeometrySDK().voxel_to_mesh_simple(
        volume,
        iso_value=5.0,
        voxel_size=(0.5, 1.0, 2.0),
    )

    assert mesh.vertex_count > 0
    assert mesh.face_count > 0
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [0.75, 1.5, 3.0])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [1.25, 2.5, 5.0])
    assert mesh.metadata["source"] == "voxel_to_mesh_simple"
    assert mesh.metadata["iso_value"] == 5.0
    assert mesh.metadata["voxel_size"] == (0.5, 1.0, 2.0)
    assert mesh.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface"
    assert mesh.metadata["parity_status"] == "partial_dual_marching_cubes_pending"
    np.testing.assert_allclose(sdk_mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(sdk_mesh.faces, mesh.faces)


def test_voxel_to_mesh_simple_honors_meshlib_level_set_less_inside_contract() -> None:
    volume_values = np.full((5, 5, 5), 10.0, dtype=np.float32)
    volume_values[2, 2, 2] = -10.0
    volume = VoxelVolume(
        dimensions=(5, 5, 5),
        voxel_size=(1.0, 1.0, 1.0),
        grid_level_set=True,
        scalar_type="float32",
        values=volume_values,
        min_value=-10.0,
        max_value=10.0,
        metadata={"default_iso_value": 0.0},
    )

    mesh = voxel_to_mesh_simple(volume, iso_value=0.0)
    signed_volume = sum(
        float(np.dot(mesh.vertices[face[0]], np.cross(mesh.vertices[face[1]], mesh.vertices[face[2]])) / 6.0)
        for face in mesh.faces
    )

    assert mesh.metadata["meshlib_level_set_less_inside"] is True
    assert signed_volume > 0.0
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [1.5, 1.5, 1.5])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [2.5, 2.5, 2.5])


def test_voxel_to_mesh_dual_extracts_meshlib_dense_dual_plane_slice() -> None:
    values = np.zeros((4, 4, 4), dtype=np.float32)
    for x in range(values.shape[0]):
        values[x, :, :] = x
    volume = VoxelVolume(
        dimensions=(4, 4, 4),
        voxel_size=(0.5, 1.0, 2.0),
        grid_level_set=True,
        scalar_type="float32",
        values=values,
        min_value=0.0,
        max_value=3.0,
        metadata={"default_iso_value": 1.5},
    )

    mesh = voxel_to_mesh_dual(volume)
    sdk_mesh = GeometrySDK().voxel_to_mesh_dual(volume)

    assert mesh.vertex_count == 9
    assert mesh.face_count == 8
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [0.75, 0.5, 1.0])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [0.75, 2.5, 5.0])
    assert mesh.metadata["source"] == "voxel_to_mesh_dual"
    assert mesh.metadata["iso_value"] == 1.5
    assert mesh.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface"
    assert mesh.metadata["parity_status"] == "dense_dual_contouring_backed_sparse_openvdb_volume_to_mesh_pending"
    np.testing.assert_allclose(sdk_mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(sdk_mesh.faces, mesh.faces)


def test_voxel_to_mesh_dual_exposes_meshlib_face_and_vertex_limits() -> None:
    shape = (4, 4, 4)
    values = np.zeros(np.prod(shape), dtype=np.float32)
    for x in range(shape[0]):
        for y in range(shape[1]):
            for z in range(shape[2]):
                values[x + y * shape[0] + z * shape[0] * shape[1]] = float(x)
    volume = VoxelVolume(
        dimensions=shape,
        voxel_size=(0.5, 1.0, 2.0),
        grid_level_set=True,
        scalar_type="float32",
        values=values,
        min_value=0.0,
        max_value=3.0,
        metadata={"default_iso_value": 1.5},
    )

    with pytest.raises(ValueError, match="Vertices number limit exceeded"):
        voxel_to_mesh_dual(volume, max_vertices=8)

    with pytest.raises(ValueError, match="Triangles number limit exceeded"):
        voxel_to_mesh_dual(volume, max_faces=7)

    mesh = GeometrySDK().voxel_to_mesh_dual(volume, max_vertices=9, max_faces=8)
    assert mesh.vertex_count == 9
    assert mesh.face_count == 8
    assert mesh.metadata["max_vertices"] == 9
    assert mesh.metadata["max_faces"] == 8


def test_voxel_to_mesh_dual_exposes_meshlib_adaptivity_setting() -> None:
    values = np.zeros((4, 4, 4), dtype=np.float32)
    for x in range(values.shape[0]):
        values[x, :, :] = x
    volume = VoxelVolume(
        dimensions=(4, 4, 4),
        voxel_size=(0.5, 1.0, 2.0),
        grid_level_set=True,
        scalar_type="float32",
        values=values,
        min_value=0.0,
        max_value=3.0,
        metadata={"default_iso_value": 1.5},
    )

    mesh = voxel_to_mesh_dual(volume, adaptivity=1.0)
    sdk_mesh = GeometrySDK().voxel_to_mesh_dual(volume, adaptivity=1.0)

    assert mesh.vertex_count == 4
    assert mesh.face_count == 2
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [0.75, 0.5, 1.0])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [0.75, 2.5, 5.0])
    assert mesh.metadata["adaptivity"] == 1.0
    np.testing.assert_allclose(sdk_mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(sdk_mesh.faces, mesh.faces)


def test_voxel_to_mesh_dual_exposes_meshlib_relax_disoriented_triangles_setting() -> None:
    values = np.zeros((4, 4, 4), dtype=np.float32)
    for x in range(values.shape[0]):
        values[x, :, :] = x
    volume = VoxelVolume(
        dimensions=(4, 4, 4),
        voxel_size=(0.5, 1.0, 2.0),
        grid_level_set=True,
        scalar_type="float32",
        values=values,
        min_value=0.0,
        max_value=3.0,
        metadata={"default_iso_value": 1.5},
    )

    mesh = voxel_to_mesh_dual(volume, relax_disoriented_triangles=False)
    sdk_mesh = GeometrySDK().voxel_to_mesh_dual(volume, relax_disoriented_triangles=False)

    assert mesh.vertex_count == 9
    assert mesh.face_count == 8
    assert mesh.metadata["relax_disoriented_triangles"] is False
    np.testing.assert_allclose(sdk_mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(sdk_mesh.faces, mesh.faces)


def test_voxel_to_mesh_dual_vdb_payload_meshes_openvdb_dense_leaf_through_rust() -> None:
    values = [float(x) for z in range(8) for y in range(8) for x in range(8)]
    payload = synthetic_openvdb_single_dense_leaf(values)

    mesh = voxel_to_mesh_dual_vdb_payload(
        payload,
        dimensions=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=3.5,
    )
    sdk_mesh = GeometrySDK().voxel_to_mesh_dual_vdb_payload(
        payload,
        dimensions=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=3.5,
    )

    assert mesh.vertex_count == 49
    assert mesh.face_count == 72
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [1.75, 0.25, 0.25])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [1.75, 3.25, 3.25])
    assert mesh.metadata["source"] == "voxel_to_mesh_dual_vdb_payload"
    assert mesh.metadata["iso_value"] == 3.5
    assert mesh.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface"
    assert "direct .vdb" in mesh.metadata["meshlib_algorithm_reference"]
    assert (
        mesh.metadata["parity_status"]
        == "openvdb_dense_floatgrid_dual_meshing_backed_sparse_adaptivity_pending"
    )
    np.testing.assert_allclose(sdk_mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(sdk_mesh.faces, mesh.faces)


def test_voxel_to_mesh_dual_vdb_payload_preserves_openvdb_active_bbox_origin_through_rust() -> None:
    values = [float(x) for z in range(8) for y in range(8) for x in range(8)]
    payload = synthetic_openvdb_single_dense_leaf(values, leaf_origin=(8, 16, 24))

    mesh = voxel_to_mesh_dual_vdb_payload(
        payload,
        dimensions=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=3.5,
    )

    assert mesh.vertex_count == 49
    assert mesh.face_count == 72
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [5.75, 8.25, 12.25])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [5.75, 11.25, 15.25])
    assert mesh.metadata["source"] == "voxel_to_mesh_dual_vdb_payload"


def test_voxel_to_mesh_dual_vdb_payload_accepts_distinct_openvdb_topology_and_buffer_masks_through_rust() -> None:
    values = [float(x) for z in range(8) for y in range(8) for x in range(8)]
    payload = synthetic_openvdb_single_dense_leaf(
        values,
        active_offsets=[0, 83, 511],
        buffer_offsets=list(range(512)),
    )

    mesh = voxel_to_mesh_dual_vdb_payload(
        payload,
        dimensions=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=3.5,
    )

    assert mesh.vertex_count == 49
    assert mesh.face_count == 72
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [1.75, 0.25, 0.25])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [1.75, 3.25, 3.25])
    assert mesh.metadata["source"] == "voxel_to_mesh_dual_vdb_payload"


def test_voxel_to_mesh_dual_vdb_payload_pads_tight_openvdb_active_bbox_through_rust() -> None:
    values = [1.0] * 512
    values[0] = -1.0
    payload = synthetic_openvdb_single_dense_leaf(
        values,
        file_bbox_min=(0, 0, 0),
        file_bbox_max=(0, 0, 0),
        active_offsets=[0],
        buffer_offsets=[0],
        root_background=1.0,
        active_mask_compression=True,
    )

    mesh = voxel_to_mesh_dual_vdb_payload(
        payload,
        dimensions=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=0.0,
        relax_disoriented_triangles=False,
    )

    assert mesh.vertex_count == 8
    assert mesh.face_count == 12
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [-1.0 / 12.0] * 3)
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [1.0 / 12.0] * 3)
    assert mesh.metadata["source"] == "voxel_to_mesh_dual_vdb_payload"


def test_voxel_to_mesh_dual_vdb_payload_pads_sparse_openvdb_active_window_boundary_through_rust() -> None:
    active_offsets = [
        x * 64 + y * 8 + z
        for x in range(2)
        for y in range(2)
        for z in range(2)
    ]
    values = [1.0] * 512
    for offset in active_offsets:
        local_x = offset >> 6
        local_y = (offset & 63) >> 3
        local_z = offset & 7
        values[local_x + local_y * 8 + local_z * 64] = -1.0
    payload = synthetic_openvdb_single_dense_leaf(
        values,
        file_bbox_min=(0, 0, 0),
        file_bbox_max=(1, 1, 1),
        active_offsets=active_offsets,
        buffer_offsets=active_offsets,
        root_background=1.0,
        active_mask_compression=True,
    )

    mesh = voxel_to_mesh_dual_vdb_payload(
        payload,
        dimensions=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=0.0,
        relax_disoriented_triangles=False,
    )

    assert mesh.vertex_count == 26
    assert mesh.face_count == 48
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [-0.25, -0.25, -0.25])
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [0.75, 0.75, 0.75])
    assert mesh.metadata["source"] == "voxel_to_mesh_dual_vdb_payload"


def test_voxel_to_mesh_dual_vdb_payload_pads_full_leaf_span_sparse_openvdb_mask_through_rust() -> None:
    active_offsets = [0, 511]
    values = [1.0] * 512
    values[0] = -1.0
    values[511] = -1.0
    payload = synthetic_openvdb_single_dense_leaf(
        values,
        active_offsets=active_offsets,
        buffer_offsets=active_offsets,
        root_background=1.0,
        active_mask_compression=True,
    )

    mesh = voxel_to_mesh_dual_vdb_payload(
        payload,
        dimensions=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=0.0,
        relax_disoriented_triangles=False,
    )

    assert mesh.vertex_count == 16
    assert mesh.face_count == 24
    np.testing.assert_allclose(mesh.vertices.min(axis=0), [-1.0 / 12.0] * 3)
    np.testing.assert_allclose(mesh.vertices.max(axis=0), [43.0 / 12.0] * 3)
    assert mesh.metadata["source"] == "voxel_to_mesh_dual_vdb_payload"


def test_voxel_volume_render_data_matches_meshlib_normalized_active_box_contract() -> None:
    volume_values = np.arange(24, dtype=np.float32).reshape((4, 3, 2), order="F")
    volume = VoxelVolume(
        dimensions=(4, 3, 2),
        voxel_size=(0.5, 1.0, 2.0),
        grid_level_set=False,
        scalar_type="float32",
        values=volume_values,
        min_value=0.0,
        max_value=23.0,
    )

    render_data = voxel_volume_render_data(
        volume,
        active_min_corner=(1, 1, 0),
        active_dimensions=(2, 2, 2),
    )

    assert render_data.dimensions == (2, 2, 2)
    assert render_data.voxel_size == (0.5, 1.0, 2.0)
    assert render_data.coordinates == [
        (1, 1, 0),
        (2, 1, 0),
        (1, 2, 0),
        (2, 2, 0),
        (1, 1, 1),
        (2, 1, 1),
        (1, 2, 1),
        (2, 2, 1),
    ]
    assert render_data.source_indices == [5, 6, 9, 10, 17, 18, 21, 22]
    np.testing.assert_allclose(
        render_data.values,
        np.array([5.0, 6.0, 9.0, 10.0, 17.0, 18.0, 21.0, 22.0], dtype=np.float32) / 23.0,
    )
    assert render_data.min_value == 0.0
    assert render_data.max_value == 1.0
    assert render_data.metadata["meshlib_reference"] == "ObjectVoxels::prepareDataForVolumeRendering"


def test_voxel_volume_render_lut_matches_meshlib_dense_map_contract() -> None:
    gray = voxel_volume_render_lut(
        lut_type="gray_shades",
        alpha_type="linear_increasing",
        alpha_limit=10,
    )
    assert gray.colors_rgba == [(255, 255, 255, 0), (0, 0, 0, 10)]

    one = voxel_volume_render_lut(
        lut_type="one_color",
        alpha_type="linear_decreasing",
        alpha_limit=10,
        one_color=(12, 34, 56, 200),
    )
    assert one.colors_rgba == [(12, 34, 56, 10), (12, 34, 56, 0)]

    rainbow = voxel_volume_render_lut(
        lut_type="rainbow",
        alpha_type="linear_increasing",
        alpha_limit=14,
    )
    assert rainbow.colors_rgba == [
        (255, 0, 0, 0),
        (255, 127, 0, 2),
        (255, 255, 0, 4),
        (0, 255, 0, 6),
        (0, 0, 255, 8),
        (75, 0, 130, 10),
        (148, 0, 211, 12),
    ]
    assert rainbow.metadata["meshlib_reference"] == "RenderVolumeObject::bindVolume_ denseMap"


def test_voxel_volume_render_ray_matches_meshlib_fixed_step_compositing_contract() -> None:
    volume_values = np.array([0.25, 0.5, 0.75], dtype=np.float32).reshape((3, 1, 1), order="F")

    ray = voxel_volume_render_ray(
        volume_values,
        shape=(3, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        min_corner=(0, 0, 0),
        ray_start=(-0.5, 0.5, 0.5),
        ray_direction=(1.0, 0.0, 0.0),
        sampling_step=1.0,
        min_value=0.0,
        max_value=1.0,
        lut_type="one_color",
        alpha_type="constant",
        alpha_limit=128,
        one_color=(100, 50, 25, 255),
        active_indices=(0, 2),
        max_steps=16,
    )

    sample_alpha = 128.0 / 255.0
    expected_alpha = sample_alpha + sample_alpha * (1.0 - sample_alpha)
    assert ray.accepted_indices == [0, 2]
    assert ray.visited_indices == [0, 1, 2]
    assert ray.first_opaque_world == (0.5, 0.5, 0.5)
    np.testing.assert_allclose(
        ray.color_rgba,
        np.array([100.0 / 255.0, 50.0 / 255.0, 25.0 / 255.0, expected_alpha], dtype=np.float32),
        atol=1e-6,
    )
    assert ray.metadata["meshlib_reference"] == "MRVolumeShader fixed-step ray compositing"


def test_voxel_volume_render_ray_matches_meshlib_clipping_plane_discard_contract() -> None:
    volume_values = np.array([0.25, 0.5, 0.75], dtype=np.float32).reshape((3, 1, 1), order="F")

    ray = voxel_volume_render_ray(
        volume_values,
        shape=(3, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        min_corner=(0, 0, 0),
        ray_start=(-0.5, 0.5, 0.5),
        ray_direction=(1.0, 0.0, 0.0),
        sampling_step=1.0,
        min_value=0.0,
        max_value=1.0,
        lut_type="one_color",
        alpha_type="constant",
        alpha_limit=128,
        one_color=(100, 50, 25, 255),
        clipping_plane=(1.0, 0.0, 0.0, 0.75),
        max_steps=16,
    )

    sample_alpha = 128.0 / 255.0
    assert ray.visited_indices == [0]
    assert ray.accepted_indices == [0]
    assert ray.first_opaque_world == (0.5, 0.5, 0.5)
    np.testing.assert_allclose(
        ray.color_rgba,
        np.array([100.0 / 255.0, 50.0 / 255.0, 25.0 / 255.0, sample_alpha], dtype=np.float32),
        atol=1e-6,
    )
    assert ray.metadata["clipping_plane"] == (1.0, 0.0, 0.0, 0.75)


def test_voxel_volume_render_ray_matches_meshlib_value_gradient_zero_normal_skip_contract() -> None:
    volume_values = np.full((3, 1, 1), 0.5, dtype=np.float32)

    ray = voxel_volume_render_ray(
        volume_values,
        shape=(3, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        min_corner=(0, 0, 0),
        ray_start=(-0.5, 0.5, 0.5),
        ray_direction=(1.0, 0.0, 0.0),
        sampling_step=1.0,
        min_value=0.0,
        max_value=1.0,
        lut_type="one_color",
        alpha_type="constant",
        alpha_limit=128,
        one_color=(100, 50, 25, 255),
        shading_mode="value_gradient",
        max_steps=16,
    )

    assert ray.visited_indices == [0, 1, 2]
    assert ray.accepted_indices == []
    assert ray.first_opaque_world is None
    np.testing.assert_allclose(ray.color_rgba, np.zeros(4, dtype=np.float32), atol=1e-6)
    assert ray.metadata["shading_mode"] == "value_gradient"


def test_voxel_volume_render_ray_matches_meshlib_alpha_gradient_no_zero_normal_discard_contract() -> None:
    volume_values = np.full((3, 1, 1), 0.5, dtype=np.float32)

    ray = voxel_volume_render_ray(
        volume_values,
        shape=(3, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        min_corner=(0, 0, 0),
        ray_start=(-0.5, 0.5, 0.5),
        ray_direction=(1.0, 0.0, 0.0),
        sampling_step=1.0,
        min_value=0.0,
        max_value=1.0,
        lut_type="one_color",
        alpha_type="constant",
        alpha_limit=128,
        one_color=(100, 50, 25, 255),
        shading_mode="alpha_gradient",
        max_steps=16,
    )

    sample_alpha = 128.0 / 255.0
    expected_alpha = 1.0 - (1.0 - sample_alpha) ** 3
    assert ray.visited_indices == [0, 1, 2]
    assert ray.accepted_indices == [0, 1, 2]
    assert ray.first_opaque_world == (0.5, 0.5, 0.5)
    np.testing.assert_allclose(
        ray.color_rgba,
        np.array([100.0 / 255.0, 50.0 / 255.0, 25.0 / 255.0, expected_alpha], dtype=np.float32),
        atol=1e-6,
    )
    assert ray.metadata["shading_mode"] == "alpha_gradient"


def test_voxel_volume_render_ray_matches_meshlib_shade_color_lighting_contract() -> None:
    volume_values = np.array([0.0, 0.5, 1.0], dtype=np.float32).reshape((3, 1, 1), order="F")

    ray = voxel_volume_render_ray(
        volume_values,
        shape=(3, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        min_corner=(0, 0, 0),
        ray_start=(-0.5, 0.5, 0.5),
        ray_direction=(1.0, 0.0, 0.0),
        sampling_step=1.0,
        min_value=0.0,
        max_value=1.0,
        lut_type="one_color",
        alpha_type="constant",
        alpha_limit=128,
        one_color=(100, 50, 25, 255),
        shading_mode="value_gradient",
        light_pos_eye=(-10.0, 0.5, 0.5),
        ambient_strength=0.25,
        specular_strength=0.0,
        spec_exp=16.0,
        max_steps=16,
    )

    sample_alpha = 128.0 / 255.0
    expected_alpha = 1.0 - (1.0 - sample_alpha) ** 3
    expected_factor = 1.25
    assert ray.accepted_indices == [0, 1, 2]
    assert ray.metadata["lighting"]["meshlib_shader"] == "shadeColor"
    np.testing.assert_allclose(
        ray.color_rgba,
        np.array(
            [
                expected_factor * 100.0 / 255.0,
                expected_factor * 50.0 / 255.0,
                expected_factor * 25.0 / 255.0,
                expected_alpha,
            ],
            dtype=np.float32,
        ),
        atol=1e-6,
    )


def test_voxel_volume_render_ray_matches_meshlib_voxel_boundary_traversal_contract() -> None:
    volume_values = np.array([0.25, 0.5, 0.75], dtype=np.float32).reshape((3, 1, 1), order="F")

    ray = voxel_volume_render_ray(
        volume_values,
        shape=(3, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        min_corner=(0, 0, 0),
        ray_start=(-0.5, 0.5, 0.5),
        ray_direction=(1.0, 0.0, 0.0),
        sampling_step=0.0,
        min_value=0.0,
        max_value=1.0,
        lut_type="one_color",
        alpha_type="constant",
        alpha_limit=128,
        one_color=(100, 50, 25, 255),
        max_steps=3,
    )

    sample_alpha = 128.0 / 255.0
    expected_alpha = 1.0 - (1.0 - sample_alpha) ** 3
    assert ray.visited_indices == [0, 1, 2]
    assert ray.accepted_indices == [0, 1, 2]
    assert ray.first_opaque_world == (0.0, 0.5, 0.5)
    np.testing.assert_allclose(
        ray.color_rgba,
        np.array([100.0 / 255.0, 50.0 / 255.0, 25.0 / 255.0, expected_alpha], dtype=np.float32),
        atol=1e-6,
    )
    assert ray.metadata["meshlib_branch"] == "step <= 0 voxel-boundary rayVoxelIntersection traversal"


def test_voxel_move_mesh_to_max_deriv_matches_meshlib_cubic_shift_contract() -> None:
    volume = np.zeros((2, 2, 6), dtype=np.float32)
    for z in range(volume.shape[2]):
        volume[:, :, z] = float((z - 2.5) ** 2)
    mesh = MeshDocument(
        vertices=np.array(
            [[0.25, 0.25, 2.5], [1.25, 0.25, 2.5], [0.25, 1.25, 2.5]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )

    refined = voxel_move_mesh_to_max_deriv(
        mesh,
        volume,
        voxel_size=(1.0, 1.0, 1.0),
        iters=1,
        sample_points=6,
        degree=3,
        outlier_threshold=1.0,
        intermediate_smooth_force=0.0,
        preparation_smooth_force=0.0,
        smooth_shift_iterations=0,
        final_relax_iterations=0,
        final_relax_force=0.0,
    )
    sdk_refined = GeometrySDK().voxel_move_mesh_to_max_deriv(
        mesh,
        volume,
        voxel_size=(1.0, 1.0, 1.0),
        iters=1,
        sample_points=6,
        degree=3,
        outlier_threshold=1.0,
        intermediate_smooth_force=0.0,
        preparation_smooth_force=0.0,
        smooth_shift_iterations=0,
        final_relax_iterations=0,
        final_relax_force=0.0,
    )

    assert refined.metadata["source"] == "voxel_move_mesh_to_max_deriv"
    assert refined.metadata["meshlib_reference"] == "MR::moveMeshToVoxelMaxDeriv"
    assert refined.metadata["corrected_indices"] == [0, 1, 2]
    np.testing.assert_allclose(refined.vertices[:, :2], mesh.vertices[:, :2])
    np.testing.assert_allclose(refined.vertices[:, 2], [2.4, 2.4, 2.4])
    np.testing.assert_allclose(sdk_refined.vertices, refined.vertices)


def test_voxel_move_mesh_to_max_deriv_supports_meshlib_degree_six_contract() -> None:
    volume = np.zeros((2, 2, 7), dtype=np.float32)
    for z in range(volume.shape[2]):
        volume[:, :, z] = float((z - 3.0) ** 2)
    mesh = MeshDocument(
        vertices=np.array(
            [[0.25, 0.25, 3.0], [1.25, 0.25, 3.0], [0.25, 1.25, 3.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )

    refined = voxel_move_mesh_to_max_deriv(
        mesh,
        volume,
        voxel_size=(1.0, 1.0, 1.0),
        iters=1,
        sample_points=7,
        degree=6,
        outlier_threshold=2.0,
        intermediate_smooth_force=0.0,
        preparation_smooth_force=0.0,
        smooth_shift_iterations=0,
        final_relax_iterations=0,
        final_relax_force=0.0,
    )

    assert refined.metadata["settings"]["degree"] == 6
    assert refined.metadata["corrected_indices"] == [0, 1, 2]
    np.testing.assert_allclose(refined.vertices[:, :2], mesh.vertices[:, :2])
    np.testing.assert_allclose(refined.vertices[:, 2], [2.9, 2.9, 2.9])


def test_voxel_to_mesh_smart_uses_single_rust_conversion_kernel() -> None:
    volume = np.zeros((2, 2, 6), dtype=np.float32)
    for z in range(volume.shape[2]):
        volume[:, :, z] = float((z - 2.5) ** 2)

    mesh = voxel_to_mesh_smart(
        volume,
        voxel_size=(1.0, 1.0, 1.0),
        iso_value=0.25,
        iters=1,
        sample_points=6,
        degree=3,
        outlier_threshold=1.0,
        intermediate_smooth_force=0.0,
        preparation_smooth_force=0.0,
        smooth_shift_iterations=0,
        final_relax_iterations=0,
        final_relax_force=0.0,
    )

    assert mesh.vertex_count > 0
    assert mesh.face_count > 0
    assert mesh.metadata["source"] == "voxel_to_mesh_smart"
    assert mesh.metadata["smart_conversion"]["settings"]["sample_points"] == 6
    assert mesh.metadata["smart_conversion"]["settings"]["degree"] == 3
    assert mesh.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface + MR::moveMeshToVoxelMaxDeriv"
    np.testing.assert_allclose(GeometrySDK().voxel_to_mesh_smart(volume, voxel_size=(1.0, 1.0, 1.0), iso_value=0.25).faces, mesh.faces)


def test_sdf_shell_keeps_wall_band_and_removes_deep_interior() -> None:
    grid = sample_sdf_grid(cube(size=4.0), voxel_size_mm=0.5, padding_mm=1.0)
    shell = sdf_shell(grid, wall_thickness_mm=1.0)

    center = tuple(size // 2 for size in shell.shape)
    assert grid.values[center] < 0.0
    assert shell.values[center] > 0.0
    assert np.any(shell.values < 0.0)


def test_engine_exposes_sdf_boolean_helpers() -> None:
    sdk = GeometrySDK()
    grid = sdk.sample_sdf_grid(cube(size=2.0), voxel_size_mm=1.0, padding_mm=1.0)
    expanded = sdk.sdf_offset(grid, 0.5)
    shell = sdk.sdf_shell(expanded, 0.5)

    assert np.allclose(sdk.sdf_union(grid, expanded).values, expanded.values)
    assert shell.values.shape == grid.values.shape


def test_extract_surface_mesh_from_cube_sdf_is_closed() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = extract_surface_mesh(grid)
    health = compute_mesh_health(mesh)
    stats = compute_mesh_stats(mesh)

    assert mesh.vertex_count > 0
    assert mesh.face_count > 0
    assert health.is_closed
    assert health.holes_count == 0
    assert np.isclose(stats.volume_mm3, 8.0, atol=4.0)


def test_extract_surface_mesh_from_shell_has_closed_boundaries() -> None:
    grid = sample_sdf_grid(cube(size=4.0), voxel_size_mm=0.5, padding_mm=1.0)
    shell = sdf_shell(grid, wall_thickness_mm=1.0)
    mesh = extract_surface_mesh(shell)
    health = compute_mesh_health(mesh)
    stats = compute_mesh_stats(mesh)

    assert health.is_closed
    assert stats.connected_components >= 2
    assert stats.volume_mm3 > 0.0


def test_extract_difference_surface_mesh_is_closed() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=1.0)
    expanded = sdf_offset(grid, 0.75)
    difference = sdf_difference(expanded, grid)
    mesh = extract_surface_mesh(difference)

    assert compute_mesh_health(mesh).is_closed
    assert mesh.face_count > 0


def test_engine_exposes_sdf_surface_extraction() -> None:
    sdk = GeometrySDK()
    grid = sdk.sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = sdk.extract_sdf_surface(grid)

    assert sdk.health(mesh).is_closed


def test_marching_tetrahedra_extracts_closed_cube_isosurface() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = extract_marching_tetrahedra(grid)
    health = compute_mesh_health(mesh)
    stats = compute_mesh_stats(mesh)

    assert mesh.vertex_count > 0
    assert mesh.face_count > 0
    assert health.is_closed
    assert np.isclose(stats.volume_mm3, 8.0, atol=2.0)


def test_rust_accelerated_marching_tetrahedra_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    python_mesh = extract_marching_tetrahedra(grid)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_mesh = extract_marching_tetrahedra(grid)

    assert compute_mesh_health(rust_mesh).is_closed
    assert rust_mesh.vertex_count == python_mesh.vertex_count
    assert rust_mesh.face_count == python_mesh.face_count
    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-9)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_rust_accelerated_sdf_boolean_marching_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    a = cube(size=2.0)
    b = cube(size=2.0).copy(vertices=cube(size=2.0).vertices + np.array([1.0, 0.0, 0.0]))
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    grid_a, grid_b = sample_aligned_sdf_grids([a, b], voxel_size_mm=0.5, origin_phase=(0.125, 0.125, 0.125))
    python_mesh = extract_marching_tetrahedra(sdf_difference(grid_a, grid_b))
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_mesh = extract_boolean_marching_tetrahedra(grid_a, grid_b, operation="difference")

    assert rust_mesh is not None
    assert rust_mesh.vertex_count == python_mesh.vertex_count
    assert rust_mesh.face_count == python_mesh.face_count
    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-9)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_rust_accelerated_sdf_offset_marching_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=1.0)
    python_mesh = extract_marching_tetrahedra(sdf_offset(grid, 0.5))
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_mesh = extract_offset_marching_tetrahedra(grid, offset_mm=0.5)

    assert rust_mesh is not None
    assert rust_mesh.vertex_count == python_mesh.vertex_count
    assert rust_mesh.face_count == python_mesh.face_count
    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-9)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_rust_accelerated_sdf_shell_marching_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    grid = sample_sdf_grid(cube(size=4.0), voxel_size_mm=0.5, padding_mm=1.0)
    python_mesh = extract_marching_tetrahedra(sdf_shell(grid, wall_thickness_mm=1.0))
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_mesh = extract_shell_marching_tetrahedra(grid, wall_thickness_mm=1.0)

    assert rust_mesh is not None
    assert rust_mesh.vertex_count == python_mesh.vertex_count
    assert rust_mesh.face_count == python_mesh.face_count
    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-9)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_rust_accelerated_face_orientation_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    faces = np.array([[0, 1, 2], [1, 2, 3], [4, 5, 6]], dtype=np.int64)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    python_faces, python_components = _orient_faces_consistently(faces)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_faces, rust_components = _orient_faces_consistently(faces)

    assert np.array_equal(rust_faces, python_faces)
    assert rust_components == python_components


def test_marching_tetrahedra_shell_output_is_closed() -> None:
    grid = sample_sdf_grid(cube(size=4.0), voxel_size_mm=0.5, padding_mm=1.0)
    shell = sdf_shell(grid, wall_thickness_mm=1.0)
    mesh = extract_marching_tetrahedra(shell)

    assert compute_mesh_health(mesh).is_closed
    assert mesh.face_count > 0


def test_marching_tetrahedra_handles_low_res_ring() -> None:
    source = ring(radial_segments=16, tube_segments=8)
    grid = sample_sdf_grid(source, voxel_size_mm=1.5, padding_mm=1.5)
    mesh = extract_marching_tetrahedra(grid)
    health = compute_mesh_health(mesh)
    stats = compute_mesh_stats(mesh)
    source_stats = compute_mesh_stats(source)

    assert mesh.vertex_count > 0
    assert mesh.face_count > 0
    assert health.boundary_edge_count == 0
    assert stats.volume_mm3 > source_stats.volume_mm3 * 0.4
    assert stats.volume_mm3 < source_stats.volume_mm3 * 1.5


def test_engine_exposes_marching_isosurface_extraction() -> None:
    sdk = GeometrySDK()
    grid = sdk.sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = sdk.extract_sdf_isosurface(grid)

    assert sdk.health(mesh).is_closed


def test_voxel_offset_mesh_expands_cube_volume() -> None:
    source = cube(size=2.0)
    expanded = voxel_offset_mesh(source, offset_mm=0.5, voxel_size_mm=0.5)
    source_stats = compute_mesh_stats(source)
    expanded_stats = compute_mesh_stats(expanded)

    assert compute_mesh_health(expanded).is_closed
    assert expanded_stats.volume_mm3 > source_stats.volume_mm3


def test_voxel_offset_mesh_accepts_official_inward_offset() -> None:
    source = cube(size=4.0)
    shrunk = voxel_offset_mesh(source, offset_mm=-0.5, voxel_size_mm=0.5)
    source_stats = compute_mesh_stats(source)
    shrunk_stats = compute_mesh_stats(shrunk)

    assert compute_mesh_health(shrunk).is_closed
    assert 0.0 < shrunk_stats.volume_mm3 < source_stats.volume_mm3


def test_voxel_thicken_mesh_keeps_original_and_offset_layers() -> None:
    source = cube(size=2.0)
    outward = voxel_offset_mesh(source, offset_mm=0.5, voxel_size_mm=0.5)
    thickened = voxel_thicken_mesh(source, thickness_mm=0.5, voxel_size_mm=0.5)
    hollowed = voxel_thicken_mesh(source, thickness_mm=-0.5, voxel_size_mm=0.5)

    assert compute_mesh_health(thickened).is_closed
    assert compute_mesh_health(hollowed).is_closed
    assert thickened.vertex_count == outward.vertex_count + source.vertex_count
    assert thickened.face_count == outward.face_count + source.face_count
    assert hollowed.vertex_count > source.vertex_count
    assert hollowed.face_count > source.face_count
    assert compute_mesh_stats(thickened).volume_mm3 > 0.0
    assert compute_mesh_stats(hollowed).volume_mm3 > 0.0


def test_voxel_weighted_shell_mesh_applies_region_additive_weight() -> None:
    source = cube(size=2.0)
    weighted_region = RegionEntry(
        region_id="corner",
        label="Corner",
        vertex_indices=np.asarray([6], dtype=np.int64),
        coverage_pct=12.5,
        protected_by_default=False,
        allowed_operations=["weighted_shell"],
    )
    constant_offset = voxel_offset_mesh(source, offset_mm=0.2, voxel_size_mm=0.5, padding_mm=1.0)
    weighted_shell = voxel_weighted_shell_mesh(
        source,
        regions=[weighted_region],
        region_weights={"corner": 0.45},
        offset_mm=0.2,
        voxel_size_mm=0.5,
        padding_mm=1.0,
        interpolation_distance_mm=1.75,
    )

    assert compute_mesh_health(weighted_shell).is_closed
    assert compute_mesh_stats(weighted_shell).volume_mm3 > compute_mesh_stats(constant_offset).volume_mm3
    assert weighted_shell.metadata["source"] == "rust_voxel_weighted_shell"
    assert weighted_shell.metadata["meshlib_reference"] == "MR::WeightedShell::meshShell"


def test_voxel_partial_offset_mesh_expands_selected_region_less_than_global_offset() -> None:
    source = cube(size=2.0)
    top_region = RegionEntry(
        region_id="top",
        label="Top",
        vertex_indices=np.asarray([4, 5, 6, 7], dtype=np.int64),
        coverage_pct=50.0,
        protected_by_default=False,
        allowed_operations=["partial_offset"],
    )

    partial = voxel_partial_offset_mesh(
        source,
        regions=[top_region],
        selected_region_ids=["top"],
        offset_mm=0.4,
        voxel_size_mm=0.5,
        padding_mm=1.0,
    )
    global_offset = voxel_offset_mesh(source, offset_mm=0.4, voxel_size_mm=0.5, padding_mm=1.0)
    source_volume = compute_mesh_stats(source).volume_mm3
    partial_volume = compute_mesh_stats(partial).volume_mm3
    global_volume = compute_mesh_stats(global_offset).volume_mm3

    assert compute_mesh_health(partial).is_closed
    assert source_volume < partial_volume < global_volume
    assert partial.metadata["source"] == "rust_voxel_partial_offset"
    assert partial.metadata["meshlib_reference"] == "MR::partialOffsetMesh"
    assert partial.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MRPartialOffset.*"


def test_global_thicken_matches_current_meshlib_service_offset_contract() -> None:
    source = cube(size=2.0)
    min_target_thickness_mm = 1.0
    diagonal = float(np.linalg.norm(np.ptp(source.vertices, axis=0)))
    service_voxel_size = max(diagonal * 0.0025, min_target_thickness_mm / 4.0)

    reference = voxel_offset_mesh(
        source,
        offset_mm=min_target_thickness_mm / 2.0,
        voxel_size_mm=service_voxel_size,
    )
    thickened = global_thicken(
        source,
        min_target_thickness_mm=min_target_thickness_mm,
    )

    assert compute_mesh_health(thickened).is_closed
    assert compute_mesh_stats(thickened).volume_mm3 > compute_mesh_stats(source).volume_mm3
    assert np.allclose(thickened.vertices, reference.vertices)
    assert np.array_equal(thickened.faces, reference.faces)


def test_global_thicken_rejects_nonpositive_target() -> None:
    with pytest.raises(ValueError, match="wall thickness must be positive and finite"):
        global_thicken(cube(size=2.0), min_target_thickness_mm=0.0)


def test_voxel_offset_mesh_preserves_ring_void_volume_envelope() -> None:
    source = ring(radial_segments=16, tube_segments=8)
    expanded = voxel_offset_mesh(source, offset_mm=0.25, voxel_size_mm=0.75, refine=True)
    source_stats = compute_mesh_stats(source)
    expanded_stats = compute_mesh_stats(expanded)

    assert compute_mesh_health(expanded).is_closed
    assert expanded_stats.volume_mm3 > source_stats.volume_mm3
    assert expanded_stats.volume_mm3 < source_stats.volume_mm3 * 1.75


def test_voxel_shell_mesh_returns_closed_hollow_band() -> None:
    shell = voxel_shell_mesh(cube(size=4.0), wall_thickness_mm=1.0, voxel_size_mm=0.5)
    health = compute_mesh_health(shell)
    stats = compute_mesh_stats(shell)

    assert health.is_closed
    assert stats.volume_mm3 > 0.0


def test_voxel_boolean_mesh_outputs_closed_results() -> None:
    a = cube(size=2.0)
    b_source = cube(size=2.0)
    b = b_source.copy(vertices=b_source.vertices + np.array([1.0, 0.0, 0.0]))

    union = voxel_boolean_mesh(a, b, operation="union", voxel_size_mm=0.5)
    intersection = voxel_boolean_mesh(a, b, operation="intersection", voxel_size_mm=0.5)
    difference = voxel_boolean_mesh(a, b, operation="difference", voxel_size_mm=0.5)

    assert compute_mesh_health(union).is_closed
    assert compute_mesh_health(intersection).is_closed
    assert compute_mesh_health(difference).is_closed
    assert compute_mesh_stats(union).volume_mm3 > compute_mesh_stats(a).volume_mm3
    assert compute_mesh_stats(intersection).volume_mm3 > 0.0
    assert compute_mesh_stats(difference).volume_mm3 > 0.0


def test_voxel_boolean_mesh_phase_shift_closes_grid_aligned_box_cutters() -> None:
    source = orient_faces_outward(pendant())
    cutter = box(3.0, 2.0, 8.0)

    for operation in ("difference", "intersection", "union"):
        result = voxel_boolean_mesh(source, cutter, operation=operation, voxel_size_mm=0.5, refine=True)
        health = compute_mesh_health(result)

        assert health.is_closed
        assert health.nonmanifold_edge_count == 0
        assert health.self_intersections == 0
        assert compute_mesh_stats(result).volume_mm3 > 0.0


def test_engine_exposes_voxel_mesh_operations() -> None:
    sdk = GeometrySDK()
    source = cube(size=2.0)
    expanded = sdk.voxel_offset_mesh(source, offset_mm=0.5, voxel_size_mm=0.5)
    shell = sdk.voxel_shell_mesh(cube(size=4.0), wall_thickness_mm=1.0, voxel_size_mm=0.5)
    union = sdk.voxel_boolean_mesh(source, expanded, operation="union", voxel_size_mm=0.5)
    thickened = sdk.global_thicken(source, min_target_thickness_mm=1.0)

    assert sdk.health(expanded).is_closed
    assert sdk.health(shell).is_closed
    assert sdk.health(union).is_closed
    assert sdk.health(thickened).is_closed


def test_sdf_value_sampling_interpolates_cube_grid() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    values = sample_sdf_values(grid, np.array([[0.0, 0.0, 0.0], [1.5, 0.0, 0.0]], dtype=np.float64))

    assert values[0] < 0.0
    assert values[1] > 0.0


def test_project_vertices_to_sdf_moves_points_toward_iso_surface() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = cube(size=1.8)
    before = np.mean(np.abs(sample_sdf_values(grid, mesh.vertices)))
    projected = project_vertices_to_sdf(mesh, grid, iterations=4)
    after = np.mean(np.abs(sample_sdf_values(grid, projected.vertices)))

    assert after < before
    assert projected.faces.shape == mesh.faces.shape


def test_rust_accelerated_sdf_projection_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = cube(size=1.8)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    python_mesh = project_vertices_to_sdf(mesh, grid, iterations=4)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_mesh = project_vertices_to_sdf(mesh, grid, iterations=4)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-6)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_rust_accelerated_laplacian_smoothing_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = extract_marching_tetrahedra(sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5))
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    python_mesh = laplacian_smooth_vertices(mesh, iterations=2, strength=0.35)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_mesh = laplacian_smooth_vertices(mesh, iterations=2, strength=0.35)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-9)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_refine_sdf_mesh_keeps_closed_mesh_and_reduces_sdf_residual() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = extract_marching_tetrahedra(grid)
    moved = mesh.copy(vertices=mesh.vertices * 0.92)
    before = np.mean(np.abs(sample_sdf_values(grid, moved.vertices)))
    refined = refine_sdf_mesh(moved, grid, smooth_iterations=1, projection_iterations=4)
    after = np.mean(np.abs(sample_sdf_values(grid, refined.vertices)))

    assert compute_mesh_health(refined).is_closed
    assert after < before


def test_rust_accelerated_refine_sdf_mesh_matches_python(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    mesh = extract_marching_tetrahedra(grid)
    moved = mesh.copy(vertices=mesh.vertices * 0.92)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    python_mesh = refine_sdf_mesh(moved, grid, smooth_iterations=1, smooth_strength=0.2, projection_iterations=4)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    rust_mesh = refine_sdf_mesh(moved, grid, smooth_iterations=1, smooth_strength=0.2, projection_iterations=4)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-6)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_refined_voxel_offset_mesh_stays_closed() -> None:
    expanded = voxel_offset_mesh(cube(size=2.0), offset_mm=0.5, voxel_size_mm=0.5, refine=True)

    assert compute_mesh_health(expanded).is_closed
    assert compute_mesh_stats(expanded).volume_mm3 > compute_mesh_stats(cube(size=2.0)).volume_mm3
