from __future__ import annotations

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import BrushStroke, MeshDocument


def _require_rust_kernel(name: str):
    _common.accelerator_mode()
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def laplacian_smooth_vertices(
    mesh: MeshDocument,
    *,
    iterations: int = 1,
    strength: float = 0.25,
) -> np.ndarray:
    kernel = _require_rust_kernel("laplacian_smooth_vertices")
    smoothed = kernel(
        mesh.vertices,
        mesh.faces,
        int(iterations),
        float(np.clip(strength, 0.0, 1.0)),
    )
    return np.asarray(smoothed, dtype=np.float64).reshape(-1, 3)


def taubin_smooth_vertices(
    mesh: MeshDocument,
    *,
    iterations: int = 10,
    lamb: float = 0.5,
    nu: float = -0.53,
) -> np.ndarray:
    kernel = _require_rust_kernel("taubin_smooth_vertices")
    smoothed = kernel(
        mesh.vertices,
        mesh.faces,
        int(iterations),
        float(np.clip(lamb, 0.0, 1.0)),
        float(nu),
    )
    return np.asarray(smoothed, dtype=np.float64).reshape(-1, 3)


def weighted_laplacian_smooth_vertices(
    mesh: MeshDocument,
    weights: np.ndarray,
    *,
    iterations: int = 1,
    strength: float = 0.25,
    active_threshold: float = 0.02,
) -> np.ndarray:
    kernel = _require_rust_kernel("weighted_laplacian_smooth_vertices")
    weight_array = np.asarray(weights, dtype=np.float32).reshape(-1)
    if weight_array.shape[0] != mesh.vertex_count:
        raise ValueError("weights length must match mesh vertex count")
    smoothed = kernel(
        mesh.vertices,
        mesh.faces,
        weight_array,
        int(iterations),
        float(np.clip(strength, 0.0, 1.0)),
        float(active_threshold),
    )
    return np.asarray(smoothed, dtype=np.float64).reshape(-1, 3)


def falloff_weights(
    mesh: MeshDocument,
    seed_indices: np.ndarray,
    *,
    falloff_mm: float,
    cutoff_multiplier: float = 3.0,
) -> np.ndarray:
    kernel = _require_rust_kernel("falloff_weights")
    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    weights = kernel(
        mesh.vertices,
        seeds,
        float(falloff_mm),
        float(cutoff_multiplier),
    )
    return np.asarray(weights, dtype=np.float32).reshape(-1)


def smooth_vertices_with_falloff(
    mesh: MeshDocument,
    seed_indices: np.ndarray,
    *,
    falloff_mm: float,
    iterations: int = 5,
    strength: float = 0.5,
    active_threshold: float = 0.02,
    cutoff_multiplier: float = 3.0,
) -> np.ndarray:
    kernel = _require_rust_kernel("smooth_vertices_with_falloff")
    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    smoothed = kernel(
        mesh.vertices,
        mesh.faces,
        seeds,
        float(falloff_mm),
        int(iterations),
        float(np.clip(strength, 0.0, 1.0)),
        float(active_threshold),
        float(cutoff_multiplier),
    )
    return np.asarray(smoothed, dtype=np.float64).reshape(-1, 3)


