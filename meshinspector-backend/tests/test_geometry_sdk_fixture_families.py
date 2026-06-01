from __future__ import annotations

from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.thickness import ray_thickness_at_vertices, summarize_thickness
from geometry_sdk.testing.fixtures import hollowed_ring, pendant, ring, thin_wall_ring


def test_extended_fixture_families_are_closed_and_non_empty() -> None:
    for mesh in (thin_wall_ring(), hollowed_ring(), pendant()):
        stats = compute_mesh_stats(mesh)
        health = compute_mesh_health(mesh)

        assert mesh.vertex_count > 0
        assert mesh.face_count > 0
        assert stats.volume_mm3 > 0.0
        assert health.is_closed


def test_thin_wall_ring_fixture_triggers_thickness_violations() -> None:
    thin = summarize_thickness(ray_thickness_at_vertices(thin_wall_ring()), threshold_mm=1.0)
    baseline = summarize_thickness(ray_thickness_at_vertices(ring()), threshold_mm=1.0)

    assert thin.avg_mm is not None
    assert baseline.avg_mm is not None
    assert thin.avg_mm < baseline.avg_mm
    assert thin.violation_count > 0


def test_hollowed_ring_fixture_has_outer_and_inner_closed_shells() -> None:
    stats = compute_mesh_stats(hollowed_ring())
    health = compute_mesh_health(hollowed_ring())

    assert stats.connected_components == 2
    assert health.holes_count == 0
    assert health.boundary_edge_count == 0
