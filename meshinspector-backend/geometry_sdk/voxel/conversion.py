"""MeshInspector-style voxel object to mesh conversion helpers."""

from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_voxel
from geometry_sdk.types import MeshDocument, VoxelVolume


def voxel_volume_from_meshlib_values(
    values: Any,
    *,
    dimensions: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    grid_level_set: bool,
    scalar_type: str,
    iso_value: float | None = None,
    min_value: float | None = None,
    max_value: float | None = None,
) -> VoxelVolume:
    flattened = _flatten_numeric_values(values)
    expected = int(dimensions[0] * dimensions[1] * dimensions[2])
    if len(flattened) != expected:
        raise ValueError("values size must match dimensions")
    volume_values = np.asarray(flattened, dtype=np.float32).reshape(dimensions, order="F")
    rust_min_value, rust_max_value = _rust_voxel.voxel_value_range(volume_values)
    metadata = {"default_iso_value": iso_value} if iso_value is not None else {}
    return VoxelVolume(
        dimensions=dimensions,
        voxel_size=voxel_size,
        grid_level_set=grid_level_set,
        scalar_type=scalar_type,
        values=volume_values,
        min_value=float(min_value if min_value is not None else rust_min_value),
        max_value=float(max_value if max_value is not None else rust_max_value),
        metadata=metadata,
    )


def _flatten_numeric_values(values: object) -> list[float]:
    if hasattr(values, "reshape") and hasattr(values, "tolist"):
        # x-fastest (Fortran) order — the convention every voxel op indexes by
        # (idx = x + y*nx + z*nx*ny) and how VoxelVolume.values is built. A bare
        # reshape(-1) is C-order (z-fastest) and transposed the serialized volume.
        values = np.ravel(values, order="F").tolist()
    elif hasattr(values, "tolist"):
        values = values.tolist()
    if isinstance(values, (list, tuple)):
        flattened: list[float] = []
        for value in values:
            flattened.extend(_flatten_numeric_values(value))
        return flattened
    return [float(values)]


def _values_shape_voxel_size(
    volume_or_values: VoxelVolume | np.ndarray,
    shape: tuple[int, int, int] | None,
    voxel_size: tuple[float, float, float] | None,
) -> tuple[np.ndarray, tuple[int, int, int], tuple[float, float, float], dict[str, Any]]:
    metadata: dict[str, Any] = {}
    if isinstance(volume_or_values, VoxelVolume):
        resolved_shape = volume_or_values.dimensions
        resolved_voxel_size = volume_or_values.voxel_size if voxel_size is None else voxel_size
        metadata.update(volume_or_values.metadata)
        metadata["grid_level_set"] = volume_or_values.grid_level_set
        return (
            np.ravel(volume_or_values.values, order="F"),
            resolved_shape,
            resolved_voxel_size,
            metadata,
        )

    values = np.asarray(volume_or_values, dtype=np.float32)
    if shape is None:
        if values.ndim != 3:
            raise ValueError("shape is required when values are not a 3D array")
        shape = tuple(int(value) for value in values.shape)  # type: ignore[assignment]
    resolved_voxel_size = (1.0, 1.0, 1.0) if voxel_size is None else voxel_size
    if values.ndim == 3 and tuple(int(value) for value in values.shape) == tuple(shape):
        return np.ravel(values, order="F"), shape, resolved_voxel_size, metadata
    return values.reshape(-1), shape, resolved_voxel_size, metadata


def voxel_to_mesh_simple(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    voxel_size: tuple[float, float, float] | None = None,
    iso_value: float | None = None,
) -> MeshDocument:
    values, resolved_shape, resolved_voxel_size, source_metadata = _values_shape_voxel_size(
        volume_or_values,
        shape,
        voxel_size,
    )
    resolved_iso = (
        float(source_metadata.get("default_iso_value"))
        if iso_value is None and "default_iso_value" in source_metadata
        else float(_rust_voxel.voxel_default_iso_value(values) if iso_value is None else iso_value)
    )
    level_set = bool(source_metadata.get("grid_level_set", False))
    payload = _rust_voxel.voxel_to_mesh_simple_values(
        values,
        shape=resolved_shape,
        voxel_size=resolved_voxel_size,
        iso_value=resolved_iso,
        level_set=level_set,
    )
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={
            "source": "voxel_to_mesh_simple",
            "iso_value": resolved_iso,
            "voxel_size": tuple(float(value) for value in resolved_voxel_size),
            "dimensions": tuple(int(value) for value in resolved_shape),
            "meshlib_reference": "ObjectVoxels::recalculateIsoSurface",
            "meshlib_dense_volume_less_inside": False,
            "meshlib_level_set_less_inside": level_set,
            "grid_level_set": level_set,
            "parity_status": "partial_dual_marching_cubes_pending",
        },
    )


