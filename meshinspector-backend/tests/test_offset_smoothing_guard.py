"""Guard regression for the morphological open/close offset sequence.

A shrink (inward) step on a feature thinner than the offset distance shatters the
mesh; the following expand step re-merges the blobs into one watertight component
and hides the damage from the post-sequence output check. The guard must catch the
destruction on the intermediate. Reproduced deterministically by stubbing the voxel
offset so the test is fast and does not depend on real voxelization.
"""
from __future__ import annotations

import numpy as np
import pytest

from api.routers import versions as versions_router
from api.routers.versions import _run_offset_smoothing_sequence
from domain.schemas import OffsetSmoothingRequest
from geometry_sdk.types import MeshDocument


def _tet(origin, scale: float = 1.0) -> tuple[np.ndarray, np.ndarray]:
    vertices = (
        np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float) * scale
        + np.asarray(origin, dtype=float)
    )
    # All four triangular faces -> every edge shared by exactly two faces (closed).
    faces = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return vertices, faces


def _mesh(origin, scale: float = 1.0) -> MeshDocument:
    v, f = _tet(origin, scale)
    return MeshDocument(vertices=v, faces=f)


def _fragmented(n: int) -> MeshDocument:
    vs, fs = [], []
    for i in range(n):
        v, f = _tet([i * 5.0, 0.0, 0.0])
        vs.append(v)
        fs.append(f + 4 * i)
    return MeshDocument(vertices=np.vstack(vs), faces=np.vstack(fs))


def test_offset_smoothing_refuses_fragmenting_shrink(monkeypatch) -> None:
    """shrink that shatters a watertight source into many components is refused."""
    def fake_offset(mesh, *, offset_mm, voxel_size_mm, padding_mm=None, refine=False):  # noqa: ANN001
        return _fragmented(30) if offset_mm < 0 else _mesh([0, 0, 0])

    monkeypatch.setattr(versions_router.default_sdk, "voxel_offset_mesh", fake_offset)
    request = OffsetSmoothingRequest(distance_mm=0.25, voxel_size_mm=0.12)
    with pytest.raises(ValueError, match="thinner than the offset distance"):
        _run_offset_smoothing_sequence(_mesh([0, 0, 0]), request, [-0.25, 0.25])


def test_offset_smoothing_allows_nondestructive_shrink(monkeypatch) -> None:
    """A shrink that stays connected and keeps most of its volume passes through."""
    def fake_offset(mesh, *, offset_mm, voxel_size_mm, padding_mm=None, refine=False):  # noqa: ANN001
        # ~73% of source volume (0.9**3), still one component -> acceptable
        return _mesh([0, 0, 0], scale=0.9) if offset_mm < 0 else _mesh([0, 0, 0])

    monkeypatch.setattr(versions_router.default_sdk, "voxel_offset_mesh", fake_offset)
    request = OffsetSmoothingRequest(distance_mm=0.08, voxel_size_mm=0.06)
    result = _run_offset_smoothing_sequence(_mesh([0, 0, 0]), request, [-0.08, 0.08])
    assert int(result.face_count) > 0


def test_offset_smoothing_close_sequence_not_falsely_refused(monkeypatch) -> None:
    """expand-then-shrink (close): the shrink acts on a thickened mesh, so it must
    not trip the destruction guard."""
    def fake_offset(mesh, *, offset_mm, voxel_size_mm, padding_mm=None, refine=False):  # noqa: ANN001
        # expand -> bigger, shrink -> back near original; both single component
        return _mesh([0, 0, 0], scale=1.2) if offset_mm > 0 else _mesh([0, 0, 0], scale=1.0)

    monkeypatch.setattr(versions_router.default_sdk, "voxel_offset_mesh", fake_offset)
    request = OffsetSmoothingRequest(distance_mm=0.25, voxel_size_mm=0.12)
    result = _run_offset_smoothing_sequence(_mesh([0, 0, 0], scale=1.2), request, [0.25, -0.25])
    assert int(result.face_count) > 0
