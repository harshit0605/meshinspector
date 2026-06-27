"""Regression: hollow fragmentation guard. An aggressive (target-weight) hollow can
thin a wall until a chip breaks off, leaving the model in multiple pieces. The
handler prunes tiny stray slivers (< 1% of surface area) and then requires a single
solid — a clean hollow is one connected piece, so >1 substantial component means the
cavity perforated the walls (refused). The make-manufacturable composite's
resize+target-weight-hollow path produced exactly this (2 substantial + a sliver).
"""
from __future__ import annotations

import numpy as np

from geometry_sdk.engine import GeometrySDK
from geometry_sdk.types import MeshDocument
from services.operations import _mesh_surface_area_mm2

SDK = GeometrySDK()


def _tet(offset, scale: float = 1.0) -> tuple[np.ndarray, np.ndarray]:
    v = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], dtype=float) * scale + np.asarray(offset, dtype=float)
    f = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return v, f


def _combine(*parts) -> MeshDocument:
    verts, faces, offset = [], [], 0
    for v, f in parts:
        verts.append(v)
        faces.append(f + offset)
        offset += len(v)
    return MeshDocument(vertices=np.vstack(verts), faces=np.vstack(faces))


def _post_prune_components(mesh: MeshDocument) -> int:
    """Mirror the handler: prune components below 1% of surface area, return the count."""
    area = _mesh_surface_area_mm2(mesh)
    _, report = SDK.prune_small_components(mesh, min_area_mm2=area * 0.01)
    return report.output_component_count


def test_surface_area_positive() -> None:
    assert _mesh_surface_area_mm2(_combine(_tet([0, 0, 0], 5.0))) > 0


def test_tiny_sliver_pruned_to_single_solid() -> None:
    # Big solid + a tiny chip -> the chip is pruned, leaving one solid (operation succeeds).
    assert _post_prune_components(_combine(_tet([0, 0, 0], 10.0), _tet([100, 0, 0], 0.3))) == 1


def test_genuine_two_piece_fragmentation_is_flagged() -> None:
    # Two substantial pieces -> not pruned -> handler refuses (>1 component).
    assert _post_prune_components(_combine(_tet([0, 0, 0], 10.0), _tet([100, 0, 0], 8.0))) > 1
