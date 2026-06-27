from __future__ import annotations

import numpy as np
from pathlib import Path
from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_voxel_common import _require_rust_kernel
from geometry_sdk.types import MeshDocument


def voxel_path_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    start: tuple[int, int, int],
    finish: tuple[int, int, int],
    metric: str = "difference",
    max_dist_ratio: float = 1.5,
    plane: str = "none",
    quarters_mask: int = 0x0F,
    exponent_modifier: float = -1.0,
) -> dict[str, Any]:
    if metric not in {"difference", "sum_diffs", "exponent"}:
        raise ValueError("metric must be 'difference' or 'exponent'")
    if plane not in {"none", "yz", "zx", "xy"}:
        raise ValueError("plane must be 'none', 'yz', 'zx', or 'xy'")
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_start = np.asarray(start, dtype=np.int64)
    rust_finish = np.asarray(finish, dtype=np.int64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_start.shape != (3,) or np.any(rust_start < 0):
        raise ValueError("start must contain three non-negative values")
    if rust_finish.shape != (3,) or np.any(rust_finish < 0):
        raise ValueError("finish must contain three non-negative values")
    kernel = _require_rust_kernel("voxel_path_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_start,
            rust_finish,
            metric,
            float(max_dist_ratio),
            plane,
            int(quarters_mask),
            float(exponent_modifier),
        )
    )

def voxel_path_build_four_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    start: tuple[int, int, int],
    finish: tuple[int, int, int],
    metric: str = "exponent",
    max_dist_ratio: float = 1.5,
    plane: str = "none",
    exponent_modifier: float = -1.0,
) -> list[dict[str, Any]]:
    if metric not in {"difference", "sum_diffs", "exponent"}:
        raise ValueError("metric must be 'difference' or 'exponent'")
    if plane not in {"none", "yz", "zx", "xy"}:
        raise ValueError("plane must be 'none', 'yz', 'zx', or 'xy'")
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_start = np.asarray(start, dtype=np.int64)
    rust_finish = np.asarray(finish, dtype=np.int64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_start.shape != (3,) or np.any(rust_start < 0):
        raise ValueError("start must contain three non-negative values")
    if rust_finish.shape != (3,) or np.any(rust_finish < 0):
        raise ValueError("finish must contain three non-negative values")
    kernel = _require_rust_kernel("voxel_path_build_four_values")
    payload = dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_start,
            rust_finish,
            metric,
            float(max_dist_ratio),
            plane,
            float(exponent_modifier),
        )
    )
    return [dict(entry) for entry in payload["paths"]]

def voxel_slice_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    plane: str,
    slice_index: int,
    min_value: float,
    max_value: float,
) -> dict[str, Any]:
    if plane not in {"yz", "zx", "xy"}:
        raise ValueError("plane must be 'yz', 'zx', or 'xy'")
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if slice_index < 0:
        raise ValueError("slice_index must be non-negative")
    kernel = _require_rust_kernel("voxel_slice_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            plane,
            int(slice_index),
            float(min_value),
            float(max_value),
        )
    )

def voxel_line_graph_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    axis: str,
    fixed_coordinate: tuple[int, int, int],
) -> dict[str, Any]:
    if axis not in {"x", "y", "z"}:
        raise ValueError("axis must be 'x', 'y', or 'z'")
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_fixed_coordinate = np.asarray(fixed_coordinate, dtype=np.int64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_fixed_coordinate.shape != (3,) or np.any(rust_fixed_coordinate < 0):
        raise ValueError("fixed_coordinate must contain three non-negative values")
    kernel = _require_rust_kernel("voxel_line_graph_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            axis,
            rust_fixed_coordinate,
        )
    )

def voxel_active_box_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    min_corner: tuple[int, int, int],
    dimensions: tuple[int, int, int],
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_min_corner = np.asarray(min_corner, dtype=np.int64)
    rust_dimensions = np.asarray(dimensions, dtype=np.int64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_min_corner.shape != (3,) or np.any(rust_min_corner < 0):
        raise ValueError("min_corner must contain three non-negative values")
    if rust_dimensions.shape != (3,) or np.any(rust_dimensions <= 0):
        raise ValueError("dimensions must contain three positive values")
    kernel = _require_rust_kernel("voxel_active_box_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_min_corner,
            rust_dimensions,
        )
    )
