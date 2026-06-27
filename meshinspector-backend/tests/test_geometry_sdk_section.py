from __future__ import annotations

import pytest

from geometry_sdk import default_sdk
from geometry_sdk.testing.fixtures import cube


def test_section_contour_slices_cube_center_with_authoritative_stats() -> None:
    contour = default_sdk.section_contour(
        cube(size=2.0),
        section_constant=0.0,
        plane_axis=(0.0, 0.0, 1.0),
    )

    assert contour.contour_count == 1
    assert contour.segment_count == 8
    assert contour.selected_region_segment_count == 0
    assert contour.perimeter_mm == pytest.approx(8.0)
    assert contour.width_mm == pytest.approx(2.0)
    assert contour.depth_mm == pytest.approx(2.0)
    assert contour.projected_bounds_min == pytest.approx((-1.0, -1.0))
    assert contour.projected_bounds_max == pytest.approx((1.0, 1.0))
    assert len(contour.segments) == 8


def test_section_contour_marks_selected_region_segments() -> None:
    contour = default_sdk.section_contour(
        cube(size=2.0),
        section_constant=0.0,
        plane_axis=(0.0, 0.0, 1.0),
        selected_vertex_indices=[0, 1, 4, 5],
    )

    assert contour.segment_count == 8
    assert contour.selected_region_segment_count > 0
    assert any(segment.selected_region_hit for segment in contour.segments)
