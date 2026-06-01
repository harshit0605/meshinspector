from __future__ import annotations

from dataclasses import asdict
import hashlib
from pathlib import Path
from typing import Any

import numpy as np
import pytest

from geometry_sdk.adapters.meshlib_reference import health_metrics as meshlib_health_metrics
from geometry_sdk.adapters.meshlib_reference import boolean_mesh as meshlib_boolean_mesh
from geometry_sdk.adapters.meshlib_reference import offset_mesh as meshlib_offset_mesh
from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.io.trimesh_adapter import load_mesh, save_mesh
from geometry_sdk.repair.voxel import rebuild_via_sdf
from geometry_sdk.testing.fixtures import box
from geometry_sdk.testing.goldens import load_golden
from geometry_sdk.testing.uploaded_fragments import load_npz_mesh, sha256_file
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh, voxel_offset_mesh


REPO_ROOT = Path(__file__).resolve().parents[2]
UPLOADED_SAMPLE_NAMES = ["uploaded_ring_processed_stl", "uploaded_pendant_processed_stl"]


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _existing_or_skip(path: Path) -> Path:
    if not path.exists():
        pytest.skip(f"Optional local uploaded sample is missing: {path}")
    return path


def _assert_metric_close(actual: Any, expected: Any) -> None:
    if isinstance(expected, dict):
        assert isinstance(actual, dict)
        assert set(actual) >= set(expected)
        for key, value in expected.items():
            _assert_metric_close(actual[key], value)
        return
    if isinstance(expected, list):
        assert np.allclose(actual, expected, atol=1e-9)
        return
    if isinstance(expected, float):
        assert np.isclose(float(actual), expected, atol=1e-9)
        return
    assert actual == expected


def _operation_payload(mesh) -> dict[str, Any]:
    return {
        "mesh": {"vertices": int(mesh.vertex_count), "faces": int(mesh.face_count)},
        "stats": asdict(compute_mesh_stats(mesh)),
        "health": asdict(compute_mesh_health(mesh)),
    }


def _translated_mesh(mesh, translation: list[float]):
    return mesh.copy(vertices=mesh.vertices + np.asarray(translation, dtype=np.float64))


def _bbox_box_cutter(mesh, params: dict[str, Any]):
    stats = compute_mesh_stats(mesh)
    bbox_size = np.asarray(stats.bbox_size, dtype=np.float64)
    bbox_center = (np.asarray(stats.bbox_min, dtype=np.float64) + np.asarray(stats.bbox_max, dtype=np.float64)) * 0.5
    size_fraction = np.asarray(params["target_size_bbox_fraction"], dtype=np.float64)
    center_shift = np.asarray(params["target_center_bbox_shift_fraction"], dtype=np.float64)
    size = bbox_size * size_fraction
    center = bbox_center + bbox_size * center_shift
    return box(float(size[0]), float(size[1]), float(size[2]), center=tuple(float(value) for value in center))


def _boolean_target_from_expected(source, expected: dict[str, Any]):
    params = expected["params"]
    if "target_translation_mm" in params:
        return _translated_mesh(source, params["target_translation_mm"])
    if params.get("target_kind") == "bbox_box_cutter":
        return _bbox_box_cutter(source, params)
    raise AssertionError(f"Unsupported uploaded-sample boolean target params: {params}")


