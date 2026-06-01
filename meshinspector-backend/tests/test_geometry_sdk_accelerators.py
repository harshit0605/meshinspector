from __future__ import annotations

import numpy as np
import pytest

from geometry_sdk.accelerators import _rust_common, rust
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.thickness import ray_thickness_at_vertices
from geometry_sdk.deform.brushes import apply_brush_strokes
from geometry_sdk.deform.local import falloff_weights, local_scoop, local_thicken, outward_directions, smooth
from geometry_sdk.deform.thicken import global_thicken
from geometry_sdk.spatial.closest_point import closest_points_on_mesh, point_mesh_distances
from geometry_sdk.spatial.intersections import self_intersecting_faces
from geometry_sdk.spatial.raycast import first_ray_hit, first_ray_hits
from geometry_sdk.spatial.signed_distance import signed_point_mesh_distances, winding_numbers
from geometry_sdk.testing.fixtures import crossing_triangles, cube
from geometry_sdk.types import BrushStroke
from geometry_sdk.voxel.marching import (
    _orient_faces_consistently,
    extract_marching_tetrahedra,
    extract_offset_marching_tetrahedra,
    extract_shell_marching_tetrahedra,
)
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh
from geometry_sdk.voxel.ops import sdf_union
from geometry_sdk.voxel.refine import laplacian_smooth_vertices, project_vertices_to_sdf, refine_sdf_mesh
from geometry_sdk.voxel.sdf import SDFGrid, sample_sdf_grid


