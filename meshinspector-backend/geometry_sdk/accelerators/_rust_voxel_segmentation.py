from __future__ import annotations

import numpy as np
from pathlib import Path
from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_voxel_common import _require_rust_kernel
from geometry_sdk.types import MeshDocument


def voxel_segmentation_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    inside_seeds: np.ndarray,
    outside_seeds: np.ndarray,
    exponent_modifier: float = 3000.0,
    voxels_expansion: int = 25,
    include_boundary_outside: bool = True,
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_inside_seeds = np.asarray(inside_seeds, dtype=np.int64)
    rust_outside_seeds = np.asarray(outside_seeds, dtype=np.int64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_inside_seeds.ndim != 2 or rust_inside_seeds.shape[1] != 3:
        raise ValueError("inside_seeds must have shape (n, 3)")
    if rust_inside_seeds.shape[0] == 0:
        raise ValueError("inside_seeds must not be empty")
    if np.any(rust_inside_seeds < 0):
        raise ValueError("inside_seeds values must be non-negative")
    if rust_outside_seeds.ndim != 2 or rust_outside_seeds.shape[1] != 3:
        raise ValueError("outside_seeds must have shape (n, 3)")
    if np.any(rust_outside_seeds < 0):
        raise ValueError("outside_seeds values must be non-negative")
    if not np.isfinite(exponent_modifier):
        raise ValueError("exponent_modifier must be finite")
    if voxels_expansion < 0:
        raise ValueError("voxels_expansion must be non-negative")
    kernel = _require_rust_kernel("voxel_segmentation_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            np.ascontiguousarray(rust_inside_seeds, dtype=np.int64),
            np.ascontiguousarray(rust_outside_seeds, dtype=np.int64),
            float(exponent_modifier),
            int(voxels_expansion),
            bool(include_boundary_outside),
        )
    )

def voxel_segmentation_mesh_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    inside_seeds: np.ndarray,
    outside_seeds: np.ndarray,
    voxel_size: tuple[float, float, float],
    exponent_modifier: float = 3000.0,
    voxels_expansion: int = 25,
    include_boundary_outside: bool = True,
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_inside_seeds = np.asarray(inside_seeds, dtype=np.int64)
    rust_outside_seeds = np.asarray(outside_seeds, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_inside_seeds.ndim != 2 or rust_inside_seeds.shape[1] != 3:
        raise ValueError("inside_seeds must have shape (n, 3)")
    if rust_inside_seeds.shape[0] == 0:
        raise ValueError("inside_seeds must not be empty")
    if np.any(rust_inside_seeds < 0):
        raise ValueError("inside_seeds values must be non-negative")
    if rust_outside_seeds.ndim != 2 or rust_outside_seeds.shape[1] != 3:
        raise ValueError("outside_seeds must have shape (n, 3)")
    if np.any(rust_outside_seeds < 0):
        raise ValueError("outside_seeds values must be non-negative")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if not np.isfinite(exponent_modifier):
        raise ValueError("exponent_modifier must be finite")
    if voxels_expansion < 0:
        raise ValueError("voxels_expansion must be non-negative")
    kernel = _require_rust_kernel("voxel_segmentation_mesh_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            np.ascontiguousarray(rust_inside_seeds, dtype=np.int64),
            np.ascontiguousarray(rust_outside_seeds, dtype=np.int64),
            rust_voxel_size,
            float(exponent_modifier),
            int(voxels_expansion),
            bool(include_boundary_outside),
        )
    )

def voxel_mask_to_mesh_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    mask_coordinates: np.ndarray,
    voxel_size: tuple[float, float, float],
    mask_expansion: int = 25,
    smooth_band_radius: int = 3,
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_mask_coordinates = np.asarray(mask_coordinates, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_mask_coordinates.ndim != 2 or rust_mask_coordinates.shape[1] != 3:
        raise ValueError("mask_coordinates must have shape (n, 3)")
    if rust_mask_coordinates.shape[0] == 0:
        raise ValueError("mask_coordinates must not be empty")
    if np.any(rust_mask_coordinates < 0):
        raise ValueError("mask_coordinates values must be non-negative")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if mask_expansion < 0:
        raise ValueError("mask_expansion must be non-negative")
    if smooth_band_radius < 0:
        raise ValueError("smooth_band_radius must be non-negative")
    kernel = _require_rust_kernel("voxel_mask_to_mesh_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            np.ascontiguousarray(rust_mask_coordinates, dtype=np.int64),
            rust_voxel_size,
            int(mask_expansion),
            int(smooth_band_radius),
        )
    )
