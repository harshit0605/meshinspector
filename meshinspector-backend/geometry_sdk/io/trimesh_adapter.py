"""trimesh adapter isolated from SDK algorithms."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import trimesh

from geometry_sdk.types import MeshDocument


def from_trimesh(mesh: trimesh.Trimesh, *, metadata: dict | None = None) -> MeshDocument:
    return MeshDocument(
        vertices=np.asarray(mesh.vertices, dtype=np.float64),
        faces=np.asarray(mesh.faces, dtype=np.int64),
        metadata=metadata or {},
    )


def to_trimesh(mesh: MeshDocument, *, process: bool = False) -> trimesh.Trimesh:
    return trimesh.Trimesh(vertices=mesh.vertices.copy(), faces=mesh.faces.copy(), process=process)


def load_mesh(path: str | Path) -> MeshDocument:
    loaded = trimesh.load(str(path), force="mesh")
    if isinstance(loaded, trimesh.Scene):
        meshes = [geom for geom in loaded.geometry.values() if isinstance(geom, trimesh.Trimesh)]
        if not meshes:
            raise ValueError("No valid mesh geometry found")
        loaded = trimesh.util.concatenate(meshes)
    if not isinstance(loaded, trimesh.Trimesh):
        raise ValueError(f"Unsupported geometry type: {type(loaded)}")
    return from_trimesh(loaded, metadata={"source_path": str(path)})


def save_mesh(mesh: MeshDocument, path: str | Path, *, file_type: str | None = None) -> Path:
    output_path = Path(path)
    to_trimesh(mesh).export(str(output_path), file_type=file_type)
    return output_path
