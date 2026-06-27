"""Guard regression: sheet-thicken (/offset/thicken, MeshLib thickenMesh) must be
refused on a CLOSED solid, where it would emit the original solid plus an
interpenetrating offset copy (two components) instead of a thickened sheet. It must
still be allowed on an OPEN surface, which is what sheet-thicken is for.
"""
from __future__ import annotations

import numpy as np
import pytest
from fastapi import HTTPException

from api.routers.versions import _reject_sheet_thicken_on_closed_solid
from geometry_sdk.types import MeshDocument


def _closed_tet() -> MeshDocument:
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float)
    faces = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)  # closed: 0 boundary edges
    return MeshDocument(vertices=vertices, faces=faces)


def _open_sheet() -> MeshDocument:
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0]], dtype=float)
    faces = np.array([[0, 1, 2]], dtype=np.int64)  # single triangle: 3 boundary edges (open)
    return MeshDocument(vertices=vertices, faces=faces)


def test_sheet_thicken_refused_on_closed_solid() -> None:
    with pytest.raises(HTTPException) as exc_info:
        _reject_sheet_thicken_on_closed_solid(_closed_tet())
    assert exc_info.value.status_code == 400
    assert "open surfaces" in exc_info.value.detail


def test_sheet_thicken_allowed_on_open_surface() -> None:
    # Must not raise — an open surface is the valid input for sheet-thicken.
    _reject_sheet_thicken_on_closed_solid(_open_sheet())
