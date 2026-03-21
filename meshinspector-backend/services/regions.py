"""Ring-oriented semantic region detection and region manifest helpers."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import trimesh

from domain.schemas import RegionManifestEntry
from services.measure_ring import RingMeasurement


@dataclass(slots=True)
class RegionArtifacts:
    region_json_path: Path
    manifest: list[RegionManifestEntry]


def _safe_normalize(vectors: np.ndarray) -> np.ndarray:
    norms = np.linalg.norm(vectors, axis=1, keepdims=True)
    return vectors / np.clip(norms, 1e-8, None)


def _region_entry(
    region_id: str,
    label: str,
    indices: np.ndarray,
    vertices: np.ndarray,
    thickness: np.ndarray,
    threshold_mm: float,
    protected_by_default: bool,
    allowed_operations: list[str],
) -> RegionManifestEntry:
    if indices.size == 0:
        return RegionManifestEntry(
            region_id=region_id,
            label=label,
            vertex_count=0,
            coverage_pct=0.0,
            protected_by_default=protected_by_default,
            allowed_operations=allowed_operations,
        )

    region_thickness = thickness[indices]
    finite = region_thickness[np.isfinite(region_thickness)]
    centroid = vertices[indices].mean(axis=0)
    return RegionManifestEntry(
        region_id=region_id,
        label=label,
        vertex_count=int(indices.size),
        coverage_pct=round(float(indices.size / max(len(vertices), 1) * 100.0), 2),
        min_thickness_mm=round(float(np.min(finite)), 4) if finite.size else None,
        avg_thickness_mm=round(float(np.mean(finite)), 4) if finite.size else None,
        violation_count=int(np.sum(finite < threshold_mm)) if finite.size else 0,
        protected_by_default=protected_by_default,
        allowed_operations=allowed_operations,
        centroid_mm=(round(float(centroid[0]), 4), round(float(centroid[1]), 4), round(float(centroid[2]), 4)),
    )


def detect_ring_regions(
    mesh: trimesh.Trimesh,
    measurement: RingMeasurement,
    thickness: np.ndarray,
    output_dir: str | Path,
    threshold_mm: float,
) -> RegionArtifacts:
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    center = vertices.mean(axis=0)
    axis = np.asarray(measurement.ring_axis, dtype=np.float64)
    axis = axis / np.clip(np.linalg.norm(axis), 1e-8, None)

    centered = vertices - center
    axial = centered @ axis
    radial_vectors = centered - np.outer(axial, axis)
    radial = np.linalg.norm(radial_vectors, axis=1)
    radial_dirs = _safe_normalize(radial_vectors)

    normals = np.asarray(mesh.vertex_normals, dtype=np.float64)
    normals = _safe_normalize(normals)
    curvature_like = 1.0 - np.clip(np.abs(np.einsum("ij,ij->i", normals, radial_dirs)), 0.0, 1.0)

    abs_axial = np.abs(axial)
    radial_p35 = float(np.percentile(radial, 35))
    radial_p72 = float(np.percentile(radial, 72))
    radial_p90 = float(np.percentile(radial, 90))
    axial_p40 = float(np.percentile(abs_axial, 40))
    axial_p68 = float(np.percentile(abs_axial, 68))
    curvature_p75 = float(np.percentile(curvature_like, 75))

    inner_band = (abs_axial <= axial_p40) & (radial <= radial_p35)
    head = (radial >= radial_p90) & (abs_axial >= axial_p40 * 0.8)
    ornament_relief = (~inner_band) & (~head) & ((radial >= radial_p72) | (curvature_like >= curvature_p75))
    outer_band = (~inner_band) & (~head) & (~ornament_relief) & (abs_axial <= axial_p68)
    unknown = ~(inner_band | head | ornament_relief | outer_band)

    region_indices: dict[str, np.ndarray] = {
        "inner_band": np.flatnonzero(inner_band),
        "outer_band": np.flatnonzero(outer_band),
        "head": np.flatnonzero(head),
        "ornament_relief": np.flatnonzero(ornament_relief),
        "unknown": np.flatnonzero(unknown),
    }

    manifest = [
        _region_entry("inner_band", "Inner Band", region_indices["inner_band"], vertices, thickness, threshold_mm, True, ["scoop", "smooth"]),
        _region_entry("outer_band", "Outer Band", region_indices["outer_band"], vertices, thickness, threshold_mm, False, ["smooth"]),
        _region_entry("head", "Head", region_indices["head"], vertices, thickness, threshold_mm, True, ["smooth"]),
        _region_entry(
            "ornament_relief",
            "Ornament Relief",
            region_indices["ornament_relief"],
            vertices,
            thickness,
            threshold_mm,
            True,
            ["smooth"],
        ),
        _region_entry("unknown", "Unknown", region_indices["unknown"], vertices, thickness, threshold_mm, False, []),
    ]

    payload = {
        "ring_axis": [round(float(axis[0]), 6), round(float(axis[1]), 6), round(float(axis[2]), 6)],
        "regions": [
            {
                **entry.model_dump(mode="json"),
                "vertex_indices": region_indices[entry.region_id].astype(int).tolist(),
            }
            for entry in manifest
        ],
    }
    region_json_path = output_dir / "regions.json"
    region_json_path.write_text(json.dumps(payload), encoding="utf-8")
    return RegionArtifacts(region_json_path=region_json_path, manifest=manifest)


def load_region_payload(path: str | Path) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))
