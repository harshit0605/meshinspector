from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators._rust_repair_payloads import (
    _component_prune_report_from_payload,
    _degenerate_face_diagnostics_from_payload,
    _duplicate_multi_hole_vertices_report_from_payload,
    _fix_self_intersections_relax_report_from_payload,
    _hole_fill_plan_diagnostics_from_payload,
    _hole_report_from_payload,
    _mesh_from_payload,
    _mesh_healer_report_from_payload,
    _multiple_edge_diagnostics_from_payload,
    _multiple_edge_repair_report_from_payload,
    _not_smooth_face_diagnostics_from_payload,
    _report_from_payload,
    _require_rust_kernel,
    _short_edge_diagnostics_from_payload,
    _voxel_rebuild_report_from_payload,
)
from geometry_sdk.types import (
    ComponentPruneReport,
    DegenerateFaceDiagnostics,
    DuplicateMultiHoleVerticesReport,
    FixSelfIntersectionsRelaxReport,
    HoleFillPlanDiagnostics,
    HoleFillReport,
    MeshDocument,
    MultipleEdgeDiagnostics,
    MultipleEdgeRepairReport,
    NotSmoothFaceDiagnostics,
    RepairReport,
    ShortEdgeDiagnostics,
    VoxelRebuildReport,
)

def remove_degenerate_faces(mesh: MeshDocument, *, area_epsilon: float = 1e-12) -> tuple[MeshDocument, int] | None:
    kernel = _require_rust_kernel("remove_degenerate_faces")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(area_epsilon))
    return _mesh_from_payload(mesh, payload), int(payload["changed_count"])


def orient_faces_outward(mesh: MeshDocument) -> MeshDocument | None:
    kernel = _require_rust_kernel("orient_faces_outward")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return mesh.copy(faces=faces)


def flip_normals(mesh: MeshDocument) -> MeshDocument | None:
    kernel = _require_rust_kernel("flip_normals")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return mesh.copy(faces=faces)


def find_disoriented_faces(
    mesh: MeshDocument,
    *,
    ray_mode: str = "shallowest",
    epsilon: float = 1e-8,
) -> list[int] | None:
    kernel = _require_rust_kernel("find_disoriented_faces")
    return [
        int(face_id)
        for face_id in kernel(mesh.vertices, mesh.faces, str(ray_mode), float(epsilon))
    ]


def basic_repair(
    mesh: MeshDocument,
    *,
    merge_tolerance: float = 1e-6,
    area_epsilon: float = 1e-12,
) -> tuple[MeshDocument, RepairReport] | None:
    kernel = _require_rust_kernel("basic_repair")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(merge_tolerance), float(area_epsilon))
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _report_from_payload(payload["report"])


def fix_self_intersections_relax(
    mesh: MeshDocument,
    *,
    relax_iterations: int = 5,
    max_expand: int = 3,
    touch_is_intersection: bool = True,
    force: float = 0.5,
    epsilon: float = 1e-8,
) -> tuple[MeshDocument, FixSelfIntersectionsRelaxReport] | None:
    kernel = _require_rust_kernel("fix_self_intersections_relax")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        int(relax_iterations),
        int(max_expand),
        bool(touch_is_intersection),
        float(force),
        float(epsilon),
    )
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _fix_self_intersections_relax_report_from_payload(payload["report"])


def mesh_healer_diagnostics(
    mesh: MeshDocument,
    *,
    merge_tolerance: float = 1e-6,
    area_epsilon: float = 1e-12,
    detect_self_intersections: bool = True,
    max_self_intersection_faces: int | None = 50000,
    epsilon: float = 1e-8,
) -> MeshHealerReport | None:
    kernel = _require_rust_kernel("mesh_healer_diagnostics")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        float(merge_tolerance),
        float(area_epsilon),
        bool(detect_self_intersections),
        max_self_intersection_faces,
        float(epsilon),
    )
    return _mesh_healer_report_from_payload(payload)


def prune_small_components(
    mesh: MeshDocument,
    *,
    min_area_mm2: float = 0.0,
) -> tuple[MeshDocument, ComponentPruneReport] | None:
    kernel = _require_rust_kernel("prune_small_components")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(min_area_mm2))
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _component_prune_report_from_payload(payload["report"])


def weld_coincident_vertices(
    mesh: MeshDocument,
    *,
    eps_mm: float = 1e-5,
) -> tuple[MeshDocument, dict[str, Any]] | None:
    kernel = _require_rust_kernel("weld_coincident_vertices")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(eps_mm))
    welded = _mesh_from_payload(mesh, payload)
    return welded, dict(payload["report"])


