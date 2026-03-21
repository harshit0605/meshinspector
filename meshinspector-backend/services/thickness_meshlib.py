"""MeshLib-backed wall thickness analysis."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np
import meshlib.mrmeshpy as mm

from core.config import settings


@dataclass(slots=True)
class ThicknessResult:
    min_mm: float | None
    avg_mm: float | None
    max_mm: float | None
    violation_count: int
    threshold_mm: float
    scalar_field_path: Path


def compute_thickness_meshlib(
    mesh_path: str | Path,
    output_dir: str | Path,
    threshold_mm: float | None = None,
) -> ThicknessResult:
    """Compute thickness using MeshLib's per-vertex thickness APIs."""
    threshold = threshold_mm or settings.DEFAULT_MIN_THICKNESS_MM
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    mesh = mm.loadMesh(str(mesh_path))

    insphere_settings = mm.InSphereSearchSettings()
    insphere_scalars = mm.computeInSphereThicknessAtVertices(mesh, insphere_settings)
    ray_scalars = mm.computeRayThicknessAtVertices(mesh)

    insphere = np.array([float(insphere_scalars.vec[i]) for i in range(insphere_scalars.size())], dtype=np.float32)
    ray = np.array([float(ray_scalars.vec[i]) for i in range(ray_scalars.size())], dtype=np.float32)

    ray[np.logical_or(~np.isfinite(ray), ray > 1e10)] = np.nan
    thickness = np.where(np.isfinite(ray), np.minimum(insphere, ray), insphere)
    thickness[np.logical_or(~np.isfinite(thickness), thickness <= 0)] = np.nan

    valid = thickness[np.isfinite(thickness)]
    scalar_field_path = output_dir / "thickness_scalars.npz"
    np.savez_compressed(scalar_field_path, thickness=thickness, threshold_mm=np.float32(threshold))

    if valid.size == 0:
        return ThicknessResult(
            min_mm=None,
            avg_mm=None,
            max_mm=None,
            violation_count=0,
            threshold_mm=threshold,
            scalar_field_path=scalar_field_path,
        )

    return ThicknessResult(
        min_mm=round(float(np.min(valid)), 4),
        avg_mm=round(float(np.mean(valid)), 4),
        max_mm=round(float(np.max(valid)), 4),
        violation_count=int(np.sum(valid < threshold)),
        threshold_mm=threshold,
        scalar_field_path=scalar_field_path,
    )
