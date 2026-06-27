from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import (
    ComponentPruneReport,
    DegenerateFaceDiagnostics,
    DegenerateFaceEntry,
    DuplicateMultiHoleVerticesReport,
    FixSelfIntersectionsRelaxReport,
    HoleFillPlanDiagnostics,
    HoleFillPlanEntry,
    HoleFillReport,
    MeshDocument,
    MeshHealerIssue,
    MeshHealerReport,
    MultipleEdgeDiagnostics,
    MultipleEdgeEntry,
    MultipleEdgeRepairReport,
    NotSmoothFaceDiagnostics,
    NotSmoothFaceEntry,
    RepairReport,
    ShortEdgeDiagnostics,
    ShortEdgeEntry,
    VoxelRebuildReport,
)


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

def _mesh_healer_report_from_payload(payload: dict[str, Any]) -> MeshHealerReport:
    return MeshHealerReport(
        input_vertex_count=int(payload["input_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        holes_count=int(payload["holes_count"]),
        boundary_edge_count=int(payload["boundary_edge_count"]),
        nonmanifold_edge_count=int(payload["nonmanifold_edge_count"]),
        self_intersections=None if payload["self_intersections"] is None else int(payload["self_intersections"]),
        self_intersections_available=bool(payload["self_intersections_available"]),
        total_issue_count=int(payload["total_issue_count"]),
        issue_category_count=int(payload["issue_category_count"]),
        fixable_issue_count=int(payload["fixable_issue_count"]),
        auto_repair_ready=bool(payload["auto_repair_ready"]),
        issues=[
            MeshHealerIssue(
                issue_id=str(issue["issue_id"]),
                label=str(issue["label"]),
                count=int(issue["count"]),
                severity=str(issue["severity"]),
                rust_repair_available=bool(issue["rust_repair_available"]),
                repair_command=None if issue["repair_command"] is None else str(issue["repair_command"]),
            )
            for issue in payload["issues"]
        ],
    )

def _component_prune_report_from_payload(payload: dict[str, Any]) -> ComponentPruneReport:
    return ComponentPruneReport(
        input_component_count=int(payload["input_component_count"]),
        output_component_count=int(payload["output_component_count"]),
        removed_component_count=int(payload["removed_component_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_face_count=int(payload["output_face_count"]),
        removed_face_count=int(payload["removed_face_count"]),
        input_vertex_count=int(payload["input_vertex_count"]),
        output_vertex_count=int(payload["output_vertex_count"]),
        removed_vertex_count=int(payload["removed_vertex_count"]),
        retained_face_count=int(payload["retained_face_count"]),
        min_area_mm2=float(payload["min_area_mm2"]),
    )

def _hole_fill_plan_from_payload(payload: dict[str, Any]) -> HoleFillPlanEntry:
    edge = payload["representative_edge"]
    return HoleFillPlanEntry(
        hole_index=int(payload["hole_index"]),
        representative_edge=(int(edge[0]), int(edge[1])),
        boundary_vertex_indices=[int(vertex_id) for vertex_id in payload["boundary_vertex_indices"]],
        boundary_edge_count=int(payload["boundary_edge_count"]),
        planned_triangles=int(payload["planned_triangles"]),
        skipped=bool(payload["skipped"]),
        skip_reason=None if payload["skip_reason"] is None else str(payload["skip_reason"]),
    )

def _hole_fill_plan_diagnostics_from_payload(payload: dict[str, Any]) -> HoleFillPlanDiagnostics:
    return HoleFillPlanDiagnostics(
        input_holes=int(payload["input_holes"]),
        planned_holes=int(payload["planned_holes"]),
        skipped_holes=int(payload["skipped_holes"]),
        total_boundary_edges=int(payload["total_boundary_edges"]),
        total_planned_triangles=int(payload["total_planned_triangles"]),
        max_edges=None if payload["max_edges"] is None else int(payload["max_edges"]),
        plans=[_hole_fill_plan_from_payload(plan) for plan in payload["plans"]],
    )

def _short_edge_entry_from_payload(payload: dict[str, Any]) -> ShortEdgeEntry:
    edge = payload["edge"]
    return ShortEdgeEntry(edge=(int(edge[0]), int(edge[1])), length_mm=float(payload["length_mm"]))


def _short_edge_diagnostics_from_payload(payload: dict[str, Any]) -> ShortEdgeDiagnostics:
    return ShortEdgeDiagnostics(
        critical_length_mm=float(payload["critical_length_mm"]),
        edge_count=int(payload["edge_count"]),
        short_edge_count=int(payload["short_edge_count"]),
        min_short_edge_length_mm=None
        if payload["min_short_edge_length_mm"] is None
        else float(payload["min_short_edge_length_mm"]),
        max_short_edge_length_mm=None
        if payload["max_short_edge_length_mm"] is None
        else float(payload["max_short_edge_length_mm"]),
        edges=[_short_edge_entry_from_payload(edge) for edge in payload["edges"]],
    )


def _degenerate_face_entry_from_payload(payload: dict[str, Any]) -> DegenerateFaceEntry:
    face = payload["face"]
    return DegenerateFaceEntry(
        face_index=int(payload["face_index"]),
        face=(int(face[0]), int(face[1]), int(face[2])),
        aspect_ratio=float(payload["aspect_ratio"]),
    )


def _degenerate_face_diagnostics_from_payload(payload: dict[str, Any]) -> DegenerateFaceDiagnostics:
    return DegenerateFaceDiagnostics(
        critical_aspect_ratio=float(payload["critical_aspect_ratio"]),
        face_count=int(payload["face_count"]),
        degenerate_face_count=int(payload["degenerate_face_count"]),
        min_degenerate_aspect_ratio=None
        if payload["min_degenerate_aspect_ratio"] is None
        else float(payload["min_degenerate_aspect_ratio"]),
        max_degenerate_aspect_ratio=None
        if payload["max_degenerate_aspect_ratio"] is None
        else float(payload["max_degenerate_aspect_ratio"]),
        faces=[_degenerate_face_entry_from_payload(face) for face in payload["faces"]],
    )


def _multiple_edge_entry_from_payload(payload: dict[str, Any]) -> MultipleEdgeEntry:
    vertex_pair = payload["vertex_pair"]
    return MultipleEdgeEntry(
        vertex_pair=(int(vertex_pair[0]), int(vertex_pair[1])),
        topology_edge_count=int(payload["topology_edge_count"]),
        face_edge_occurrences=int(payload["face_edge_occurrences"]),
        forward_occurrences=int(payload["forward_occurrences"]),
        reverse_occurrences=int(payload["reverse_occurrences"]),
    )


def _multiple_edge_diagnostics_from_payload(payload: dict[str, Any]) -> MultipleEdgeDiagnostics:
    return MultipleEdgeDiagnostics(
        edge_count=int(payload["edge_count"]),
        multiple_edge_count=int(payload["multiple_edge_count"]),
        edges=[_multiple_edge_entry_from_payload(edge) for edge in payload["edges"]],
    )


def _multiple_edge_repair_report_from_payload(payload: dict[str, Any]) -> MultipleEdgeRepairReport:
    return MultipleEdgeRepairReport(
        input_edge_count=int(payload["input_edge_count"]),
        output_edge_count=int(payload["output_edge_count"]),
        input_multiple_edge_count=int(payload["input_multiple_edge_count"]),
        output_multiple_edge_count=int(payload["output_multiple_edge_count"]),
        split_edge_count=int(payload["split_edge_count"]),
        split_face_count=int(payload["split_face_count"]),
        added_vertex_count=int(payload["added_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_face_count=int(payload["output_face_count"]),
    )


def _duplicate_multi_hole_vertices_report_from_payload(payload: dict[str, Any]) -> DuplicateMultiHoleVerticesReport:
    return DuplicateMultiHoleVerticesReport(
        input_multi_hole_vertex_count=int(payload["input_multi_hole_vertex_count"]),
        output_multi_hole_vertex_count=int(payload["output_multi_hole_vertex_count"]),
        duplicated_vertex_count=int(payload["duplicated_vertex_count"]),
        input_vertex_count=int(payload["input_vertex_count"]),
        output_vertex_count=int(payload["output_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_face_count=int(payload["output_face_count"]),
    )


def _not_smooth_face_diagnostics_from_payload(payload: dict[str, Any]) -> NotSmoothFaceDiagnostics:
    return NotSmoothFaceDiagnostics(
        min_angle_radians=float(payload["min_angle_radians"]),
        face_count=int(payload["face_count"]),
        not_smooth_face_count=int(payload["not_smooth_face_count"]),
        faces=[
            NotSmoothFaceEntry(
                face_index=int(face["face_index"]),
                face=(int(face["face"][0]), int(face["face"][1]), int(face["face"][2])),
                angle_delta_radians=float(face["angle_delta_radians"]),
            )
            for face in payload["faces"]
        ],
    )


def _hole_report_from_payload(payload: dict[str, Any]) -> HoleFillReport:
    return HoleFillReport(
        input_holes=int(payload["input_holes"]),
        filled_holes=int(payload["filled_holes"]),
        added_vertices=int(payload["added_vertices"]),
        added_faces=int(payload["added_faces"]),
        new_face_indices=[int(face) for face in payload.get("new_face_indices", [])],
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


def _fix_self_intersections_relax_report_from_payload(payload: dict[str, Any]) -> FixSelfIntersectionsRelaxReport:
    return FixSelfIntersectionsRelaxReport(
        input_vertex_count=int(payload["input_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_vertex_count=int(payload["output_vertex_count"]),
        output_face_count=int(payload["output_face_count"]),
        input_self_intersections=int(payload["input_self_intersections"]),
        output_self_intersections=int(payload["output_self_intersections"]),
        relaxed_face_count=int(payload["relaxed_face_count"]),
        moved_vertex_count=int(payload["moved_vertex_count"]),
        relax_iterations=int(payload["relax_iterations"]),
        max_expand=int(payload["max_expand"]),
        force=float(payload["force"]),
        method=str(payload["method"]),
        subdivide_edge_len_disabled=bool(payload["subdivide_edge_len_disabled"]),
        topology_changed=bool(payload["topology_changed"]),
    )