def voxel_to_mesh_dual(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    voxel_size: tuple[float, float, float] | None = None,
    iso_value: float | None = None,
    adaptivity: float = 0.0,
    max_faces: int | None = None,
    max_vertices: int | None = None,
    relax_disoriented_triangles: bool = True,
) -> MeshDocument:
    values, resolved_shape, resolved_voxel_size, source_metadata = _values_shape_voxel_size(
        volume_or_values,
        shape,
        voxel_size,
    )
    resolved_iso = (
        float(source_metadata.get("default_iso_value"))
        if iso_value is None and "default_iso_value" in source_metadata
        else float(_rust_voxel.voxel_default_iso_value(values) if iso_value is None else iso_value)
    )
    level_set = bool(source_metadata.get("grid_level_set", False))
    payload = _rust_voxel.voxel_to_mesh_dual_values(
        values,
        shape=resolved_shape,
        voxel_size=resolved_voxel_size,
        iso_value=resolved_iso,
        level_set=level_set,
        adaptivity=adaptivity,
        max_faces=max_faces,
        max_vertices=max_vertices,
        relax_disoriented_triangles=relax_disoriented_triangles,
    )
    metadata = {
        "source": "voxel_to_mesh_dual",
        "iso_value": resolved_iso,
        "voxel_size": tuple(float(value) for value in resolved_voxel_size),
        "dimensions": tuple(int(value) for value in resolved_shape),
        "meshlib_reference": "ObjectVoxels::recalculateIsoSurface",
        "meshlib_algorithm_reference": "openvdb::tools::VolumeToMesh dense dual-contouring slice",
        "meshlib_dense_volume_less_inside": False,
        "meshlib_level_set_less_inside": level_set,
        "grid_level_set": level_set,
        "parity_status": "dense_dual_contouring_backed_sparse_openvdb_volume_to_mesh_pending",
    }
    if max_faces is not None:
        metadata["max_faces"] = int(max_faces)
    if max_vertices is not None:
        metadata["max_vertices"] = int(max_vertices)
    if float(adaptivity) != 0.0:
        metadata["adaptivity"] = float(adaptivity)
    if not bool(relax_disoriented_triangles):
        metadata["relax_disoriented_triangles"] = False
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata=metadata,
    )


def voxel_to_mesh_dual_vdb_payload(
    model_bytes: bytes,
    *,
    dimensions: tuple[int, int, int] = (1, 1, 1),
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0),
    iso_value: float = 0.0,
    adaptivity: float = 0.0,
    max_faces: int | None = None,
    max_vertices: int | None = None,
    relax_disoriented_triangles: bool = True,
) -> MeshDocument:
    payload = _rust_voxel.meshlib_vdb_payload_to_dual_mesh(
        model_bytes,
        dimensions=dimensions,
        voxel_size=voxel_size,
        iso_value=float(iso_value),
        adaptivity=adaptivity,
        max_faces=max_faces,
        max_vertices=max_vertices,
        relax_disoriented_triangles=relax_disoriented_triangles,
    )
    metadata = {
        "source": "voxel_to_mesh_dual_vdb_payload",
        "iso_value": float(iso_value),
        "fallback_dimensions": tuple(int(value) for value in dimensions),
        "fallback_voxel_size": tuple(float(value) for value in voxel_size),
        "meshlib_reference": "ObjectVoxels::recalculateIsoSurface",
        "meshlib_algorithm_reference": "openvdb::tools::VolumeToMesh direct .vdb FloatGrid dense decode slice",
        "grid_level_set": True,
        "parity_status": "openvdb_dense_floatgrid_dual_meshing_backed_sparse_adaptivity_pending",
    }
    if max_faces is not None:
        metadata["max_faces"] = int(max_faces)
    if max_vertices is not None:
        metadata["max_vertices"] = int(max_vertices)
    if float(adaptivity) != 0.0:
        metadata["adaptivity"] = float(adaptivity)
    if not bool(relax_disoriented_triangles):
        metadata["relax_disoriented_triangles"] = False
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata=metadata,
    )


