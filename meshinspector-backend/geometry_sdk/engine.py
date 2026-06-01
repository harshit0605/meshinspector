"""Stable public facade for the parallel geometry SDK."""

from __future__ import annotations

from pathlib import Path

import numpy as np

from geometry_sdk.analysis.compare import (
    compare_summary,
    nearest_surface_distances,
    service_compare_distances,
    service_compare_summary,
    signed_compare_summary,
    signed_surface_distances,
    version_compare_distances,
    version_compare_summary,
)
from geometry_sdk.analysis.artifacts import save_compare_npz, save_thickness_npz
from geometry_sdk.analysis.health import compute_mesh_health, service_mesh_health
from geometry_sdk.analysis.manufacturability import compute_manufacturability_report
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.thickness import ray_thickness_at_vertices, service_thickness_at_vertices, summarize_thickness
from geometry_sdk.deform.brushes import apply_brush_strokes, brush_stroke_from_regions
from geometry_sdk.deform.local import local_scoop, local_thicken, local_thicken_to_minimum, smooth, taubin_smooth
from geometry_sdk.deform.resize import radial_scale, resize_ring
from geometry_sdk.deform.thicken import global_thicken
from geometry_sdk.io.trimesh_adapter import load_mesh, save_mesh
from geometry_sdk.jewelry.hollow import (
    adaptive_hollow_to_weight,
    adaptive_protected_hollow_to_weight,
    apply_drain_holes_voxel,
    drain_hole_cutter_mesh,
    drain_hole_cutters_mesh,
    plan_drain_holes,
    protected_hollow_mesh,
    protected_hollow_scale_field,
    service_hollow_mesh,
    service_hollow_voxel_size,
    weighted_inner_offset_preview,
)
from geometry_sdk.jewelry.regions import detect_ring_regions
from geometry_sdk.jewelry.ring_measurement import measure_ring
from geometry_sdk.repair.basic import basic_repair, orient_faces_outward
from geometry_sdk.repair.holes import fill_planar_holes, service_fill_holes
from geometry_sdk.repair.voxel import rebuild_via_sdf
from geometry_sdk.voxel.extract import extract_surface_mesh
from geometry_sdk.voxel.marching import extract_marching_tetrahedra
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh, voxel_offset_mesh, voxel_shell_mesh
from geometry_sdk.voxel.ops import sdf_difference, sdf_intersection, sdf_offset, sdf_shell, sdf_union
from geometry_sdk.voxel.sdf import SDFGrid, sample_sdf_grid
from geometry_sdk.types import AdaptiveHollowReport, BrushStroke, DrainHolePlan, HoleFillReport, ManufacturabilityReport, MeshDocument, MeshHealth, MeshStats, RegionEntry, RepairReport, RingMeasurement, ServiceMeshHealth, ThicknessSummary, VersionCompareSummary, VoxelRebuildReport


