from __future__ import annotations

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common

def orient_faces_consistently(faces: np.ndarray) -> tuple[np.ndarray, list[list[int]]] | None:
    if _common._rs is None:
        raise RuntimeError("Rust orient_faces_consistently kernel requires _zennah_geometry_rs")
    if not hasattr(_common._rs, "orient_faces_consistently"):
        raise RuntimeError("_zennah_geometry_rs does not expose orient_faces_consistently")

    face_array = np.asarray(faces, dtype=np.int64)
    if face_array.ndim != 2 or face_array.shape[1] != 3:
        raise ValueError("faces must have shape (n, 3)")
    payload = _common._rs.orient_faces_consistently(face_array)
    oriented_faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    offsets = np.asarray(payload["component_offsets"], dtype=np.int64).reshape(-1)
    component_faces = np.asarray(payload["component_faces"], dtype=np.int64).reshape(-1)
    components = [
        [int(face_id) for face_id in component_faces[int(start) : int(end)]]
        for start, end in zip(offsets[:-1], offsets[1:])
    ]
    return oriented_faces, components
