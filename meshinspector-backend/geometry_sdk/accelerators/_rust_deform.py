from __future__ import annotations

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import BrushStroke, MeshDocument

def laplacian_smooth_vertices(
    mesh: MeshDocument,
    *,
    iterations: int = 1,
    strength: float = 0.25,
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "laplacian_smooth_vertices"):
        if mode == "rust":
            raise RuntimeError(
                "GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose laplacian_smooth_vertices"
            )
        return None

    smoothed = _common._rs.laplacian_smooth_vertices(
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
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "taubin_smooth_vertices"):
        if mode == "rust":
            raise RuntimeError(
                "GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose taubin_smooth_vertices"
            )
        return None

    smoothed = _common._rs.taubin_smooth_vertices(
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
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    # Precomputed weighted smoothing is mainly for forced parity and future
    # resident pipelines. Product seeded smoothing should prefer
    # smooth_vertices_with_falloff so falloff and smoothing share one boundary.
    if mode != "rust":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "weighted_laplacian_smooth_vertices"):
        if mode == "rust":
            raise RuntimeError(
                "GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose weighted_laplacian_smooth_vertices"
            )
        return None

    weight_array = np.asarray(weights, dtype=np.float32).reshape(-1)
    if weight_array.shape[0] != mesh.vertex_count:
        raise ValueError("weights length must match mesh vertex count")
    smoothed = _common._rs.weighted_laplacian_smooth_vertices(
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
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "falloff_weights"):
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose falloff_weights")
        return None

    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    weights = _common._rs.falloff_weights(
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
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "smooth_vertices_with_falloff"):
        if mode == "rust":
            raise RuntimeError(
                "GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose smooth_vertices_with_falloff"
            )
        return None

    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    smoothed = _common._rs.smooth_vertices_with_falloff(
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


def outward_directions(mesh: MeshDocument) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    # NumPy is faster for the standalone normal/outward pass because it avoids
    # copying a full direction field across the extension boundary. Keep this
    # exposed for forced parity and resident Rust deformation pipelines.
    if mode != "rust":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "outward_directions"):
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose outward_directions")
        return None

    directions = _common._rs.outward_directions(mesh.vertices, mesh.faces)
    return np.asarray(directions, dtype=np.float64).reshape(-1, 3)


def local_offset_vertices(
    mesh: MeshDocument,
    seed_indices: np.ndarray,
    *,
    falloff_mm: float,
    amount_mm: float,
    cutoff_multiplier: float = 3.0,
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "local_offset_vertices"):
        if mode == "rust":
            raise RuntimeError(
                "GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose local_offset_vertices"
            )
        return None

    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    displaced = _common._rs.local_offset_vertices(
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
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "local_thicken_to_minimum_vertices"):
        if mode == "rust":
            raise RuntimeError(
                "GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose local_thicken_to_minimum_vertices"
            )
        return None

    seeds = np.unique(np.asarray(seed_indices, dtype=np.int64)).reshape(-1)
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    thickness = np.asarray(thickness_values, dtype=np.float32).reshape(-1)
    if thickness.shape[0] != mesh.vertex_count:
        raise ValueError("thickness_values length must match mesh vertex count")
    displaced = _common._rs.local_thicken_to_minimum_vertices(
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
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "apply_brush_strokes"):
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose apply_brush_strokes")
        return None

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

    displaced = _common._rs.apply_brush_strokes(
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
