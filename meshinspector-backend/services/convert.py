"""Format conversion and mesh normalization helpers."""

from __future__ import annotations

from pathlib import Path
from typing import Union

import numpy as np
import trimesh

from core.logging import get_logger

logger = get_logger(__name__)


def load_mesh(file_path: Union[str, Path]) -> trimesh.Trimesh:
    """Load any supported mesh file into a Trimesh mesh."""
    loaded = trimesh.load(str(file_path))
    if isinstance(loaded, trimesh.Scene):
        meshes = [geom for geom in loaded.geometry.values() if isinstance(geom, trimesh.Trimesh)]
        if not meshes:
            raise ValueError("No valid mesh geometry found")
        mesh = trimesh.util.concatenate(meshes)
        logger.info(f"Merged {len(meshes)} meshes from scene")
        return mesh
    if isinstance(loaded, trimesh.Trimesh):
        return loaded
    raise ValueError(f"Unsupported geometry type: {type(loaded)}")


def detect_unit_scale(mesh: trimesh.Trimesh) -> float:
    """Infer scale factor that converts the mesh into millimeters."""
    bounds = mesh.bounds
    raw_bbox = bounds[1] - bounds[0]
    max_dim = float(max(raw_bbox))
    if max_dim < 0.1:
        return 1000.0
    if max_dim < 10:
        return 10.0
    if max_dim > 10000:
        return 0.001
    return 1.0


def normalize_mesh_to_mm(input_path: Union[str, Path]) -> trimesh.Trimesh:
    """Load a mesh and normalize it into millimeters."""
    mesh = load_mesh(input_path)
    scale_factor = detect_unit_scale(mesh)
    if scale_factor != 1.0:
        mesh.apply_scale(scale_factor)
        logger.info(f"Applied unit normalization scale: {scale_factor}")
    return mesh


def to_stl(input_path: Union[str, Path], output_path: Union[str, Path]) -> Path:
    """Convert any supported mesh file to STL."""
    mesh = normalize_mesh_to_mm(input_path)
    output_path = Path(output_path)
    mesh.export(str(output_path), file_type="stl")
    return output_path


def to_ply(input_path: Union[str, Path], output_path: Union[str, Path]) -> Path:
    """Convert any supported mesh file to normalized PLY."""
    mesh = normalize_mesh_to_mm(input_path)
    output_path = Path(output_path)
    mesh.export(str(output_path), file_type="ply")
    return output_path


MATERIAL_DEFINITIONS = {
    "gold_24k": {"baseColor": [1.0, 0.84, 0.0, 1.0]},
    "gold_22k": {"baseColor": [1.0, 0.78, 0.2, 1.0]},
    "gold_18k": {"baseColor": [1.0, 0.71, 0.31, 1.0]},
    "gold_14k": {"baseColor": [0.94, 0.67, 0.35, 1.0]},
    "gold_10k": {"baseColor": [0.86, 0.63, 0.39, 1.0]},
    "silver_925": {"baseColor": [0.97, 0.96, 0.95, 1.0]},
    "platinum": {"baseColor": [0.9, 0.89, 0.88, 1.0]},
}


def to_glb(input_path: Union[str, Path], output_path: Union[str, Path], material: str = "gold_18k") -> Path:
    """Convert any mesh file to GLB with a simple metallic material preview."""
    mesh = normalize_mesh_to_mm(input_path)
    output_path = Path(output_path)

    base_color = MATERIAL_DEFINITIONS.get(material, MATERIAL_DEFINITIONS["gold_18k"])["baseColor"]
    color_255 = [int(c * 255) for c in base_color[:3]] + [255]
    face_colors = np.tile(color_255, (len(mesh.faces), 1)).astype(np.uint8)
    noise = np.random.uniform(-6, 6, (len(mesh.faces), 3)).astype(np.int16)
    face_colors[:, :3] = np.clip(face_colors[:, :3].astype(np.int16) + noise, 0, 255).astype(np.uint8)
    mesh.visual = trimesh.visual.ColorVisuals(mesh=mesh, face_colors=face_colors)
    mesh.export(str(output_path), file_type="glb")
    return output_path