def _rebuilt_fragment_from_manifest(fragment: dict[str, Any]):
    operation_name, expected = next(iter(fragment["sdk_rebuild_operations"].items()))
    params = expected["params"]
    mesh = load_npz_mesh(REPO_ROOT / fragment["path"])
    rebuilt, report = rebuild_via_sdf(
        mesh,
        voxel_size_mm=params["voxel_size_mm"],
        offset_mm=params["offset_mm"],
        padding_mm=params["padding_mm"],
        refine=expected["report"]["refine"],
    )

    assert operation_name.startswith("sdf_rebuild_")
    _assert_metric_close(_operation_payload(rebuilt), {key: expected[key] for key in ("mesh", "stats", "health")})
    _assert_metric_close(asdict(report), expected["report"])
    return rebuilt, expected


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_local_uploaded_processed_samples_match_stats_and_health_goldens(sample_name: str) -> None:
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    source_path = _existing_or_skip(REPO_ROOT / golden["source"]["path"])
    processed = golden["processed"]
    processed_path = _existing_or_skip(REPO_ROOT / processed["path"])

    assert source_path.stat().st_size == golden["source"]["bytes"]
    assert processed_path.stat().st_size == processed["bytes"]
    assert _sha256(source_path) == golden["source"]["sha256"]
    assert _sha256(processed_path) == processed["sha256"]

    mesh = load_mesh(processed_path)
    stats = asdict(compute_mesh_stats(mesh))
    health = asdict(compute_mesh_health(mesh))

    assert mesh.vertex_count == processed["mesh"]["vertices"]
    assert mesh.face_count == processed["mesh"]["faces"]
    _assert_metric_close(stats, processed["sdk_stats"])
    _assert_metric_close(health, processed["sdk_health"])


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_packaged_uploaded_fragments_match_portable_goldens(sample_name: str) -> None:
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    fragment = golden["processed"]["packaged_fragment"]
    fragment_path = REPO_ROOT / fragment["path"]

    assert fragment_path.exists(), f"Missing packaged uploaded fragment: {fragment_path}"
    assert fragment_path.stat().st_size == fragment["bytes"]
    assert sha256_file(fragment_path) == fragment["sha256"]

    mesh = load_npz_mesh(fragment_path)
    stats = asdict(compute_mesh_stats(mesh))
    health = asdict(compute_mesh_health(mesh))

    assert mesh.vertex_count == fragment["mesh"]["vertices"]
    assert mesh.face_count == fragment["mesh"]["faces"]
    assert mesh.metadata["source"] == "uploaded_processed_component"
    assert mesh.metadata["component_rank_by_size"] == fragment["component_rank_by_size"]
    _assert_metric_close(stats, fragment["sdk_stats"])
    _assert_metric_close(health, fragment["sdk_health"])


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_live_meshlib_health_matches_local_uploaded_sample_goldens(sample_name: str) -> None:
    pytest.importorskip("meshlib")
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    processed = golden["processed"]
    processed_path = _existing_or_skip(REPO_ROOT / processed["path"])

    assert meshlib_health_metrics(processed_path) == processed["meshlib_health"]


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_live_meshlib_offset_matches_packaged_uploaded_fragment_goldens(sample_name: str, tmp_path: Path) -> None:
    pytest.importorskip("meshlib")
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    fragment = golden["processed"]["packaged_fragment"]
    expected = fragment["meshlib_operations"]["offset_0_05mm_voxel_0_025mm"]
    fragment_path = REPO_ROOT / fragment["path"]
    mesh = load_npz_mesh(fragment_path)
    source_path = save_mesh(mesh, tmp_path / "fragment.stl")

    output_path = meshlib_offset_mesh(
        source_path,
        tmp_path / "meshlib_offset.stl",
        offset_mm=expected["params"]["offset_mm"],
        voxel_size_mm=expected["params"]["voxel_size_mm"],
    )

    _assert_metric_close(
        _operation_payload(load_mesh(output_path)),
        {key: expected[key] for key in ("mesh", "stats", "health")},
    )


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_sdk_offset_stays_inside_uploaded_fragment_meshlib_envelopes(sample_name: str) -> None:
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    fragment = golden["processed"]["packaged_fragment"]
    expected = fragment["meshlib_operations"]["offset_0_05mm_voxel_0_025mm"]
    mesh = load_npz_mesh(REPO_ROOT / fragment["path"])

    offset = voxel_offset_mesh(
        mesh,
        offset_mm=expected["params"]["offset_mm"],
        voxel_size_mm=0.05,
        refine=True,
    )
    payload = _operation_payload(offset)

    assert payload["health"]["is_closed"]
    assert payload["health"]["nonmanifold_edge_count"] == 0
    assert payload["health"]["self_intersections"] == 0
    assert np.allclose(payload["stats"]["bbox_size"], expected["stats"]["bbox_size"], atol=0.01)
    assert np.isclose(payload["stats"]["surface_area_mm2"], expected["stats"]["surface_area_mm2"], rtol=0.05, atol=0.01)
    assert np.isclose(payload["stats"]["volume_mm3"], expected["stats"]["volume_mm3"], rtol=0.05, atol=0.005)


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_sdk_rebuild_via_sdf_repairs_uploaded_fragment_topology(sample_name: str) -> None:
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    fragment = golden["processed"]["packaged_fragment"]
    _, expected = _rebuilt_fragment_from_manifest(fragment)

    assert expected["report"]["input_nonmanifold_edge_count"] > expected["report"]["output_nonmanifold_edge_count"]
    assert expected["report"]["output_nonmanifold_edge_count"] == 0
    assert expected["report"]["output_self_intersections"] == 0


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_live_meshlib_booleans_match_rebuilt_uploaded_fragment_goldens(sample_name: str, tmp_path: Path) -> None:
    pytest.importorskip("meshlib")
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    fragment = golden["processed"]["packaged_fragment"]
    rebuilt, rebuild_expected = _rebuilt_fragment_from_manifest(fragment)
    source_path = save_mesh(rebuilt, tmp_path / "rebuilt_source.stl")

    for operation_name, expected in rebuild_expected["meshlib_boolean_operations"].items():
        target = _boolean_target_from_expected(rebuilt, expected)
        target_path = save_mesh(target, tmp_path / f"{operation_name}_target.stl")
        output_path = meshlib_boolean_mesh(
            source_path,
            target_path,
            tmp_path / f"{operation_name}.stl",
            operation=expected["operation"],
        )

        _assert_metric_close(
            _operation_payload(load_mesh(output_path)),
            {key: expected[key] for key in ("mesh", "stats", "health")},
        )


