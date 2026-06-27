from __future__ import annotations

from collections.abc import Iterable
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import ExactBooleanMeshResult, MeshDocument, RegionEntry, SDFGrid


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _mesh_from_payload(
    source: MeshDocument,
    payload: dict[str, Any],
    *,
    metadata: dict[str, Any] | None = None,
) -> MeshDocument:
    output_metadata = dict(source.metadata)
    if metadata is not None:
        output_metadata.update(metadata)
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        unit=source.unit,
        metadata=output_metadata,
    )


def _mesh_from_grid_payload(grid: SDFGrid, payload: dict[str, Any]) -> MeshDocument:
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={"source": "sdf_grid_mesh", "voxel_size_mm": float(grid.voxel_size_mm)},
    )


def _region_buffers(regions: Iterable[RegionEntry]) -> tuple[list[str], np.ndarray, np.ndarray]:
    region_ids: list[str] = []
    vertex_offsets = [0]
    flat_vertex_indices: list[int] = []
    for region in regions:
        region_ids.append(str(region.region_id))
        flat_vertex_indices.extend(int(index) for index in np.asarray(region.vertex_indices, dtype=np.int64).reshape(-1))
        vertex_offsets.append(len(flat_vertex_indices))
    return (
        region_ids,
        np.asarray(vertex_offsets, dtype=np.int64),
        np.asarray(flat_vertex_indices, dtype=np.int64),
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


def voxel_thicken_mesh(
    mesh: MeshDocument,
    *,
    thickness_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    payload = _require_rust_kernel("voxel_thicken_mesh")(
        mesh.vertices,
        mesh.faces,
        float(thickness_mm),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        extractor,
        bool(refine),
    )
    return _mesh_from_payload(mesh, payload)


def voxel_weighted_shell_mesh(
    mesh: MeshDocument,
    *,
    regions: Iterable[RegionEntry],
    region_weights: dict[str, float],
    offset_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    interpolation_distance_mm: float = 0.0,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    region_ids, vertex_offsets, vertex_indices = _region_buffers(regions)
    weighted_region_ids = [str(region_id) for region_id in region_weights]
    weights = np.asarray([float(region_weights[region_id]) for region_id in weighted_region_ids], dtype=np.float32)
    payload = _require_rust_kernel("voxel_weighted_shell_mesh")(
        mesh.vertices,
        mesh.faces,
        region_ids,
        vertex_offsets,
        vertex_indices,
        weighted_region_ids,
        weights,
        float(offset_mm),
        float(interpolation_distance_mm),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        extractor,
        bool(refine),
    )
    return _mesh_from_payload(
        mesh,
        payload,
        metadata={
            "source": "rust_voxel_weighted_shell",
            "meshlib_reference": "MR::WeightedShell::meshShell",
            "offset_mm": float(offset_mm),
            "region_weights": {str(key): float(value) for key, value in region_weights.items()},
            "interpolation_distance_mm": float(interpolation_distance_mm),
        },
    )


def voxel_partial_offset_mesh(
    mesh: MeshDocument,
    *,
    regions: Iterable[RegionEntry],
    selected_region_ids: list[str],
    offset_mm: float,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = False,
) -> MeshDocument:
    region_ids, vertex_offsets, vertex_indices = _region_buffers(regions)
    selected_ids = [str(region_id) for region_id in selected_region_ids]
    payload = _require_rust_kernel("voxel_partial_offset_mesh")(
        mesh.vertices,
        mesh.faces,
        region_ids,
        vertex_offsets,
        vertex_indices,
        selected_ids,
        float(offset_mm),
        float(voxel_size_mm),
        None if padding_mm is None else float(padding_mm),
        extractor,
        bool(refine),
    )
    return _mesh_from_payload(
        mesh,
        payload,
        metadata={
            "source": "rust_voxel_partial_offset",
            "meshlib_reference": "MR::partialOffsetMesh",
            "meshlib_source": "MeshLib/source/MRVoxels/MRPartialOffset.*",
            "offset_mm": float(offset_mm),
            "selected_region_ids": selected_ids,
        },
    )


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


def exact_boolean_mesh(
    a: MeshDocument,
    b: MeshDocument,
    *,
    operation: str,
    leaf_size: int = 16,
    epsilon: float = 1e-8,
) -> ExactBooleanMeshResult:
    payload = _require_rust_kernel("exact_boolean_mesh")(
        a.vertices,
        a.faces,
        b.vertices,
        b.faces,
        operation,
        int(leaf_size),
        float(epsilon),
    )
    mesh = _mesh_from_payload(
        a,
        payload,
        metadata={
            "source": "rust_exact_boolean",
            "operation": operation,
        },
    )
    return ExactBooleanMeshResult(
        mesh=mesh,
        operation=operation,
        diagnostics=dict(payload["diagnostics"]),
    )


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
