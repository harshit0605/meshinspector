"""Boolean and offset compatibility wrappers for aligned SDF grids."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.voxel.sdf import SDFGrid


def _assert_aligned(a: SDFGrid, b: SDFGrid) -> None:
    if a.origin != b.origin or a.shape != b.shape or abs(a.voxel_size_mm - b.voxel_size_mm) > 1e-9:
        raise ValueError("SDF grids must share origin, shape, and voxel size")


def _grid_like(grid: SDFGrid, values) -> SDFGrid:
    return SDFGrid(
        origin=grid.origin,
        voxel_size_mm=grid.voxel_size_mm,
        shape=grid.shape,
        values=values,
    )


def voxel_binary_values(a: SDFGrid, b: SDFGrid, *, operation: str) -> SDFGrid:
    _assert_aligned(a, b)
    return _grid_like(a, _rust_voxel.voxel_binary_values_required(a.values, b.values, operation=operation))


def voxel_binary_iso_value(
    left_iso_value: float,
    right_iso_value: float,
    *,
    operation: str,
) -> float:
    return _rust_voxel.voxel_binary_iso_value(
        left_iso_value,
        right_iso_value,
        operation=operation,
    )


def voxel_binary_operation(
    a: SDFGrid,
    b: SDFGrid,
    *,
    operation: str,
    left_iso_value: float = 0.0,
    right_iso_value: float = 0.0,
) -> tuple[SDFGrid, float]:
    return (
        voxel_binary_values(a, b, operation=operation),
        voxel_binary_iso_value(left_iso_value, right_iso_value, operation=operation),
    )


def sdf_union(a: SDFGrid, b: SDFGrid) -> SDFGrid:
    _assert_aligned(a, b)
    return _grid_like(a, _rust_voxel.sdf_boolean_values_required(a.values, b.values, operation="union"))


def sdf_intersection(a: SDFGrid, b: SDFGrid) -> SDFGrid:
    _assert_aligned(a, b)
    return _grid_like(a, _rust_voxel.sdf_boolean_values_required(a.values, b.values, operation="intersection"))


def sdf_difference(a: SDFGrid, b: SDFGrid) -> SDFGrid:
    _assert_aligned(a, b)
    return _grid_like(a, _rust_voxel.sdf_boolean_values_required(a.values, b.values, operation="difference"))


def sdf_offset(grid: SDFGrid, offset_mm: float) -> SDFGrid:
    return _grid_like(grid, _rust_voxel.sdf_offset_values(grid.values, offset_mm))


def sdf_shell(grid: SDFGrid, wall_thickness_mm: float) -> SDFGrid:
    return _grid_like(grid, _rust_voxel.sdf_shell_values(grid.values, wall_thickness_mm))
