from __future__ import annotations

from dataclasses import asdict
from pathlib import Path

import pytest

from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.io.trimesh_adapter import load_mesh
from geometry_sdk.testing.goldens import assert_metric_dict_close, load_golden


REPO_ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("sample_name", ["frontend_ring_glb", "frontend_pendant_glb"])
def test_real_app_sample_meshes_load_and_match_stats_goldens(sample_name: str) -> None:
    golden = load_golden("real_sample_reference_v1.json")["samples"][sample_name]
    mesh_path = REPO_ROOT / golden["path"]

    assert mesh_path.exists(), f"Missing real sample mesh: {mesh_path}"
    mesh = load_mesh(mesh_path)
    stats = asdict(compute_mesh_stats(mesh))

    assert mesh.vertex_count == golden["mesh"]["vertices"]
    assert mesh.face_count == golden["mesh"]["faces"]
    assert_metric_dict_close(stats, golden["sdk_stats"], abs_tol=1e-9)
    assert stats["boundary_edge_count"] > 0
    assert stats["connected_components"] > 1
