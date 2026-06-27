from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, TunnelDiagnostics, TunnelEliminationReport


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _tunnel_diagnostics_from_payload(payload: dict[str, Any]) -> TunnelDiagnostics:
    return TunnelDiagnostics(
        vertex_count=int(payload["vertex_count"]),
        face_count=int(payload["face_count"]),
        edge_count=int(payload["edge_count"]),
        connected_component_count=int(payload["connected_component_count"]),
        boundary_edge_count=int(payload["boundary_edge_count"]),
        nonmanifold_edge_count=int(payload["nonmanifold_edge_count"]),
        euler_characteristic=int(payload["euler_characteristic"]),
        genus=None if payload["genus"] is None else int(payload["genus"]),
        tunnel_count=int(payload["tunnel_count"]),
        closed=bool(payload["closed"]),
    )


def _mesh_from_payload(source: MeshDocument, payload: dict[str, Any]) -> MeshDocument:
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return MeshDocument(vertices, faces, unit=source.unit, metadata=dict(source.metadata))


def _tunnel_elimination_report_from_payload(payload: dict[str, Any]) -> TunnelEliminationReport:
    return TunnelEliminationReport(
        input_face_count=int(payload["input_face_count"]),
        detected_tunnel_face_count=int(payload["detected_tunnel_face_count"]),
        removed_face_count=int(payload["removed_face_count"]),
        filled_holes=int(payload["filled_holes"]),
        added_faces=int(payload["added_faces"]),
        output_face_count=int(payload["output_face_count"]),
        output_boundary_edge_count=int(payload["output_boundary_edge_count"]),
        output_tunnel_count=int(payload["output_tunnel_count"]),
        tunnel_face_indices=[int(face_index) for face_index in payload["tunnel_face_indices"]],
    )


def tunnel_diagnostics(mesh: MeshDocument) -> TunnelDiagnostics | None:
    kernel = _require_rust_kernel("tunnel_diagnostics")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _tunnel_diagnostics_from_payload(payload)


def detect_tunnel_faces(mesh: MeshDocument) -> list[int] | None:
    kernel = _require_rust_kernel("detect_tunnel_faces")
    return [int(face_index) for face_index in kernel(mesh.vertices, mesh.faces)]


def eliminate_tunnels(mesh: MeshDocument) -> tuple[MeshDocument, TunnelEliminationReport] | None:
    kernel = _require_rust_kernel("eliminate_tunnels")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    repaired = _mesh_from_payload(mesh, payload)
    return repaired, _tunnel_elimination_report_from_payload(payload["report"])
