from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import (
    CreaseEdgeDiagnostics,
    CreaseEdgeEntry,
    CreaseRepairPlanDiagnostics,
    CreaseRepairPlanRegion,
    FixMeshCreasesReport,
    MeshDocument,
)


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _crease_edge_diagnostics_from_payload(payload: dict[str, Any]) -> CreaseEdgeDiagnostics:
    return CreaseEdgeDiagnostics(
        angle_from_planar_radians=float(payload["angle_from_planar_radians"]),
        min_component_length_mm=(
            None if payload["min_component_length_mm"] is None else float(payload["min_component_length_mm"])
        ),
        min_branch_length_mm=(
            None if payload["min_branch_length_mm"] is None else float(payload["min_branch_length_mm"])
        ),
        edge_count=int(payload["edge_count"]),
        raw_crease_edge_count=int(payload["raw_crease_edge_count"]),
        crease_edge_count=int(payload["crease_edge_count"]),
        edges=[
            CreaseEdgeEntry(
                edge=(int(edge["edge"][0]), int(edge["edge"][1])),
                face_indices=(int(edge["face_indices"][0]), int(edge["face_indices"][1])),
                dihedral_cosine=float(edge["dihedral_cosine"]),
            )
            for edge in payload["edges"]
        ],
    )


def _crease_repair_plan_diagnostics_from_payload(payload: dict[str, Any]) -> CreaseRepairPlanDiagnostics:
    return CreaseRepairPlanDiagnostics(
        angle_from_planar_radians=float(payload["angle_from_planar_radians"]),
        critical_tri_aspect_ratio=float(payload["critical_tri_aspect_ratio"]),
        crease_edge_count=int(payload["crease_edge_count"]),
        planned_region_count=int(payload["planned_region_count"]),
        planned_face_count=int(payload["planned_face_count"]),
        regions=[
            CreaseRepairPlanRegion(
                crease_edge=(int(region["crease_edge"][0]), int(region["crease_edge"][1])),
                selected_origin_vertex=int(region["selected_origin_vertex"]),
                selected_face_indices=[int(face_index) for face_index in region["selected_face_indices"]],
            )
            for region in payload["regions"]
        ],
    )


def _fix_mesh_creases_report_from_payload(payload: dict[str, Any]) -> FixMeshCreasesReport:
    return FixMeshCreasesReport(
        input_face_count=int(payload["input_face_count"]),
        output_face_count=int(payload["output_face_count"]),
        input_crease_edge_count=int(payload["input_crease_edge_count"]),
        output_crease_edge_count=int(payload["output_crease_edge_count"]),
        repaired_region_count=int(payload["repaired_region_count"]),
        removed_face_count=int(payload["removed_face_count"]),
        added_face_count=int(payload["added_face_count"]),
        filled_hole_count=int(payload["filled_hole_count"]),
        skipped_hole_count=int(payload["skipped_hole_count"]),
        iteration_count=int(payload["iteration_count"]),
    )


def _mesh_from_payload(source: MeshDocument, payload: dict[str, Any]) -> MeshDocument:
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return MeshDocument(vertices, faces, unit=source.unit, metadata=dict(source.metadata))


def crease_edge_diagnostics(
    mesh: MeshDocument,
    *,
    angle_from_planar_radians: float,
    min_component_length_mm: float | None = None,
    min_branch_length_mm: float | None = None,
) -> CreaseEdgeDiagnostics | None:
    kernel = _require_rust_kernel("crease_edge_diagnostics")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        angle_from_planar_radians,
        min_component_length_mm,
        min_branch_length_mm,
    )
    return _crease_edge_diagnostics_from_payload(payload)


def crease_repair_plan_diagnostics(
    mesh: MeshDocument,
    *,
    angle_from_planar_radians: float,
    critical_tri_aspect_ratio: float,
) -> CreaseRepairPlanDiagnostics | None:
    kernel = _require_rust_kernel("crease_repair_plan_diagnostics")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        angle_from_planar_radians,
        critical_tri_aspect_ratio,
    )
    return _crease_repair_plan_diagnostics_from_payload(payload)


def fix_mesh_creases(
    mesh: MeshDocument,
    *,
    angle_from_planar_radians: float,
    critical_tri_aspect_ratio: float,
    max_iters: int,
) -> tuple[MeshDocument, FixMeshCreasesReport] | None:
    kernel = _require_rust_kernel("fix_mesh_creases")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        angle_from_planar_radians,
        critical_tri_aspect_ratio,
        max_iters,
    )
    return _mesh_from_payload(mesh, payload), _fix_mesh_creases_report_from_payload(payload["report"])