@pytest.mark.parametrize("sample_name", UPLOADED_SAMPLE_NAMES)
def test_sdk_booleans_stay_inside_rebuilt_uploaded_fragment_meshlib_envelopes(sample_name: str) -> None:
    golden = load_golden("uploaded_sample_reference_v1.json")["samples"][sample_name]
    fragment = golden["processed"]["packaged_fragment"]
    rebuilt, rebuild_expected = _rebuilt_fragment_from_manifest(fragment)
    rebuild_params = rebuild_expected["params"]

    for expected in rebuild_expected["meshlib_boolean_operations"].values():
        target = _boolean_target_from_expected(rebuilt, expected)
        result = voxel_boolean_mesh(
            rebuilt,
            target,
            operation=expected["operation"],
            voxel_size_mm=rebuild_params["voxel_size_mm"],
            padding_mm=rebuild_params["padding_mm"],
            refine=True,
        )
        payload = _operation_payload(result)

        assert payload["mesh"]["faces"] > 0
        assert payload["health"]["is_closed"]
        assert payload["health"]["nonmanifold_edge_count"] == 0
        assert payload["health"]["self_intersections"] == 0
        tolerances = expected.get("sdk_tolerances", {})
        assert np.allclose(
            payload["stats"]["bbox_size"],
            expected["stats"]["bbox_size"],
            atol=tolerances.get("bbox_size_atol", 0.05),
        )
        assert np.isclose(
            payload["stats"]["surface_area_mm2"],
            expected["stats"]["surface_area_mm2"],
            rtol=tolerances.get("surface_area_rtol", 0.3),
            atol=tolerances.get("surface_area_atol", 0.03),
        )
        assert np.isclose(
            payload["stats"]["volume_mm3"],
            expected["stats"]["volume_mm3"],
            rtol=tolerances.get("volume_rtol", 0.3),
            atol=tolerances.get("volume_atol", 0.005),
        )
