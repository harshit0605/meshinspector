from __future__ import annotations

import numpy as np
import pytest

from geometry_sdk.accelerators import _rust_common
from geometry_sdk.accelerators import rust
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.health import boundary_loops, compute_mesh_health
from geometry_sdk.core.mesh import boundary_edges, connected_face_components, vertex_normals
from geometry_sdk.testing.fixtures import crossing_triangles, cube, open_cube


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


def test_vertex_normals_match_vertex_count() -> None:
    mesh = cube(size=1.0)
    normals = vertex_normals(mesh)

    assert normals.shape == mesh.vertices.shape
    assert np.all(np.isfinite(normals))


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
