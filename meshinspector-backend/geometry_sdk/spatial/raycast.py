"""Raycast compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from geometry_sdk.accelerators import _rust_raycast
from geometry_sdk.types import MeshDocument


@dataclass(slots=True)
class RayHit:
    face_index: int
    distance: float
    point: tuple[float, float, float]


def _ray_hit(payload: dict[str, Any]) -> RayHit:
    return RayHit(
        face_index=int(payload["face_index"]),
        distance=float(payload["distance"]),
        point=tuple(float(value) for value in payload["point"]),
    )


def _ray_hits(payload: dict[str, Any]) -> list[RayHit]:
    return [
        RayHit(
            face_index=int(face_index),
            distance=float(distance),
            point=(float(point[0]), float(point[1]), float(point[2])),
        )
        for face_index, distance, point in zip(
            payload["face_indices"],
            payload["distances"],
            payload["points"],
        )
    ]


def ray_triangle_hits(
    mesh: MeshDocument,
    origin: Any,
    direction: Any,
    *,
    epsilon: float = 1e-8,
    ignore_faces: Any = None,
    tree: Any = None,
) -> list[RayHit]:
    _ = tree
    payload = _rust_raycast.ray_triangle_hits(
        mesh,
        origin,
        direction,
        epsilon=epsilon,
        ignore_faces=ignore_faces,
    )
    return _ray_hits(payload)


def first_ray_hit(
    mesh: MeshDocument,
    origin: Any,
    direction: Any,
    *,
    epsilon: float = 1e-8,
    ignore_faces: Any = None,
    tree: Any = None,
) -> RayHit | None:
    _ = tree
    payload = _rust_raycast.first_ray_hit(
        mesh,
        origin,
        direction,
        epsilon=epsilon,
        ignore_faces=ignore_faces,
    )
    return None if payload is None else _ray_hit(payload)


def first_ray_hits(
    mesh: MeshDocument,
    origins: Any,
    directions: Any,
    *,
    epsilon: float = 1e-8,
    ignore_faces: Any = None,
    tree: Any = None,
) -> list[RayHit | None]:
    _ = tree
    payload = _rust_raycast.first_ray_hits(
        mesh,
        origins,
        directions,
        epsilon=epsilon,
        ignore_faces=ignore_faces,
    )
    output: list[RayHit | None] = []
    for face_index, distance, point in zip(payload["face_indices"], payload["distances"], payload["points"]):
        distance_value = float(distance)
        if int(face_index) < 0 or not distance_value < float("inf"):
            output.append(None)
        else:
            output.append(
                RayHit(
                    face_index=int(face_index),
                    distance=distance_value,
                    point=(float(point[0]), float(point[1]), float(point[2])),
                )
            )
    return output
