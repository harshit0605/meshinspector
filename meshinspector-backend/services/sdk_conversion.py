"""SDK-backed mesh conversion helpers for versioned production artifacts."""

from __future__ import annotations

from pathlib import Path

import numpy as np

from core.logging import get_logger
from geometry_sdk import MeshDocument, default_sdk

logger = get_logger(__name__)


def detect_unit_scale(mesh: MeshDocument) -> float:
    """Infer scale factor that converts the mesh into millimeters."""
    if mesh.vertex_count == 0:
        return 1.0
    raw_bbox = np.ptp(mesh.vertices, axis=0)
    max_dim = float(np.max(raw_bbox)) if raw_bbox.size else 0.0
    if max_dim < 0.1:
        return 1000.0
    if max_dim < 10:
        return 10.0
    if max_dim > 10000:
        return 0.001
    return 1.0


def normalize_mesh_to_mm(input_path: str | Path) -> MeshDocument:
    mesh = default_sdk.load_mesh(input_path)
    scale_factor = detect_unit_scale(mesh)
    if scale_factor == 1.0:
        return mesh
    logger.info("Applied unit normalization scale: %s", scale_factor)
    return mesh.copy(vertices=mesh.vertices * scale_factor)


def to_stl(input_path: str | Path, output_path: str | Path) -> Path:
    mesh = normalize_mesh_to_mm(input_path)
    return default_sdk.save_mesh(mesh, Path(output_path), file_type="stl")


def to_ply(input_path: str | Path, output_path: str | Path) -> Path:
    mesh = normalize_mesh_to_mm(input_path)
    return default_sdk.save_mesh(mesh, Path(output_path), file_type="ply")


def to_glb(
    input_path: str | Path,
    output_path: str | Path,
    material: str = "gold_18k",
    decimate_faces: int | None = None,
) -> Path:
    mesh = normalize_mesh_to_mm(input_path)
    if decimate_faces is not None and len(mesh.faces) > decimate_faces:
        # Lightweight "low" LOD preview: decimate (the kernel is O(F log F), cheap)
        # then drop the now-unreferenced vertices so the GLB is actually small
        # (~50x on a 1M-face mesh) — not a full-resolution duplicate of "high".
        from geometry_sdk.mesh_edit import decimate_mesh

        reduced = decimate_mesh(mesh, target_face_ratio=decimate_faces / len(mesh.faces)).mesh
        faces = np.asarray(reduced.faces)
        vertices = np.asarray(reduced.vertices)
        used = np.unique(faces)
        remap = np.full(vertices.shape[0], -1, dtype=faces.dtype)
        remap[used] = np.arange(used.shape[0])
        mesh = MeshDocument(vertices=vertices[used], faces=remap[faces])
    mesh.metadata["preview_material"] = material
    return default_sdk.save_mesh(mesh, Path(output_path), file_type="glb")
