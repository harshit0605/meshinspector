from __future__ import annotations

import numpy as np
from pathlib import Path
from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_voxel_common import _require_rust_kernel
from geometry_sdk.types import MeshDocument


def voxel_volume_render_data_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    active_min_corner: tuple[int, int, int],
    active_dimensions: tuple[int, int, int],
    source_min_value: float,
    source_max_value: float,
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    rust_active_min_corner = np.asarray(active_min_corner, dtype=np.int64)
    rust_active_dimensions = np.asarray(active_dimensions, dtype=np.int64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if rust_active_min_corner.shape != (3,) or np.any(rust_active_min_corner < 0):
        raise ValueError("active_min_corner must contain three non-negative values")
    if rust_active_dimensions.shape != (3,) or np.any(rust_active_dimensions <= 0):
        raise ValueError("active_dimensions must contain three positive values")
    if np.any(rust_active_min_corner + rust_active_dimensions > rust_shape):
        raise ValueError("active box must fit inside shape")
    if not np.isfinite(source_min_value) or not np.isfinite(source_max_value) or source_max_value <= source_min_value:
        raise ValueError("source value range must be finite and increasing")
    kernel = _require_rust_kernel("voxel_volume_render_data_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_voxel_size,
            rust_active_min_corner,
            rust_active_dimensions,
            float(source_min_value),
            float(source_max_value),
        )
    )

def voxel_volume_render_lut_values(
    *,
    lut_type: str,
    alpha_type: str = "constant",
    alpha_limit: int = 10,
    one_color: tuple[int, int, int, int] | None = None,
) -> dict[str, Any]:
    if not isinstance(alpha_limit, int) or alpha_limit < 0 or alpha_limit > 255:
        raise ValueError("alpha_limit must be an integer between 0 and 255")
    rust_one_color = None
    if one_color is not None:
        if len(one_color) != 4:
            raise ValueError("one_color must contain four RGBA values")
        rust_one_color = np.asarray(one_color, dtype=np.int64)
        if np.any(rust_one_color < 0) or np.any(rust_one_color > 255):
            raise ValueError("one_color values must be between 0 and 255")
    kernel = _require_rust_kernel("voxel_volume_render_lut_values")
    return dict(
        kernel(
            str(lut_type),
            str(alpha_type),
            int(alpha_limit),
            rust_one_color,
        )
    )

def voxel_volume_render_ray_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    min_corner: tuple[int, int, int],
    ray_start: tuple[float, float, float],
    ray_direction: tuple[float, float, float],
    sampling_step: float,
    min_value: float,
    max_value: float,
    lut_type: str,
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
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    rust_min_corner = np.asarray(min_corner, dtype=np.int64)
    rust_ray_start = np.asarray(ray_start, dtype=np.float64)
    rust_ray_direction = np.asarray(ray_direction, dtype=np.float64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if rust_min_corner.shape != (3,) or np.any(rust_min_corner < 0):
        raise ValueError("min_corner must contain three non-negative values")
    if rust_ray_start.shape != (3,) or np.any(~np.isfinite(rust_ray_start)):
        raise ValueError("ray_start must contain three finite values")
    if rust_ray_direction.shape != (3,) or np.any(~np.isfinite(rust_ray_direction)):
        raise ValueError("ray_direction must contain three finite values")
    if not np.isfinite(sampling_step):
        raise ValueError("sampling_step must be finite")
    if not np.isfinite(min_value) or not np.isfinite(max_value) or max_value <= min_value:
        raise ValueError("value range must be finite and increasing")
    if not isinstance(max_steps, int) or max_steps <= 0:
        raise ValueError("max_steps must be a positive integer")
    if not isinstance(alpha_limit, int) or alpha_limit < 0 or alpha_limit > 255:
        raise ValueError("alpha_limit must be an integer between 0 and 255")
    rust_one_color = None
    if one_color is not None:
        if len(one_color) != 4:
            raise ValueError("one_color must contain four RGBA values")
        rust_one_color = np.asarray(one_color, dtype=np.int64)
        if np.any(rust_one_color < 0) or np.any(rust_one_color > 255):
            raise ValueError("one_color values must be between 0 and 255")
    rust_clipping_plane = None
    if clipping_plane is not None:
        if len(clipping_plane) != 4:
            raise ValueError("clipping_plane must contain four plane values")
        rust_clipping_plane = np.asarray(clipping_plane, dtype=np.float64)
        if np.any(~np.isfinite(rust_clipping_plane)):
            raise ValueError("clipping_plane values must be finite")
    rust_light_pos_eye = None
    if light_pos_eye is not None:
        if len(light_pos_eye) != 3:
            raise ValueError("light_pos_eye must contain three values")
        rust_light_pos_eye = np.asarray(light_pos_eye, dtype=np.float64)
        if np.any(~np.isfinite(rust_light_pos_eye)):
            raise ValueError("light_pos_eye values must be finite")
    if not np.isfinite(ambient_strength) or ambient_strength < 0.0:
        raise ValueError("ambient_strength must be a non-negative finite value")
    if not np.isfinite(specular_strength) or specular_strength < 0.0:
        raise ValueError("specular_strength must be a non-negative finite value")
    if not np.isfinite(spec_exp) or spec_exp < 0.0:
        raise ValueError("spec_exp must be a non-negative finite value")
    rust_active_indices = None
    if active_indices is not None:
        rust_active_indices = np.asarray(active_indices, dtype=np.int64)
        if rust_active_indices.ndim != 1 or np.any(rust_active_indices < 0):
            raise ValueError("active_indices must be a 1D collection of non-negative indices")
    kernel = _require_rust_kernel("voxel_volume_render_ray_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_voxel_size,
            rust_min_corner,
            rust_ray_start,
            rust_ray_direction,
            float(sampling_step),
            float(min_value),
            float(max_value),
            str(lut_type),
            str(alpha_type),
            int(alpha_limit),
            rust_one_color,
            rust_clipping_plane,
            str(shading_mode),
            rust_light_pos_eye,
            float(ambient_strength),
            float(specular_strength),
            float(spec_exp),
            rust_active_indices,
            int(max_steps),
        )
    )
