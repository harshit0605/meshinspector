"""Regression: voxel mask-to-mesh / segmentation gross-fragmentation guard. A
connected mask/segmentation should mesh to ~1 solid; a coordinate-convention
mismatch shatters it into hundreds of blobs (verified: a contiguous snake mask
produced 1289 components). The guard refuses a gross shatter instead of shipping it.
"""
from __future__ import annotations

import numpy as np
import pytest

from api.routers.versions import _reject_shattered_voxel_mesh
from geometry_sdk.types import MeshDocument


def _disjoint_tets(n: int) -> MeshDocument:
    base_v = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float)
    base_f = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    verts = np.vstack([base_v + [i * 10.0, 0, 0] for i in range(n)])
    faces = np.vstack([base_f + 4 * i for i in range(n)])
    return MeshDocument(vertices=verts, faces=faces)


def test_rejects_gross_shatter() -> None:
    with pytest.raises(ValueError, match="fragmented into"):
        _reject_shattered_voxel_mesh(_disjoint_tets(80), "Voxel mask-to-mesh")


def test_allows_single_solid() -> None:
    # 1 component -> not a shatter -> returned unchanged.
    out = _reject_shattered_voxel_mesh(_disjoint_tets(1), "Voxel mask-to-mesh")
    assert int(out.face_count) == 4
