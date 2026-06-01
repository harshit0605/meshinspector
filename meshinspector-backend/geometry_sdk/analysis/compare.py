"""Compare compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_compare
from geometry_sdk.types import MeshDocument, VersionCompareSummary


def nearest_vertex_distances(source: MeshDocument, target: MeshDocument, *, chunk_size: int = 4096) -> Any:
    _ = chunk_size
    return _rust_compare.nearest_vertex_distances(source, target)


def compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
    return _rust_compare.compare_summary(source, target)


def signed_compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
    return _rust_compare.signed_compare_summary(source, target)


def version_compare_summary(source: MeshDocument, target: MeshDocument) -> VersionCompareSummary:
    return _rust_compare.version_compare_summary(source, target)


def version_compare_distances(source: MeshDocument, target: MeshDocument) -> Any:
    return _rust_compare.version_compare_distances(source, target)


def service_compare_distances(source: MeshDocument, other: MeshDocument) -> Any:
    return _rust_compare.service_compare_distances(source, other)


def service_compare_summary(source: MeshDocument, other: MeshDocument) -> VersionCompareSummary:
    return _rust_compare.service_compare_summary(source, other)


def nearest_surface_distances(source: MeshDocument, target: MeshDocument) -> Any:
    return _rust_compare.nearest_surface_distances(source, target)


def signed_surface_distances(source: MeshDocument, target: MeshDocument) -> Any:
    return _rust_compare.signed_surface_distances(source, target)
