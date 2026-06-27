from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_mesh_metadata import metadata_color_array, metadata_uv_array
from geometry_sdk.types import MeshDocument


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def mesh_from_ply(source: bytes, texture_dir: str | Path | None = None) -> MeshDocument:
    kernel = _require_core_kernel("mesh_from_ply")
    payload: dict[str, Any] = kernel(bytes(source), None if texture_dir is None else str(texture_dir))
    metadata = {
        "vertex_colors": payload["vertex_colors"],
        "face_colors": payload["face_colors"],
        "vertex_uvs": payload["vertex_uvs"],
        "vertex_normals_ply": payload["vertex_normals"],
        "tri_corner_uvs": payload["tri_corner_uvs"],
        "edges": payload["edges"],
        "texture_files": payload["texture_files"],
        "texture_images": payload["texture_images"],
    }
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape((-1, 3)),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape((-1, 3)),
        metadata=metadata,
    )


def mesh_from_obj(source: bytes | bytearray | str, material_dir: str | Path | None = None) -> MeshDocument:
    kernel = _require_core_kernel("mesh_from_obj")
    source_bytes = source.encode("utf-8") if isinstance(source, str) else bytes(source)
    payload: dict[str, Any] = kernel(source_bytes, None if material_dir is None else str(material_dir))
    metadata = {
        "source": "rust_mesh_from_obj",
        "meshlib_reference": "MR::MeshLoad::fromSceneObjFile",
        "meshlib_source": "MeshLib/source/MRMesh/MRMeshLoadObj.cpp",
        "object_names": [str(name) for name in payload.get("object_names", [])],
        "material_names": [str(name) for name in payload.get("material_names", [])],
    }
    if payload.get("diffuse_color") is not None:
        metadata["diffuse_color"] = [int(channel) for channel in payload["diffuse_color"]]
    if payload.get("texture_files"):
        metadata["texture_files"] = [str(file_name) for file_name in payload["texture_files"]]
    if payload.get("texture_images"):
        metadata["texture_images"] = payload["texture_images"]
    if payload.get("texture_per_face"):
        metadata["texture_per_face"] = [int(texture_id) for texture_id in payload["texture_per_face"]]
    if payload.get("tri_corner_uvs"):
        metadata["tri_corner_uvs"] = payload["tri_corner_uvs"]
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape((-1, 3)),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape((-1, 3)),
        metadata=metadata,
    )


def mesh_to_ply_bytes(mesh: MeshDocument) -> bytes:
    kernel = _require_core_kernel("mesh_to_ply")
    return bytes(
        kernel(
            mesh.vertices,
            mesh.faces,
            [str(item) for item in mesh.metadata.get("texture_files", []) if str(item)],
            metadata_uv_array(mesh, "vertex_uvs", shape_tail=(2,)),
            metadata_uv_array(mesh, "tri_corner_uvs", shape_tail=(3, 2)),
            metadata_color_array(mesh, "vertex_colors", count=mesh.vertex_count),
            metadata_color_array(mesh, "face_colors", count=mesh.face_count),
        )
    )
