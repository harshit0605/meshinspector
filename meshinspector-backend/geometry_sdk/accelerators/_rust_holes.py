from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import (
    HoleComplicatingFaceEntry,
    HoleComplicatingFacesDiagnostics,
    MeshDocument,
    RepeatedHoleBoundaryVertexEntry,
    RepeatedHoleBoundaryVerticesDiagnostics,
    RemoveHoleComplicatingFacesReport,
)


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _repeated_vertex_entry_from_payload(payload: dict[str, Any]) -> RepeatedHoleBoundaryVertexEntry:
    return RepeatedHoleBoundaryVertexEntry(
        vertex_index=int(payload["vertex_index"]),
        hole_indices=[int(hole_index) for hole_index in payload["hole_indices"]],
        occurrences=int(payload["occurrences"]),
    )


def _repeated_vertices_diagnostics_from_payload(payload: dict[str, Any]) -> RepeatedHoleBoundaryVerticesDiagnostics:
    return RepeatedHoleBoundaryVerticesDiagnostics(
        input_holes=int(payload["input_holes"]),
        repeated_vertex_count=int(payload["repeated_vertex_count"]),
        vertices=[_repeated_vertex_entry_from_payload(entry) for entry in payload["vertices"]],
    )


def _mesh_from_payload(source: MeshDocument, payload: dict[str, Any]) -> MeshDocument:
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return MeshDocument(vertices, faces, unit=source.unit, metadata=dict(source.metadata))


def _complicating_face_entry_from_payload(payload: dict[str, Any]) -> HoleComplicatingFaceEntry:
    return HoleComplicatingFaceEntry(
        repeated_vertex_index=int(payload["repeated_vertex_index"]),
        face_index=int(payload["face_index"]),
    )


def _complicating_faces_diagnostics_from_payload(payload: dict[str, Any]) -> HoleComplicatingFacesDiagnostics:
    return HoleComplicatingFacesDiagnostics(
        input_repeated_vertex_count=int(payload["input_repeated_vertex_count"]),
        complicating_face_count=int(payload["complicating_face_count"]),
        faces=[_complicating_face_entry_from_payload(entry) for entry in payload["faces"]],
    )


def _remove_complicating_faces_report_from_payload(payload: dict[str, Any]) -> RemoveHoleComplicatingFacesReport:
    return RemoveHoleComplicatingFacesReport(
        input_face_count=int(payload["input_face_count"]),
        output_face_count=int(payload["output_face_count"]),
        removed_face_count=int(payload["removed_face_count"]),
        input_repeated_vertex_count=int(payload["input_repeated_vertex_count"]),
        output_repeated_vertex_count=int(payload["output_repeated_vertex_count"]),
    )


def repeated_hole_boundary_vertices_diagnostics(
    mesh: MeshDocument,
) -> RepeatedHoleBoundaryVerticesDiagnostics | None:
    kernel = _require_rust_kernel("repeated_hole_boundary_vertices_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _repeated_vertices_diagnostics_from_payload(payload)


def hole_complicating_faces_diagnostics(mesh: MeshDocument) -> HoleComplicatingFacesDiagnostics | None:
    kernel = _require_rust_kernel("hole_complicating_faces_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _complicating_faces_diagnostics_from_payload(payload)


def remove_hole_complicating_faces(
    mesh: MeshDocument,
) -> tuple[MeshDocument, RemoveHoleComplicatingFacesReport] | None:
    kernel = _require_rust_kernel("remove_hole_complicating_faces")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _mesh_from_payload(mesh, payload), _remove_complicating_faces_report_from_payload(payload["report"])
