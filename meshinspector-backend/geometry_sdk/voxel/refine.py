"""SDF-aware refinement compatibility wrappers for extracted voxel meshes."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_refine
from geometry_sdk.types import MeshDocument
from geometry_sdk.voxel.sdf import SDFGrid


def project_vertices_to_sdf(mesh: MeshDocument, grid: SDFGrid, *, iso_value: float = 0.0, iterations: int = 3) -> MeshDocument:
    return mesh.copy(
        vertices=_rust_refine.project_vertices_to_sdf(
            mesh,
            grid.values,
            origin=grid.origin,
            shape=grid.shape,
            voxel_size_mm=grid.voxel_size_mm,
            iso_value=iso_value,
            iterations=iterations,
        )
    )


def laplacian_smooth_vertices(mesh: MeshDocument, *, iterations: int = 1, strength: float = 0.25) -> MeshDocument:
    return mesh.copy(vertices=_rust_refine.laplacian_smooth_vertices(mesh, iterations=iterations, strength=strength))


def refine_sdf_mesh(
    mesh: MeshDocument,
    grid: SDFGrid,
    *,
    iso_value: float = 0.0,
    smooth_iterations: int = 1,
    smooth_strength: float = 0.2,
    projection_iterations: int = 3,
) -> MeshDocument:
    return mesh.copy(
        vertices=_rust_refine.refine_vertices_with_sdf(
            mesh,
            grid.values,
            origin=grid.origin,
            shape=grid.shape,
            voxel_size_mm=grid.voxel_size_mm,
            iso_value=iso_value,
            smooth_iterations=smooth_iterations,
            smooth_strength=smooth_strength,
            projection_iterations=projection_iterations,
        )
    )
