"""Ring semantic region detection compatibility wrappers."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_jewelry
from geometry_sdk.types import MeshDocument, RegionEntry, RingMeasurement


def detect_ring_regions(
    mesh: MeshDocument,
    measurement: RingMeasurement,
    thickness: Any = None,
    threshold_mm: float = 0.6,
) -> list[RegionEntry]:
    return _rust_jewelry.detect_ring_regions(
        mesh,
        measurement,
        thickness=thickness,
        threshold_mm=threshold_mm,
    )
