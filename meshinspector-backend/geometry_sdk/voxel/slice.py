"""MeshLib-style voxel slice extraction."""

from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import VoxelSliceResult, VoxelVolume


def _values_and_shape(
    volume_or_values: VoxelVolume | np.ndarray,
    shape: tuple[int, int, int] | None,
) -> tuple[np.ndarray, tuple[int, int, int]]:
    if isinstance(volume_or_values, VoxelVolume):
        # x-fastest (Fortran), matching this op's idx = x + y*nx + z*nx*ny and the
        # 3D-array branch below; a bare reshape(-1) is C-order and transposed it.
        return np.ravel(volume_or_values.values, order="F"), volume_or_values.dimensions
    values = np.asarray(volume_or_values, dtype=np.float32)
    if shape is None:
        if values.ndim != 3:
            raise ValueError("shape is required when values are not a 3D array")
        shape = tuple(int(value) for value in values.shape)  # type: ignore[assignment]
    if values.ndim == 3 and tuple(int(value) for value in values.shape) == tuple(shape):
        return np.ravel(values, order="F"), shape
    return values.reshape(-1), shape


def _slice_result_from_payload(payload: dict[str, Any]) -> VoxelSliceResult:
    return VoxelSliceResult(
        width=int(payload["width"]),
        height=int(payload["height"]),
        values=np.asarray(payload["values"], dtype=np.float32),
        normalized_values=np.asarray(payload["normalized_values"], dtype=np.float32),
        coordinates=[tuple(int(axis) for axis in coord) for coord in payload["coordinates"]],  # type: ignore[list-item]
    )


def voxel_slice(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    plane: str,
    slice_index: int,
    min_value: float,
    max_value: float,
) -> VoxelSliceResult:
    values, resolved_shape = _values_and_shape(volume_or_values, shape)
    payload = _rust_voxel.voxel_slice_values(
        values,
        shape=resolved_shape,
        plane=plane,
        slice_index=slice_index,
        min_value=min_value,
        max_value=max_value,
    )
    return _slice_result_from_payload(payload)
