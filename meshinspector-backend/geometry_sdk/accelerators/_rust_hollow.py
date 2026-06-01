from __future__ import annotations

from collections.abc import Iterable
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import AdaptiveHollowReport, DrainHolePlan, MeshDocument, RegionEntry


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _region_buffers(regions: Iterable[RegionEntry]) -> tuple[list[str], np.ndarray, np.ndarray]:
    region_ids: list[str] = []
    vertex_offsets = [0]
    flat_vertex_indices: list[int] = []
    for region in regions:
        region_ids.append(str(region.region_id))
        flat_vertex_indices.extend(int(index) for index in np.asarray(region.vertex_indices, dtype=np.int64).reshape(-1))
        vertex_offsets.append(len(flat_vertex_indices))
    return (
        region_ids,
        np.asarray(vertex_offsets, dtype=np.int64),
        np.asarray(flat_vertex_indices, dtype=np.int64),
    )


def _plan_arrays(plans: Iterable[DrainHolePlan]) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, int]:
    plan_list = list(plans)
    if not plan_list:
        return (
            np.zeros((0, 3), dtype=np.float64),
            np.zeros((0, 3), dtype=np.float64),
            np.zeros(0, dtype=np.float64),
            np.zeros(0, dtype=np.float64),
            0,
        )
    return (
        np.asarray([plan.center_mm for plan in plan_list], dtype=np.float64).reshape(-1, 3),
        np.asarray([plan.direction for plan in plan_list], dtype=np.float64).reshape(-1, 3),
        np.asarray([plan.radius_mm for plan in plan_list], dtype=np.float64).reshape(-1),
        np.asarray([plan.length_mm for plan in plan_list], dtype=np.float64).reshape(-1),
        len(plan_list),
    )


def _mesh_from_payload(payload: dict[str, Any], source: str, metadata: dict[str, Any] | None = None) -> MeshDocument:
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={"source": source, **(metadata or {})},
    )


def protected_hollow_scale_field(
    mesh: MeshDocument,
    regions: Iterable[RegionEntry],
    protect_region_ids: Iterable[str],
    base_thickness_mm: float,
) -> np.ndarray:
    kernel = _require_rust_kernel("protected_hollow_scale_field")
    region_ids, vertex_offsets, vertex_indices = _region_buffers(regions)
    scales = kernel(
        mesh.vertices,
        region_ids,
        vertex_offsets,
        vertex_indices,
        [str(region_id) for region_id in protect_region_ids],
        float(base_thickness_mm),
    )
    return np.asarray(scales, dtype=np.float32).reshape(-1)


