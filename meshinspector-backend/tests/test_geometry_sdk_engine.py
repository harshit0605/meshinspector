from __future__ import annotations

import numpy as np
import pytest

from geometry_sdk import GeometrySDK, default_sdk
from geometry_sdk.testing.fixtures import cube, ring
from geometry_sdk.types import BrushStroke, MeshDocument


def test_default_sdk_exposes_core_facade() -> None:
    mesh = cube(size=2.0)

    assert isinstance(default_sdk, GeometrySDK)
    assert default_sdk.stats(mesh).volume_mm3 == 8.0
    assert default_sdk.health(mesh).is_closed
    assert default_sdk.compare(mesh, mesh)["max_distance_mm"] == 0.0
    assert default_sdk.compare_field(mesh, mesh).shape == (mesh.vertex_count,)
    assert default_sdk.signed_compare(mesh, mesh)["mean_signed_distance_mm"] == 0.0
    assert default_sdk.version_compare(mesh, mesh).volume_delta_mm3 == 0.0
    assert default_sdk.signed_compare_field(mesh, mesh).shape == (mesh.vertex_count,)


def test_facade_runs_jewelry_and_deform_operations() -> None:
    sdk = GeometrySDK()
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    measurement = sdk.measure_ring(mesh)
    regions = sdk.detect_ring_regions(mesh, measurement)
    seed = regions[0].vertex_indices[:6]

    resized = sdk.radial_scale(mesh, 1.05, ring_axis=measurement.ring_axis)
    thickened = sdk.local_thicken(mesh, seed, amount_mm=0.15)
    thickened_to_minimum = sdk.local_thicken_to_minimum(
        mesh,
        seed,
        np.full(mesh.vertex_count, 0.4, dtype=np.float32),
        min_target_thickness_mm=0.8,
    )
    brushed = sdk.apply_brush_strokes(
        mesh,
        [
            BrushStroke("thicken", seed, amount_mm=0.12),
            BrushStroke("smooth", seed, iterations=1, strength=0.2),
        ],
    )
    region_stroke = sdk.brush_stroke_from_regions("smooth", seed, regions, iterations=1, strength=0.2)
    region_brushed = sdk.apply_brush_strokes(mesh, [region_stroke])
    smoothed = sdk.smooth(mesh, iterations=1, strength=0.2, seed_indices=seed)

    assert measurement.inner_diameter_mm is not None
    assert sum(region.vertex_indices.size for region in regions) == mesh.vertex_count
    assert np.ptp(resized.vertices[:, 0]) > np.ptp(mesh.vertices[:, 0])
    assert thickened.vertices.shape == mesh.vertices.shape
    assert thickened_to_minimum.vertices.shape == mesh.vertices.shape
    assert brushed.vertices.shape == mesh.vertices.shape
    assert region_brushed.vertices.shape == mesh.vertices.shape
    assert smoothed.faces.shape == mesh.faces.shape


def test_facade_passes_meshlib_smooth_mode_subdivide_settings() -> None:
    sdk = GeometrySDK()
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )

    smoothed = sdk.subdivide_mesh(
        mesh,
        max_edge_len=0.0,
        max_edge_splits=3,
        smooth_mode=True,
        min_sharp_dihedral_angle=999.0,
    )

    np.testing.assert_allclose(
        smoothed.mesh.vertices[-1],
        [0.873372078, 0.47443521, 0.031970274],
        rtol=0,
        atol=1e-6,
    )
    assert smoothed.mesh.metadata["smooth_mode"] is True
    assert smoothed.mesh.metadata["min_sharp_dihedral_angle"] == 999.0


def test_facade_exposes_scalar_artifact_writers(tmp_path) -> None:
    sdk = GeometrySDK()
    mesh = cube(size=2.0)
    field, _ = sdk.ray_thickness(mesh, threshold_mm=0.6)
    path = sdk.save_thickness_npz(tmp_path / "thickness.npz", field, vertex_count=mesh.vertex_count, threshold_mm=0.6)
    compare_path = sdk.save_compare_npz(
        tmp_path / "compare.npz",
        sdk.version_compare_field(mesh, mesh),
        vertex_count=mesh.vertex_count,
        other_version_id="same-version",
    )

    assert path.exists()
    assert compare_path.exists()
    assert sdk.thickness_overlay_payload(path)["overlay_type"] == "thickness"
    assert sdk.compare_overlay_payload(compare_path, other_version_id="same-version")["overlay_type"] == "compare"


def test_facade_exposes_measure_inspect_surface_queries() -> None:
    sdk = GeometrySDK()
    mesh = cube(size=2.0)

    closest_points, distances, face_indices = sdk.closest_points_on_mesh([[2.0, 0.0, 0.0]], mesh)
    point_distances = sdk.point_mesh_distances([[2.0, 0.0, 0.0]], mesh)
    geodesic_path = sdk.mesh_geodesic_path(mesh, start_vertex=0, end_vertex=1)
    geodesic_field = sdk.mesh_geodesic_distance_field(mesh, seed_vertices=[0])

    np.testing.assert_allclose(closest_points[0], [1.0, 0.0, 0.0], atol=1e-6)
    assert distances[0] == pytest.approx(1.0)
    assert point_distances[0] == pytest.approx(1.0)
    assert int(face_indices[0]) >= 0
    assert geodesic_path["length_mm"] == pytest.approx(2.0)
    assert geodesic_path["line_segments"] == 1
    assert geodesic_field["distances_mm"][0] == pytest.approx(0.0)
    assert geodesic_field["reachable_vertex_count"] == mesh.vertex_count
