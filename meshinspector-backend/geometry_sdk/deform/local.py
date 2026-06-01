"""Local deformation compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_local_deform
from geometry_sdk.types import MeshDocument


def outward_directions(mesh: MeshDocument):
    return _rust_local_deform.outward_directions(mesh)


def falloff_weights(mesh: MeshDocument, seed_indices: Any, falloff_mm: float):
    return _rust_local_deform.falloff_weights(mesh, seed_indices, falloff_mm)


def local_thicken(mesh: MeshDocument, seed_indices: Any, amount_mm: float, falloff_mm: float = 1.5) -> MeshDocument:
    vertices = _rust_local_deform.local_offset_vertices(
        mesh,
        seed_indices,
        falloff_mm=falloff_mm,
        amount_mm=amount_mm,
    )
    return mesh.copy(vertices=vertices)


def local_thicken_to_minimum(
    mesh: MeshDocument,
    seed_indices: Any,
    thickness_values: Any,
    *,
    min_target_thickness_mm: float,
    falloff_mm: float = 1.5,
    deficit_scale: float = 0.75,
) -> MeshDocument:
    vertices = _rust_local_deform.local_thicken_to_minimum_vertices(
        mesh,
        seed_indices,
        thickness_values,
        min_target_thickness_mm=min_target_thickness_mm,
        falloff_mm=falloff_mm,
        deficit_scale=deficit_scale,
    )
    return mesh.copy(vertices=vertices)


def local_scoop(mesh: MeshDocument, seed_indices: Any, depth_mm: float, falloff_mm: float) -> MeshDocument:
    vertices = _rust_local_deform.local_offset_vertices(
        mesh,
        seed_indices,
        falloff_mm=falloff_mm,
        amount_mm=-depth_mm,
    )
    return mesh.copy(vertices=vertices)


def taubin_smooth(
    mesh: MeshDocument,
    *,
    iterations: int = 10,
    lamb: float = 0.5,
    nu: float = -0.53,
) -> MeshDocument:
    vertices = _rust_local_deform.taubin_smooth_vertices(
        mesh,
        iterations=iterations,
        lamb=lamb,
        nu=nu,
    )
    return mesh.copy(vertices=vertices)


def smooth(
    mesh: MeshDocument,
    *,
    iterations: int = 5,
    strength: float = 0.5,
    seed_indices: Any = None,
    falloff_mm: float = 1.8,
    nu: float = -0.53,
) -> MeshDocument:
    if seed_indices is None:
        vertices = _rust_local_deform.taubin_smooth_vertices(
            mesh,
            iterations=iterations,
            lamb=strength,
            nu=nu,
        )
    else:
        vertices = _rust_local_deform.smooth_vertices_with_falloff(
            mesh,
            seed_indices,
            falloff_mm=falloff_mm,
            iterations=iterations,
            strength=strength,
        )
    return mesh.copy(vertices=vertices)
