"""Regression: voxel→mesh (simple/smart) output finalize. The dense iso-surface
mesher can emit inverted face winding (negative signed volume) and, on a signed
SDF fed without grid_level_set, collapse to an empty/degenerate mesh.
_finalize_voxel_to_mesh_output orients faces outward and rejects an empty result
with guidance toward grid_level_set / the Dual mesher.
"""
from __future__ import annotations

import numpy as np
import pytest

from api.routers.versions import _finalize_voxel_to_mesh_output
from geometry_sdk.types import MeshDocument


def _signed_volume(mesh: MeshDocument) -> float:
    v = np.asarray(mesh.vertices, dtype=float)
    f = np.asarray(mesh.faces, dtype=np.int64)
    t = v[f]
    return float(np.einsum("ij,ij->i", t[:, 0], np.cross(t[:, 1], t[:, 2])).sum() / 6.0)


def _inward_tet() -> MeshDocument:
    # Tet with every face wound inward -> negative signed volume.
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float)
    faces = np.array([[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def test_finalize_orients_faces_outward() -> None:
    inward = _inward_tet()
    assert _signed_volume(inward) < 0  # starts inverted
    fixed = _finalize_voxel_to_mesh_output(inward, "Voxel→mesh (simple)")
    assert _signed_volume(fixed) > 0  # now outward


def test_finalize_rejects_empty_with_guidance() -> None:
    empty = MeshDocument(vertices=np.zeros((0, 3), dtype=float), faces=np.zeros((0, 3), dtype=np.int64))
    with pytest.raises(ValueError, match="grid_level_set"):
        _finalize_voxel_to_mesh_output(empty, "Voxel→mesh (simple)")
