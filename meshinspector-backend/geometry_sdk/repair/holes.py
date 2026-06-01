"""Hole repair compatibility wrappers for Rust-owned repair kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import rust
from geometry_sdk.types import HoleFillReport, MeshDocument


def ordered_boundary_loops(mesh: MeshDocument) -> list[list[int]]:
    return _require_rust(rust.ordered_boundary_loops(mesh), "ordered_boundary_loops")


def fill_planar_holes(mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport]:
    return _require_rust(rust.fill_planar_holes(mesh, max_edges=max_edges), "fill_planar_holes")


def service_fill_holes(mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport]:
    return _require_rust(rust.service_fill_holes(mesh, max_edges=max_edges), "service_fill_holes")


def _require_rust(value: Any, kernel_name: str) -> Any:
    if value is None:
        raise RuntimeError(
            f"Rust kernel {kernel_name} is required for geometry_sdk.repair.holes. "
            "Build the extension with `uv tool run maturin develop --manifest-path "
            "geometry-rs/crates/zennah-geometry-py/Cargo.toml`."
        )
    return value
