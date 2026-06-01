from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, ThicknessSummary


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def ray_thickness_at_vertices(mesh: MeshDocument, *, epsilon: float = 1e-5) -> np.ndarray:
    kernel = _require_rust_kernel("ray_thickness_at_vertices")
    return np.asarray(kernel(mesh.vertices, mesh.faces, float(epsilon)), dtype=np.float32)


def insphere_thickness_at_vertices(
    mesh: MeshDocument,
    *,
    max_radius: float = 1.0,
    max_iters: int = 16,
    min_shrinkage: float = 0.99999,
    min_angle_cos: float = -1.0,
    epsilon: float = 1e-5,
) -> np.ndarray:
    kernel = _require_rust_kernel("insphere_thickness_at_vertices")
    return np.asarray(
        kernel(
            mesh.vertices,
            mesh.faces,
            float(max_radius),
            int(max_iters),
            float(min_shrinkage),
            float(min_angle_cos),
            float(epsilon),
        ),
        dtype=np.float32,
    )


def service_thickness_at_vertices(
    mesh: MeshDocument,
    *,
    max_radius: float = 1.0,
    max_iters: int = 16,
    min_shrinkage: float = 0.99999,
    min_angle_cos: float = -1.0,
    epsilon: float = 1e-5,
) -> np.ndarray:
    kernel = _require_rust_kernel("service_thickness_at_vertices")
    return np.asarray(
        kernel(
            mesh.vertices,
            mesh.faces,
            float(max_radius),
            int(max_iters),
            float(min_shrinkage),
            float(min_angle_cos),
            float(epsilon),
        ),
        dtype=np.float32,
    )


def summarize_thickness(thickness: Any, *, threshold_mm: float = 0.6) -> ThicknessSummary:
    kernel = _require_rust_kernel("summarize_thickness")
    values = np.asarray(thickness, dtype=np.float32).reshape(-1)
    payload: dict[str, Any] = kernel(values, float(threshold_mm))
    return ThicknessSummary(
        min_mm=None if payload["min_mm"] is None else float(payload["min_mm"]),
        avg_mm=None if payload["avg_mm"] is None else float(payload["avg_mm"]),
        max_mm=None if payload["max_mm"] is None else float(payload["max_mm"]),
        valid_vertex_count=int(payload["valid_vertex_count"]),
        violation_count=int(payload["violation_count"]),
    )
