"""MeshLib-style voxel volume rendering data preparation."""

from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import (
    VoxelVolume,
    VoxelVolumeRenderDataResult,
    VoxelVolumeRenderLutResult,
    VoxelVolumeRenderRayResult,
)


def _values_shape_voxel_size_scale(
    volume_or_values: VoxelVolume | np.ndarray,
    shape: tuple[int, int, int] | None,
    voxel_size: tuple[float, float, float] | None,
    source_min_value: float | None,
    source_max_value: float | None,
) -> tuple[np.ndarray, tuple[int, int, int], tuple[float, float, float], float, float]:
    if isinstance(volume_or_values, VoxelVolume):
        return (
            np.ravel(volume_or_values.values, order="F"),
            volume_or_values.dimensions,
            volume_or_values.voxel_size if voxel_size is None else voxel_size,
            float(volume_or_values.min_value if source_min_value is None else source_min_value),
            float(volume_or_values.max_value if source_max_value is None else source_max_value),
        )

    values = np.asarray(volume_or_values, dtype=np.float32)
    if shape is None:
        if values.ndim != 3:
            raise ValueError("shape is required when values are not a 3D array")
        shape = tuple(int(value) for value in values.shape)  # type: ignore[assignment]
    resolved_voxel_size = (1.0, 1.0, 1.0) if voxel_size is None else voxel_size
    flat_values = np.ravel(values, order="F") if values.ndim == 3 and tuple(values.shape) == tuple(shape) else values.reshape(-1)
    rust_min_value, rust_max_value = _rust_voxel.voxel_value_range(flat_values)
    resolved_min = float(rust_min_value if source_min_value is None else source_min_value)
    resolved_max = float(rust_max_value if source_max_value is None else source_max_value)
    return flat_values, shape, resolved_voxel_size, resolved_min, resolved_max


def _render_data_result_from_payload(payload: dict[str, Any]) -> VoxelVolumeRenderDataResult:
    return VoxelVolumeRenderDataResult(
        dimensions=tuple(int(value) for value in payload["dimensions"]),  # type: ignore[arg-type]
        voxel_size=tuple(float(value) for value in payload["voxel_size"]),  # type: ignore[arg-type]
        source_indices=[int(index) for index in payload["source_indices"]],
        coordinates=[tuple(int(axis) for axis in coord) for coord in payload["coordinates"]],  # type: ignore[list-item]
        values=np.asarray(payload["values"], dtype=np.float32),
        min_value=float(payload["min_value"]),
        max_value=float(payload["max_value"]),
        metadata={
            "source": "voxel_volume_render_data",
            "meshlib_reference": "ObjectVoxels::prepareDataForVolumeRendering",
            "meshlib_conversion": "vdbVolumeToSimpleVolumeNorm",
            "normalization": "source_scale_to_0_1_clamped",
        },
    )


def _render_lut_result_from_payload(payload: dict[str, Any]) -> VoxelVolumeRenderLutResult:
    return VoxelVolumeRenderLutResult(
        lut_type=str(payload["lut_type"]),
        alpha_type=str(payload["alpha_type"]),
        alpha_limit=int(payload["alpha_limit"]),
        colors_rgba=[tuple(int(channel) for channel in color) for color in payload["colors_rgba"]],  # type: ignore[list-item]
        metadata={
            "source": "voxel_volume_render_lut",
            "meshlib_reference": payload["meshlib_reference"],
            "meshlib_params": "ObjectVoxels::VolumeRenderingParams",
            "meshlib_lut_type": "VolumeRenderingParams::LutType",
            "meshlib_alpha_type": "VolumeRenderingParams::AlphaType",
        },
    )


def _render_ray_result_from_payload(payload: dict[str, Any]) -> VoxelVolumeRenderRayResult:
    first_opaque = payload["first_opaque_world"]
    return VoxelVolumeRenderRayResult(
        color_rgba=np.asarray(payload["color_rgba"], dtype=np.float32),
        first_opaque_world=None
        if first_opaque is None
        else tuple(float(coord) for coord in first_opaque),  # type: ignore[arg-type]
        visited_indices=[int(index) for index in payload["visited_indices"]],
        accepted_indices=[int(index) for index in payload["accepted_indices"]],
        metadata={
            "source": "voxel_volume_render_ray",
            "meshlib_reference": payload["meshlib_reference"],
            "meshlib_shader": "MRVolumeShader",
        },
    )


