"""Regression: the exact-boolean cut cap + the post-cap watertight/manifold guard.
_cap_planar_cut closes the open cross-section an exact boolean leaves when its cut
contour cannot be stitched (difference/intersection/union on organic input): a planar
cut closes via fill_planar_holes, a NON-planar cut via the general hole-fill, and a
fill that leaves non-manifold edges is repaired. It is a no-op on an already-clean
result. Whatever the cap cannot seal into a clean solid is detected by the boundary /
non-manifold helpers so run_exact_boolean_for_version refuses rather than shipping
defective geometry (no silent garbage).
"""
from __future__ import annotations

import numpy as np
from collections import Counter

from api.routers.versions import (
    _boundary_edge_count,
    _cap_planar_cut,
    _nonmanifold_edge_count,
)
from geometry_sdk.types import MeshDocument


def _boundary_edges(mesh: MeshDocument) -> int:
    faces = np.asarray(mesh.faces, dtype=np.int64)
    counter: Counter = Counter()
    for tri in faces:
        for edge in ((tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])):
            counter[tuple(sorted((int(edge[0]), int(edge[1]))))] += 1
    return sum(1 for c in counter.values() if c == 1)


def _closed_tet() -> MeshDocument:
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float)
    faces = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def _open_tet_missing_base() -> MeshDocument:
    # Tet with the base face (the planar z=0 triangle [0,2,1]) removed -> one open
    # planar triangular hole (3 boundary edges).
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float)
    faces = np.array([[0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def test_cap_closes_open_planar_cut() -> None:
    open_mesh = _open_tet_missing_base()
    assert _boundary_edges(open_mesh) == 3  # open
    capped = _cap_planar_cut(open_mesh)
    assert _boundary_edges(capped) == 0  # cap filled the planar hole -> watertight


def test_cap_is_noop_on_closed_mesh() -> None:
    closed = _closed_tet()
    capped = _cap_planar_cut(closed)
    # Already watertight -> returned unchanged (same face count, still closed).
    assert int(capped.face_count) == int(closed.face_count)
    assert _boundary_edges(capped) == 0


def test_cap_is_noop_on_empty_mesh() -> None:
    empty = MeshDocument(vertices=np.zeros((0, 3), dtype=float), faces=np.zeros((0, 3), dtype=np.int64))
    capped = _cap_planar_cut(empty)
    assert int(capped.face_count) == 0


def _open_nonplanar_tent() -> MeshDocument:
    # Cone without its base: apex fanned to a NON-coplanar rim (z = 0, 0.3, 0, 0.3).
    # The open boundary is a 4-edge non-planar loop that fill_planar_holes alone cannot
    # close — only the general hole-fill fallback can.
    apex = [0.5, 0.5, 1.0]
    rim = [[0, 0, 0], [1, 0, 0.3], [1, 1, 0], [0, 1, 0.3]]
    vertices = np.array([apex, *rim], dtype=float)
    faces = np.array([[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def _nonmanifold_fan() -> MeshDocument:
    # Edge (0,1) shared by three triangles -> one non-manifold edge.
    vertices = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1]], dtype=float)
    faces = np.array([[0, 1, 2], [0, 1, 3], [0, 1, 4]], dtype=np.int64)
    return MeshDocument(vertices=vertices, faces=faces)


def test_cap_closes_nonplanar_cut_via_general_fill() -> None:
    tent = _open_nonplanar_tent()
    assert _boundary_edge_count(tent) == 4  # open, non-planar rim
    capped = _cap_planar_cut(tent)
    # The general hole-fill fallback seals the curved rim into a clean solid.
    assert _boundary_edge_count(capped) == 0
    assert _nonmanifold_edge_count(capped) == 0


def test_health_helpers_zero_on_clean_solid() -> None:
    tet = _closed_tet()
    assert _boundary_edge_count(tet) == 0
    assert _nonmanifold_edge_count(tet) == 0


def test_guard_helpers_detect_defects() -> None:
    # The endpoint guard refuses when (open OR non-manifold) edges remain after capping.
    # These are the primitives that fire it.
    assert _boundary_edge_count(_open_tet_missing_base()) == 3
    assert _nonmanifold_edge_count(_nonmanifold_fan()) >= 1
