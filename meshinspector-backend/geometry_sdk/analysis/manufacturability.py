"""Manufacturability compatibility wrappers for Rust-owned report decisions."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_manufacturability
from geometry_sdk.types import ManufacturabilityReport, MeshDocument, MeshHealth, RegionEntry, RingMeasurement, ThicknessSummary


def health_score(health: MeshHealth) -> int:
    return _rust_manufacturability.health_score(health)


def build_recommendations(
    health: MeshHealth,
    measurement: RingMeasurement,
    thickness: ThicknessSummary,
    regions: list[RegionEntry],
    *,
    threshold_mm: float,
) -> list[str]:
    return _rust_manufacturability.build_recommendations(
        health,
        measurement,
        thickness,
        regions,
        threshold_mm=threshold_mm,
    )


def compute_manufacturability_report(mesh: MeshDocument, *, threshold_mm: float = 0.6) -> ManufacturabilityReport:
    return _rust_manufacturability.compute_manufacturability_report(mesh, threshold_mm=threshold_mm)