def outward_directions(mesh: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("outward_directions")
    directions = kernel(mesh.vertices, mesh.faces)
    return np.asarray(directions, dtype=np.float64).reshape(-1, 3)


def local_offset_vertices(
    mesh: MeshDocument,
    seed_indices: np.ndarray,
    *,
    falloff_mm: float,
    amount_mm: float,
    cutoff_multiplier: float = 3.0,
) -> np.ndarray:
    kernel = _require_rust_kernel("local_offset_vertices")
    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    displaced = kernel(
        mesh.vertices,
        mesh.faces,
        seeds,
        float(falloff_mm),
        float(amount_mm),
        float(cutoff_multiplier),
    )
    return np.asarray(displaced, dtype=np.float64).reshape(-1, 3)


def local_thicken_to_minimum_vertices(
    mesh: MeshDocument,
    seed_indices: np.ndarray,
    thickness_values: np.ndarray,
    *,
    min_target_thickness_mm: float,
    falloff_mm: float,
    deficit_scale: float = 0.75,
) -> np.ndarray:
    kernel = _require_rust_kernel("local_thicken_to_minimum_vertices")
    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    thickness = np.asarray(thickness_values, dtype=np.float32).reshape(-1)
    if thickness.shape[0] != mesh.vertex_count:
        raise ValueError("thickness_values length must match mesh vertex count")
    displaced = kernel(
        mesh.vertices,
        mesh.faces,
        seeds,
        thickness,
        float(min_target_thickness_mm),
        float(falloff_mm),
        float(deficit_scale),
    )
    return np.asarray(displaced, dtype=np.float64).reshape(-1, 3)


def apply_brush_strokes(
    mesh: MeshDocument,
    strokes: list[BrushStroke],
    *,
    cutoff_multiplier: float = 3.0,
) -> np.ndarray:
    kernel = _require_rust_kernel("apply_brush_strokes")
    if not strokes:
        return mesh.vertices.copy()
    operations: list[int] = []
    seed_offsets = [0]
    flat_seed_indices: list[int] = []
    mask_enabled: list[int] = []
    mask_offsets = [0]
    flat_mask_indices: list[int] = []
    protected_offsets = [0]
    flat_protected_indices: list[int] = []
    amounts: list[float] = []
    falloffs: list[float] = []
    iterations: list[int] = []
    strengths: list[float] = []
    for stroke in strokes:
        if stroke.operation not in _common.BRUSH_OPERATION_CODES:
            raise ValueError("operation must be 'thicken', 'scoop', or 'smooth'")
        seeds = np.unique(np.asarray(stroke.seed_indices, dtype=np.int64).reshape(-1))
        if seeds.size == 0:
            raise ValueError("seed_indices must not be empty")
        operations.append(_common.BRUSH_OPERATION_CODES[stroke.operation])
        flat_seed_indices.extend(int(seed) for seed in seeds)
        seed_offsets.append(len(flat_seed_indices))
        if stroke.mask_indices is None:
            mask_enabled.append(0)
        else:
            mask_enabled.append(1)
            mask_indices = np.unique(np.asarray(stroke.mask_indices, dtype=np.int64).reshape(-1))
            flat_mask_indices.extend(int(index) for index in mask_indices)
        mask_offsets.append(len(flat_mask_indices))
        if stroke.protected_indices is not None:
            protected_indices = np.unique(np.asarray(stroke.protected_indices, dtype=np.int64).reshape(-1))
            flat_protected_indices.extend(int(index) for index in protected_indices)
        protected_offsets.append(len(flat_protected_indices))
        amounts.append(float(stroke.amount_mm))
        falloffs.append(float(stroke.falloff_mm))
        iterations.append(max(1, int(stroke.iterations)))
        strengths.append(float(np.clip(stroke.strength, 0.0, 1.0)))

    displaced = kernel(
        mesh.vertices,
        mesh.faces,
        np.asarray(operations, dtype=np.int64),
        np.asarray(seed_offsets, dtype=np.int64),
        np.asarray(flat_seed_indices, dtype=np.int64),
        np.asarray(mask_enabled, dtype=np.int64),
        np.asarray(mask_offsets, dtype=np.int64),
        np.asarray(flat_mask_indices, dtype=np.int64),
        np.asarray(protected_offsets, dtype=np.int64),
        np.asarray(flat_protected_indices, dtype=np.int64),
        np.asarray(amounts, dtype=np.float64),
        np.asarray(falloffs, dtype=np.float64),
        np.asarray(iterations, dtype=np.int64),
        np.asarray(strengths, dtype=np.float64),
        float(cutoff_multiplier),
    )
    return np.asarray(displaced, dtype=np.float64).reshape(-1, 3)
