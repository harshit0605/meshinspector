"""Distance compatibility wrappers for Rust-owned deformation helpers."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_distance


def nearest_distances(vertices, target_indices, chunk_size: int = 4096):
    _ = chunk_size
    return _rust_distance.nearest_distances(vertices, target_indices)
