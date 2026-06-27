"""Regression: distance_map_merge co-registration guard. Merge aligns by pixel
index, so maps must share dimensions / origin / pixel_size; otherwise it silently
blends maps that don't overlap in world space. The guard refuses a mismatch.
"""
from __future__ import annotations

import pytest
from fastapi import HTTPException

from api.routers.versions import _ensure_distance_maps_coregistered
from geometry_sdk import DistanceMapDocument


def _dm(width: int = 4, height: int = 4, origin=(0.0, 0.0), pixel_size=(1.0, 1.0)) -> DistanceMapDocument:
    return DistanceMapDocument(
        width=width,
        height=height,
        origin=origin,
        pixel_size=pixel_size,
        values=[0.0] * (width * height),
        valid_count=width * height,
        min_value=0.0,
        max_value=0.0,
        metadata={},
    )


def test_merge_allows_coregistered_maps() -> None:
    _ensure_distance_maps_coregistered(_dm(), _dm())  # must not raise


def test_merge_allows_differing_extent_when_coregistered() -> None:
    # Same origin + pixel_size, different width -> co-registered (MeshLib overlays
    # the common region). Must NOT be refused.
    _ensure_distance_maps_coregistered(_dm(width=4), _dm(width=8))


def test_merge_rejects_origin_mismatch() -> None:
    with pytest.raises(HTTPException):
        _ensure_distance_maps_coregistered(_dm(origin=(0.0, 0.0)), _dm(origin=(5.0, 5.0)))


def test_merge_rejects_pixel_size_mismatch() -> None:
    with pytest.raises(HTTPException):
        _ensure_distance_maps_coregistered(_dm(pixel_size=(1.0, 1.0)), _dm(pixel_size=(2.0, 2.0)))
