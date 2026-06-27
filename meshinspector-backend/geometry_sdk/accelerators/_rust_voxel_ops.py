from __future__ import annotations

import numpy as np
from pathlib import Path
from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_voxel_common import _require_rust_kernel
from geometry_sdk.types import MeshDocument


def sdf_boolean_values(
    a_values: np.ndarray,
    b_values: np.ndarray,
    *,
    operation: str,
) -> np.ndarray:
    return sdf_boolean_values_required(a_values, b_values, operation=operation)

def sdf_boolean_values_required(
    a_values: np.ndarray,
    b_values: np.ndarray,
    *,
    operation: str,
) -> np.ndarray:
    if operation not in _common.SDF_BOOLEAN_OPERATIONS:
        raise ValueError("operation must be 'union', 'intersection', or 'difference'")
    left = np.asarray(a_values, dtype=np.float32)
    right = np.asarray(b_values, dtype=np.float32)
    if left.shape != right.shape:
        raise ValueError("SDF value arrays must have the same shape")
    kernel = _require_rust_kernel("sdf_boolean_values")
    values = kernel(left.reshape(-1), right.reshape(-1), operation)
    return np.asarray(values, dtype=np.float32).reshape(left.shape)

def voxel_binary_values_required(
    a_values: np.ndarray,
    b_values: np.ndarray,
    *,
    operation: str,
) -> np.ndarray:
    if operation not in _common.VOXEL_BINARY_OPERATIONS:
        raise ValueError("operation must be one of: union, intersection, difference, max, min, sum, multiply, divide")
    left = np.asarray(a_values, dtype=np.float32)
    right = np.asarray(b_values, dtype=np.float32)
    if left.shape != right.shape:
        raise ValueError("voxel value arrays must have the same shape")
    kernel = _require_rust_kernel("voxel_binary_values")
    values = kernel(left.reshape(-1), right.reshape(-1), operation)
    return np.asarray(values, dtype=np.float32).reshape(left.shape)

def voxel_binary_iso_value(left_iso_value: float, right_iso_value: float, *, operation: str) -> float:
    if operation not in _common.VOXEL_BINARY_OPERATIONS:
        raise ValueError("operation must be one of: union, intersection, difference, max, min, sum, multiply, divide")
    kernel = _require_rust_kernel("voxel_binary_iso_value")
    return float(kernel(float(left_iso_value), float(right_iso_value), operation))

def voxel_default_iso_value(values: np.ndarray) -> float:
    voxel_values = np.asarray(values, dtype=np.float32)
    if voxel_values.size == 0:
        raise ValueError("voxel values must not be empty")
    kernel = _require_rust_kernel("voxel_default_iso_value")
    return float(kernel(voxel_values.reshape(-1)))

def voxel_value_range(values: np.ndarray) -> tuple[float, float]:
    voxel_values = np.asarray(values, dtype=np.float32)
    if voxel_values.size == 0:
        raise ValueError("voxel values must not be empty")
    kernel = _require_rust_kernel("voxel_value_range")
    minimum, maximum = kernel(voxel_values.reshape(-1))
    return float(minimum), float(maximum)

def sdf_offset_values(values: np.ndarray, offset_mm: float) -> np.ndarray:
    grid_values = np.asarray(values, dtype=np.float32)
    kernel = _require_rust_kernel("sdf_offset_values")
    output = kernel(grid_values.reshape(-1), float(offset_mm))
    return np.asarray(output, dtype=np.float32).reshape(grid_values.shape)

def sdf_shell_values(values: np.ndarray, wall_thickness_mm: float) -> np.ndarray:
    grid_values = np.asarray(values, dtype=np.float32)
    kernel = _require_rust_kernel("sdf_shell_values")
    output = kernel(grid_values.reshape(-1), float(wall_thickness_mm))
    return np.asarray(output, dtype=np.float32).reshape(grid_values.shape)
