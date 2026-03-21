"""Ring resizing service with axis-aware radial scaling."""

from __future__ import annotations

from pathlib import Path
from typing import Sequence, Union

import numpy as np
from scipy.spatial import cKDTree
import trimesh

from core.logging import get_logger
from services.convert import load_mesh
from services.measure_ring import normalize_axis
from utils.units import get_ring_diameter

logger = get_logger(__name__)


def resize_ring(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    current_size: float,
    target_size: float,
    axis_override: Sequence[float] | None = None,
    preserve_indices: np.ndarray | None = None,
) -> Path:
    """Resize a ring to a target size using radial scaling around a confirmed axis."""
    input_path = Path(input_path)
    output_path = Path(output_path)

    current_diameter = get_ring_diameter(current_size)
    target_diameter = get_ring_diameter(target_size)
    scale_factor = target_diameter / current_diameter
    logger.info("Resizing ring from size %.2f to %.2f with scale %.4f", current_size, target_size, scale_factor)

    mesh = load_mesh(input_path)
    ring_axis = normalize_axis(axis_override) if axis_override is not None else None
    scaled_mesh = radial_scale(mesh, scale_factor, ring_axis=ring_axis, preserve_indices=preserve_indices)
    scaled_mesh.export(str(output_path))
    logger.info("Ring resized: saved to %s", output_path.name)
    return output_path


def radial_scale(
    mesh: trimesh.Trimesh,
    scale_factor: float,
    ring_axis: np.ndarray | None = None,
    preserve_indices: np.ndarray | None = None,
) -> trimesh.Trimesh:
    """Scale mesh radially while preserving the confirmed ring axis and optional protected regions."""
    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    center = mesh.centroid

    if ring_axis is None:
        centered = vertices - center
        cov = np.cov(centered.T)
        eigenvalues, eigenvectors = np.linalg.eigh(cov)
        ring_axis = normalize_axis(eigenvectors[:, np.argsort(eigenvalues)[0]])
    else:
        ring_axis = normalize_axis(ring_axis)

    local_scale = np.full(len(vertices), scale_factor, dtype=np.float64)
    if preserve_indices is not None and len(preserve_indices) > 0:
        preserve_indices = np.unique(np.asarray(preserve_indices, dtype=np.int32))
        preserved_positions = vertices[preserve_indices]
        distances, _ = cKDTree(preserved_positions).query(vertices, workers=-1)
        falloff_mm = max(float(np.percentile(distances[preserve_indices], 95)) if preserve_indices.size else 0.0, 2.5)
        falloff_mm = max(falloff_mm * 2.2, 2.5)
        protection = np.exp(-0.5 * np.square(distances / max(falloff_mm, 1e-3)))
        protection[distances > falloff_mm * 2.5] = 0.0
        local_scale = 1.0 + (scale_factor - 1.0) * (1.0 - 0.88 * protection)
        local_scale[preserve_indices] = 1.0 + (scale_factor - 1.0) * 0.08

    relative = vertices - center
    axial_distance = relative @ ring_axis
    axial_component = np.outer(axial_distance, ring_axis)
    radial_component = relative - axial_component
    scaled_vertices = center + axial_component + radial_component * local_scale[:, None]

    return trimesh.Trimesh(vertices=scaled_vertices, faces=mesh.faces.copy(), process=True)


def uniform_scale(mesh: trimesh.Trimesh, scale_factor: float) -> trimesh.Trimesh:
    """Uniformly scale a mesh."""
    scaled = mesh.copy()
    scaled.apply_scale(scale_factor)
    return scaled
