"""Opt-in voxel-remesh repair mode. Default repair is gentle (voxel_remesh=False);
opting in tries voxel-remesh candidate sizes (fine->coarse, stop at the first fully
clean) to heal self-intersections the gentle path can't. Covers the request schema
defaults, candidate-size selection, and the defect score used to pick the best.
"""
from __future__ import annotations

import numpy as np

from domain.schemas import RepairRequest
from geometry_sdk.types import MeshDocument
from services.operations import _mesh_defect_score, _repair_remesh_candidate_sizes


def _closed_tet() -> MeshDocument:
    v = np.array([[0, 0, 0], [2, 0, 0], [0, 2, 0], [0, 0, 2]], dtype=float)
    f = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return MeshDocument(vertices=v, faces=f)


def test_repair_request_defaults_to_gentle() -> None:
    request = RepairRequest()
    assert request.voxel_remesh is False
    assert request.voxel_size_mm is None


def test_remesh_candidate_sizes_respect_explicit_override() -> None:
    assert _repair_remesh_candidate_sizes(_closed_tet(), 0.1) == [0.1]


def test_remesh_candidate_sizes_default_fine_to_coarse() -> None:
    sizes = _repair_remesh_candidate_sizes(_closed_tet(), None)
    assert len(sizes) >= 2
    assert sizes == sorted(sizes)  # ascending == fine -> coarse (finest tried first)
    assert all(size > 0 for size in sizes)


def test_defect_score_clean_zero_open_positive() -> None:
    assert _mesh_defect_score(_closed_tet()) == 0
    open_sheet = MeshDocument(
        vertices=np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0]], dtype=float),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    assert _mesh_defect_score(open_sheet) > 0
