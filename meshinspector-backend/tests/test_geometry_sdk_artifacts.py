from __future__ import annotations

import hashlib

import numpy as np
import pytest

from geometry_sdk.analysis.artifacts import (
    compare_overlay_payload,
    load_compare_npz,
    load_thickness_npz,
    save_compare_npz,
    save_thickness_npz,
    thickness_overlay_payload,
)
from geometry_sdk.analysis.compare import signed_surface_distances
from geometry_sdk.analysis.thickness import ray_thickness_at_vertices
from geometry_sdk.adapters.meshlib_reference import save_compare_npz as meshlib_save_compare_npz
from geometry_sdk.adapters.meshlib_reference import save_thickness_npz as meshlib_save_thickness_npz
from geometry_sdk.io.trimesh_adapter import save_mesh
from geometry_sdk.testing.fixtures import cube
from geometry_sdk.testing.goldens import GOLDEN_DIR, assert_metric_dict_close, load_golden
from services import manufacturability as manufacturability_service


def test_thickness_npz_matches_current_overlay_contract(tmp_path) -> None:
    mesh = cube(size=2.0)
    field = ray_thickness_at_vertices(mesh)
    path = save_thickness_npz(tmp_path / "thickness.npz", field, vertex_count=mesh.vertex_count, threshold_mm=0.6)
    loaded, threshold = load_thickness_npz(path)
    raw = np.load(path)

    assert set(raw.files) == {"thickness", "threshold_mm"}
    assert loaded.dtype == np.float32
    assert loaded.shape == (mesh.vertex_count,)
    assert np.array_equal(loaded, field)
    assert threshold == pytest.approx(0.6)


def test_compare_npz_matches_current_overlay_contract(tmp_path) -> None:
    source = cube(size=2.0)
    target = cube(size=2.0).copy(vertices=cube(size=2.0).vertices + np.array([0.5, 0.0, 0.0]))
    values = signed_surface_distances(source, target)
    values[0] = np.nan
    path = save_compare_npz(tmp_path / "compare.npz", values, vertex_count=source.vertex_count, other_version_id="version-b")
    loaded, other_version_id = load_compare_npz(path)
    raw = np.load(path)

    assert set(raw.files) == {"values", "other_version_id"}
    assert loaded.dtype == np.float32
    assert loaded.shape == (source.vertex_count,)
    assert loaded[0] == 0.0
    assert other_version_id == "version-b"


def test_scalar_overlay_payloads_match_frontend_contract(tmp_path) -> None:
    thickness_path = save_thickness_npz(
        tmp_path / "thickness.npz",
        np.array([0.5, np.nan, np.inf, -np.inf, np.finfo(np.float32).max, 1.25], dtype=np.float32),
        vertex_count=6,
        threshold_mm=0.6,
    )
    compare_path = save_compare_npz(
        tmp_path / "compare.npz",
        np.array([-0.25, 0.0, 0.75], dtype=np.float32),
        vertex_count=3,
        other_version_id="version-b",
    )

    thickness = thickness_overlay_payload(thickness_path)
    compare = compare_overlay_payload(compare_path, other_version_id="version-b")

    assert thickness == {
        "overlay_type": "thickness",
        "values": [0.5, 0.0, 0.0, 0.0, 0.0, 1.25],
        "min_value": 0.5,
        "max_value": 1.25,
        "center_value": 0.6,
        "threshold_mm": 0.6,
    }
    assert compare["overlay_type"] == "compare"
    assert compare["values"] == [-0.25, 0.0, 0.75]
    assert compare["min_value"] == -0.25
    assert compare["max_value"] == 0.75
    assert compare["center_value"] == 0.0
    assert compare["threshold_mm"] is None
    assert compare["summary"] == {
        "other_version_id": "version-b",
        "max_abs_distance_mm": 0.75,
        "mean_distance_mm": 0.16667,
        "cached": True,
    }


def test_scalar_artifact_helpers_reject_vertex_count_mismatch(tmp_path) -> None:
    with pytest.raises(ValueError, match="vertex count"):
        save_thickness_npz(tmp_path / "bad.npz", np.zeros(3, dtype=np.float32), vertex_count=4, threshold_mm=0.6)