def voxel_move_mesh_to_max_deriv(
    mesh: MeshDocument,
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    voxel_size: tuple[float, float, float] | None = None,
    iters: int = 30,
    sample_points: int = 6,
    degree: int = 3,
    outlier_threshold: float = 1.0,
    intermediate_smooth_force: float = 0.3,
    preparation_smooth_force: float = 0.1,
    smooth_shift_iterations: int = 15,
    final_relax_iterations: int = 15,
    final_relax_force: float = 0.01,
) -> MeshDocument:
    values, resolved_shape, resolved_voxel_size, _source_metadata = _values_shape_voxel_size(
        volume_or_values,
        shape,
        voxel_size,
    )
    payload = _rust_voxel.voxel_move_mesh_to_max_deriv_values(
        mesh,
        values,
        shape=resolved_shape,
        voxel_size=resolved_voxel_size,
        iters=iters,
        sample_points=sample_points,
        degree=degree,
        outlier_threshold=outlier_threshold,
        intermediate_smooth_force=intermediate_smooth_force,
        preparation_smooth_force=preparation_smooth_force,
        smooth_shift_iterations=smooth_shift_iterations,
        final_relax_iterations=final_relax_iterations,
        final_relax_force=final_relax_force,
    )
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "source": "voxel_move_mesh_to_max_deriv",
            "meshlib_reference": "MR::moveMeshToVoxelMaxDeriv",
            "corrected_indices": [int(index) for index in payload["corrected_indices"]],
            "voxel_size": tuple(float(value) for value in resolved_voxel_size),
            "dimensions": tuple(int(value) for value in resolved_shape),
            "settings": {
                "iters": int(iters),
                "sample_points": int(sample_points),
                "degree": int(degree),
                "outlier_threshold": float(outlier_threshold),
                "intermediate_smooth_force": float(intermediate_smooth_force),
                "preparation_smooth_force": float(preparation_smooth_force),
                "smooth_shift_iterations": int(smooth_shift_iterations),
                "final_relax_iterations": int(final_relax_iterations),
                "final_relax_force": float(final_relax_force),
            },
        }
    )
    return MeshDocument(vertices=vertices, faces=np.array(mesh.faces, copy=True), unit=mesh.unit, metadata=metadata)


def voxel_to_mesh_smart(
    volume_or_values: VoxelVolume | np.ndarray,
    *,
    shape: tuple[int, int, int] | None = None,
    voxel_size: tuple[float, float, float] | None = None,
    iso_value: float | None = None,
    iters: int = 30,
    sample_points: int = 6,
    degree: int = 3,
    outlier_threshold: float = 1.0,
    intermediate_smooth_force: float = 0.3,
    preparation_smooth_force: float = 0.1,
    smooth_shift_iterations: int = 15,
    final_relax_iterations: int = 15,
    final_relax_force: float = 0.01,
) -> MeshDocument:
    values, resolved_shape, resolved_voxel_size, source_metadata = _values_shape_voxel_size(
        volume_or_values,
        shape,
        voxel_size,
    )
    resolved_iso = (
        float(source_metadata.get("default_iso_value"))
        if iso_value is None and "default_iso_value" in source_metadata
        else float(_rust_voxel.voxel_default_iso_value(values) if iso_value is None else iso_value)
    )
    level_set = bool(source_metadata.get("grid_level_set", False))
    payload = _rust_voxel.voxel_to_mesh_smart_values(
        values,
        shape=resolved_shape,
        voxel_size=resolved_voxel_size,
        iso_value=resolved_iso,
        level_set=level_set,
        iters=iters,
        sample_points=sample_points,
        degree=degree,
        outlier_threshold=outlier_threshold,
        intermediate_smooth_force=intermediate_smooth_force,
        preparation_smooth_force=preparation_smooth_force,
        smooth_shift_iterations=smooth_shift_iterations,
        final_relax_iterations=final_relax_iterations,
        final_relax_force=final_relax_force,
    )
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={
            "source": "voxel_to_mesh_smart",
            "meshlib_reference": "ObjectVoxels::recalculateIsoSurface + MR::moveMeshToVoxelMaxDeriv",
            "iso_value": resolved_iso,
            "voxel_size": tuple(float(value) for value in resolved_voxel_size),
            "dimensions": tuple(int(value) for value in resolved_shape),
            "meshlib_dense_volume_less_inside": False,
            "meshlib_level_set_less_inside": level_set,
            "grid_level_set": level_set,
            "corrected_indices": [int(index) for index in payload["corrected_indices"]],
            "simple_conversion": {
                "source": "voxel_to_mesh_simple",
                "iso_value": resolved_iso,
                "voxel_size": tuple(float(value) for value in resolved_voxel_size),
                "dimensions": tuple(int(value) for value in resolved_shape),
                "meshlib_reference": "ObjectVoxels::recalculateIsoSurface",
                "meshlib_dense_volume_less_inside": False,
                "meshlib_level_set_less_inside": level_set,
                "grid_level_set": level_set,
                "parity_status": "partial_dual_marching_cubes_pending",
            },
            "smart_conversion": {
                "source": "voxel_move_mesh_to_max_deriv",
                "meshlib_reference": "MR::moveMeshToVoxelMaxDeriv",
                "corrected_indices": [int(index) for index in payload["corrected_indices"]],
                "voxel_size": tuple(float(value) for value in resolved_voxel_size),
                "dimensions": tuple(int(value) for value in resolved_shape),
                "settings": {
                    "iters": int(iters),
                    "sample_points": int(sample_points),
                    "degree": int(degree),
                    "outlier_threshold": float(outlier_threshold),
                    "intermediate_smooth_force": float(intermediate_smooth_force),
                    "preparation_smooth_force": float(preparation_smooth_force),
                    "smooth_shift_iterations": int(smooth_shift_iterations),
                    "final_relax_iterations": int(final_relax_iterations),
                    "final_relax_force": float(final_relax_force),
                },
            },
            "parity_status": "smart_refinement_degree3_to6_backed_dual_marching_cubes_pending",
        },
    )
