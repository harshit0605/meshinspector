from __future__ import annotations

import numpy as np

from geometry_sdk import GeometrySDK, default_sdk
from geometry_sdk.testing.fixtures import cube, ring
from geometry_sdk.types import BrushStroke


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
