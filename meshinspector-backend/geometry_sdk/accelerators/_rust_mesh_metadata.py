from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.types import MeshDocument


def metadata_uv_array(mesh: MeshDocument, key: str, *, shape_tail: tuple[int, ...]) -> np.ndarray | None:
    values = mesh.metadata.get(key)
    if values is None:
        return None
    array = np.asarray(values, dtype=np.float64)
    expected_count = mesh.face_count if shape_tail == (3, 2) else mesh.vertex_count
    if array.shape != (expected_count, *shape_tail):
        return None
    if not np.all(np.isfinite(array)):
        return None
    return array


def texture_images_for_rust(mesh: MeshDocument) -> list[dict[str, Any]]:
    texture_images = mesh.metadata.get("texture_images")
    if not isinstance(texture_images, list):
        return []
    return [texture for texture in texture_images if isinstance(texture, dict)]


def texture_per_face_for_rust(mesh: MeshDocument) -> np.ndarray:
    texture_per_face = mesh.metadata.get("texture_per_face")
    if not isinstance(texture_per_face, list):
        texture_per_face = []
    return np.asarray([int(texture_id) for texture_id in texture_per_face], dtype=np.int64).reshape((-1,))


def metadata_color_array(mesh: MeshDocument, key: str, *, count: int) -> np.ndarray | None:
    values = mesh.metadata.get(key)
    if values is None:
        return None
    array = np.asarray(values, dtype=np.int64)
    if array.ndim != 2 or array.shape[0] != count or array.shape[1] < 3:
        return None
    return array
