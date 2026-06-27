"""Regression: the brush fold guard. _brush_fold_reason flags a brush result that
folded the surface onto itself (thicken/scoop displace along normals with no fold
check — a default 0.15mm thicken took the snake from 2 to 34 self-intersecting
faces). It must flag an INCREASE in self-intersections, not pre-existing ones, and
leave the clean smooth brush alone.
"""
from __future__ import annotations

from geometry_sdk.testing.fixtures import cube, meshlib_self_intersecting_torus
from services.operations import _brush_fold_reason


def test_brush_guard_flags_new_self_intersections() -> None:
    clean = cube(size=2.0)  # 0 self-intersections
    folded = meshlib_self_intersecting_torus()  # 128 self-intersecting faces
    reason = _brush_fold_reason(clean, folded)
    assert reason is not None and "self-intersecting" in reason


def test_brush_guard_ignores_preexisting_self_intersections() -> None:
    # Source already self-intersecting; an identical "output" is not an increase,
    # so the guard must NOT fire (it flags folds the stroke created, not prior state).
    folded = meshlib_self_intersecting_torus()
    assert _brush_fold_reason(folded, folded) is None


def test_brush_guard_allows_clean_result() -> None:
    assert _brush_fold_reason(cube(size=2.0), cube(size=2.0)) is None
