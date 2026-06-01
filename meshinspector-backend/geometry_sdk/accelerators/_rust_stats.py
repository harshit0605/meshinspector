from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, MeshStats


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def mesh_stats(mesh: MeshDocument) -> MeshStats:
    kernel = _require_rust_kernel("mesh_stats")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return MeshStats(
        bbox_min=tuple(float(x) for x in payload["bbox_min"]),
        bbox_max=tuple(float(x) for x in payload["bbox_max"]),
        bbox_size=tuple(float(x) for x in payload["bbox_size"]),
        surface_area_mm2=float(payload["surface_area_mm2"]),
        volume_mm3=float(payload["volume_mm3"]),
        vertex_count=int(payload["vertex_count"]),
        face_count=int(payload["face_count"]),
        connected_components=int(payload["connected_components"]),
        boundary_edge_count=int(payload["boundary_edge_count"]),
    )
