"""Ring resizing compatibility wrappers for Rust-owned deformation kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import rust
from geometry_sdk.types import MeshDocument, RingFitResult


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


def fit_ring_to_diameter(
    mesh: MeshDocument,
    measured_diameter_mm: float,
    target_diameter_mm: float,
    ring_axis: Any = None,
    preserve_indices: Any = None,
    max_preserve_scale_ratio: float = 1.5,
) -> RingFitResult:
    vertices, applied_uniform_fallback, scale_factor = rust.fit_ring_to_diameter_vertices(
        mesh,
        measured_diameter_mm,
        target_diameter_mm,
        ring_axis=ring_axis,
        preserve_indices=preserve_indices,
        max_preserve_scale_ratio=max_preserve_scale_ratio,
    )
    _require_rust(vertices, "fit_ring_to_diameter_vertices")
    return RingFitResult(
        mesh=mesh.copy(vertices=vertices),
        applied_uniform_fallback=applied_uniform_fallback,
        scale_factor=scale_factor,
    )


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
