"""Ring resizing compatibility wrappers for Rust-owned deformation kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import rust
from geometry_sdk.types import MeshDocument


def radial_scale(
    mesh: MeshDocument,
    scale_factor: float,
    ring_axis: Any = None,
    preserve_indices: Any = None,
) -> MeshDocument:
    vertices = _require_rust(
        rust.radial_scale_vertices(
            mesh,
            scale_factor,
            ring_axis=ring_axis,
            preserve_indices=preserve_indices,
        ),
        "radial_scale_vertices",
    )
    return mesh.copy(vertices=vertices)


def resize_ring(
    mesh: MeshDocument,
    current_size: float,
    target_size: float,
    ring_axis: Any = None,
    preserve_indices: Any = None,
) -> MeshDocument:
    vertices = _require_rust(
        rust.resize_ring_vertices(
            mesh,
            current_size,
            target_size,
            ring_axis=ring_axis,
            preserve_indices=preserve_indices,
        ),
        "resize_ring_vertices",
    )
    return mesh.copy(vertices=vertices)


def _require_rust(value: Any, kernel_name: str) -> Any:
    if value is None:
        raise RuntimeError(
            f"Rust kernel {kernel_name} is required for geometry_sdk.deform.resize. "
            "Build the extension with `uv tool run maturin develop --manifest-path "
            "geometry-rs/crates/zennah-geometry-py/Cargo.toml`."
        )
    return value
