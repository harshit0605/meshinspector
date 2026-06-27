"""Regression: make-manufacturable health honesty. The composite could report
health_score 100 and not flag a result split into multiple disconnected shells
(the Rust health score ignores component count, and export_ready didn't require a
single solid). The snapshot now penalizes the score per extra shell, requires a
single solid for export_ready, and recommends pruning fragments.
"""
from __future__ import annotations

import tempfile
from pathlib import Path

import numpy as np

from geometry_sdk.engine import GeometrySDK
from geometry_sdk.types import MeshDocument
from services.manufacturability import compute_manufacturability_snapshot

SDK = GeometrySDK()


def _tet(offset) -> tuple[np.ndarray, np.ndarray]:
    v = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float) + np.asarray(offset, dtype=float)
    f = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return v, f


def _snapshot(mesh: MeshDocument):
    path = Path(tempfile.mktemp(suffix=".ply"))
    SDK.save_mesh(mesh, path, file_type="ply")
    snap, _ = compute_manufacturability_snapshot(path, tempfile.mkdtemp())
    return snap


def test_disconnected_shells_penalize_health_and_block_export() -> None:
    v1, f1 = _tet([0, 0, 0])
    v2, f2 = _tet([10, 0, 0])  # disjoint -> 2 components
    snap = _snapshot(MeshDocument(vertices=np.vstack([v1, v2]), faces=np.vstack([f1, f2 + 4])))
    assert snap.mesh_health.disconnected_shells == 1
    assert snap.mesh_health.health_score < 100
    assert snap.export_ready is False
    assert any("disconnected shell" in r for r in snap.recommendations)


def test_single_solid_not_penalized_for_fragmentation() -> None:
    v1, f1 = _tet([0, 0, 0])
    snap = _snapshot(MeshDocument(vertices=v1, faces=f1))
    assert snap.mesh_health.disconnected_shells == 0
    # No fragmentation penalty applied (score reflects only the real health rubric).
    assert snap.mesh_health.health_score == 100
    assert not any("disconnected shell" in r for r in snap.recommendations)
