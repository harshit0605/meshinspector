"""Regression: the /repair do-no-harm guard. _repair_regression_reason flags a
repair result that is WORSE than the input (opened a watertight mesh, added
non-manifold edges, or added self-intersections) so the handler keeps the input
rather than persisting a regression — the failure mode found on the
self-intersecting torus (weld fused the seam → non-manifold + open).
"""
from __future__ import annotations

import numpy as np

from geometry_sdk.types import MeshDocument
from services.operations import _repair_regression_reason


def _closed_tet() -> MeshDocument:
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float)
    faces = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def _nonmanifold_fan() -> MeshDocument:
    # Edge (0,1) shared by THREE faces -> a non-manifold edge.
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1]], dtype=float)
    faces = np.array([[0, 1, 2], [0, 1, 3], [0, 1, 4]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def _open_sheet() -> MeshDocument:
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0]], dtype=float)
    faces = np.array([[0, 1, 2]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def test_repair_guard_flags_nonmanifold_regression() -> None:
    reason = _repair_regression_reason(_closed_tet(), _nonmanifold_fan())
    assert reason is not None and "non-manifold" in reason


def test_repair_guard_flags_opening_a_watertight_mesh() -> None:
    # Closed tet -> open sheet is a regression (lost watertightness).
    reason = _repair_regression_reason(_closed_tet(), _open_sheet())
    assert reason is not None


def test_repair_guard_allows_genuine_improvement() -> None:
    # Open sheet -> closed tet is an improvement, not a regression.
    assert _repair_regression_reason(_open_sheet(), _closed_tet()) is None
