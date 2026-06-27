"""MeshLib-style RAW voxel loading."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import VoxelVolume


def _volume_from_payload(payload: dict[str, Any], *, source: str) -> VoxelVolume:
    dimensions = tuple(int(value) for value in payload["dimensions"])
    metadata = {
        "source": source,
        "source_path": str(payload.get("source_path", "")),
    }
    if "default_iso_value" in payload:
        metadata["default_iso_value"] = float(payload["default_iso_value"])
    if "default_iso_value_source" in payload:
        metadata["default_iso_value_source"] = str(payload["default_iso_value_source"])
    if "source_files" in payload:
        metadata["source_files"] = [str(path) for path in payload["source_files"]]
    return VoxelVolume(
        dimensions=dimensions,  # type: ignore[arg-type]
        voxel_size=tuple(float(value) for value in payload["voxel_size"]),  # type: ignore[arg-type]
        grid_level_set=bool(payload["grid_level_set"]),
        scalar_type=str(payload["scalar_type"]),
        values=np.asarray(payload["values"], dtype=np.float32).reshape(dimensions),
        min_value=float(payload["min"]),
        max_value=float(payload["max"]),
        metadata=metadata,
    )


def load_raw_voxels(
    path: str | Path,
    *,
    dimensions: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    scalar_type: str,
    grid_level_set: bool = False,
) -> VoxelVolume:
    payload = _rust_voxel.load_raw_voxels(
        path,
        dimensions=dimensions,
        voxel_size=voxel_size,
        scalar_type=scalar_type,
        grid_level_set=grid_level_set,
    )
    return _volume_from_payload(payload, source="MeshLib VoxelsLoad::fromRaw")


def load_raw_voxels_auto(path: str | Path) -> VoxelVolume:
    return _volume_from_payload(
        _rust_voxel.load_raw_voxels_auto(path),
        source="MeshLib VoxelsLoad::fromRaw",
    )


def load_tiff_voxels_dir(
    directory: str | Path,
    *,
    voxel_size: tuple[float, float, float],
    grid_level_set: bool = False,
) -> VoxelVolume:
    payload = _rust_voxel.load_tiff_voxels_dir(
        directory,
        voxel_size=voxel_size,
        grid_level_set=grid_level_set,
    )
    return _volume_from_payload(payload, source="MeshLib VoxelsLoad::loadTiffDir")


def voxel_default_iso_value(values: np.ndarray) -> float:
    return _rust_voxel.voxel_default_iso_value(values)
