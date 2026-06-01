from __future__ import annotations

from dataclasses import asdict
import warnings

import numpy as np

from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.analysis.manufacturability import compute_manufacturability_report
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.thickness import ray_thickness_at_vertices, summarize_thickness
from geometry_sdk.io.trimesh_adapter import to_trimesh
from geometry_sdk.jewelry.regions import detect_ring_regions
from geometry_sdk.jewelry.ring_measurement import measure_ring
from geometry_sdk.testing.fixtures import crossing_triangles, cube, hollowed_ring, open_cube, pendant, ring, ring_with_head, thin_wall_ring
from geometry_sdk.testing.goldens import assert_metric_dict_close, load_golden


FIXTURES = {
    "cube_2mm": lambda: cube(2.0),
    "open_cube_2mm": lambda: open_cube(2.0),
    "ring_default": ring,
    "ring_with_head": ring_with_head,
    "thin_wall_ring": thin_wall_ring,
    "hollowed_ring": hollowed_ring,
    "pendant": pendant,
    "crossing_triangles": crossing_triangles,
}


def _trimesh_reference_metrics(mesh) -> dict[str, float | bool]:
    trimesh_mesh = to_trimesh(mesh, process=False)
    with warnings.catch_warnings():
        warnings.filterwarnings("ignore", category=RuntimeWarning, module="trimesh.triangles")
        volume = abs(float(trimesh_mesh.volume))
    return {
        "volume_mm3": volume,
        "surface_area_mm2": float(trimesh_mesh.area),
        "is_watertight": bool(trimesh_mesh.is_watertight),
    }


def test_stored_golden_schema_has_all_fixture_entries() -> None:
    golden = load_golden("geometry_reference_v1.json")

    assert golden["schema_version"] == 1
    assert set(golden["fixtures"]) == set(FIXTURES)


def test_core_stats_and_health_match_stored_goldens() -> None:
    golden = load_golden("geometry_reference_v1.json")["fixtures"]

    for fixture_name, maker in FIXTURES.items():
        mesh = maker()
        expected = golden[fixture_name]
        stats = asdict(compute_mesh_stats(mesh))
        health = asdict(compute_mesh_health(mesh))

        assert mesh.vertex_count == expected["mesh"]["vertices"]
        assert mesh.face_count == expected["mesh"]["faces"]
        assert_metric_dict_close(stats, expected["sdk_stats"], abs_tol=1e-6)
        assert_metric_dict_close(health, expected["sdk_health"], abs_tol=1e-6)
        assert_metric_dict_close(_trimesh_reference_metrics(mesh), expected["trimesh"], abs_tol=1e-6)


def test_sdk_thickness_matches_stored_goldens() -> None:
    golden = load_golden("geometry_reference_v1.json")["fixtures"]

    for fixture_name, maker in FIXTURES.items():
        summary = asdict(summarize_thickness(ray_thickness_at_vertices(maker())))

        assert_metric_dict_close(summary, golden[fixture_name]["sdk_ray_thickness"], abs_tol=1e-5)


def test_ring_measurement_region_and_manufacturability_goldens() -> None:
    golden = load_golden("geometry_reference_v1.json")["fixtures"]

    for fixture_name, maker in {"ring_default": ring, "ring_with_head": ring_with_head}.items():
        mesh = maker()
        measurement = measure_ring(mesh)
        regions = detect_ring_regions(mesh, measurement)
        region_counts = {region.region_id: int(region.vertex_indices.size) for region in regions}

        assert_metric_dict_close(asdict(measurement), golden[fixture_name]["sdk_ring_measurement"], abs_tol=1e-3)
        assert region_counts == golden[fixture_name]["sdk_region_counts"]

    report = compute_manufacturability_report(ring())
    expected = golden["ring_default"]["sdk_manufacturability"]
    assert report.export_ready == expected["export_ready"]
    assert report.health_score == expected["health_score"]
    assert np.isclose(report.material_weights["gold_18k"].weight_g, expected["gold_18k_weight_g"])
