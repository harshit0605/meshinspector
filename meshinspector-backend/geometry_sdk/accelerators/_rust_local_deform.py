from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _unique_seeds(seed_indices: Any) -> np.ndarray:
    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    return seeds


def outward_directions(mesh: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("outward_directions")
    return np.asarray(kernel(mesh.vertices, mesh.faces), dtype=np.float64).reshape(-1, 3)


def falloff_weights(mesh: MeshDocument, seed_indices: Any, falloff_mm: float) -> np.ndarray:
    kernel = _require_rust_kernel("falloff_weights")
    weights = kernel(mesh.vertices, _unique_seeds(seed_indices), float(falloff_mm), 3.0)
    return np.asarray(weights, dtype=np.float32).reshape(-1)


def local_offset_vertices(mesh: MeshDocument, seed_indices: Any, *, falloff_mm: float, amount_mm: float) -> np.ndarray:
    kernel = _require_rust_kernel("local_offset_vertices")
    vertices = kernel(
        mesh.vertices,
        mesh.faces,
        _unique_seeds(seed_indices),
        float(falloff_mm),
        float(amount_mm),
        3.0,
    )
    return np.asarray(vertices, dtype=np.float64).reshape(-1, 3)


def local_thicken_to_minimum_vertices(
    mesh: MeshDocument,
    seed_indices: Any,
    thickness_values: Any,
    *,
    min_target_thickness_mm: float,
    falloff_mm: float,
    deficit_scale: float = 0.75,
) -> np.ndarray:
    kernel = _require_rust_kernel("local_thicken_to_minimum_vertices")
    thickness = np.asarray(thickness_values, dtype=np.float32).reshape(-1)
    if thickness.shape[0] != mesh.vertex_count:
        raise ValueError("thickness_values length must match mesh vertex count")
    vertices = kernel(
        mesh.vertices,
        mesh.faces,
        _unique_seeds(seed_indices),
        thickness,
        float(min_target_thickness_mm),
        float(falloff_mm),
        float(deficit_scale),
    )
    return np.asarray(vertices, dtype=np.float64).reshape(-1, 3)


def laplacian_smooth_vertices(mesh: MeshDocument, *, iterations: int, strength: float) -> np.ndarray:
    kernel = _require_rust_kernel("laplacian_smooth_vertices")
    vertices = kernel(
        mesh.vertices,
        mesh.faces,
        max(1, int(iterations)),
        float(np.clip(strength, 0.0, 1.0)),
    )
    return np.asarray(vertices, dtype=np.float64).reshape(-1, 3)


def taubin_smooth_vertices(mesh: MeshDocument, *, iterations: int, lamb: float, nu: float = -0.53) -> np.ndarray:
    kernel = _require_rust_kernel("taubin_smooth_vertices")
    vertices = kernel(
        mesh.vertices,
        mesh.faces,
        max(1, int(iterations)),
        float(np.clip(lamb, 0.0, 1.0)),
        float(nu),
    )
    return np.asarray(vertices, dtype=np.float64).reshape(-1, 3)


def smooth_vertices_with_falloff(
    mesh: MeshDocument,
    seed_indices: Any,
    *,
    falloff_mm: float,
    iterations: int,
    strength: float,
) -> np.ndarray:
    kernel = _require_rust_kernel("smooth_vertices_with_falloff")
    vertices = kernel(
        mesh.vertices,
        mesh.faces,
        _unique_seeds(seed_indices),
        float(falloff_mm),
        max(1, int(iterations)),
        float(np.clip(strength, 0.0, 1.0)),
        0.02,
        3.0,
    )
    return np.asarray(vertices, dtype=np.float64).reshape(-1, 3)