def test_manufacturability_snapshot_defers_full_thickness_for_large_mesh(monkeypatch, tmp_path) -> None:
    source = save_mesh(cube(size=2.0), tmp_path / "cube.ply")
    monkeypatch.setattr(manufacturability_service.settings, "MANUFACTURABILITY_THICKNESS_MAX_VERTICES", 1)

    def fail_service_thickness(*_args, **_kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("full thickness should be deferred")

    monkeypatch.setattr(manufacturability_service.default_sdk, "service_thickness", fail_service_thickness)

    snapshot, artifacts = manufacturability_service.compute_manufacturability_snapshot(source, tmp_path / "snapshot")
    thickness_values = np.load(artifacts.thickness_scalar_path)["thickness"]

    assert snapshot.thickness.min_mm is None
    assert snapshot.thickness.avg_mm is None
    assert snapshot.thickness.max_mm is None
    assert np.isnan(thickness_values).all()
    assert thickness_values.shape == (cube(size=2.0).vertex_count,)
    assert any("deferred" in recommendation for recommendation in snapshot.recommendations)


def test_checked_in_meshlib_scalar_artifact_goldens_match_contract() -> None:
    manifest = load_golden("scalar_artifact_reference_v1.json")

    assert manifest["schema_version"] == 1
    for name, artifact in manifest["artifacts"].items():
        path = GOLDEN_DIR / artifact["path"]
        assert path.exists(), name
        assert hashlib.sha256(path.read_bytes()).hexdigest() == artifact["sha256"]

        if artifact["kind"] == "thickness_npz":
            values, threshold = load_thickness_npz(path)
            finite = values[np.isfinite(values)]
            summary = {
                "min_mm": float(np.min(finite)),
                "avg_mm": float(np.mean(finite, dtype=np.float64)),
                "max_mm": float(np.max(finite)),
                "violation_count": int(np.sum(finite < threshold, dtype=np.int64)),
            }

            assert values.dtype == np.float32
            assert values.shape == (artifact["vertex_count"],)
            assert threshold == pytest.approx(artifact["threshold_mm"])
            assert_metric_dict_close(summary, artifact["summary"], abs_tol=1e-6)
        elif artifact["kind"] == "compare_npz":
            values, other_version_id = load_compare_npz(path)
            summary = {
                "min_signed_distance_mm": float(np.min(values)),
                "max_signed_distance_mm": float(np.max(values)),
                "mean_signed_distance_mm": float(np.mean(values, dtype=np.float64)),
            }

            assert values.dtype == np.float32
            assert values.shape == (artifact["vertex_count"],)
            assert other_version_id == artifact["other_version_id"]
            assert_metric_dict_close(summary, artifact["summary"], abs_tol=1e-6)
        else:
            raise AssertionError(f"Unknown scalar artifact kind: {artifact['kind']}")


def test_live_meshlib_scalar_artifacts_match_checked_in_goldens(tmp_path) -> None:
    pytest.importorskip("meshlib")
    manifest = load_golden("scalar_artifact_reference_v1.json")["artifacts"]
    source = cube(size=2.0)
    target = source.copy(vertices=source.vertices + np.array([0.5, 0.0, 0.0]))
    source_path = save_mesh(source, tmp_path / "cube_2mm.stl")
    target_path = save_mesh(target, tmp_path / "cube_2mm_shifted.stl")

    expected_thickness, expected_threshold = load_thickness_npz(GOLDEN_DIR / manifest["meshlib_cube_2mm_thickness"]["path"])
    live_thickness_path = meshlib_save_thickness_npz(source_path, tmp_path / "live_thickness.npz", threshold_mm=0.6)
    live_thickness, live_threshold = load_thickness_npz(live_thickness_path)

    assert live_threshold == pytest.approx(expected_threshold)
    assert np.array_equal(live_thickness, expected_thickness, equal_nan=True)

    expected_compare, expected_other_version_id = load_compare_npz(GOLDEN_DIR / manifest["meshlib_cube_2mm_shifted_compare"]["path"])
    live_compare_path = meshlib_save_compare_npz(
        source_path,
        target_path,
        tmp_path / "live_compare.npz",
        other_version_id=expected_other_version_id,
    )
    live_compare, live_other_version_id = load_compare_npz(live_compare_path)

    assert live_other_version_id == expected_other_version_id
    assert np.array_equal(live_compare, expected_compare)
