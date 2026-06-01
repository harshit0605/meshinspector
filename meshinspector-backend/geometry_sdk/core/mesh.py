"""Core mesh calculation compatibility wrappers."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_mesh, _rust_stats
from geometry_sdk.types import MeshDocument, MeshStats


def safe_normalize(vectors):
    return _rust_mesh.safe_normalize(vectors)


def normalize_axis(axis):
    return _rust_mesh.normalize_axis(axis)


def bounds(mesh: MeshDocument):
    return _rust_mesh.bounds(mesh)


def face_normals(mesh: MeshDocument):
    return _rust_mesh.face_normals(mesh)


def vertex_normals(mesh: MeshDocument):
    return _rust_mesh.vertex_normals(mesh)


def surface_area(mesh: MeshDocument) -> float:
    return _rust_mesh.surface_area(mesh)


def signed_volume(mesh: MeshDocument) -> float:
    return _rust_mesh.signed_volume(mesh)


def volume(mesh: MeshDocument) -> float:
    return _rust_mesh.volume(mesh)


def edge_face_map(mesh: MeshDocument) -> dict[tuple[int, int], list[int]]:
    return _rust_mesh.edge_face_map(mesh)


def boundary_edges(mesh: MeshDocument) -> list[tuple[int, int]]:
    return _rust_mesh.boundary_edges_for_core(mesh)


def face_adjacency(mesh: MeshDocument) -> list[list[int]]:
    return _rust_mesh.face_adjacency(mesh)


def connected_face_components(mesh: MeshDocument) -> list[list[int]]:
    return _rust_mesh.connected_face_components(mesh)


def vertex_neighbors(mesh: MeshDocument) -> list[list[int]]:
    return _rust_mesh.vertex_neighbors(mesh)


def mesh_stats(mesh: MeshDocument) -> MeshStats:
    return _rust_stats.mesh_stats(mesh)
