from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import HoleFillReport, MeshDocument, RepairReport, VoxelRebuildReport


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _mesh_from_payload(source: MeshDocument, payload: dict[str, Any]) -> MeshDocument:
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return MeshDocument(vertices, faces, unit=source.unit, metadata=dict(source.metadata))


def _report_from_payload(payload: dict[str, Any]) -> RepairReport:
    return RepairReport(
        input_vertex_count=int(payload["input_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_vertex_count=int(payload["output_vertex_count"]),
        output_face_count=int(payload["output_face_count"]),
        merged_vertices=int(payload["merged_vertices"]),
        removed_degenerate_faces=int(payload["removed_degenerate_faces"]),
        removed_unreferenced_vertices=int(payload["removed_unreferenced_vertices"]),
    )


def _hole_report_from_payload(payload: dict[str, Any]) -> HoleFillReport:
    return HoleFillReport(
        input_holes=int(payload["input_holes"]),
        filled_holes=int(payload["filled_holes"]),
        added_vertices=int(payload["added_vertices"]),
        added_faces=int(payload["added_faces"]),
        skipped_holes=int(payload["skipped_holes"]),
    )


def _voxel_rebuild_report_from_payload(payload: dict[str, Any]) -> VoxelRebuildReport:
    return VoxelRebuildReport(
        input_vertex_count=int(payload["input_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_vertex_count=int(payload["output_vertex_count"]),
        output_face_count=int(payload["output_face_count"]),
        input_boundary_edge_count=int(payload["input_boundary_edge_count"]),
        output_boundary_edge_count=int(payload["output_boundary_edge_count"]),
        input_nonmanifold_edge_count=int(payload["input_nonmanifold_edge_count"]),
        output_nonmanifold_edge_count=int(payload["output_nonmanifold_edge_count"]),
        input_self_intersections=None
        if payload["input_self_intersections"] is None
        else int(payload["input_self_intersections"]),
        output_self_intersections=None
        if payload["output_self_intersections"] is None
        else int(payload["output_self_intersections"]),
        voxel_size_mm=float(payload["voxel_size_mm"]),
        offset_mm=float(payload["offset_mm"]),
        extractor=str(payload["extractor"]),
        refine=bool(payload["refine"]),
    )


def remove_degenerate_faces(mesh: MeshDocument, *, area_epsilon: float = 1e-12) -> tuple[MeshDocument, int] | None:
    kernel = _require_rust_kernel("remove_degenerate_faces")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(area_epsilon))
    return _mesh_from_payload(mesh, payload), int(payload["changed_count"])


def remove_unreferenced_vertices(mesh: MeshDocument) -> tuple[MeshDocument, int] | None:
    kernel = _require_rust_kernel("remove_unreferenced_vertices")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _mesh_from_payload(mesh, payload), int(payload["changed_count"])


def merge_close_vertices(mesh: MeshDocument, *, tolerance: float = 1e-6) -> tuple[MeshDocument, int] | None:
    kernel = _require_rust_kernel("merge_close_vertices")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(tolerance))
    return _mesh_from_payload(mesh, payload), int(payload["changed_count"])


def orient_faces_outward(mesh: MeshDocument) -> MeshDocument | None:
    kernel = _require_rust_kernel("orient_faces_outward")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return mesh.copy(faces=faces)


def basic_repair(
    mesh: MeshDocument,
    *,
    merge_tolerance: float = 1e-6,
    area_epsilon: float = 1e-12,
) -> tuple[MeshDocument, RepairReport] | None:
    kernel = _require_rust_kernel("basic_repair")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(merge_tolerance), float(area_epsilon))
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _report_from_payload(payload["report"])


def repaired_surface_area(mesh: MeshDocument) -> float | None:
    kernel = _require_rust_kernel("repaired_surface_area")
    if kernel is None:
        return None
    return float(kernel(mesh.vertices, mesh.faces))


def ordered_boundary_loops(mesh: MeshDocument) -> list[list[int]] | None:
    kernel = _require_rust_kernel("ordered_boundary_loops")
    if kernel is None:
        return None
    return [[int(vertex_id) for vertex_id in boundary_loop] for boundary_loop in kernel(mesh.vertices, mesh.faces)]


def fill_planar_holes(mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport] | None:
    kernel = _require_rust_kernel("fill_planar_holes")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, max_edges)
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _hole_report_from_payload(payload["report"])


def service_fill_holes(mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport] | None:
    kernel = _require_rust_kernel("service_fill_holes")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, max_edges)
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _hole_report_from_payload(payload["report"])


def rebuild_via_sdf(
    mesh: MeshDocument,
    *,
    voxel_size_mm: float,
    offset_mm: float = 0.0,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = True,
) -> tuple[MeshDocument, VoxelRebuildReport]:
    kernel = _require_rust_kernel("rebuild_via_sdf")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        float(voxel_size_mm),
        float(offset_mm),
        None if padding_mm is None else float(padding_mm),
        extractor,
        bool(refine),
    )
    rebuilt = _mesh_from_payload(mesh, payload)
    return rebuilt, _voxel_rebuild_report_from_payload(payload["report"])
