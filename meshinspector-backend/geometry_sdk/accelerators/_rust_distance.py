from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def nearest_distances(vertices: Any, target_indices: Any) -> np.ndarray:
    kernel = _require_rust_kernel("nearest_distances_to_indices")
    distances = kernel(
        np.asarray(vertices, dtype=np.float64),
        np.asarray(target_indices, dtype=np.int64).reshape(-1),
    )
    return np.asarray(distances, dtype=np.float64).reshape(-1)


def distance_map_from_contours(
    contour_points: Any,
    contour_offsets: Any,
    *,
    width: int,
    height: int,
    origin: tuple[float, float],
    pixel_size: tuple[float, float],
    signed: bool,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("distance_map_from_contours")
    return kernel(
        np.asarray(contour_points, dtype=np.float64).reshape(-1, 2),
        np.asarray(contour_offsets, dtype=np.int64).reshape(-1),
        int(width),
        int(height),
        (float(origin[0]), float(origin[1])),
        (float(pixel_size[0]), float(pixel_size[1])),
        bool(signed),
    )


def distance_map_from_mesh(
    vertices: Any,
    faces: Any,
    *,
    width: int,
    height: int,
    origin: tuple[float, float, float],
    x_range: tuple[float, float, float],
    y_range: tuple[float, float, float],
    direction: tuple[float, float, float],
    epsilon: float,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("distance_map_from_mesh")
    return kernel(
        np.asarray(vertices, dtype=np.float64).reshape(-1, 3),
        np.asarray(faces, dtype=np.int64).reshape(-1, 3),
        int(width),
        int(height),
        np.asarray(origin, dtype=np.float64).reshape(3),
        np.asarray(x_range, dtype=np.float64).reshape(3),
        np.asarray(y_range, dtype=np.float64).reshape(3),
        np.asarray(direction, dtype=np.float64).reshape(3),
        float(epsilon),
    )


def distance_map_from_tiff(path: str) -> dict[str, Any]:
    kernel = _require_rust_kernel("distance_map_from_tiff")
    return kernel(str(path))


def distance_map_to_tiff(
    values: Any,
    *,
    origin: tuple[float, float],
    pixel_size: tuple[float, float],
    model_transform: tuple[float, ...] | None,
    path: str,
) -> None:
    kernel = _require_rust_kernel("distance_map_to_tiff")
    transform_values = (
        None
        if model_transform is None
        else np.asarray(model_transform, dtype=np.float64).reshape(-1).tolist()
    )
    kernel(
        np.asarray(values, dtype=np.float32),
        (float(origin[0]), float(origin[1])),
        (float(pixel_size[0]), float(pixel_size[1])),
        transform_values,
        str(path),
    )


def distance_map_to_iso_segments(
    values: Any,
    *,
    origin: tuple[float, float],
    pixel_size: tuple[float, float],
    iso_value: float,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("distance_map_to_iso_segments")
    return kernel(
        np.asarray(values, dtype=np.float32),
        (float(origin[0]), float(origin[1])),
        (float(pixel_size[0]), float(pixel_size[1])),
        float(iso_value),
    )


def distance_map_merge(
    left_values: Any,
    right_values: Any,
    *,
    left_origin: tuple[float, float],
    left_pixel_size: tuple[float, float],
    right_origin: tuple[float, float],
    right_pixel_size: tuple[float, float],
    mode: str,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("distance_map_merge")
    return kernel(
        np.asarray(left_values, dtype=np.float32),
        (float(left_origin[0]), float(left_origin[1])),
        (float(left_pixel_size[0]), float(left_pixel_size[1])),
        np.asarray(right_values, dtype=np.float32),
        (float(right_origin[0]), float(right_origin[1])),
        (float(right_pixel_size[0]), float(right_pixel_size[1])),
        str(mode),
    )


def distance_map_contour_boolean(
    contour_points_a: Any,
    contour_offsets_a: Any,
    contour_points_b: Any,
    contour_offsets_b: Any,
    *,
    mode: str,
    width: int,
    height: int,
    origin: tuple[float, float],
    pixel_size: tuple[float, float],
    iso_value: float,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("distance_map_contour_boolean")
    return kernel(
        np.asarray(contour_points_a, dtype=np.float64).reshape(-1, 2),
        np.asarray(contour_offsets_a, dtype=np.int64).reshape(-1),
        np.asarray(contour_points_b, dtype=np.float64).reshape(-1, 2),
        np.asarray(contour_offsets_b, dtype=np.int64).reshape(-1),
        str(mode),
        int(width),
        int(height),
        (float(origin[0]), float(origin[1])),
        (float(pixel_size[0]), float(pixel_size[1])),
        float(iso_value),
    )