class GeometrySDK:
    """Public API boundary for in-house algorithms.

    The facade is intentionally thin in V0. It gives future service migration a
    stable import target while the underlying modules continue to evolve.
    """

    def load_mesh(self, path: str | Path) -> MeshDocument:
        return load_mesh(path)

    def save_mesh(self, mesh: MeshDocument, path: str | Path, *, file_type: str | None = None) -> Path:
        return save_mesh(mesh, path, file_type=file_type)

    def stats(self, mesh: MeshDocument) -> MeshStats:
        return compute_mesh_stats(mesh)

    def health(self, mesh: MeshDocument) -> MeshHealth:
        return compute_mesh_health(mesh)

    def service_health(self, mesh: MeshDocument, *, max_listed_faces: int = 100) -> ServiceMeshHealth:
        return service_mesh_health(mesh, max_listed_faces=max_listed_faces)

    def basic_repair(self, mesh: MeshDocument, *, merge_tolerance: float = 1e-6, area_epsilon: float = 1e-12) -> tuple[MeshDocument, RepairReport]:
        return basic_repair(mesh, merge_tolerance=merge_tolerance, area_epsilon=area_epsilon)

    def orient_faces_outward(self, mesh: MeshDocument) -> MeshDocument:
        return orient_faces_outward(mesh)

    def fill_planar_holes(self, mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport]:
        return fill_planar_holes(mesh, max_edges=max_edges)

    def service_fill_holes(self, mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport]:
        return service_fill_holes(mesh, max_edges=max_edges)

    def rebuild_via_sdf(
        self,
        mesh: MeshDocument,
        *,
        voxel_size_mm: float,
        offset_mm: float = 0.0,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = True,
    ) -> tuple[MeshDocument, VoxelRebuildReport]:
        return rebuild_via_sdf(
            mesh,
            voxel_size_mm=voxel_size_mm,
            offset_mm=offset_mm,
            padding_mm=padding_mm,
            extractor=extractor,
            refine=refine,
        )

    def compare(self, source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
        return compare_summary(source, target)

    def compare_field(self, source: MeshDocument, target: MeshDocument) -> np.ndarray:
        return nearest_surface_distances(source, target)

    def signed_compare(self, source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
        return signed_compare_summary(source, target)

    def version_compare(self, source: MeshDocument, target: MeshDocument) -> VersionCompareSummary:
        return version_compare_summary(source, target)

    def version_compare_field(self, source: MeshDocument, target: MeshDocument) -> np.ndarray:
        return version_compare_distances(source, target)

    def service_compare_field(self, source: MeshDocument, other: MeshDocument) -> np.ndarray:
        return service_compare_distances(source, other)

    def service_compare(self, source: MeshDocument, other: MeshDocument) -> VersionCompareSummary:
        return service_compare_summary(source, other)

    def signed_compare_field(self, source: MeshDocument, target: MeshDocument) -> np.ndarray:
        return signed_surface_distances(source, target)

    def ray_thickness(self, mesh: MeshDocument, *, threshold_mm: float = 0.6) -> tuple[np.ndarray, ThicknessSummary]:
        field = ray_thickness_at_vertices(mesh)
        return field, summarize_thickness(field, threshold_mm=threshold_mm)

    def service_thickness(self, mesh: MeshDocument, *, threshold_mm: float = 0.6) -> tuple[np.ndarray, ThicknessSummary]:
        field = service_thickness_at_vertices(mesh)
        return field, summarize_thickness(field, threshold_mm=threshold_mm)

    def save_thickness_npz(self, path: str | Path, thickness: np.ndarray, *, vertex_count: int, threshold_mm: float) -> Path:
        return save_thickness_npz(path, thickness, vertex_count=vertex_count, threshold_mm=threshold_mm)

    def save_compare_npz(self, path: str | Path, values: np.ndarray, *, vertex_count: int, other_version_id: str) -> Path:
        return save_compare_npz(path, values, vertex_count=vertex_count, other_version_id=other_version_id)

    def sample_sdf_grid(
        self,
        mesh: MeshDocument,
        *,
        voxel_size_mm: float,
        padding_mm: float | None = None,
    ) -> SDFGrid:
        return sample_sdf_grid(mesh, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm)

    def sdf_union(self, a: SDFGrid, b: SDFGrid) -> SDFGrid:
        return sdf_union(a, b)

    def sdf_intersection(self, a: SDFGrid, b: SDFGrid) -> SDFGrid:
        return sdf_intersection(a, b)

    def sdf_difference(self, a: SDFGrid, b: SDFGrid) -> SDFGrid:
        return sdf_difference(a, b)

    def sdf_offset(self, grid: SDFGrid, offset_mm: float) -> SDFGrid:
        return sdf_offset(grid, offset_mm)

    def sdf_shell(self, grid: SDFGrid, wall_thickness_mm: float) -> SDFGrid:
        return sdf_shell(grid, wall_thickness_mm)

    def extract_sdf_surface(self, grid: SDFGrid, *, iso_value: float = 0.0) -> MeshDocument:
        return extract_surface_mesh(grid, iso_value=iso_value)

    def extract_sdf_isosurface(self, grid: SDFGrid, *, iso_value: float = 0.0) -> MeshDocument:
        return extract_marching_tetrahedra(grid, iso_value=iso_value)

    def voxel_offset_mesh(
        self,
        mesh: MeshDocument,
        *,
        offset_mm: float,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_offset_mesh(mesh, offset_mm=offset_mm, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm, extractor=extractor, refine=refine)

    def voxel_shell_mesh(
        self,
        mesh: MeshDocument,
        *,
        wall_thickness_mm: float,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_shell_mesh(mesh, wall_thickness_mm=wall_thickness_mm, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm, extractor=extractor, refine=refine)

    def voxel_boolean_mesh(
        self,
        a: MeshDocument,
        b: MeshDocument,
        *,
        operation: str,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        origin_phase: tuple[float, float, float] | None = (0.125, 0.125, 0.125),
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_boolean_mesh(
            a,
            b,
            operation=operation,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            origin_phase=origin_phase,
            extractor=extractor,
            refine=refine,
        )

    def global_thicken(self, mesh: MeshDocument, *, min_target_thickness_mm: float) -> MeshDocument:
        return global_thicken(mesh, min_target_thickness_mm=min_target_thickness_mm)

    def service_hollow(self, mesh: MeshDocument, *, wall_thickness_mm: float) -> MeshDocument:
        return service_hollow_mesh(mesh, wall_thickness_mm=wall_thickness_mm)

    def service_hollow_voxel_size(self, mesh: MeshDocument, *, wall_thickness_mm: float) -> float:
        return service_hollow_voxel_size(mesh, wall_thickness_mm=wall_thickness_mm)

    def manufacturability(self, mesh: MeshDocument, *, threshold_mm: float = 0.6) -> ManufacturabilityReport:
        return compute_manufacturability_report(mesh, threshold_mm=threshold_mm)

    def measure_ring(
        self,
        mesh: MeshDocument,
        *,
        axis_override: tuple[float, float, float] | np.ndarray | None = None,
    ) -> RingMeasurement:
        return measure_ring(mesh, axis_override=axis_override)

    def detect_ring_regions(
        self,
        mesh: MeshDocument,
        measurement: RingMeasurement,
        *,
        thickness: np.ndarray | None = None,
        threshold_mm: float = 0.6,
    ) -> list[RegionEntry]:
        return detect_ring_regions(mesh, measurement, thickness=thickness, threshold_mm=threshold_mm)

    def protected_hollow_scale_field(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        protect_region_ids: list[str],
        *,
        base_thickness_mm: float,
    ) -> np.ndarray:
        return protected_hollow_scale_field(mesh, regions, protect_region_ids, base_thickness_mm)

    def weighted_inner_offset_preview(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        protect_region_ids: list[str],
        *,
        wall_thickness_mm: float,
    ) -> MeshDocument:
        return weighted_inner_offset_preview(mesh, regions, protect_region_ids, wall_thickness_mm)

    def protected_hollow_mesh(
        self,
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
        return protected_hollow_mesh(
            mesh,
            regions,
            protect_region_ids,
            wall_thickness_mm=wall_thickness_mm,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            extractor=extractor,
            refine=refine,
        )

    def plan_drain_holes(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        ring_axis: tuple[float, float, float] | np.ndarray,
        *,
        wall_thickness_mm: float,
        hole_diameter_mm: float = 0.8,
    ) -> list[DrainHolePlan]:
        return plan_drain_holes(
            mesh,
            regions,
            ring_axis,
            wall_thickness_mm=wall_thickness_mm,
            hole_diameter_mm=hole_diameter_mm,
        )

    def drain_hole_cutter_mesh(self, plan: DrainHolePlan, *, sections: int = 32) -> MeshDocument:
        return drain_hole_cutter_mesh(plan, sections=sections)

    def drain_hole_cutters_mesh(self, plans: list[DrainHolePlan], *, sections: int = 32) -> MeshDocument:
        return drain_hole_cutters_mesh(plans, sections=sections)

    def apply_drain_holes_voxel(
        self,
        shell_mesh: MeshDocument,
        plans: list[DrainHolePlan],
        *,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        sections: int = 32,
        extractor: str = "marching",
    ) -> MeshDocument:
        return apply_drain_holes_voxel(
            shell_mesh,
            plans,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            sections=sections,
            extractor=extractor,
        )

    def adaptive_hollow_to_weight(
        self,
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
        return adaptive_hollow_to_weight(
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
        self,
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
        return adaptive_protected_hollow_to_weight(
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

    def radial_scale(
        self,
        mesh: MeshDocument,
        scale_factor: float,
        *,
        ring_axis: np.ndarray | tuple[float, float, float] | None = None,
        preserve_indices: np.ndarray | None = None,
    ) -> MeshDocument:
        return radial_scale(mesh, scale_factor, ring_axis=ring_axis, preserve_indices=preserve_indices)

    def resize_ring(
        self,
        mesh: MeshDocument,
        *,
        current_size: float,
        target_size: float,
        ring_axis: np.ndarray | tuple[float, float, float] | None = None,
        preserve_indices: np.ndarray | None = None,
    ) -> MeshDocument:
        return resize_ring(
            mesh,
            current_size=current_size,
            target_size=target_size,
            ring_axis=ring_axis,
            preserve_indices=preserve_indices,
        )

    def local_thicken(
        self,
        mesh: MeshDocument,
        seed_indices: np.ndarray,
        *,
        amount_mm: float,
        falloff_mm: float = 1.5,
    ) -> MeshDocument:
        return local_thicken(mesh, seed_indices, amount_mm=amount_mm, falloff_mm=falloff_mm)

    def local_thicken_to_minimum(
        self,
        mesh: MeshDocument,
        seed_indices: np.ndarray,
        thickness_values: np.ndarray,
        *,
        min_target_thickness_mm: float,
        falloff_mm: float = 1.5,
        deficit_scale: float = 0.75,
    ) -> MeshDocument:
        return local_thicken_to_minimum(
            mesh,
            seed_indices,
            thickness_values,
            min_target_thickness_mm=min_target_thickness_mm,
            falloff_mm=falloff_mm,
            deficit_scale=deficit_scale,
        )

    def local_scoop(
        self,
        mesh: MeshDocument,
        seed_indices: np.ndarray,
        *,
        depth_mm: float,
        falloff_mm: float = 1.5,
    ) -> MeshDocument:
        return local_scoop(mesh, seed_indices, depth_mm=depth_mm, falloff_mm=falloff_mm)

    def smooth(
        self,
        mesh: MeshDocument,
        *,
        iterations: int = 5,
        strength: float = 0.5,
        seed_indices: np.ndarray | None = None,
        falloff_mm: float = 1.8,
        nu: float = -0.53,
    ) -> MeshDocument:
        return smooth(mesh, iterations=iterations, strength=strength, seed_indices=seed_indices, falloff_mm=falloff_mm, nu=nu)

    def taubin_smooth(
        self,
        mesh: MeshDocument,
        *,
        iterations: int = 10,
        lamb: float = 0.5,
        nu: float = -0.53,
    ) -> MeshDocument:
        return taubin_smooth(mesh, iterations=iterations, lamb=lamb, nu=nu)

    def apply_brush_strokes(self, mesh: MeshDocument, strokes: list[BrushStroke]) -> MeshDocument:
        return apply_brush_strokes(mesh, strokes)

    def brush_stroke_from_regions(
        self,
        operation: str,
        seed_indices: np.ndarray,
        regions: list[RegionEntry],
        *,
        amount_mm: float = 0.0,
        falloff_mm: float = 1.5,
        iterations: int = 1,
        strength: float = 0.5,
        editable_region_ids: list[str] | None = None,
        protected_region_ids: list[str] | None = None,
        respect_allowed_operations: bool = True,
    ) -> BrushStroke:
        return brush_stroke_from_regions(
            operation,
            seed_indices,
            regions,
            amount_mm=amount_mm,
            falloff_mm=falloff_mm,
            iterations=iterations,
            strength=strength,
            editable_region_ids=editable_region_ids,
            protected_region_ids=protected_region_ids,
            respect_allowed_operations=respect_allowed_operations,
        )


default_sdk = GeometrySDK()