class FakeRustModule:
    @staticmethod
    def mesh_bounds(vertices):
        assert vertices.shape == (8, 3)
        return {"min": [-1.0, -1.0, -1.0], "max": [1.0, 1.0, 1.0]}

    @staticmethod
    def mesh_stats(vertices, faces):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        return {
            "bbox_min": [-1.0, -1.0, -1.0],
            "bbox_max": [1.0, 1.0, 1.0],
            "bbox_size": [2.0, 2.0, 2.0],
            "surface_area_mm2": 24.0,
            "volume_mm3": 8.0,
            "vertex_count": 8,
            "face_count": 12,
            "connected_components": 1,
            "boundary_edge_count": 0,
        }

    @staticmethod
    def self_intersecting_faces(vertices, faces, epsilon=1e-8):
        assert epsilon == 1e-8
        if vertices.shape == (8, 3):
            assert faces.shape == (12, 3)
            return []
        assert vertices.shape == (6, 3)
        assert faces.shape == (2, 3)
        return [0, 1]

    @staticmethod
    def point_mesh_distances(points, vertices, faces):
        assert points.shape == (2, 3)
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        return np.array([1.0, 1.0], dtype=np.float32)

    @staticmethod
    def closest_points_on_mesh(points, vertices, faces):
        assert points.shape == (2, 3)
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        return {
            "closest_points": np.array([1.0, 0.0, 0.0, 0.0, 0.0, 0.0], dtype=np.float64),
            "distances": np.array([1.0, 1.0], dtype=np.float64),
            "face_indices": np.array([6, 0], dtype=np.int64),
        }

    @staticmethod
    def winding_numbers(points, vertices, faces):
        assert points.shape == (2, 3)
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        return np.array([1.0, 0.0], dtype=np.float64)

    @staticmethod
    def signed_point_mesh_distances(points, vertices, faces, winding_threshold=0.5):
        assert points.shape == (2, 3)
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert winding_threshold == 0.5
        return np.array([-1.0, 1.0], dtype=np.float32)

    @staticmethod
    def supports_winding_sign(vertices, faces, reject_self_intersections=True, max_self_intersection_faces=50000, epsilon=1e-8):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert reject_self_intersections is True
        assert max_self_intersection_faces == 50000
        assert epsilon == 1e-8
        return True

    @staticmethod
    def signed_point_mesh_distances_with_method(
        points,
        vertices,
        faces,
        sign_method="auto",
        winding_threshold=0.5,
        topology_epsilon=1e-8,
        ray_epsilon=1e-7,
    ):
        assert points.shape == (2, 3)
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert sign_method == "auto"
        assert winding_threshold == 0.5
        assert topology_epsilon == 1e-8
        assert ray_epsilon == 1e-7
        return np.array([-1.0, 1.0], dtype=np.float32)

    @staticmethod
    def ray_thickness_at_vertices(vertices, faces, epsilon=1e-5):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert epsilon == 1e-5
        return np.full(8, 2.0, dtype=np.float32)

    @staticmethod
    def sdf_grid_values(vertices, faces, origin, shape, voxel_size_mm, winding_threshold=0.5):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert origin.shape == (3,)
        assert tuple(shape.tolist()) == (5, 5, 5)
        assert voxel_size_mm == 1.0
        assert winding_threshold == 0.5
        values = np.ones(int(np.prod(shape)), dtype=np.float32)
        values[(2 * 25) + (2 * 5) + 2] = -1.0
        return values

    @staticmethod
    def sdf_boolean_values(left, right, operation):
        assert left.shape == (8,)
        assert right.shape == (8,)
        assert operation == "union"
        return np.full(8, -7.0, dtype=np.float32)

    @staticmethod
    def sdf_boolean_marching_tetrahedra(left, right, operation, origin, shape, voxel_size_mm, iso_value=0.0):
        assert left.shape == (125,)
        assert right.shape == (125,)
        assert operation == "union"
        assert origin.shape == (3,)
        assert tuple(shape.tolist()) == (5, 5, 5)
        assert voxel_size_mm == 1.0
        assert iso_value == 0.0
        return {
            "vertices": np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], dtype=np.float64),
            "faces": np.array([0, 1, 2], dtype=np.int64),
        }

    @staticmethod
    def finalized_sdf_boolean_marching_tetrahedra(left, right, operation, origin, shape, voxel_size_mm, iso_value=0.0):
        return FakeRustModule.sdf_boolean_marching_tetrahedra(
            left,
            right,
            operation,
            origin,
            shape,
            voxel_size_mm,
            iso_value,
        )

    @staticmethod
    def voxel_boolean_mesh(
        left_vertices,
        left_faces,
        right_vertices,
        right_faces,
        operation,
        voxel_size_mm,
        padding_mm=None,
        origin_phase=None,
        extractor="marching",
        refine=False,
    ):
        assert left_vertices.shape == (8, 3)
        assert left_faces.shape == (12, 3)
        assert right_vertices.shape == (8, 3)
        assert right_faces.shape == (12, 3)
        assert operation == "union"
        assert voxel_size_mm == 1.0
        assert padding_mm is None
        assert origin_phase is None or origin_phase.shape == (3,)
        assert extractor == "marching"
        assert refine is False
        return {
            "vertices": np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], dtype=np.float64),
            "faces": np.array([0, 1, 2], dtype=np.int64),
        }

    @staticmethod
    def global_thicken_mesh(vertices, faces, min_target_thickness_mm):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert min_target_thickness_mm == 1.0
        return {
            "vertices": np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], dtype=np.float64),
            "faces": np.array([0, 1, 2], dtype=np.int64),
        }

    @staticmethod
    def sdf_offset_marching_tetrahedra(values, origin, shape, voxel_size_mm, offset_mm, iso_value=0.0):
        assert values.shape == (8,)
        assert origin.shape == (3,)
        assert tuple(shape.tolist()) == (2, 2, 2)
        assert voxel_size_mm == 1.0
        assert offset_mm == 0.5
        assert iso_value == 0.0
        return {
            "vertices": np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], dtype=np.float64),
            "faces": np.array([0, 1, 2], dtype=np.int64),
        }

    @staticmethod
    def finalized_sdf_offset_marching_tetrahedra(values, origin, shape, voxel_size_mm, offset_mm, iso_value=0.0):
        return FakeRustModule.sdf_offset_marching_tetrahedra(
            values,
            origin,
            shape,
            voxel_size_mm,
            offset_mm,
            iso_value,
        )

    @staticmethod
    def sdf_shell_marching_tetrahedra(values, origin, shape, voxel_size_mm, wall_thickness_mm, iso_value=0.0):
        assert values.shape == (8,)
        assert origin.shape == (3,)
        assert tuple(shape.tolist()) == (2, 2, 2)
        assert voxel_size_mm == 1.0
        assert wall_thickness_mm == 0.5
        assert iso_value == 0.0
        return {
            "vertices": np.array([0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], dtype=np.float64),
            "faces": np.array([0, 1, 2], dtype=np.int64),
        }

    @staticmethod
    def finalized_sdf_shell_marching_tetrahedra(values, origin, shape, voxel_size_mm, wall_thickness_mm, iso_value=0.0):
        return FakeRustModule.sdf_shell_marching_tetrahedra(
            values,
            origin,
            shape,
            voxel_size_mm,
            wall_thickness_mm,
            iso_value,
        )

    @staticmethod
    def basic_repair(vertices, faces, merge_tolerance=1e-6, area_epsilon=1e-12):
        assert vertices.shape == (3, 3)
        assert faces.shape == (1, 3)
        return {
            "vertices": vertices.reshape(-1),
            "faces": faces.reshape(-1),
            "report": {
                "input_vertex_count": 3,
                "input_face_count": 1,
                "output_vertex_count": 3,
                "output_face_count": 1,
                "merged_vertices": 0,
                "removed_degenerate_faces": 0,
                "removed_unreferenced_vertices": 0,
            },
        }

    @staticmethod
    def project_vertices_to_sdf(vertices, values, origin, shape, voxel_size_mm, iso_value=0.0, iterations=3):
        assert vertices.shape == (8, 3)
        assert values.shape == (8,)
        assert origin.shape == (3,)
        assert tuple(shape.tolist()) == (2, 2, 2)
        assert voxel_size_mm == 1.0
        assert iso_value == 0.0
        assert iterations == 3
        return (vertices + np.array([1.0, 0.0, 0.0], dtype=np.float64)).reshape(-1)

    @staticmethod
    def refine_vertices_with_sdf(
        vertices,
        faces,
        values,
        origin,
        shape,
        voxel_size_mm,
        iso_value=0.0,
        smooth_iterations=1,
        smooth_strength=0.2,
        projection_iterations=3,
    ):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert values.shape == (8,)
        assert origin.shape == (3,)
        assert tuple(shape.tolist()) == (2, 2, 2)
        assert voxel_size_mm == 1.0
        assert iso_value == 0.0
        assert smooth_iterations == 1
        assert smooth_strength == 0.2
        assert projection_iterations == 3
        return (vertices + np.array([3.0, 0.0, 0.0], dtype=np.float64)).reshape(-1)

    @staticmethod
    def laplacian_smooth_vertices(vertices, faces, iterations=1, strength=0.25):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert iterations == 1
        assert strength == 0.25
        return (vertices * 0.5).reshape(-1)

    @staticmethod
    def taubin_smooth_vertices(vertices, faces, iterations=10, lamb=0.5, nu=-0.53):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert iterations == 5
        assert lamb == 0.5
        assert nu == -0.53
        return (vertices + 4.0).reshape(-1)

    @staticmethod
    def weighted_laplacian_smooth_vertices(vertices, faces, weights, iterations=1, strength=0.25, active_threshold=0.02):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert weights.shape == (8,)
        assert iterations == 5
        assert strength == 0.5
        assert active_threshold == 0.02
        return (vertices + weights[:, None]).reshape(-1)

    @staticmethod
    def falloff_weights(vertices, seed_indices, falloff_mm, cutoff_multiplier=3.0):
        assert vertices.shape == (8, 3)
        assert seed_indices.shape == (8,)
        assert falloff_mm == 1.8
        assert cutoff_multiplier == 3.0
        return np.ones(8, dtype=np.float32)

    @staticmethod
    def smooth_vertices_with_falloff(
        vertices,
        faces,
        seed_indices,
        falloff_mm,
        iterations=5,
        strength=0.5,
        active_threshold=0.02,
        cutoff_multiplier=3.0,
    ):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert seed_indices.shape == (8,)
        assert falloff_mm == 1.8
        assert iterations == 5
        assert strength == 0.5
        assert active_threshold == 0.02
        assert cutoff_multiplier == 3.0
        return (vertices + 2.0).reshape(-1)

    @staticmethod
    def outward_directions(vertices, faces):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        return np.tile(np.array([0.0, 0.0, 1.0], dtype=np.float64), (8, 1)).reshape(-1)

    @staticmethod
    def local_offset_vertices(vertices, faces, seed_indices, falloff_mm, amount_mm, cutoff_multiplier=3.0):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert seed_indices.shape == (8,)
        assert falloff_mm == 1.8
        assert amount_mm in (0.2, -0.2)
        assert cutoff_multiplier == 3.0
        return (vertices + amount_mm).reshape(-1)

    @staticmethod
    def apply_brush_strokes(
        vertices,
        faces,
        operations,
        seed_offsets,
        seed_indices,
        mask_enabled,
        mask_offsets,
        mask_indices,
        protected_offsets,
        protected_indices,
        amounts_mm,
        falloffs_mm,
        iterations,
        strengths,
        cutoff_multiplier=3.0,
    ):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert operations.tolist() == [0, 1, 2]
        assert seed_offsets.tolist() == [0, 8, 16, 24]
        assert seed_indices.shape == (24,)
        assert mask_enabled.tolist() == [0, 0, 0]
        assert mask_offsets.tolist() == [0, 0, 0, 0]
        assert mask_indices.shape == (0,)
        assert protected_offsets.tolist() == [0, 0, 0, 0]
        assert protected_indices.shape == (0,)
        assert np.allclose(amounts_mm, [0.2, 0.1, 0.0])
        assert np.allclose(falloffs_mm, [1.8, 1.8, 2.0])
        assert iterations.tolist() == [1, 1, 2]
        assert np.allclose(strengths, [0.5, 0.5, 0.25])
        assert cutoff_multiplier == 3.0
        return (vertices + 4.0).reshape(-1)

    @staticmethod
    def marching_tetrahedra(values, origin, shape, voxel_size_mm, iso_value=0.0):
        assert values.shape == (8,)
        assert origin.shape == (3,)
        assert tuple(shape.tolist()) == (2, 2, 2)
        assert voxel_size_mm == 1.0
        assert iso_value == 0.0
        return {
            "vertices": np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], dtype=np.float64),
            "faces": np.array([0, 1, 2], dtype=np.int64),
        }

    @staticmethod
    def finalized_marching_tetrahedra(values, origin, shape, voxel_size_mm, iso_value=0.0):
        return FakeRustModule.marching_tetrahedra(values, origin, shape, voxel_size_mm, iso_value)

    @staticmethod
    def orient_faces_consistently(faces):
        assert faces.ndim == 2
        assert faces.shape[1] == 3
        return {
            "faces": faces.reshape(-1).copy(),
            "component_offsets": np.array([0, faces.shape[0]], dtype=np.int64),
            "component_faces": np.arange(faces.shape[0], dtype=np.int64),
        }

    @staticmethod
    def first_ray_hit(vertices, faces, origin, direction, epsilon, ignored_faces):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert origin.shape == (3,)
        assert direction.shape == (3,)
        assert ignored_faces.shape == (0,)
        assert epsilon == 1e-8
        return {"face_index": 2, "distance": 2.0, "point": [0.0, 0.0, 1.0]}

    @staticmethod
    def first_ray_hits(vertices, faces, origins, directions, epsilon, ignored_faces):
        assert vertices.shape == (8, 3)
        assert faces.shape == (12, 3)
        assert origins.shape == (2, 3)
        assert directions.shape == (2, 3)
        assert ignored_faces.shape == (0,)
        assert epsilon == 1e-8
        return {
            "face_indices": np.array([2, -1], dtype=np.int64),
            "distances": np.array([2.0, np.inf], dtype=np.float64),
            "points": np.array([0.0, 0.0, 1.0, np.nan, np.nan, np.nan], dtype=np.float64),
        }


def test_accelerator_auto_mode_falls_back_to_python_when_rust_is_absent(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "auto")
    monkeypatch.setattr(_rust_common, "_rs", None)

    assert rust.backend_name() == "python"
    assert rust.point_mesh_distances(np.array([[2.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0)) is None


def test_accelerator_python_mode_ignores_available_rust(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    assert rust.mesh_stats(cube(size=2.0)) is None
    assert rust.self_intersecting_faces(crossing_triangles()) is None
    assert rust.point_mesh_distances(np.array([[2.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0)) is None
    assert rust.closest_points_on_mesh(np.array([[2.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0)) is None
    assert rust.winding_numbers(np.array([[0.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0)) is None
    assert rust.signed_point_mesh_distances(np.array([[0.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0)) is None
    assert rust.ray_thickness_at_vertices(cube(size=2.0)) is None
    assert (
        rust.sdf_grid_values(
            cube(size=2.0),
            origin=(0.0, 0.0, 0.0),
            shape=(5, 5, 5),
            voxel_size_mm=1.0,
        )
        is None
    )
    assert (
        rust.sdf_boolean_values(
            np.zeros((2, 2, 2), dtype=np.float32),
            np.ones((2, 2, 2), dtype=np.float32),
            operation="union",
        )
        is None
    )
    assert (
        rust.sdf_boolean_marching_tetrahedra(
            np.zeros((2, 2, 2), dtype=np.float32),
            np.ones((2, 2, 2), dtype=np.float32),
            operation="union",
            origin=(0.0, 0.0, 0.0),
            shape=(2, 2, 2),
            voxel_size_mm=1.0,
        )
        is None
    )
    assert (
        rust.sdf_offset_marching_tetrahedra(
            np.zeros((2, 2, 2), dtype=np.float32),
            origin=(0.0, 0.0, 0.0),
            shape=(2, 2, 2),
            voxel_size_mm=1.0,
            offset_mm=0.5,
        )
        is None
    )
    assert (
        rust.sdf_shell_marching_tetrahedra(
            np.zeros((2, 2, 2), dtype=np.float32),
            origin=(0.0, 0.0, 0.0),
            shape=(2, 2, 2),
            voxel_size_mm=1.0,
            wall_thickness_mm=0.5,
        )
        is None
    )
    assert (
        rust.marching_tetrahedra(
            np.zeros((2, 2, 2), dtype=np.float32),
            origin=(0.0, 0.0, 0.0),
            shape=(2, 2, 2),
            voxel_size_mm=1.0,
        )
        is None
    )
    assert (
        rust.project_vertices_to_sdf(
            cube(size=2.0).vertices,
            np.zeros((2, 2, 2), dtype=np.float32),
            origin=(0.0, 0.0, 0.0),
            shape=(2, 2, 2),
            voxel_size_mm=1.0,
        )
        is None
    )
    assert (
        rust.refine_vertices_with_sdf(
            cube(size=2.0),
            np.zeros((2, 2, 2), dtype=np.float32),
            origin=(0.0, 0.0, 0.0),
            shape=(2, 2, 2),
            voxel_size_mm=1.0,
        )
        is None
    )
    assert rust.laplacian_smooth_vertices(cube(size=2.0)) is None
    assert rust.taubin_smooth_vertices(cube(size=2.0)) is None
    assert rust.weighted_laplacian_smooth_vertices(cube(size=2.0), np.ones(8, dtype=np.float32)) is None
    assert rust.falloff_weights(cube(size=2.0), np.arange(8, dtype=np.int32), falloff_mm=1.8) is None
    assert rust.smooth_vertices_with_falloff(cube(size=2.0), np.arange(8, dtype=np.int32), falloff_mm=1.8) is None
    assert rust.outward_directions(cube(size=2.0)) is None
    assert rust.local_offset_vertices(cube(size=2.0), np.arange(8, dtype=np.int32), falloff_mm=1.8, amount_mm=0.2) is None
    assert (
        rust.apply_brush_strokes(
            cube(size=2.0),
            [BrushStroke("thicken", np.arange(8, dtype=np.int32), amount_mm=0.2, falloff_mm=1.8)],
        )
        is None
    )
    assert rust.orient_faces_consistently(np.array([[0, 1, 2]], dtype=np.int64)) is None
    assert rust.first_ray_hit(cube(size=2.0), (0.0, 0.0, 3.0), (0.0, 0.0, -1.0)) is None
    assert (
        rust.first_ray_hits(
            cube(size=2.0),
            np.array([[0.0, 0.0, 3.0]], dtype=np.float64),
            np.array([[0.0, 0.0, -1.0]], dtype=np.float64),
        )
        is None
    )
    assert rust.backend_name() == "python"


def test_accelerator_rust_mode_uses_native_stats(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    stats = compute_mesh_stats(cube(size=2.0))

    assert rust.backend_name() == "rust"
    assert stats.bbox_size == (2.0, 2.0, 2.0)
    assert stats.volume_mm3 == 8.0


def test_stats_module_is_rust_owned(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = cube(size=2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    stats = compute_mesh_stats(mesh)
    assert stats.volume_mm3 == 8.0

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel mesh_stats is required"):
        compute_mesh_stats(mesh)


def test_accelerator_rust_mode_uses_native_self_intersections(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    assert self_intersecting_faces(crossing_triangles()) == {0, 1}


def test_accelerator_rust_mode_uses_native_point_distances(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    distances = point_mesh_distances(np.array([[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0))

    assert np.allclose(distances, [1.0, 1.0])


def test_accelerator_rust_mode_uses_native_closest_points(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    closest, distances, face_indices = closest_points_on_mesh(
        np.array([[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]], dtype=np.float64),
        cube(size=2.0),
    )

    assert np.allclose(closest, [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0]])
    assert np.allclose(distances, [1.0, 1.0])
    assert np.array_equal(face_indices, [6, 0])


def test_accelerator_rust_mode_uses_native_winding_numbers(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    values = winding_numbers(np.array([[0.0, 0.0, 0.0], [3.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0))

    assert np.allclose(values, [1.0, 0.0])


def test_accelerator_rust_mode_uses_native_signed_distances(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    values = signed_point_mesh_distances(np.array([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0))

    assert np.allclose(values, [-1.0, 1.0])


def test_accelerator_rust_mode_uses_native_thickness(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    values = ray_thickness_at_vertices(cube(size=2.0))

    assert np.allclose(values, np.full(8, 2.0, dtype=np.float32))


def test_accelerator_rust_mode_uses_native_sdf_grid_values(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=1.0, padding_mm=1.0)

    assert grid.shape == (5, 5, 5)
    assert grid.values[2, 2, 2] < 0.0
    assert grid.values[0, 0, 0] > 0.0


def test_accelerator_rust_mode_uses_native_sdf_boolean_values(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    left = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.zeros((2, 2, 2), dtype=np.float32),
    )
    right = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.ones((2, 2, 2), dtype=np.float32),
    )

    result = sdf_union(left, right)

    assert result.shape == (2, 2, 2)
    assert np.allclose(result.values, -7.0)


def test_accelerator_rust_mode_uses_native_sdf_boolean_marching(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)

    result = voxel_boolean_mesh(source, source, operation="union", voxel_size_mm=1.0)

    assert result.vertex_count == 3
    assert result.face_count == 1


def test_accelerator_rust_mode_uses_native_global_thicken(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)

    result = global_thicken(source, min_target_thickness_mm=1.0)

    assert result.vertex_count == 3
    assert result.face_count == 1
    assert result.metadata["operation"] == "global_thicken"


def test_accelerator_rust_mode_uses_native_sdf_offset_marching(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    grid = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.zeros((2, 2, 2), dtype=np.float32),
    )

    result = extract_offset_marching_tetrahedra(grid, offset_mm=0.5)

    assert result is not None
    assert result.vertex_count == 3
    assert result.face_count == 1


def test_accelerator_rust_mode_uses_native_sdf_shell_marching(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    grid = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.zeros((2, 2, 2), dtype=np.float32),
    )

    result = extract_shell_marching_tetrahedra(grid, wall_thickness_mm=0.5)

    assert result is not None
    assert result.vertex_count == 3
    assert result.face_count == 1


def test_accelerator_rust_mode_uses_native_marching_tetrahedra(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    grid = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.array([[[-1.0, 1.0], [1.0, 1.0]], [[1.0, 1.0], [1.0, 1.0]]], dtype=np.float32),
    )

    result = extract_marching_tetrahedra(grid)

    assert result.vertex_count == 3
    assert result.face_count == 1


def test_accelerator_rust_mode_uses_native_sdf_projection(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)
    grid = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.zeros((2, 2, 2), dtype=np.float32),
    )

    projected = project_vertices_to_sdf(source, grid)

    assert np.allclose(projected.vertices, source.vertices + np.array([1.0, 0.0, 0.0]))


def test_accelerator_rust_mode_uses_native_sdf_refinement(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)
    grid = SDFGrid(
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        shape=(2, 2, 2),
        values=np.zeros((2, 2, 2), dtype=np.float32),
    )

    refined = refine_sdf_mesh(source, grid)

    assert np.allclose(refined.vertices, source.vertices + np.array([3.0, 0.0, 0.0]))


def test_accelerator_rust_mode_uses_native_laplacian_smoothing(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)

    smoothed = laplacian_smooth_vertices(source)

    assert np.allclose(smoothed.vertices, source.vertices * 0.5)


def test_accelerator_rust_mode_uses_native_taubin_smoothing(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)

    smoothed = smooth(source)

    assert np.allclose(smoothed.vertices, source.vertices + 4.0)


def test_accelerator_rust_mode_uses_native_falloff_weights(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)

    weights = falloff_weights(source, np.arange(source.vertex_count, dtype=np.int32), 1.8)

    assert np.allclose(weights, np.ones(source.vertex_count, dtype=np.float32))


def test_accelerator_rust_mode_uses_native_resident_seeded_smoothing(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)
    seed = np.arange(source.vertex_count, dtype=np.int32)

    smoothed = smooth(source, seed_indices=seed)

    assert np.allclose(smoothed.vertices, source.vertices + 2.0)


def test_accelerator_rust_mode_uses_native_local_offset(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)
    seed = np.arange(source.vertex_count, dtype=np.int32)

    thickened = local_thicken(source, seed, amount_mm=0.2, falloff_mm=1.8)
    scooped = local_scoop(source, seed, depth_mm=0.2, falloff_mm=1.8)

    assert np.allclose(thickened.vertices, source.vertices + 0.2)
    assert np.allclose(scooped.vertices, source.vertices - 0.2)


def test_accelerator_rust_mode_uses_native_outward_directions(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)

    directions = outward_directions(source)

    assert np.allclose(directions, np.tile([0.0, 0.0, 1.0], (source.vertex_count, 1)))


def test_accelerator_rust_mode_uses_native_brush_composition(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    source = cube(size=2.0)
    seeds = np.arange(source.vertex_count, dtype=np.int32)

    result = apply_brush_strokes(
        source,
        [
            BrushStroke("thicken", seeds, amount_mm=0.2, falloff_mm=1.8),
            BrushStroke("scoop", seeds, amount_mm=0.1, falloff_mm=1.8),
            BrushStroke("smooth", seeds, falloff_mm=2.0, iterations=2, strength=0.25),
        ],
    )

    assert np.allclose(result.vertices, source.vertices + 4.0)


def test_accelerator_rust_mode_uses_native_face_orientation(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)
    faces = np.array([[0, 1, 2], [1, 2, 3]], dtype=np.int64)

    oriented, components = _orient_faces_consistently(faces)

    assert np.array_equal(oriented, faces)
    assert components == [[0, 1]]


def test_accelerator_rust_mode_uses_native_first_ray_hit(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    hit = first_ray_hit(cube(size=2.0), (0.0, 0.0, 3.0), (0.0, 0.0, -1.0))

    assert hit is not None
    assert hit.face_index == 2
    assert hit.distance == 2.0
    assert hit.point == (0.0, 0.0, 1.0)


def test_accelerator_rust_mode_uses_native_first_ray_hits(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", FakeRustModule)

    hits = first_ray_hits(
        cube(size=2.0),
        np.array([[0.0, 0.0, 3.0], [4.0, 4.0, 4.0]], dtype=np.float64),
        np.array([[0.0, 0.0, -1.0], [1.0, 0.0, 0.0]], dtype=np.float64),
    )

    assert hits[0] is not None
    assert hits[0].face_index == 2
    assert hits[0].distance == 2.0
    assert hits[0].point == (0.0, 0.0, 1.0)
    assert hits[1] is None


def test_accelerator_rust_mode_requires_extension(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    monkeypatch.setattr(_rust_common, "_rs", None)

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        compute_mesh_stats(cube(size=2.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        self_intersecting_faces(crossing_triangles())

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        point_mesh_distances(np.array([[2.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        winding_numbers(np.array([[0.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        signed_point_mesh_distances(np.array([[0.0, 0.0, 0.0]], dtype=np.float64), cube(size=2.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        ray_thickness_at_vertices(cube(size=2.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        sample_sdf_grid(cube(size=2.0), voxel_size_mm=1.0, padding_mm=1.0)

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        sdf_union(
            SDFGrid(
                origin=(0.0, 0.0, 0.0),
                voxel_size_mm=1.0,
                shape=(2, 2, 2),
                values=np.zeros((2, 2, 2), dtype=np.float32),
            ),
            SDFGrid(
                origin=(0.0, 0.0, 0.0),
                voxel_size_mm=1.0,
                shape=(2, 2, 2),
                values=np.ones((2, 2, 2), dtype=np.float32),
            ),
        )

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        extract_marching_tetrahedra(
            SDFGrid(
                origin=(0.0, 0.0, 0.0),
                voxel_size_mm=1.0,
                shape=(2, 2, 2),
                values=np.zeros((2, 2, 2), dtype=np.float32),
            )
        )

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        project_vertices_to_sdf(
            cube(size=2.0),
            SDFGrid(
                origin=(0.0, 0.0, 0.0),
                voxel_size_mm=1.0,
                shape=(2, 2, 2),
                values=np.zeros((2, 2, 2), dtype=np.float32),
            ),
        )

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        laplacian_smooth_vertices(cube(size=2.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        falloff_weights(cube(size=2.0), np.arange(8, dtype=np.int32), 1.8)

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        rust.smooth_vertices_with_falloff(cube(size=2.0), np.arange(8, dtype=np.int32), falloff_mm=1.8)

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        smooth(cube(size=2.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        _orient_faces_consistently(np.array([[0, 1, 2]], dtype=np.int64))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        first_ray_hit(cube(size=2.0), (0.0, 0.0, 3.0), (0.0, 0.0, -1.0))

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        first_ray_hits(
            cube(size=2.0),
            np.array([[0.0, 0.0, 3.0]], dtype=np.float64),
            np.array([[0.0, 0.0, -1.0]], dtype=np.float64),
        )

    with pytest.raises(RuntimeError, match="_zennah_geometry_rs"):
        global_thicken(cube(size=2.0), min_target_thickness_mm=1.0)


def test_accelerator_rejects_invalid_mode(monkeypatch) -> None:
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "gpu")

    with pytest.raises(ValueError, match="GEOMETRY_SDK_ACCELERATOR"):
        rust.backend_name()