def repaired_surface_area(mesh: MeshDocument) -> float | None:
    kernel = _require_rust_kernel("repaired_surface_area")
    return float(kernel(mesh.vertices, mesh.faces))


def short_edge_diagnostics(mesh: MeshDocument, *, critical_length_mm: float) -> ShortEdgeDiagnostics | None:
    kernel = _require_rust_kernel("short_edge_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(critical_length_mm))
    return _short_edge_diagnostics_from_payload(payload)


def select_short_edges(mesh: MeshDocument, *, max_edge_length_mm: float) -> list[tuple[int, int]] | None:
    kernel = _require_rust_kernel("select_short_edges")
    return [
        (int(edge[0]), int(edge[1]))
        for edge in kernel(mesh.vertices, mesh.faces, float(max_edge_length_mm))
    ]


def degenerate_face_diagnostics(mesh: MeshDocument, *, critical_aspect_ratio: float) -> DegenerateFaceDiagnostics | None:
    kernel = _require_rust_kernel("degenerate_face_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(critical_aspect_ratio))
    return _degenerate_face_diagnostics_from_payload(payload)


def select_degenerate_faces(
    mesh: MeshDocument,
    *,
    min_aspect_ratio: float,
    boundary_only: bool = False,
) -> list[int] | None:
    kernel = _require_rust_kernel("select_degenerate_faces")
    return [
        int(face_id)
        for face_id in kernel(mesh.vertices, mesh.faces, float(min_aspect_ratio), bool(boundary_only))
    ]


def multiple_edge_diagnostics(mesh: MeshDocument) -> MultipleEdgeDiagnostics | None:
    kernel = _require_rust_kernel("multiple_edge_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _multiple_edge_diagnostics_from_payload(payload)


def repair_multiple_edges(mesh: MeshDocument) -> tuple[MeshDocument, MultipleEdgeRepairReport] | None:
    kernel = _require_rust_kernel("repair_multiple_edges")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _multiple_edge_repair_report_from_payload(payload["report"])


def duplicate_multi_hole_vertices(mesh: MeshDocument) -> tuple[MeshDocument, DuplicateMultiHoleVerticesReport] | None:
    kernel = _require_rust_kernel("duplicate_multi_hole_vertices")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _duplicate_multi_hole_vertices_report_from_payload(payload["report"])


def not_smooth_face_diagnostics(mesh: MeshDocument, *, min_angle_radians: float = 0.3) -> NotSmoothFaceDiagnostics | None:
    kernel = _require_rust_kernel("not_smooth_face_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, min_angle_radians)
    return _not_smooth_face_diagnostics_from_payload(payload)


def select_not_smooth_faces(mesh: MeshDocument, *, min_angle_radians: float = 0.3) -> list[int] | None:
    kernel = _require_rust_kernel("select_not_smooth_faces")
    return [int(face_id) for face_id in kernel(mesh.vertices, mesh.faces, float(min_angle_radians))]


def ordered_boundary_loops(mesh: MeshDocument) -> list[list[int]] | None:
    kernel = _require_rust_kernel("ordered_boundary_loops")
    return [[int(vertex_id) for vertex_id in boundary_loop] for boundary_loop in kernel(mesh.vertices, mesh.faces)]


def hole_fill_plan_diagnostics(mesh: MeshDocument, *, max_edges: int | None = None) -> HoleFillPlanDiagnostics | None:
    kernel = _require_rust_kernel("hole_fill_plan_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, max_edges)
    return _hole_fill_plan_diagnostics_from_payload(payload)


def fill_planar_holes(mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport] | None:
    kernel = _require_rust_kernel("fill_planar_holes")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, max_edges)
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _hole_report_from_payload(payload["report"])


def service_fill_holes(
    mesh: MeshDocument,
    *,
    max_edges: int | None = None,
    max_polygon_subdivisions: int | None = None,
    multiple_edges_resolve_mode: str | None = None,
    make_degenerate_band: bool = False,
    stop_before_bad_triangulation: bool = False,
    smooth_bd: bool = True,
    fill_metric: str | None = None,
    fill_metric_up_dir: tuple[float, float, float] | list[float] | None = None,
) -> tuple[MeshDocument, HoleFillReport] | None:
    kernel = _require_rust_kernel("service_fill_holes")
    payload: dict[str, Any] = kernel(
        mesh.vertices, mesh.faces, max_edges, max_polygon_subdivisions,
        multiple_edges_resolve_mode, make_degenerate_band,
        stop_before_bad_triangulation, smooth_bd, fill_metric, fill_metric_up_dir,
    )
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