def voxel_volume_render_lut(
    *,
    lut_type: str = "rainbow",
    alpha_type: str = "constant",
    alpha_limit: int = 10,
    one_color: tuple[int, int, int, int] | None = None,
) -> VoxelVolumeRenderLutResult:
    payload = _rust_voxel.voxel_volume_render_lut_values(
        lut_type=lut_type,
        alpha_type=alpha_type,
        alpha_limit=alpha_limit,
        one_color=one_color,
    )
    return _render_lut_result_from_payload(payload)


def voxel_volume_render_ray(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    voxel_size: tuple[float, float, float] | None = None,
    min_corner: tuple[int, int, int] = (0, 0, 0),
    ray_start: tuple[float, float, float],
    ray_direction: tuple[float, float, float],
    sampling_step: float,
    min_value: float = 0.0,
    max_value: float = 1.0,
    lut_type: str = "rainbow",
    alpha_type: str = "constant",
    alpha_limit: int = 10,
    one_color: tuple[int, int, int, int] | None = None,
    clipping_plane: tuple[float, float, float, float] | None = None,
    shading_mode: str = "none",
    light_pos_eye: tuple[float, float, float] | None = None,
    ambient_strength: float = 0.1,
    specular_strength: float = 0.5,
    spec_exp: float = 35.0,
    active_indices: tuple[int, ...] | None = None,
    max_steps: int = 4096,
) -> VoxelVolumeRenderRayResult:
    values, resolved_shape, resolved_voxel_size, _, _ = _values_shape_voxel_size_scale(
        volume_or_values,
        shape,
        voxel_size,
        None,
        None,
    )
    payload = _rust_voxel.voxel_volume_render_ray_values(
        values,
        shape=resolved_shape,
        voxel_size=resolved_voxel_size,
        min_corner=min_corner,
        ray_start=ray_start,
        ray_direction=ray_direction,
        sampling_step=sampling_step,
        min_value=min_value,
        max_value=max_value,
        lut_type=lut_type,
        alpha_type=alpha_type,
        alpha_limit=alpha_limit,
        one_color=one_color,
        clipping_plane=clipping_plane,
        shading_mode=shading_mode,
        light_pos_eye=light_pos_eye,
        ambient_strength=ambient_strength,
        specular_strength=specular_strength,
        spec_exp=spec_exp,
        active_indices=active_indices,
        max_steps=max_steps,
    )
    result = _render_ray_result_from_payload(payload)
    meshlib_branch = (
        "samplingStep > 0 fixed-step ray compositing"
        if sampling_step > 0.0
        else "step <= 0 voxel-boundary rayVoxelIntersection traversal"
    )
    result.metadata.update(
        {
            "min_corner": tuple(int(value) for value in min_corner),
            "ray_start": tuple(float(value) for value in ray_start),
            "ray_direction": tuple(float(value) for value in ray_direction),
            "sampling_step": float(sampling_step),
            "value_range": (float(min_value), float(max_value)),
            "meshlib_branch": meshlib_branch,
            "clipping_plane": None if clipping_plane is None else tuple(float(value) for value in clipping_plane),
            "shading": str(shading_mode),
            "shading_mode": str(shading_mode),
            "lighting": {
                "meshlib_shader": "shadeColor" if light_pos_eye is not None else None,
                "light_pos_eye": None if light_pos_eye is None else tuple(float(value) for value in light_pos_eye),
                "ambient_strength": float(ambient_strength),
                "specular_strength": float(specular_strength),
                "spec_exp": float(spec_exp),
            },
        }
    )
    return result


def voxel_volume_render_data(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    voxel_size: tuple[float, float, float] | None = None,
    active_min_corner: tuple[int, int, int] = (0, 0, 0),
    active_dimensions: tuple[int, int, int] | None = None,
    source_min_value: float | None = None,
    source_max_value: float | None = None,
) -> VoxelVolumeRenderDataResult:
    values, resolved_shape, resolved_voxel_size, resolved_min, resolved_max = _values_shape_voxel_size_scale(
        volume_or_values,
        shape,
        voxel_size,
        source_min_value,
        source_max_value,
    )
    resolved_active_dimensions = resolved_shape if active_dimensions is None else active_dimensions
    payload = _rust_voxel.voxel_volume_render_data_values(
        values,
        shape=resolved_shape,
        voxel_size=resolved_voxel_size,
        active_min_corner=active_min_corner,
        active_dimensions=resolved_active_dimensions,
        source_min_value=resolved_min,
        source_max_value=resolved_max,
    )
    result = _render_data_result_from_payload(payload)
    result.metadata.update(
        {
            "active_min_corner": tuple(int(value) for value in active_min_corner),
            "active_dimensions": tuple(int(value) for value in resolved_active_dimensions),
            "source_scale": (resolved_min, resolved_max),
        }
    )
    return result
