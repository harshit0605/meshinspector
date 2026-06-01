"""Parallel in-house geometry SDK for MeshInspector.

This package is intentionally not wired into production services yet. It is a
development surface for building and testing in-house geometry algorithms beside
the current MeshLib-backed application.
"""

from geometry_sdk.engine import GeometrySDK, default_sdk
from geometry_sdk.types import (
    BrushStroke,
    DrainHolePlan,
    HoleFillReport,
    ManufacturabilityReport,
    MaterialWeightEntry,
    MeshDocument,
    MeshHealth,
    MeshStats,
    RegionEntry,
    RepairReport,
    RingMeasurement,
    ServiceMeshHealth,
    ThicknessSummary,
    VersionCompareSummary,
    VoxelRebuildReport,
)

__all__ = [
    "GeometrySDK",
    "BrushStroke",
    "DrainHolePlan",
    "HoleFillReport",
    "ManufacturabilityReport",
    "MaterialWeightEntry",
    "MeshDocument",
    "MeshHealth",
    "MeshStats",
    "RegionEntry",
    "RepairReport",
    "RingMeasurement",
    "ServiceMeshHealth",
    "ThicknessSummary",
    "VersionCompareSummary",
    "VoxelRebuildReport",
    "default_sdk",
]
