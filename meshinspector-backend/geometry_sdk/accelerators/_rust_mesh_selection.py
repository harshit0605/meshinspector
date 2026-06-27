from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def extract_selected_faces_as_mesh(mesh: MeshDocument, selected_face_ids) -> MeshDocument:
    kernel = _require_core_kernel("extract_selected_faces_as_mesh")
    selected = np.asarray(selected_face_ids, dtype=np.int64).reshape((-1,))
    metadata = mesh.metadata
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        selected,
        _metadata_array(metadata, "vertex_uvs", np.float64, 2),
        _metadata_array(metadata, "vertex_colors", np.uint8, 4),
        _metadata_array(metadata, "face_colors", np.uint8, 4),
        _metadata_array(metadata, "texture_per_face", np.int64),
    )
    metadata = dict(mesh.metadata)
    for key in ("vertex_uvs", "vertex_colors", "face_colors", "texture_per_face"):
        metadata.pop(key, None)
    metadata.update(
        {
            "meshlib_operation": "Mesh::cloneRegion",
            "source_face_indices": [int(index) for index in payload["source_face_indices"]],
            "source_vertex_indices": [int(index) for index in payload["source_vertex_indices"]],
        }
    )
    if "vertex_uvs" in payload:
        metadata["vertex_uvs"] = np.asarray(payload["vertex_uvs"], dtype=np.float64).reshape((-1, 2)).tolist()
    if "vertex_colors" in payload:
        metadata["vertex_colors"] = np.asarray(payload["vertex_colors"], dtype=np.uint8).reshape((-1, 4)).astype(int).tolist()
    if "face_colors" in payload:
        metadata["face_colors"] = np.asarray(payload["face_colors"], dtype=np.uint8).reshape((-1, 4)).astype(int).tolist()
    if "texture_per_face" in payload:
        metadata["texture_per_face"] = [int(index) for index in payload["texture_per_face"]]
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape((-1, 3)),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape((-1, 3)),
        unit=mesh.unit,
        metadata=metadata,
    )


def bounded_seed_indices(mesh: MeshDocument, indices, max_count: int) -> np.ndarray:
    kernel = _require_core_kernel("bounded_seed_indices")
    values = np.asarray(indices, dtype=np.int64).reshape((-1,))
    return np.asarray(kernel(mesh.vertices, values, int(max_count)), dtype=np.int64)


def selection_seed_indices(
    mesh: MeshDocument,
    *,
    vertex_ids=None,
    face_ids=None,
    region_vertex_indices=None,
    brush_points_world=None,
) -> np.ndarray:
    kernel = _require_core_kernel("selection_seed_indices")
    vertex_values = np.asarray([] if vertex_ids is None else vertex_ids, dtype=np.int64).reshape((-1,))
    face_values = np.asarray([] if face_ids is None else face_ids, dtype=np.int64).reshape((-1,))
    region_values = np.asarray([] if region_vertex_indices is None else region_vertex_indices, dtype=np.int64).reshape((-1,))
    brush_points = np.asarray([] if brush_points_world is None else brush_points_world, dtype=np.float64).reshape((-1, 3))
    return np.asarray(
        kernel(
            mesh.vertices,
            mesh.faces,
            vertex_values,
            face_values,
            region_values,
            brush_points,
        ),
        dtype=np.int64,
    )


def _metadata_array(metadata: dict[str, Any], key: str, dtype, width: int | None = None) -> np.ndarray | None:
    values = metadata.get(key)
    if values is None:
        return None
    array = np.asarray(values, dtype=dtype)
    if width is not None:
        return array.reshape((-1, width))
    return array.reshape((-1,))
