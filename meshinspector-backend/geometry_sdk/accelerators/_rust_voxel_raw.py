from __future__ import annotations

import numpy as np
from pathlib import Path
from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_voxel_common import _require_rust_kernel
from geometry_sdk.types import MeshDocument


def load_raw_voxels(
    path: str | Path,
    *,
    dimensions: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    scalar_type: str,
    grid_level_set: bool = False,
) -> dict[str, Any]:
    if scalar_type not in _common.RAW_VOXEL_SCALAR_TYPES:
        raise ValueError(
            "scalar_type must be one of: uint8, int8, uint16, int16, uint32, int32, uint64, int64, float32, float64, float32_4"
        )
    rust_dimensions = np.asarray(dimensions, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_dimensions.shape != (3,) or np.any(rust_dimensions <= 0):
        raise ValueError("dimensions must contain three positive values")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    kernel = _require_rust_kernel("load_raw_voxels")
    return dict(
        kernel(
            str(path),
            rust_dimensions,
            rust_voxel_size,
            scalar_type,
            bool(grid_level_set),
        )
    )

def load_raw_voxels_auto(path: str | Path) -> dict[str, Any]:
    kernel = _require_rust_kernel("load_raw_voxels_auto")
    return dict(kernel(str(path)))

def load_tiff_voxels_dir(
    directory: str | Path,
    *,
    voxel_size: tuple[float, float, float],
    grid_level_set: bool = False,
) -> dict[str, Any]:
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    kernel = _require_rust_kernel("load_tiff_voxels_dir")
    return dict(kernel(str(directory), rust_voxel_size, bool(grid_level_set)))
