"""MeshInspector-style CT voxel line graph sampling."""

from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import VoxelLineGraphResult, VoxelVolume


def _values_and_shape(
    volume_or_values: VoxelVolume | np.ndarray,
    shape: tuple[int, int, int] | None,
) -> tuple[np.ndarray, tuple[int, int, int]]:
    if isinstance(volume_or_values, VoxelVolume):
        # x-fastest (Fortran), matching this op's flat indexing; bare reshape(-1) is
        # C-order and transposed VoxelVolume input.
        return np.ravel(volume_or_values.values, order="F"), volume_or_values.dimensions
    values = np.asarray(volume_or_values, dtype=np.float32)
    if shape is None:
        if values.ndim != 3:
            raise ValueError("shape is required when values are not a 3D array")
        shape = tuple(int(value) for value in values.shape)  # type: ignore[assignment]
    if values.ndim == 3 and tuple(int(value) for value in values.shape) == tuple(shape):
        return np.ravel(values, order="F"), shape
    return values.reshape(-1), shape


def _line_graph_result_from_payload(payload: dict[str, Any]) -> VoxelLineGraphResult:
    return VoxelLineGraphResult(
        axis=int(payload["axis"]),
        positions=[int(position) for position in payload["positions"]],
        voxel_indices=[int(voxel_index) for voxel_index in payload["voxel_indices"]],
        coordinates=[tuple(int(axis) for axis in coord) for coord in payload["coordinates"]],  # type: ignore[list-item]
        values=np.asarray(payload["values"], dtype=np.float32),
    )


def voxel_line_graph(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    axis: str,
    fixed_coordinate: tuple[int, int, int],
) -> VoxelLineGraphResult:
    values, resolved_shape = _values_and_shape(volume_or_values, shape)
    payload = _rust_voxel.voxel_line_graph_values(
        values,
        shape=resolved_shape,
        axis=axis,
        fixed_coordinate=fixed_coordinate,
    )
    return _line_graph_result_from_payload(payload)
