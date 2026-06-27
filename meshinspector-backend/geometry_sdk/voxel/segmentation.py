"""MeshInspector-style voxel segmentation by seeded graph cut."""

from __future__ import annotations

from typing import Any, Sequence

import numpy as np

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import MeshDocument, VoxelSegmentationResult, VoxelVolume


VoxelSeed = tuple[int, int, int]


def _values_and_shape(
    volume_or_values: VoxelVolume | np.ndarray,
    shape: tuple[int, int, int] | None,
) -> tuple[np.ndarray, tuple[int, int, int]]:
    if isinstance(volume_or_values, VoxelVolume):
        return volume_or_values.values.reshape(-1), volume_or_values.dimensions
    values = np.asarray(volume_or_values, dtype=np.float32)
    if shape is None:
        if values.ndim != 3:
            raise ValueError("shape is required when values are not a 3D array")
        shape = tuple(int(value) for value in values.shape)  # type: ignore[assignment]
    if values.ndim == 3 and tuple(int(value) for value in values.shape) == tuple(shape):
        return np.ravel(values, order="F"), shape
    return values.reshape(-1), shape


def _seed_array(name: str, seeds: Sequence[VoxelSeed]) -> np.ndarray:
    values = np.asarray(seeds, dtype=np.int64)
    if values.size == 0:
        return np.empty((0, 3), dtype=np.int64)
    if values.ndim != 2 or values.shape[1] != 3:
        raise ValueError(f"{name} must contain (x, y, z) seed coordinates")
    if np.any(values < 0):
        raise ValueError(f"{name} values must be non-negative")
    return np.ascontiguousarray(values, dtype=np.int64)


def _segmentation_result_from_payload(payload: dict[str, Any]) -> VoxelSegmentationResult:
    return VoxelSegmentationResult(
        min_corner=tuple(int(axis) for axis in payload["min_corner"]),  # type: ignore[arg-type]
        dimensions=tuple(int(axis) for axis in payload["dimensions"]),  # type: ignore[arg-type]
        source_indices=[int(index) for index in payload["source_indices"]],
        part_indices=[int(index) for index in payload["part_indices"]],
        selected_coordinates=[
            tuple(int(axis) for axis in coord) for coord in payload["selected_coordinates"]
        ],  # type: ignore[list-item]
        selected_values=np.asarray(payload["selected_values"], dtype=np.float32),
    )


def voxel_segmentation(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    inside_seeds: Sequence[VoxelSeed],
    outside_seeds: Sequence[VoxelSeed] = (),
    exponent_modifier: float = 3000.0,
    voxels_expansion: int = 25,
    include_boundary_outside: bool = True,
) -> VoxelSegmentationResult:
    values, resolved_shape = _values_and_shape(volume_or_values, shape)
    payload = _rust_voxel.voxel_segmentation_values(
        values,
        shape=resolved_shape,
        inside_seeds=_seed_array("inside_seeds", inside_seeds),
        outside_seeds=_seed_array("outside_seeds", outside_seeds),
        exponent_modifier=exponent_modifier,
        voxels_expansion=voxels_expansion,
        include_boundary_outside=include_boundary_outside,
    )
    return _segmentation_result_from_payload(payload)


def voxel_segmentation_mesh(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    inside_seeds: Sequence[VoxelSeed],
    outside_seeds: Sequence[VoxelSeed] = (),
    exponent_modifier: float = 3000.0,
    voxels_expansion: int = 25,
    include_boundary_outside: bool = True,
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0),
) -> MeshDocument:
    values, resolved_shape = _values_and_shape(volume_or_values, shape)
    payload = _rust_voxel.voxel_segmentation_mesh_values(
        values,
        shape=resolved_shape,
        inside_seeds=_seed_array("inside_seeds", inside_seeds),
        outside_seeds=_seed_array("outside_seeds", outside_seeds),
        voxel_size=voxel_size,
        exponent_modifier=exponent_modifier,
        voxels_expansion=voxels_expansion,
        include_boundary_outside=include_boundary_outside,
    )
    segmentation = _segmentation_result_from_payload(dict(payload["segmentation"]))
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={
            "source": "voxel_segmentation_mesh",
            "voxel_size": tuple(float(value) for value in voxel_size),
            "segmentation": {
                "min_corner": segmentation.min_corner,
                "dimensions": segmentation.dimensions,
                "source_indices": segmentation.source_indices,
                "part_indices": segmentation.part_indices,
                "selected_coordinates": segmentation.selected_coordinates,
            },
        },
    )


def voxel_mask_to_mesh(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    mask_coordinates: Sequence[VoxelSeed],
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0),
    mask_expansion: int = 25,
    smooth_band_radius: int = 3,
) -> MeshDocument:
    values, resolved_shape = _values_and_shape(volume_or_values, shape)
    payload = _rust_voxel.voxel_mask_to_mesh_values(
        values,
        shape=resolved_shape,
        mask_coordinates=_seed_array("mask_coordinates", mask_coordinates),
        voxel_size=voxel_size,
        mask_expansion=mask_expansion,
        smooth_band_radius=smooth_band_radius,
    )
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={
            "source": "voxel_mask_to_mesh",
            "voxel_size": tuple(float(value) for value in voxel_size),
            "mask": {
                "min_corner": tuple(int(axis) for axis in payload["min_corner"]),
                "dimensions": tuple(int(axis) for axis in payload["dimensions"]),
                "source_indices": [int(index) for index in payload["source_indices"]],
                "part_indices": [int(index) for index in payload["part_indices"]],
                "selected_coordinates": [
                    tuple(int(axis) for axis in coord)
                    for coord in payload["selected_coordinates"]
                ],
            },
        },
    )
