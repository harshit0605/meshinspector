"""Surface extraction compatibility wrappers for SDF grids."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import MeshDocument
from geometry_sdk.voxel.sdf import SDFGrid


def extract_surface_mesh(grid: SDFGrid, *, iso_value: float = 0.0) -> MeshDocument:
    return _rust_voxel.extract_surface_mesh_from_sdf_cells(
        grid.values,
        origin=grid.origin,
        shape=grid.shape,
        voxel_size_mm=grid.voxel_size_mm,
        iso_value=iso_value,
    )
