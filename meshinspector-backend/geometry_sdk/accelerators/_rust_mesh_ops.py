from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, SDFGrid


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _mesh_from_payload(source: MeshDocument, payload: dict[str, Any]) -> MeshDocument:
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        unit=source.unit,
        metadata=dict(source.metadata),
    )


def _mesh_from_grid_payload(grid: SDFGrid, payload: dict[str, Any]) -> MeshDocument:
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={"source": "sdf_grid_mesh", "voxel_size_mm": float(grid.voxel_size_mm)},
    )


def extract_grid_mesh(
    grid: SDFGrid,
    *,
    extractor: str = "marching",
    refine: bool = False,
    smooth_iterations: int = 1,
    smooth_strength: float = 0.2,
    projection_iterations: int = 3,
) -> MeshDocument:
    payload = _require_rust_kernel("extract_grid_mesh")(
        np.asarray(grid.values, dtype=np.float32).reshape(-1),
        np.asarray(grid.origin, dtype=np.float64),
        np.asarray(grid.shape, dtype=np.int64),
        float(grid.voxel_size_mm),
        extractor,
        bool(refine),
        int(smooth_iterations),
        float(np.clip(smooth_strength, 0.0, 1.0)),
        int(projection_iterations),
    )
    return _mesh_from_grid_payload(grid, payload)


def voxel_offset_mesh(
    mesh: MeshDocument,
    *,
    offset_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    payload = _require_rust_kernel("voxel_offset_mesh")(
        mesh.vertices,
        mesh.faces,
        float(offset_mm),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        extractor,
        bool(refine),
    )
    return _mesh_from_payload(mesh, payload)


def voxel_shell_mesh(
    mesh: MeshDocument,
    *,
    wall_thickness_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    payload = _require_rust_kernel("voxel_shell_mesh")(
        mesh.vertices,
        mesh.faces,
        float(wall_thickness_mm),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        extractor,
        bool(refine),
    )
    return _mesh_from_payload(mesh, payload)


def voxel_boolean_mesh(
    a: MeshDocument,
    b: MeshDocument,
    *,
    operation: str,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    origin_phase: tuple[float, float, float] | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    payload = _require_rust_kernel("voxel_boolean_mesh")(
        a.vertices,
        a.faces,
        b.vertices,
        b.faces,
        operation,
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        None if origin_phase is None else np.asarray(origin_phase, dtype=np.float64),
        extractor,
        bool(refine),
    )
    return _mesh_from_payload(a, payload)


def global_thicken_mesh(mesh: MeshDocument, *, min_target_thickness_mm: float) -> MeshDocument:
    payload = _require_rust_kernel("global_thicken_mesh")(
        mesh.vertices,
        mesh.faces,
        float(min_target_thickness_mm),
    )
    result = _mesh_from_payload(mesh, payload)
    result.metadata.update(
        {
            "operation": "global_thicken",
            "min_target_thickness_mm": float(min_target_thickness_mm),
        }
    )
    return result
