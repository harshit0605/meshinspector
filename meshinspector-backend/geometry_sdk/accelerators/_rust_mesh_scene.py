from __future__ import annotations

import json
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_mesh_metadata import (
    metadata_uv_array,
    texture_images_for_rust,
    texture_per_face_for_rust,
)
from geometry_sdk.types import MeshDocument


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def meshlib_object_mesh_scene_json(
    mesh: MeshDocument,
    *,
    object_name: str,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> str:
    kernel = _require_core_kernel("meshlib_object_mesh_scene_payload")
    tri_corner_uvs = metadata_uv_array(mesh, "tri_corner_uvs", shape_tail=(3, 2))
    vertex_uvs = metadata_uv_array(mesh, "vertex_uvs", shape_tail=(2,))
    return str(
        kernel(
            str(object_name),
            int(child_index),
            str(model_extension),
            texture_images_for_rust(mesh),
            texture_per_face_for_rust(mesh),
            tri_corner_uvs,
            vertex_uvs,
        )
    )


def meshlib_object_mesh_scene_payload(
    mesh: MeshDocument,
    *,
    object_name: str,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> dict[str, Any]:
    return json.loads(
        meshlib_object_mesh_scene_json(
            mesh,
            object_name=object_name,
            child_index=child_index,
            model_extension=model_extension,
        )
    )


def meshlib_object_mesh_mru_scene_bytes(
    mesh: MeshDocument,
    *,
    object_name: str,
    model_bytes: bytes,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> bytes:
    kernel = _require_core_kernel("meshlib_object_mesh_mru_scene")
    tri_corner_uvs = metadata_uv_array(mesh, "tri_corner_uvs", shape_tail=(3, 2))
    vertex_uvs = metadata_uv_array(mesh, "vertex_uvs", shape_tail=(2,))
    return bytes(
        kernel(
            str(object_name),
            bytes(model_bytes),
            int(child_index),
            str(model_extension),
            texture_images_for_rust(mesh),
            texture_per_face_for_rust(mesh),
            tri_corner_uvs,
            vertex_uvs,
        )
    )


def meshlib_multi_object_mru_scene_bytes(
    mesh: MeshDocument,
    *,
    root_name: str = "Root",
    root_key: str = "0_Root",
) -> bytes:
    kernel = _require_core_kernel("meshlib_multi_object_mru_scene")
    scene_objects = mesh.metadata.get("scene_objects")
    if not isinstance(scene_objects, list):
        scene_objects = []
    scene_line_objects = mesh.metadata.get("scene_line_objects")
    if not isinstance(scene_line_objects, list):
        scene_line_objects = []
    scene_point_objects = mesh.metadata.get("scene_point_objects")
    if not isinstance(scene_point_objects, list):
        scene_point_objects = []
    scene_distance_map_objects = mesh.metadata.get("scene_distance_map_objects")
    if not isinstance(scene_distance_map_objects, list):
        scene_distance_map_objects = []
    scene_feature_objects = mesh.metadata.get("scene_feature_objects")
    if not isinstance(scene_feature_objects, list):
        scene_feature_objects = []
    scene_voxel_objects = mesh.metadata.get("scene_voxel_objects")
    if not isinstance(scene_voxel_objects, list):
        scene_voxel_objects = []
    scene_group_objects = mesh.metadata.get("scene_group_objects")
    if not isinstance(scene_group_objects, list):
        scene_group_objects = []
    scene_child_order = mesh.metadata.get("scene_child_order")
    if not isinstance(scene_child_order, list):
        scene_child_order = []
    if not any(
        (
            scene_objects,
            scene_group_objects,
            scene_line_objects,
            scene_point_objects,
            scene_distance_map_objects,
            scene_feature_objects,
            scene_voxel_objects,
        )
    ):
        raise ValueError("multi-object MRU export requires at least one scene object collection")
    return bytes(
        kernel(
            str(root_name),
            str(root_key),
            mesh.vertices,
            mesh.faces,
            scene_objects,
            scene_line_objects,
            scene_point_objects,
            scene_distance_map_objects,
            scene_feature_objects,
            scene_voxel_objects,
            scene_child_order,
            scene_group_objects,
        )
    )


def mesh_from_mru_scene(source: bytes | bytearray) -> MeshDocument:
    kernel = _require_core_kernel("mesh_from_mru_scene")
    payload: dict[str, Any] = kernel(bytes(source))
    metadata = {
        "source": "rust_mesh_from_mru_scene",
        "meshlib_reference": "MR::deserializeObjectTree",
        "meshlib_source": "MeshLib/source/MRMesh/MRObjectLoad.cpp;MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp",
        "root_file": str(payload["root_file"]),
        "root_key": str(payload["root_key"]),
        "object_name": str(payload["object_name"]),
        "object_key": str(payload["object_key"]),
        "model_file": str(payload["model_file"]),
        "model_extension": str(payload.get("model_extension") or ""),
        "vertex_colors": payload["vertex_colors"],
        "face_colors": payload["face_colors"],
        "vertex_uvs": payload["vertex_uvs"],
        "vertex_normals_ply": payload["vertex_normals_ply"],
        "tri_corner_uvs": payload["tri_corner_uvs"],
        "edges": payload["edges"],
        "texture_files": payload["texture_files"],
        "object_names": payload["object_names"],
        "material_names": payload["material_names"],
        "diffuse_color": payload["diffuse_color"],
        "scene_objects": payload["scene_objects"],
        "scene_object_count": len(payload["scene_objects"]),
        "scene_group_objects": payload["scene_group_objects"],
        "scene_group_object_count": len(payload["scene_group_objects"]),
        "scene_line_objects": payload["scene_line_objects"],
        "scene_line_object_count": len(payload["scene_line_objects"]),
        "scene_point_objects": payload["scene_point_objects"],
        "scene_point_object_count": len(payload["scene_point_objects"]),
        "scene_distance_map_objects": payload["scene_distance_map_objects"],
        "scene_distance_map_object_count": len(payload["scene_distance_map_objects"]),
        "scene_voxel_objects": payload["scene_voxel_objects"],
        "scene_voxel_object_count": len(payload["scene_voxel_objects"]),
        "scene_feature_objects": payload["scene_feature_objects"],
        "scene_feature_object_count": len(payload["scene_feature_objects"]),
        "scene_child_order": payload["scene_child_order"],
    }
    if payload.get("texture_per_face"):
        metadata["texture_per_face"] = [int(texture_id) for texture_id in payload["texture_per_face"]]
    if payload.get("texture_images"):
        metadata["texture_images"] = payload["texture_images"]
    if payload.get("meshlib_uv_coordinates"):
        metadata["meshlib_uv_coordinates"] = payload["meshlib_uv_coordinates"]
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape((-1, 3)),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape((-1, 3)),
        metadata=metadata,
    )
