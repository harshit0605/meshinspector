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


def pairwise_point_to_point_icp(
    floating_points: np.ndarray,
    reference_points: np.ndarray,
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("pairwise_point_to_point_icp")
    return kernel(
        np.asarray(floating_points, dtype=np.float64),
        np.asarray(reference_points, dtype=np.float64),
        int(max_iterations),
        float(tolerance),
        str(mode),
    )


def pairwise_point_to_plane_icp(
    floating_points: np.ndarray,
    reference_points: np.ndarray,
    reference_normals: np.ndarray,
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    floating_normals: np.ndarray | None = None,
    max_pair_distance: float | None = None,
    cos_threshold: float | None = None,
    far_dist_factor: float | None = None,
    mutual_closest: bool = False,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("pairwise_point_to_plane_icp")
    return kernel(
        np.asarray(floating_points, dtype=np.float64),
        np.asarray(reference_points, dtype=np.float64),
        np.asarray(reference_normals, dtype=np.float64),
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if floating_normals is None else np.asarray(floating_normals, dtype=np.float64),
        None if max_pair_distance is None else float(max_pair_distance),
        None if cos_threshold is None else float(cos_threshold),
        None if far_dist_factor is None else float(far_dist_factor),
        bool(mutual_closest),
    )


def _flatten_multiway_objects(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
) -> tuple[np.ndarray, np.ndarray]:
    clouds = [np.asarray(points, dtype=np.float64) for points in objects]
    counts = np.asarray([cloud.shape[0] for cloud in clouds], dtype=np.int64)
    flat_points = np.vstack(clouds) if clouds else np.empty((0, 3), dtype=np.float64)
    return flat_points, counts


def _flatten_multiway_normals(normals: tuple[np.ndarray, ...] | list[np.ndarray]) -> np.ndarray:
    normal_rows = [np.asarray(values, dtype=np.float64) for values in normals]
    return np.vstack(normal_rows) if normal_rows else np.empty((0, 3), dtype=np.float64)


def multiway_point_to_point_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_point_to_point_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    return kernel(
        flat_points,
        counts,
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_point_to_plane_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_point_to_plane_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_combined_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_combined_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_all_object_point_to_point_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_all_object_point_to_point_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    return kernel(
        flat_points,
        counts,
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_all_object_point_to_plane_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_all_object_point_to_plane_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_all_object_combined_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_all_object_combined_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_sequential_cascade_point_to_point_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_sequential_cascade_point_to_point_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    return kernel(
        flat_points,
        counts,
        int(max_group_size),
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_sequential_cascade_point_to_plane_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_sequential_cascade_point_to_plane_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_group_size),
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_sequential_cascade_combined_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_sequential_cascade_combined_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_group_size),
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_aabb_cascade_point_to_point_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_aabb_cascade_point_to_point_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    return kernel(
        flat_points,
        counts,
        int(max_group_size),
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_aabb_cascade_point_to_plane_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_aabb_cascade_point_to_plane_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_group_size),
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )


def multiway_aabb_cascade_combined_icp(
    objects: tuple[np.ndarray, ...] | list[np.ndarray],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: str = "rigid",
    fixed_object_index: int | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("multiway_aabb_cascade_combined_icp")
    flat_points, counts = _flatten_multiway_objects(objects)
    flat_normals = _flatten_multiway_normals(normals)
    return kernel(
        flat_points,
        flat_normals,
        counts,
        int(max_group_size),
        int(max_iterations),
        float(tolerance),
        str(mode),
        None if fixed_object_index is None else int(fixed_object_index),
    )
