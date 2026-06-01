from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument

def self_intersecting_faces(mesh: MeshDocument, *, epsilon: float = 1e-8) -> set[int] | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None

    face_ids = _common._rs.self_intersecting_faces(mesh.vertices, mesh.faces, float(epsilon))
    return {int(face_id) for face_id in face_ids}


def point_mesh_distances(points: np.ndarray, mesh: MeshDocument) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None

    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    if query.ndim != 2 or query.shape[1] != 3:
        raise ValueError("points must have shape (n, 3)")
    return np.asarray(_common._rs.point_mesh_distances(query, mesh.vertices, mesh.faces), dtype=np.float32)


def closest_points_on_mesh(points: np.ndarray, mesh: MeshDocument) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "closest_points_on_mesh"):
        if mode == "rust":
            raise RuntimeError(
                "GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose closest_points_on_mesh"
            )
        return None

    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    if query.ndim != 2 or query.shape[1] != 3:
        raise ValueError("points must have shape (n, 3)")
    payload = _common._rs.closest_points_on_mesh(query, mesh.vertices, mesh.faces)
    closest = np.asarray(payload["closest_points"], dtype=np.float64).reshape(-1, 3)
    distances = np.asarray(payload["distances"], dtype=np.float64)
    face_indices = np.asarray(payload["face_indices"], dtype=np.int64)
    return closest, distances, face_indices


def winding_numbers(points: np.ndarray, mesh: MeshDocument) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None

    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    if query.ndim != 2 or query.shape[1] != 3:
        raise ValueError("points must have shape (n, 3)")
    return np.asarray(_common._rs.winding_numbers(query, mesh.vertices, mesh.faces), dtype=np.float64)


def signed_point_mesh_distances(points: np.ndarray, mesh: MeshDocument, *, winding_threshold: float = 0.5) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None

    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    if query.ndim != 2 or query.shape[1] != 3:
        raise ValueError("points must have shape (n, 3)")
    return np.asarray(
        _common._rs.signed_point_mesh_distances(query, mesh.vertices, mesh.faces, float(winding_threshold)),
        dtype=np.float32,
    )


def ray_thickness_at_vertices(mesh: MeshDocument, *, epsilon: float = 1e-5) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None

    return np.asarray(_common._rs.ray_thickness_at_vertices(mesh.vertices, mesh.faces, float(epsilon)), dtype=np.float32)


def sdf_grid_values(
    mesh: MeshDocument,
    *,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    winding_threshold: float = 0.5,
) -> np.ndarray | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None

    rust_origin = np.asarray(origin, dtype=np.float64)
    rust_shape = np.asarray(shape, dtype=np.int64)
    if rust_origin.shape != (3,):
        raise ValueError("origin must have shape (3,)")
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if not np.isfinite(voxel_size_mm) or voxel_size_mm <= 0:
        raise ValueError("voxel_size_mm must be positive")
    values = _common._rs.sdf_grid_values(
        mesh.vertices,
        mesh.faces,
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(winding_threshold),
    )
    return np.asarray(values, dtype=np.float32).reshape(tuple(int(value) for value in rust_shape))


def first_ray_hit(
    mesh: MeshDocument,
    origin: np.ndarray | tuple[float, float, float],
    direction: np.ndarray | tuple[float, float, float],
    *,
    epsilon: float = 1e-8,
    ignore_faces: set[int] | np.ndarray | None = None,
) -> dict[str, Any] | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None

    ray_origin = np.asarray(origin, dtype=np.float64)
    ray_direction = np.asarray(direction, dtype=np.float64)
    if ray_origin.shape != (3,):
        raise ValueError("origin must have shape (3,)")
    if ray_direction.shape != (3,):
        raise ValueError("direction must have shape (3,)")
    if ignore_faces is None:
        ignored = np.zeros(0, dtype=np.int64)
    else:
        ignored = np.asarray(list(ignore_faces) if isinstance(ignore_faces, set) else ignore_faces, dtype=np.int64).reshape(-1)

    payload = _common._rs.first_ray_hit(mesh.vertices, mesh.faces, ray_origin, ray_direction, float(epsilon), ignored)
    if payload is None:
        return None
    return {
        "face_index": int(payload["face_index"]),
        "distance": float(payload["distance"]),
        "point": tuple(float(value) for value in payload["point"]),
    }


def first_ray_hits(
    mesh: MeshDocument,
    origins: np.ndarray,
    directions: np.ndarray,
    *,
    epsilon: float = 1e-8,
    ignore_faces: set[int] | np.ndarray | None = None,
) -> dict[str, np.ndarray] | None:
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, "first_ray_hits"):
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose first_ray_hits")
        return None

    ray_origins = np.asarray(origins, dtype=np.float64)
    ray_directions = np.asarray(directions, dtype=np.float64)
    if ray_origins.ndim != 2 or ray_origins.shape[1] != 3:
        raise ValueError("origins must have shape (n, 3)")
    if ray_directions.ndim != 2 or ray_directions.shape[1] != 3:
        raise ValueError("directions must have shape (n, 3)")
    if ray_origins.shape[0] != ray_directions.shape[0]:
        raise ValueError("origins and directions must contain the same number of rays")
    if ignore_faces is None:
        ignored = np.zeros(0, dtype=np.int64)
    else:
        ignored = np.asarray(list(ignore_faces) if isinstance(ignore_faces, set) else ignore_faces, dtype=np.int64).reshape(-1)

    payload = _common._rs.first_ray_hits(
        mesh.vertices,
        mesh.faces,
        ray_origins,
        ray_directions,
        float(epsilon),
        ignored,
    )
    return {
        "face_indices": np.asarray(payload["face_indices"], dtype=np.int64),
        "distances": np.asarray(payload["distances"], dtype=np.float64),
        "points": np.asarray(payload["points"], dtype=np.float64).reshape(-1, 3),
    }
