"""Basic mesh healing compatibility wrappers for Rust-owned repair kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import rust
from geometry_sdk.types import MeshDocument, RepairReport


def remove_degenerate_faces(mesh: MeshDocument, *, area_epsilon: float = 1e-12) -> tuple[MeshDocument, int]:
    return _require_rust(rust.remove_degenerate_faces(mesh, area_epsilon=area_epsilon), "remove_degenerate_faces")


def remove_unreferenced_vertices(mesh: MeshDocument) -> tuple[MeshDocument, int]:
    return _require_rust(rust.remove_unreferenced_vertices(mesh), "remove_unreferenced_vertices")


def merge_close_vertices(mesh: MeshDocument, *, tolerance: float = 1e-6) -> tuple[MeshDocument, int]:
    return _require_rust(rust.merge_close_vertices(mesh, tolerance=tolerance), "merge_close_vertices")


def orient_faces_outward(mesh: MeshDocument) -> MeshDocument:
    return _require_rust(rust.orient_faces_outward(mesh), "orient_faces_outward")


def basic_repair(mesh: MeshDocument, *, merge_tolerance: float = 1e-6, area_epsilon: float = 1e-12) -> tuple[MeshDocument, RepairReport]:
    """Run deterministic low-risk repair passes.

    This intentionally does not fill holes or repair self-intersections. Those
    passes require the spatial/topology kernels planned for later phases.
    """

    return _require_rust(
        rust.basic_repair(mesh, merge_tolerance=merge_tolerance, area_epsilon=area_epsilon),
        "basic_repair",
    )


def repaired_surface_area(mesh: MeshDocument) -> float:
    return _require_rust(rust.repaired_surface_area(mesh), "repaired_surface_area")


def _require_rust(value: Any, kernel_name: str) -> Any:
    if value is None:
        raise RuntimeError(
            f"Rust kernel {kernel_name} is required for geometry_sdk.repair.basic. "
            "Build the extension with `uv tool run maturin develop --manifest-path "
            "geometry-rs/crates/zennah-geometry-py/Cargo.toml`."
        )
    return value
