"""trimesh adapter isolated from SDK algorithms."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import trimesh
from trimesh.visual.texture import TextureVisuals

from geometry_sdk.core.mesh import (
    mesh_from_mru_scene,
    mesh_from_obj,
    mesh_from_ply,
    mesh_to_ply_bytes,
    meshlib_multi_object_mru_scene_bytes as _rust_meshlib_multi_object_mru_scene_bytes,
    meshlib_object_mesh_mru_scene_bytes as _rust_meshlib_object_mesh_mru_scene_bytes,
    meshlib_object_mesh_scene_json as _rust_meshlib_object_mesh_scene_json,
    meshlib_object_mesh_scene_payload as _rust_meshlib_object_mesh_scene_payload,
)
from geometry_sdk.types import MeshDocument


def meshlib_object_mesh_scene_payload(
    mesh: MeshDocument,
    *,
    object_name: str,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> dict[str, object]:
    """Build a MeshLib ObjectMesh scene JSON payload through the Rust SDK."""
    return _rust_meshlib_object_mesh_scene_payload(
        mesh,
        object_name=object_name,
        child_index=child_index,
        model_extension=model_extension,
    )


def save_meshlib_object_mesh_scene_json(
    mesh: MeshDocument,
    path: str | Path,
    *,
    object_name: str,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> Path:
    output_path = Path(path)
    payload_json = _rust_meshlib_object_mesh_scene_json(
        mesh,
        object_name=object_name,
        child_index=child_index,
        model_extension=model_extension,
    )
    output_path.write_text(f"{payload_json}\n", encoding="utf-8")
    return output_path


def save_meshlib_object_mesh_mru_scene(
    mesh: MeshDocument,
    path: str | Path,
    *,
    object_name: str,
    model_path: str | Path | None = None,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> Path:
    output_path = Path(path)
    if mesh.metadata.get("scene_objects") and model_path is None:
        archive = _rust_meshlib_multi_object_mru_scene_bytes(
            mesh,
            root_name=object_name,
            root_key=str(mesh.metadata.get("root_key") or "0_Root"),
        )
    else:
        if model_path is None:
            raise ValueError("model_path is required for single-object MeshLib MRU scene export")
        archive = _rust_meshlib_object_mesh_mru_scene_bytes(
            mesh,
            object_name=object_name,
            model_bytes=Path(model_path).read_bytes(),
            child_index=child_index,
            model_extension=model_extension,
        )
    output_path.write_bytes(archive)
    return output_path


def from_trimesh(mesh: trimesh.Trimesh, *, metadata: dict | None = None) -> MeshDocument:
    return MeshDocument(
        vertices=np.asarray(mesh.vertices, dtype=np.float64),
        faces=np.asarray(mesh.faces, dtype=np.int64),
        metadata=metadata or {},
    )


def _metadata_array(mesh: MeshDocument, key: str, *, shape_tail: tuple[int, ...]) -> np.ndarray | None:
    values = mesh.metadata.get(key)
    if values is None:
        return None
    array = np.asarray(values, dtype=np.float64)
    if array.shape != (mesh.face_count if shape_tail == (3, 2) else mesh.vertex_count, *shape_tail):
        return None
    if not np.all(np.isfinite(array)):
        return None
    return array


def _vertex_uvs(mesh: MeshDocument) -> np.ndarray | None:
    return _metadata_array(mesh, "vertex_uvs", shape_tail=(2,))


def _tri_corner_uvs(mesh: MeshDocument) -> np.ndarray | None:
    return _metadata_array(mesh, "tri_corner_uvs", shape_tail=(3, 2))


def to_trimesh(mesh: MeshDocument, *, process: bool = False, texture_preview: bool = False) -> trimesh.Trimesh:
    tri_corner_uvs = _tri_corner_uvs(mesh) if texture_preview else None
    if tri_corner_uvs is not None:
        flattened_vertices = mesh.vertices[mesh.faces.reshape(-1)].reshape((-1, 3))
        flattened_faces = np.arange(flattened_vertices.shape[0], dtype=np.int64).reshape((-1, 3))
        result = trimesh.Trimesh(vertices=flattened_vertices, faces=flattened_faces, process=process)
        result.visual = TextureVisuals(uv=tri_corner_uvs.reshape((-1, 2)))
        return result

    result = trimesh.Trimesh(vertices=mesh.vertices.copy(), faces=mesh.faces.copy(), process=process)
    vertex_uvs = _vertex_uvs(mesh)
    if vertex_uvs is not None:
        result.visual = TextureVisuals(uv=vertex_uvs)
    return result


def load_mesh(path: str | Path) -> MeshDocument:
    mesh_path = Path(path)
    if mesh_path.suffix.lower() == ".ply":
        mesh = mesh_from_ply(mesh_path.read_bytes(), texture_dir=mesh_path.parent)
        mesh.metadata.update(
            {
                "source": "rust_mesh_from_ply",
                "meshlib_reference": "MR::loadPly",
                "meshlib_source": "MeshLib/source/MRMesh/MRPly.cpp",
                "source_path": str(mesh_path),
            }
        )
        return mesh
    if mesh_path.suffix.lower() == ".obj":
        mesh = mesh_from_obj(mesh_path.read_bytes(), material_dir=mesh_path.parent)
        mesh.metadata.update({"source_path": str(mesh_path)})
        return mesh
    if mesh_path.suffix.lower() == ".mru":
        mesh = mesh_from_mru_scene(mesh_path.read_bytes())
        mesh.metadata.update({"source_path": str(mesh_path)})
        return mesh
    loaded = trimesh.load(str(path), force="mesh")
    if isinstance(loaded, trimesh.Scene):
        meshes = [geom for geom in loaded.geometry.values() if isinstance(geom, trimesh.Trimesh)]
        if not meshes:
            raise ValueError("No valid mesh geometry found")
        loaded = trimesh.util.concatenate(meshes)
    if not isinstance(loaded, trimesh.Trimesh):
        raise ValueError(f"Unsupported geometry type: {type(loaded)}")
    return from_trimesh(loaded, metadata={"source_path": str(path)})


def _should_write_meshlib_ascii_ply(mesh: MeshDocument) -> bool:
    return any(
        key in mesh.metadata
        for key in ("texture_files", "vertex_uvs", "tri_corner_uvs", "vertex_colors", "face_colors")
    )


def save_mesh(mesh: MeshDocument, path: str | Path, *, file_type: str | None = None) -> Path:
    output_path = Path(path)
    resolved_file_type = (file_type or output_path.suffix.lstrip(".")).lower()
    if resolved_file_type == "ply" and _should_write_meshlib_ascii_ply(mesh):
        output_path.write_bytes(mesh_to_ply_bytes(mesh))
        return output_path
    to_trimesh(mesh, texture_preview=resolved_file_type in {"glb", "gltf"}).export(
        str(output_path),
        file_type=file_type,
    )
    return output_path
