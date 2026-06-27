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


def object_lines_from_contours(
    contour_points: Any,
    contour_offsets: Any,
    *,
    line_width: float,
    show_points: int,
    smooth_connections: int,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("object_lines_from_contours")
    return kernel(
        np.asarray(contour_points, dtype=np.float64).reshape(-1, 3),
        np.asarray(contour_offsets, dtype=np.int64).reshape(-1),
        float(line_width),
        int(show_points),
        int(smooth_connections),
    )


def object_lines_to_contours(
    points: Any,
    lines: Any,
    *,
    line_width: float,
    show_points: int,
    smooth_connections: int,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("object_lines_to_contours")
    return kernel(
        np.asarray(points, dtype=np.float64).reshape(-1, 3),
        np.asarray(lines, dtype=np.int64).reshape(-1, 2),
        float(line_width),
        int(show_points),
        int(smooth_connections),
    )


def offset_contours(
    contour_points: Any,
    contour_offsets: Any,
    *,
    offset: float,
    min_angle_precision: float,
    mode: str = "offset",
    end_type: str = "round",
    corner_type: str = "round",
    max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
    z_restore: str = "default",
    z_value: float | None = None,
    relax_iterations: int = 1,
    z_values: Any | None = None,
    z_value_offsets: Any | None = None,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("offset_contours")
    z_callback = z_values if callable(z_values) else None
    z_value_array = None if z_callback is not None else z_values
    return kernel(
        np.asarray(contour_points, dtype=np.float64).reshape(-1, 3),
        np.asarray(contour_offsets, dtype=np.int64).reshape(-1),
        float(offset),
        float(min_angle_precision),
        str(mode),
        str(end_type),
        str(corner_type),
        float(max_sharp_angle),
        str(z_restore),
        None if z_value is None else float(z_value),
        int(relax_iterations),
        None if z_value_array is None else np.asarray(z_value_array, dtype=np.float64).reshape(-1),
        None
        if z_value_offsets is None
        else np.asarray(z_value_offsets, dtype=np.int64).reshape(-1),
        z_callback,
    )


def offset_contours_with_origins(
    contour_points: Any,
    contour_offsets: Any,
    *,
    offset: float,
    min_angle_precision: float,
    mode: str = "offset",
    end_type: str = "round",
    corner_type: str = "round",
    max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
    z_restore: str = "default",
    z_value: float | None = None,
    relax_iterations: int = 1,
    z_values: Any | None = None,
    z_value_offsets: Any | None = None,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("offset_contours_with_origins")
    z_callback = z_values if callable(z_values) else None
    z_value_array = None if z_callback is not None else z_values
    return kernel(
        np.asarray(contour_points, dtype=np.float64).reshape(-1, 3),
        np.asarray(contour_offsets, dtype=np.int64).reshape(-1),
        float(offset),
        float(min_angle_precision),
        str(mode),
        str(end_type),
        str(corner_type),
        float(max_sharp_angle),
        str(z_restore),
        None if z_value is None else float(z_value),
        int(relax_iterations),
        None if z_value_array is None else np.asarray(z_value_array, dtype=np.float64).reshape(-1),
        None
        if z_value_offsets is None
        else np.asarray(z_value_offsets, dtype=np.int64).reshape(-1),
        z_callback,
    )


def offset_contours_variable(
    contour_points: Any,
    contour_offsets: Any,
    point_offsets: Any,
    offset_offsets: Any,
    *,
    min_angle_precision: float,
    mode: str = "offset",
    end_type: str = "round",
    corner_type: str = "round",
    max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
    z_restore: str = "default",
    z_value: float | None = None,
    relax_iterations: int = 1,
    z_values: Any | None = None,
    z_value_offsets: Any | None = None,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("offset_contours_variable")
    z_callback = z_values if callable(z_values) else None
    z_value_array = None if z_callback is not None else z_values
    return kernel(
        np.asarray(contour_points, dtype=np.float64).reshape(-1, 3),
        np.asarray(contour_offsets, dtype=np.int64).reshape(-1),
        np.asarray(point_offsets, dtype=np.float64).reshape(-1),
        np.asarray(offset_offsets, dtype=np.int64).reshape(-1),
        float(min_angle_precision),
        str(mode),
        str(end_type),
        str(corner_type),
        float(max_sharp_angle),
        str(z_restore),
        None if z_value is None else float(z_value),
        int(relax_iterations),
        None if z_value_array is None else np.asarray(z_value_array, dtype=np.float64).reshape(-1),
        None
        if z_value_offsets is None
        else np.asarray(z_value_offsets, dtype=np.int64).reshape(-1),
        z_callback,
    )


def offset_contours_variable_with_origins(
    contour_points: Any,
    contour_offsets: Any,
    point_offsets: Any,
    offset_offsets: Any,
    *,
    min_angle_precision: float,
    mode: str = "offset",
    end_type: str = "round",
    corner_type: str = "round",
    max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
    z_restore: str = "default",
    z_value: float | None = None,
    relax_iterations: int = 1,
    z_values: Any | None = None,
    z_value_offsets: Any | None = None,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("offset_contours_variable_with_origins")
    z_callback = z_values if callable(z_values) else None
    z_value_array = None if z_callback is not None else z_values
    return kernel(
        np.asarray(contour_points, dtype=np.float64).reshape(-1, 3),
        np.asarray(contour_offsets, dtype=np.int64).reshape(-1),
        np.asarray(point_offsets, dtype=np.float64).reshape(-1),
        np.asarray(offset_offsets, dtype=np.int64).reshape(-1),
        float(min_angle_precision),
        str(mode),
        str(end_type),
        str(corner_type),
        float(max_sharp_angle),
        str(z_restore),
        None if z_value is None else float(z_value),
        int(relax_iterations),
        None if z_value_array is None else np.asarray(z_value_array, dtype=np.float64).reshape(-1),
        None
        if z_value_offsets is None
        else np.asarray(z_value_offsets, dtype=np.int64).reshape(-1),
        z_callback,
    )


def object_lines_to_pts(
    points: Any,
    lines: Any,
    *,
    line_width: float,
    show_points: int,
    smooth_connections: int,
) -> str:
    kernel = _require_rust_kernel("object_lines_to_pts")
    return str(
        kernel(
            np.asarray(points, dtype=np.float64).reshape(-1, 3),
            np.asarray(lines, dtype=np.int64).reshape(-1, 2),
            float(line_width),
            int(show_points),
            int(smooth_connections),
        )
    )


def object_lines_from_pts(source: str) -> dict[str, Any]:
    kernel = _require_rust_kernel("object_lines_from_pts")
    return kernel(str(source))


def object_lines_to_dxf(
    points: Any,
    lines: Any,
    *,
    line_width: float,
    show_points: int,
    smooth_connections: int,
) -> str:
    kernel = _require_rust_kernel("object_lines_to_dxf")
    return str(
        kernel(
            np.asarray(points, dtype=np.float64).reshape(-1, 3),
            np.asarray(lines, dtype=np.int64).reshape(-1, 2),
            float(line_width),
            int(show_points),
            int(smooth_connections),
        )
    )


def object_lines_to_mrlines(
    points: Any,
    lines: Any,
    *,
    line_width: float,
    show_points: int,
    smooth_connections: int,
) -> bytes:
    kernel = _require_rust_kernel("object_lines_to_mrlines")
    return bytes(
        kernel(
            np.asarray(points, dtype=np.float64).reshape(-1, 3),
            np.asarray(lines, dtype=np.int64).reshape(-1, 2),
            float(line_width),
            int(show_points),
            int(smooth_connections),
        )
    )


def object_lines_from_mrlines(source: bytes) -> dict[str, Any]:
    kernel = _require_rust_kernel("object_lines_from_mrlines")
    return kernel(bytes(source))


def object_lines_to_ply(
    points: Any,
    lines: Any,
    *,
    line_width: float,
    show_points: int,
    smooth_connections: int,
    vert_colors: Any | None = None,
) -> bytes:
    kernel = _require_rust_kernel("object_lines_to_ply")
    return bytes(
        kernel(
            np.asarray(points, dtype=np.float64).reshape(-1, 3),
            np.asarray(lines, dtype=np.int64).reshape(-1, 2),
            float(line_width),
            int(show_points),
            int(smooth_connections),
            np.asarray([] if vert_colors is None else vert_colors, dtype=np.uint8).reshape(-1, 4),
        )
    )


def object_lines_from_ply(source: bytes) -> dict[str, Any]:
    kernel = _require_rust_kernel("object_lines_from_ply")
    return kernel(bytes(source))


def object_lines_from_svg(source: str) -> dict[str, Any]:
    kernel = _require_rust_kernel("object_lines_from_svg")
    return kernel(str(source))
