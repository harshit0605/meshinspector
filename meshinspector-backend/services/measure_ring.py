"""Ring measurement heuristics for generated meshes."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np
import trimesh

from services.convert import normalize_mesh_to_mm
from utils.units import RING_SIZE_CHART


@dataclass(slots=True)
class RingMeasurement:
    ring_axis: tuple[float, float, float]
    ring_axis_confidence: float
    estimated_ring_size_us: float | None
    inner_diameter_mm: float | None
    band_width_min_mm: float | None
    band_width_max_mm: float | None
    head_height_mm: float | None
    bbox_mm: tuple[float, float, float]
    needs_axis_confirmation: bool


def _closest_ring_size(inner_diameter_mm: float | None) -> float | None:
    if inner_diameter_mm is None:
        return None
    return min(RING_SIZE_CHART.keys(), key=lambda size: abs(RING_SIZE_CHART[size] - inner_diameter_mm))


def normalize_axis(axis: tuple[float, float, float] | np.ndarray) -> np.ndarray:
    axis_array = np.asarray(axis, dtype=np.float64)
    norm = np.linalg.norm(axis_array)
    if norm < 1e-8:
        raise ValueError("Axis vector magnitude is too small")
    return axis_array / norm


def measure_ring_from_mesh(mesh: trimesh.Trimesh, axis_override: tuple[float, float, float] | None = None) -> RingMeasurement:
    """Estimate ring dimensions and axis from a normalized mesh."""
    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    center = vertices.mean(axis=0)
    centered = vertices - center

    cov = np.cov(centered.T)
    eigenvalues, eigenvectors = np.linalg.eigh(cov)
    order = np.argsort(eigenvalues)
    detected_axis = normalize_axis(eigenvectors[:, order[0]])
    ring_axis = normalize_axis(axis_override) if axis_override is not None else detected_axis

    axial = centered @ ring_axis
    radial_vectors = centered - np.outer(axial, ring_axis)
    radial_dist = np.linalg.norm(radial_vectors, axis=1)

    band_window = np.abs(axial) <= np.percentile(np.abs(axial), 45)
    band_radial = radial_dist[band_window] if np.any(band_window) else radial_dist

    inner_radius = float(np.percentile(band_radial, 12)) if len(band_radial) else None
    outer_radius = float(np.percentile(band_radial, 75)) if len(band_radial) else None
    inner_diameter = round(inner_radius * 2.0, 3) if inner_radius is not None else None

    band_width = np.abs(axial[band_window]) if np.any(band_window) else np.abs(axial)
    band_width_min = round(float(np.percentile(band_width, 50)) * 2.0, 3) if len(band_width) else None
    band_width_max = round(float(np.max(np.abs(axial)) * 2.0), 3) if len(axial) else None
    head_height = round(max(float(np.max(radial_dist) - np.median(radial_dist)), 0.0), 3) if len(radial_dist) else None

    spread_ratio = float(eigenvalues[1] / max(eigenvalues[0], 1e-8))
    radial_consistency = float(np.clip(1.0 - (np.std(band_radial) / max(np.mean(band_radial), 1e-6)), 0.0, 1.0))
    auto_confidence = float(np.clip(min(spread_ratio / 4.0, 1.0) * 0.6 + radial_consistency * 0.4, 0.0, 1.0))
    confidence = 1.0 if axis_override is not None else auto_confidence

    bbox = mesh.bounds[1] - mesh.bounds[0]
    return RingMeasurement(
        ring_axis=(float(ring_axis[0]), float(ring_axis[1]), float(ring_axis[2])),
        ring_axis_confidence=round(confidence, 3),
        estimated_ring_size_us=_closest_ring_size(inner_diameter),
        inner_diameter_mm=inner_diameter,
        band_width_min_mm=band_width_min,
        band_width_max_mm=band_width_max,
        head_height_mm=head_height,
        bbox_mm=(round(float(bbox[0]), 3), round(float(bbox[1]), 3), round(float(bbox[2]), 3)),
        needs_axis_confirmation=axis_override is None and confidence < 0.6,
    )


def measure_ring_from_path(
    mesh_path: str | Path,
    axis_override: tuple[float, float, float] | None = None,
) -> RingMeasurement:
    mesh = normalize_mesh_to_mm(mesh_path)
    return measure_ring_from_mesh(mesh, axis_override=axis_override)
