from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import (
    DuplicateNonManifoldVerticesReport,
    MeshDocument,
    NonManifoldEdgeRepairReport,
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


def _report_from_payload(payload: dict[str, Any]) -> NonManifoldEdgeRepairReport:
    return NonManifoldEdgeRepairReport(
        input_nonmanifold_edge_count=int(payload["input_nonmanifold_edge_count"]),
        output_nonmanifold_edge_count=int(payload["output_nonmanifold_edge_count"]),
        removed_face_count=int(payload["removed_face_count"]),
        input_vertex_count=int(payload["input_vertex_count"]),
        output_vertex_count=int(payload["output_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_face_count=int(payload["output_face_count"]),
    )


def repair_nonmanifold_edges(mesh: MeshDocument) -> tuple[MeshDocument, NonManifoldEdgeRepairReport]:
    kernel = _require_rust_kernel("repair_nonmanifold_edges")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _mesh_from_payload(mesh, payload), _report_from_payload(payload["report"])


def _duplicate_report_from_payload(payload: dict[str, Any]) -> DuplicateNonManifoldVerticesReport:
    return DuplicateNonManifoldVerticesReport(
        input_nonmanifold_vertex_count=int(payload["input_nonmanifold_vertex_count"]),
        output_nonmanifold_vertex_count=int(payload["output_nonmanifold_vertex_count"]),
        duplicated_vertex_count=int(payload["duplicated_vertex_count"]),
        input_vertex_count=int(payload["input_vertex_count"]),
        output_vertex_count=int(payload["output_vertex_count"]),
        input_face_count=int(payload["input_face_count"]),
        output_face_count=int(payload["output_face_count"]),
    )


def duplicate_nonmanifold_vertices(
    mesh: MeshDocument,
    *,
    region_face_indices: Sequence[int] | None = None,
) -> tuple[MeshDocument, DuplicateNonManifoldVerticesReport]:
    kernel = _require_rust_kernel("duplicate_nonmanifold_vertices")
    region = None if region_face_indices is None else [int(index) for index in region_face_indices]
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, region)
    return _mesh_from_payload(mesh, payload), _duplicate_report_from_payload(payload["report"])
