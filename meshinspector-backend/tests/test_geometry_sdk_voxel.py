from __future__ import annotations

import os

import numpy as np
import pytest

from geometry_sdk import GeometrySDK
from geometry_sdk.accelerators import rust
from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.deform.thicken import global_thicken
from geometry_sdk.repair.basic import orient_faces_outward
from geometry_sdk.testing.fixtures import box, cube, pendant, ring
from geometry_sdk.voxel.extract import extract_surface_mesh
from geometry_sdk.voxel.marching import (
    _orient_faces_consistently,
    extract_boolean_marching_tetrahedra,
    extract_marching_tetrahedra,
    extract_offset_marching_tetrahedra,
    extract_shell_marching_tetrahedra,
)
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh, voxel_offset_mesh, voxel_shell_mesh
from geometry_sdk.voxel.ops import sdf_difference, sdf_intersection, sdf_offset, sdf_shell, sdf_union
from geometry_sdk.voxel.refine import laplacian_smooth_vertices, project_vertices_to_sdf, refine_sdf_mesh
from geometry_sdk.voxel.sdf import estimate_sdf_volume, sample_aligned_sdf_grids, sample_sdf_grid, sample_sdf_values


def test_sdf_grid_samples_negative_inside_and_positive_outside_cube() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=1.0, padding_mm=1.0)

    assert grid.shape == (5, 5, 5)
    assert grid.values[2, 2, 2] < 0.0
    assert grid.values[0, 0, 0] > 0.0


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
