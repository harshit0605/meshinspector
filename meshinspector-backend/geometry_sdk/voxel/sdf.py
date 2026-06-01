"""Signed-distance grid compatibility wrappers."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_sdf
from geometry_sdk.types import MeshDocument, SDFGrid


def sample_sdf_grid(
    mesh: MeshDocument,
    *,
    voxel_size_mm: float,
    padding_mm: float | None = None,
) -> SDFGrid:
    return _rust_sdf.sample_sdf_grid(mesh, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm)


def sample_sdf_grid_in_bounds(
    mesh: MeshDocument,
    *,
    bbox_min,
    bbox_max,
    voxel_size_mm: float,
    padding_mm: float = 0.0,
    origin_phase: tuple[float, float, float] | None = None,
) -> SDFGrid:
    return _rust_sdf.sample_sdf_grid_in_bounds(
        mesh,
        bbox_min=bbox_min,
        bbox_max=bbox_max,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        origin_phase=origin_phase,
    )


def sample_aligned_sdf_grids(
    meshes: list[MeshDocument],
    *,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    origin_phase: tuple[float, float, float] | None = None,
) -> list[SDFGrid]:
    return _rust_sdf.sample_aligned_sdf_grids(
        meshes,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        origin_phase=origin_phase,
    )


def sdf_cell_values(grid: SDFGrid):
    return _rust_sdf.sdf_cell_values(grid)


def sdf_occupancy(grid: SDFGrid, *, iso_value: float = 0.0):
    return _rust_sdf.sdf_occupancy(grid, iso_value=iso_value)


def estimate_sdf_volume(grid: SDFGrid, *, iso_value: float = 0.0) -> float:
    return _rust_sdf.estimate_sdf_volume(grid, iso_value=iso_value)


def sample_sdf_values(grid: SDFGrid, points):
    return _rust_sdf.sample_sdf_values(grid, points)


def sample_sdf_gradients(grid: SDFGrid, points):
    return _rust_sdf.sample_sdf_gradients(grid, points)
