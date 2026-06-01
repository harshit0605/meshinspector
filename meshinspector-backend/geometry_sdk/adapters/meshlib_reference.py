"""MeshLib reference helpers for parity tests.

These helpers are intentionally test-facing. Production services should not call
them while the SDK is being built in parallel.
"""

from __future__ import annotations

from pathlib import Path
from typing import Literal

import numpy as np

from geometry_sdk.analysis.artifacts import save_compare_npz as save_compare_artifact_npz
from geometry_sdk.analysis.artifacts import save_thickness_npz as save_thickness_artifact_npz

MeshLibBooleanOperation = Literal["union", "intersection", "difference"]


def _extract_mesh(result):
    if hasattr(result, "valid") and callable(result.valid) and not result.valid():
        error = getattr(result, "errorString", "")
        raise RuntimeError(error or "MeshLib operation failed")
    if hasattr(result, "mesh"):
        mesh = result.mesh
        return mesh() if callable(mesh) else mesh
    if hasattr(result, "resultMesh"):
        mesh = result.resultMesh
        return mesh() if callable(mesh) else mesh
    return result


def health_metrics(mesh_path: str | Path) -> dict[str, int | bool]:
    from meshlib import mrmeshpy as mm

    mesh = mm.loadMesh(str(mesh_path))
    try:
        self_colliding = mm.findSelfCollidingTrianglesBS(mesh).count()
    except Exception:
        self_colliding = 0
    try:
        holes = len(mesh.topology.findHoleRepresentiveEdges())
    except Exception:
        holes = 0
    return {
        "is_closed": holes == 0,
        "holes_count": int(holes),
        "self_intersections": int(self_colliding),
    }


def thickness_summary(mesh_path: str | Path, threshold_mm: float = 0.6) -> dict[str, float | int | None]:
    thickness = thickness_field(mesh_path)
    valid = thickness[np.isfinite(thickness)]
    if valid.size == 0:
        return {"min_mm": None, "avg_mm": None, "max_mm": None, "violation_count": 0}
    return {
        "min_mm": float(np.min(valid)),
        "avg_mm": float(np.mean(valid, dtype=np.float64)),
        "max_mm": float(np.max(valid)),
        "violation_count": int(np.sum(valid < threshold_mm, dtype=np.int64)),
    }


def thickness_field(mesh_path: str | Path) -> np.ndarray:
    from meshlib import mrmeshpy as mm

    mesh = mm.loadMesh(str(mesh_path))
    insphere_scalars = mm.computeInSphereThicknessAtVertices(mesh, mm.InSphereSearchSettings())
    ray_scalars = mm.computeRayThicknessAtVertices(mesh)
    insphere = np.array([float(insphere_scalars.vec[i]) for i in range(insphere_scalars.size())], dtype=np.float32)
    ray = np.array([float(ray_scalars.vec[i]) for i in range(ray_scalars.size())], dtype=np.float32)
    ray[np.logical_or(~np.isfinite(ray), ray > 1e10)] = np.nan
    thickness = np.where(np.isfinite(ray), np.minimum(insphere, ray), insphere)
    thickness[np.logical_or(~np.isfinite(thickness), thickness <= 0)] = np.nan
    return thickness.astype(np.float32)


def save_thickness_npz(
    mesh_path: str | Path,
    output_path: str | Path,
    *,
    threshold_mm: float = 0.6,
) -> Path:
    thickness = thickness_field(mesh_path)
    return save_thickness_artifact_npz(
        output_path,
        thickness,
        vertex_count=int(thickness.shape[0]),
        threshold_mm=threshold_mm,
    )


def signed_distance_values(
    source_path: str | Path,
    target_path: str | Path,
    *,
    max_expected_distance: float | None = None,
) -> np.ndarray:
    from meshlib import mrmeshpy as mm

    source = mm.loadMesh(str(source_path))
    target = mm.loadMesh(str(target_path))
    scalars = mm.findSignedDistances(source, target)
    values = np.array([float(scalars.vec[i]) for i in range(scalars.size())], dtype=np.float32)
    if max_expected_distance is not None:
        values = np.where(np.abs(values) <= float(max_expected_distance), values, np.nan)
    return values.astype(np.float32)


def signed_distance_summary(values: np.ndarray) -> dict[str, float | None]:
    finite = np.asarray(values, dtype=np.float32)
    finite = finite[np.isfinite(finite)]
    if finite.size == 0:
        return {"min_signed_distance_mm": None, "max_signed_distance_mm": None, "mean_signed_distance_mm": None}
    return {
        "min_signed_distance_mm": float(np.min(finite)),
        "max_signed_distance_mm": float(np.max(finite)),
        "mean_signed_distance_mm": float(np.mean(finite, dtype=np.float64)),
    }


def save_compare_npz(
    source_path: str | Path,
    target_path: str | Path,
    output_path: str | Path,
    *,
    other_version_id: str,
    max_expected_distance: float | None = None,
) -> Path:
    values = signed_distance_values(source_path, target_path, max_expected_distance=max_expected_distance)
    return save_compare_artifact_npz(
        output_path,
        values,
        vertex_count=int(values.shape[0]),
        other_version_id=other_version_id,
    )


def offset_mesh(
    input_path: str | Path,
    output_path: str | Path,
    *,
    offset_mm: float,
    voxel_size_mm: float | None = None,
) -> Path:
    """Run MeshLib's voxel offset and save the result for parity tests."""

    from meshlib import mrmeshpy as mm

    mesh = mm.loadMesh(str(input_path))
    params = mm.OffsetParameters()
    if voxel_size_mm is not None:
        params.voxelSize = float(voxel_size_mm)
    result = _extract_mesh(mm.offsetMesh(mesh, float(offset_mm), params))
    output = Path(output_path)
    mm.saveMesh(result, str(output))
    return output


def boolean_mesh(
    a_path: str | Path,
    b_path: str | Path,
    output_path: str | Path,
    *,
    operation: MeshLibBooleanOperation,
) -> Path:
    """Run MeshLib's exact boolean and save the result for parity tests."""

    from meshlib import mrmeshpy as mm

    operation_map = {
        "union": mm.BooleanOperation.Union,
        "intersection": mm.BooleanOperation.Intersection,
        "difference": mm.BooleanOperation.DifferenceAB,
    }
    mesh_a = mm.loadMesh(str(a_path))
    mesh_b = mm.loadMesh(str(b_path))
    result = _extract_mesh(mm.boolean(mesh_a, mesh_b, operation_map[operation]))
    output = Path(output_path)
    mm.saveMesh(result, str(output))
    return output
