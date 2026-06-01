"""Jewelry hollowing compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_hollow
from geometry_sdk.types import AdaptiveHollowReport, DrainHolePlan, MeshDocument, RegionEntry
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh


def protected_hollow_scale_field(
    mesh: MeshDocument,
    regions: list[RegionEntry],
    protect_region_ids: list[str],
    base_thickness_mm: float,
):
    return _rust_hollow.protected_hollow_scale_field(mesh, regions, protect_region_ids, base_thickness_mm)


def inward_directions_for_hollow(mesh: MeshDocument):
    return _rust_hollow.inward_directions_for_hollow(mesh)


def weighted_inner_offset_preview(
    mesh: MeshDocument,
    regions: list[RegionEntry],
    protect_region_ids: list[str],
    wall_thickness_mm: float,
) -> MeshDocument:
    return _rust_hollow.weighted_inner_offset_preview(mesh, regions, protect_region_ids, wall_thickness_mm)


def protected_hollow_mesh(
    mesh: MeshDocument,
    regions: list[RegionEntry],
    protect_region_ids: list[str],
    *,
    wall_thickness_mm: float,
    voxel_size_mm: float = 0.5,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    return _rust_hollow.protected_hollow_mesh(
        mesh,
        regions,
        protect_region_ids,
        wall_thickness_mm=wall_thickness_mm,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )


def service_hollow_voxel_size(mesh: MeshDocument, *, wall_thickness_mm: float) -> float:
    return _rust_hollow.service_hollow_voxel_size(mesh, wall_thickness_mm=wall_thickness_mm)


def service_hollow_mesh(mesh: MeshDocument, *, wall_thickness_mm: float) -> MeshDocument:
    return _rust_hollow.service_hollow_mesh(mesh, wall_thickness_mm=wall_thickness_mm)


def plan_drain_holes(
    mesh: MeshDocument,
    regions: list[RegionEntry],
    ring_axis: tuple[float, float, float],
    *,
    wall_thickness_mm: float,
    hole_diameter_mm: float = 0.8,
) -> list[DrainHolePlan]:
    return _rust_hollow.plan_drain_holes(
        mesh,
        regions,
        ring_axis,
        wall_thickness_mm=wall_thickness_mm,
        hole_diameter_mm=hole_diameter_mm,
    )


def drain_hole_cutter_mesh(plan: DrainHolePlan, *, sections: int = 32) -> MeshDocument:
    return _rust_hollow.drain_hole_cutter_mesh(plan, sections=sections)


def drain_hole_cutters_mesh(plans: list[DrainHolePlan], *, sections: int = 32) -> MeshDocument:
    return _rust_hollow.drain_hole_cutters_mesh(plans, sections=sections)


def apply_drain_holes_voxel(
    shell_mesh: MeshDocument,
    plans: list[DrainHolePlan],
    *,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    sections: int = 32,
    extractor: str = "marching",
) -> MeshDocument:
    cutters = drain_hole_cutters_mesh(plans, sections=sections)
    if cutters.face_count == 0:
        return shell_mesh.copy()
    return voxel_boolean_mesh(
        shell_mesh,
        cutters,
        operation="difference",
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
    )


def adaptive_hollow_to_weight(
    mesh: MeshDocument,
    *,
    target_weight_g: float,
    material: str = "gold_18k",
    tolerance_g: float = 0.1,
    min_thickness_mm: float = 0.5,
    max_thickness_mm: float = 3.0,
    max_iterations: int = 20,
    voxel_size_mm: float = 0.5,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> tuple[MeshDocument, AdaptiveHollowReport]:
    return _rust_hollow.adaptive_hollow_to_weight(
        mesh,
        target_weight_g=target_weight_g,
        material=material,
        tolerance_g=tolerance_g,
        min_thickness_mm=min_thickness_mm,
        max_thickness_mm=max_thickness_mm,
        max_iterations=max_iterations,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )


def adaptive_protected_hollow_to_weight(
    mesh: MeshDocument,
    regions: list[RegionEntry],
    protect_region_ids: list[str],
    *,
    target_weight_g: float,
    material: str = "gold_18k",
    tolerance_g: float = 0.1,
    min_thickness_mm: float = 0.5,
    max_thickness_mm: float = 3.0,
    max_iterations: int = 20,
    voxel_size_mm: float = 0.5,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> tuple[MeshDocument, AdaptiveHollowReport]:
    return _rust_hollow.adaptive_protected_hollow_to_weight(
        mesh,
        regions,
        protect_region_ids,
        target_weight_g=target_weight_g,
        material=material,
        tolerance_g=tolerance_g,
        min_thickness_mm=min_thickness_mm,
        max_thickness_mm=max_thickness_mm,
        max_iterations=max_iterations,
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )
