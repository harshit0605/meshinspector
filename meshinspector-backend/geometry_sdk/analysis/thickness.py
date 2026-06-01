"""Thickness compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_thickness
from geometry_sdk.types import MeshDocument, ThicknessSummary


def ray_thickness_at_vertices(mesh: MeshDocument, *, epsilon: float = 1e-5) -> Any:
    return _rust_thickness.ray_thickness_at_vertices(mesh, epsilon=epsilon)


def insphere_thickness_at_vertices(
    mesh: MeshDocument,
    *,
    max_radius: float = 1.0,
    max_iters: int = 16,
    min_shrinkage: float = 0.99999,
    min_angle_cos: float = -1.0,
    epsilon: float = 1e-5,
) -> Any:
    return _rust_thickness.insphere_thickness_at_vertices(
        mesh,
        max_radius=max_radius,
        max_iters=max_iters,
        min_shrinkage=min_shrinkage,
        min_angle_cos=min_angle_cos,
        epsilon=epsilon,
    )


def service_thickness_at_vertices(
    mesh: MeshDocument,
    *,
    max_radius: float = 1.0,
    max_iters: int = 16,
    min_shrinkage: float = 0.99999,
    min_angle_cos: float = -1.0,
    epsilon: float = 1e-5,
) -> Any:
    return _rust_thickness.service_thickness_at_vertices(
        mesh,
        max_radius=max_radius,
        max_iters=max_iters,
        min_shrinkage=min_shrinkage,
        min_angle_cos=min_angle_cos,
        epsilon=epsilon,
    )


def summarize_thickness(thickness: Any, *, threshold_mm: float = 0.6) -> ThicknessSummary:
    return _rust_thickness.summarize_thickness(thickness, threshold_mm=threshold_mm)
