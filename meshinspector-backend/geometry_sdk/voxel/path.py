"""MeshLib-style voxel path tools."""

from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import VoxelPathResult, VoxelVolume


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


def _path_result_from_payload(payload: dict[str, Any]) -> VoxelPathResult:
    return VoxelPathResult(
        voxel_indices=[int(value) for value in payload["voxel_indices"]],
        coordinates=[tuple(int(axis) for axis in coord) for coord in payload["coordinates"]],  # type: ignore[list-item]
        total_metric=float(payload["total_metric"]),
    )


def voxel_path(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    start: tuple[int, int, int],
    finish: tuple[int, int, int],
    metric: str = "difference",
    max_dist_ratio: float = 1.5,
    plane: str = "none",
    quarters_mask: int = 0x0F,
    exponent_modifier: float = -1.0,
) -> VoxelPathResult:
    values, resolved_shape = _values_and_shape(volume_or_values, shape)
    payload = _rust_voxel.voxel_path_values(
        values,
        shape=resolved_shape,
        start=start,
        finish=finish,
        metric=metric,
        max_dist_ratio=max_dist_ratio,
        plane=plane,
        quarters_mask=quarters_mask,
        exponent_modifier=exponent_modifier,
    )
    return _path_result_from_payload(payload)


def _path_entry_from_payload(payload: dict[str, Any]) -> dict[str, Any]:
    return {
        "quarters_mask": int(payload["quarters_mask"]),
        "path": _path_result_from_payload(dict(payload["path"])),
    }


def voxel_path_build_four(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    start: tuple[int, int, int],
    finish: tuple[int, int, int],
    metric: str = "exponent",
    max_dist_ratio: float = 1.5,
    plane: str = "none",
    exponent_modifier: float = -1.0,
) -> list[dict[str, Any]]:
    values, resolved_shape = _values_and_shape(volume_or_values, shape)
    payload = _rust_voxel.voxel_path_build_four_values(
        values,
        shape=resolved_shape,
        start=start,
        finish=finish,
        metric=metric,
        max_dist_ratio=max_dist_ratio,
        plane=plane,
        exponent_modifier=exponent_modifier,
    )
    return [_path_entry_from_payload(entry) for entry in payload]
