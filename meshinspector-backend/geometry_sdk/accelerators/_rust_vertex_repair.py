from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


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


def remove_unreferenced_vertices(mesh: MeshDocument) -> tuple[MeshDocument, int] | None:
    kernel = _require_rust_kernel("remove_unreferenced_vertices")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return _mesh_from_payload(mesh, payload), int(payload["changed_count"])


def merge_close_vertices(mesh: MeshDocument, *, tolerance: float = 1e-6) -> tuple[MeshDocument, int] | None:
    kernel = _require_rust_kernel("merge_close_vertices")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(tolerance))
    return _mesh_from_payload(mesh, payload), int(payload["changed_count"])


def unite_close_vertices(
    mesh: MeshDocument,
    *,
    close_dist: float = 0.0,
    unite_only_boundary: bool = True,
) -> tuple[MeshDocument, int] | None:
    kernel = _require_rust_kernel("unite_close_vertices")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(close_dist), bool(unite_only_boundary))
    return _mesh_from_payload(mesh, payload), int(payload["changed_count"])
