"""Mesh-in/mesh-out voxel operation compatibility wrappers."""

from __future__ import annotations

from typing import Literal

from geometry_sdk.accelerators import _rust_mesh_ops
from geometry_sdk.types import MeshDocument, RegionEntry, SDFGrid


ExtractorName = Literal["marching", "cells"]
BooleanOperation = Literal["union", "intersection", "difference"]
DEFAULT_BOOLEAN_ORIGIN_PHASE: tuple[float, float, float] = (0.125, 0.125, 0.125)


def extract_grid_mesh(
    grid: SDFGrid,
    *,
    extractor: ExtractorName = "marching",
    refine: bool = False,
    smooth_iterations: int = 1,
    smooth_strength: float = 0.2,
    projection_iterations: int = 3,
) -> MeshDocument:
    return _rust_mesh_ops.extract_grid_mesh(
        grid,
        extractor=extractor,
        refine=refine,
        smooth_iterations=smooth_iterations,
        smooth_strength=smooth_strength,
        projection_iterations=projection_iterations,
    )


def voxel_offset_mesh(
    mesh: MeshDocument,
    *,
    offset_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: ExtractorName = "marching",
    refine: bool = False,
) -> MeshDocument:
    return _rust_mesh_ops.voxel_offset_mesh(
        mesh,
        offset_mm=offset_mm,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )


def voxel_shell_mesh(
    mesh: MeshDocument,
    *,
    wall_thickness_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: ExtractorName = "marching",
    refine: bool = False,
) -> MeshDocument:
    return _rust_mesh_ops.voxel_shell_mesh(
        mesh,
        wall_thickness_mm=wall_thickness_mm,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )


def voxel_thicken_mesh(
    mesh: MeshDocument,
    *,
    thickness_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: ExtractorName = "marching",
    refine: bool = False,
) -> MeshDocument:
    return _rust_mesh_ops.voxel_thicken_mesh(
        mesh,
        thickness_mm=thickness_mm,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )


def voxel_weighted_shell_mesh(
    mesh: MeshDocument,
    *,
    regions: list[RegionEntry],
    region_weights: dict[str, float],
    offset_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    interpolation_distance_mm: float = 0.0,
    extractor: ExtractorName = "marching",
    refine: bool = False,
) -> MeshDocument:
    return _rust_mesh_ops.voxel_weighted_shell_mesh(
        mesh,
        regions=regions,
        region_weights=region_weights,
        offset_mm=offset_mm,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        interpolation_distance_mm=interpolation_distance_mm,
        extractor=extractor,
        refine=refine,
    )


def voxel_partial_offset_mesh(
    mesh: MeshDocument,
    *,
    regions: list[RegionEntry],
    selected_region_ids: list[str],
    offset_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: ExtractorName = "marching",
    refine: bool = False,
) -> MeshDocument:
    return _rust_mesh_ops.voxel_partial_offset_mesh(
        mesh,
        regions=regions,
        selected_region_ids=selected_region_ids,
        offset_mm=offset_mm,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )


def voxel_boolean_mesh(
    a: MeshDocument,
    b: MeshDocument,
    *,
    operation: BooleanOperation,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    origin_phase: tuple[float, float, float] | None = DEFAULT_BOOLEAN_ORIGIN_PHASE,
    extractor: ExtractorName = "marching",
    refine: bool = False,
) -> MeshDocument:
    return _rust_mesh_ops.voxel_boolean_mesh(
        a,
        b,
        operation=operation,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        origin_phase=origin_phase,
        extractor=extractor,
        refine=refine,
    )
