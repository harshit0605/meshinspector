"""Regression: offset_contours handles counter-clockwise closed contours. The Rust
kernel only implements the clockwise convention (a CCW closed contour previously
raised, surfacing as HTTP 400). The SDK now normalizes closed-contour winding to
clockwise for the scalar-offset path, so a CCW square offsets outward like a CW one.
"""
from __future__ import annotations

import numpy as np

from geometry_sdk.distance_map.lines import offset_contours


def _bbox(result):
    pts = np.array([p for contour in result for p in contour], dtype=float)
    return pts.min(axis=0)[:2], pts.max(axis=0)[:2]


def test_offset_contours_ccw_closed_offsets_outward_like_cw() -> None:
    ccw = [(0, 0, 0), (10, 0, 0), (10, 10, 0), (0, 10, 0), (0, 0, 0)]  # counter-clockwise
    result = offset_contours([ccw], offset=2.0, corner_type="sharp")
    mn, mx = _bbox(result)
    np.testing.assert_allclose(mn, [-2.0, -2.0], atol=0.01)
    np.testing.assert_allclose(mx, [12.0, 12.0], atol=0.01)


def test_offset_contours_cw_closed_unchanged() -> None:
    cw = [(0, 0, 0), (0, 10, 0), (10, 10, 0), (10, 0, 0), (0, 0, 0)]  # clockwise
    result = offset_contours([cw], offset=2.0, corner_type="sharp")
    mn, mx = _bbox(result)
    np.testing.assert_allclose(mn, [-2.0, -2.0], atol=0.01)
    np.testing.assert_allclose(mx, [12.0, 12.0], atol=0.01)
