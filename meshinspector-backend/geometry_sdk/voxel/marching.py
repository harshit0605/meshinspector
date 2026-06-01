"""Marching-tetrahedra compatibility wrappers for SDF grids."""

from __future__ import annotations

from typing import Literal

from geometry_sdk.accelerators import _rust_marching
from geometry_sdk.types import MeshDocument
from geometry_sdk.voxel.sdf import SDFGrid


BooleanOperation = Literal["union", "intersection", "difference"]


def extract_marching_tetrahedra(grid: SDFGrid, *, iso_value: float = 0.0) -> MeshDocument:
    return _rust_marching.extract_marching_tetrahedra(
        grid.values,
        origin=grid.origin,
        shape=grid.shape,
        voxel_size_mm=grid.voxel_size_mm,
        iso_value=iso_value,
    )


def extract_boolean_marching_tetrahedra(
    a: SDFGrid,
    b: SDFGrid,
    *,
    operation: BooleanOperation,
    iso_value: float = 0.0,
) -> MeshDocument:
    _assert_aligned_grids(a, b)
    return _rust_marching.extract_boolean_marching_tetrahedra(
        a.values,
        b.values,
        operation=operation,
        origin=a.origin,
        shape=a.shape,
        voxel_size_mm=a.voxel_size_mm,
        iso_value=iso_value,
    )


def extract_offset_marching_tetrahedra(
    grid: SDFGrid,
    *,
    offset_mm: float,
    iso_value: float = 0.0,
) -> MeshDocument:
    return _rust_marching.extract_offset_marching_tetrahedra(
        grid.values,
        origin=grid.origin,
        shape=grid.shape,
        voxel_size_mm=grid.voxel_size_mm,
        offset_mm=offset_mm,
        iso_value=iso_value,
    )


def extract_shell_marching_tetrahedra(
    grid: SDFGrid,
    *,
    wall_thickness_mm: float,
    iso_value: float = 0.0,
) -> MeshDocument:
    return _rust_marching.extract_shell_marching_tetrahedra(
        grid.values,
        origin=grid.origin,
        shape=grid.shape,
        voxel_size_mm=grid.voxel_size_mm,
        wall_thickness_mm=wall_thickness_mm,
        iso_value=iso_value,
    )


def _assert_aligned_grids(a: SDFGrid, b: SDFGrid) -> None:
    if a.origin != b.origin or a.shape != b.shape or abs(a.voxel_size_mm - b.voxel_size_mm) > 1e-9:
        raise ValueError("SDF grids must share origin, shape, and voxel size")


def _orient_faces_consistently(faces):
    return _rust_marching.orient_faces_consistently(faces)