def inward_directions_for_hollow(mesh: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("inward_directions_for_hollow")
    directions = kernel(mesh.vertices, mesh.faces)
    return np.asarray(directions, dtype=np.float64).reshape(-1, 3)


def weighted_inner_offset_preview(
    mesh: MeshDocument,
    regions: Iterable[RegionEntry],
    protect_region_ids: Iterable[str],
    wall_thickness_mm: float,
) -> MeshDocument:
    kernel = _require_rust_kernel("weighted_inner_offset_vertices")
    region_ids, vertex_offsets, vertex_indices = _region_buffers(regions)
    vertices = kernel(
        mesh.vertices,
        mesh.faces,
        region_ids,
        vertex_offsets,
        vertex_indices,
        [str(region_id) for region_id in protect_region_ids],
        float(wall_thickness_mm),
    )
    return mesh.copy(vertices=np.asarray(vertices, dtype=np.float64).reshape(-1, 3))


def protected_hollow_mesh(
    mesh: MeshDocument,
    regions: Iterable[RegionEntry],
    protect_region_ids: Iterable[str],
    *,
    wall_thickness_mm: float,
    voxel_size_mm: float = 0.5,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    kernel = _require_rust_kernel("protected_hollow_mesh")
    region_ids, vertex_offsets, vertex_indices = _region_buffers(regions)
    payload = kernel(
        mesh.vertices,
        mesh.faces,
        region_ids,
        vertex_offsets,
        vertex_indices,
        [str(region_id) for region_id in protect_region_ids],
        float(wall_thickness_mm),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        str(extractor),
        bool(refine),
    )
    return _mesh_from_payload(
        payload,
        "protected_hollow",
        {"wall_thickness_mm": float(wall_thickness_mm)},
    )


def service_hollow_voxel_size(mesh: MeshDocument, *, wall_thickness_mm: float) -> float:
    kernel = _require_rust_kernel("service_hollow_voxel_size")
    return float(kernel(mesh.vertices, float(wall_thickness_mm)))


def service_hollow_mesh(mesh: MeshDocument, *, wall_thickness_mm: float) -> MeshDocument:
    kernel = _require_rust_kernel("service_hollow_mesh")
    payload = kernel(mesh.vertices, mesh.faces, float(wall_thickness_mm))
    voxel_size = service_hollow_voxel_size(mesh, wall_thickness_mm=wall_thickness_mm)
    return _mesh_from_payload(
        payload,
        "service_hollow",
        {"wall_thickness_mm": float(wall_thickness_mm), "voxel_size_mm": voxel_size},
    )


def plan_drain_holes(
    mesh: MeshDocument,
    regions: Iterable[RegionEntry],
    ring_axis: Any,
    *,
    wall_thickness_mm: float,
    hole_diameter_mm: float = 0.8,
) -> list[DrainHolePlan]:
    kernel = _require_rust_kernel("plan_drain_holes")
    region_ids, vertex_offsets, vertex_indices = _region_buffers(regions)
    plans = kernel(
        mesh.vertices,
        region_ids,
        vertex_offsets,
        vertex_indices,
        np.asarray(ring_axis, dtype=np.float64).reshape(3),
        float(wall_thickness_mm),
        float(hole_diameter_mm),
    )
    return [
        DrainHolePlan(
            center_mm=tuple(float(value) for value in plan["center_mm"]),
            direction=tuple(float(value) for value in plan["direction"]),
            radius_mm=float(plan["radius_mm"]),
            length_mm=float(plan["length_mm"]),
        )
        for plan in plans
    ]


def drain_hole_cutter_mesh(plan: DrainHolePlan, *, sections: int = 32) -> MeshDocument:
    kernel = _require_rust_kernel("drain_hole_cutter_mesh")
    payload = kernel(
        np.asarray(plan.center_mm, dtype=np.float64).reshape(3),
        np.asarray(plan.direction, dtype=np.float64).reshape(3),
        float(plan.radius_mm),
        float(plan.length_mm),
        int(sections),
    )
    return _mesh_from_payload(
        payload,
        "drain_hole_cutter",
        {"radius_mm": plan.radius_mm, "length_mm": plan.length_mm},
    )


def drain_hole_cutters_mesh(plans: Iterable[DrainHolePlan], *, sections: int = 32) -> MeshDocument:
    kernel = _require_rust_kernel("drain_hole_cutters_mesh")
    centers, directions, radii, lengths, count = _plan_arrays(plans)
    payload = kernel(centers, directions, radii, lengths, int(sections))
    return _mesh_from_payload(payload, "drain_hole_cutters", {"count": count})


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
    kernel = _require_rust_kernel("adaptive_hollow_to_weight")
    payload = kernel(
        mesh.vertices,
        mesh.faces,
        float(target_weight_g),
        str(material),
        float(tolerance_g),
        float(min_thickness_mm),
        float(max_thickness_mm),
        int(max_iterations),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        str(extractor),
        bool(refine),
    )
    hollowed = _mesh_from_payload(
        payload,
        "adaptive_hollow",
        {
            "target_weight_g": float(payload["target_weight_g"]),
            "wall_thickness_mm": payload["wall_thickness_mm"],
        },
    )
    report = AdaptiveHollowReport(
        achieved_weight_g=float(payload["achieved_weight_g"]),
        wall_thickness_mm=None if payload["wall_thickness_mm"] is None else float(payload["wall_thickness_mm"]),
        iterations=int(payload["iterations"]),
        warning=None if payload["warning"] is None else str(payload["warning"]),
        original_weight_g=float(payload["original_weight_g"]),
        target_weight_g=float(payload["target_weight_g"]),
    )
    return hollowed, report


def adaptive_protected_hollow_to_weight(
    mesh: MeshDocument,
    regions: Iterable[RegionEntry],
    protect_region_ids: Iterable[str],
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
    kernel = _require_rust_kernel("adaptive_protected_hollow_to_weight")
    region_ids, vertex_offsets, vertex_indices = _region_buffers(regions)
    payload = kernel(
        mesh.vertices,
        mesh.faces,
        region_ids,
        vertex_offsets,
        vertex_indices,
        [str(region_id) for region_id in protect_region_ids],
        float(target_weight_g),
        str(material),
        float(tolerance_g),
        float(min_thickness_mm),
        float(max_thickness_mm),
        int(max_iterations),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        str(extractor),
        bool(refine),
    )
    hollowed = _mesh_from_payload(
        payload,
        "adaptive_protected_hollow",
        {
            "target_weight_g": float(payload["target_weight_g"]),
            "wall_thickness_mm": payload["wall_thickness_mm"],
        },
    )
    report = AdaptiveHollowReport(
        achieved_weight_g=float(payload["achieved_weight_g"]),
        wall_thickness_mm=None if payload["wall_thickness_mm"] is None else float(payload["wall_thickness_mm"]),
        iterations=int(payload["iterations"]),
        warning=None if payload["warning"] is None else str(payload["warning"]),
        original_weight_g=float(payload["original_weight_g"]),
        target_weight_g=float(payload["target_weight_g"]),
    )
    return hollowed, report
