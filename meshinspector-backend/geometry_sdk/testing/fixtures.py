"""Procedural fixture meshes for parity tests."""

from __future__ import annotations

import numpy as np

from geometry_sdk.types import MeshDocument


def cube(size: float = 1.0) -> MeshDocument:
    half = size / 2.0
    vertices = np.array(
        [
            [-half, -half, -half],
            [half, -half, -half],
            [half, half, -half],
            [-half, half, -half],
            [-half, -half, half],
            [half, -half, half],
            [half, half, half],
            [-half, half, half],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 3, 2], [0, 2, 1],
            [4, 5, 6], [4, 6, 7],
            [0, 1, 5], [0, 5, 4],
            [1, 2, 6], [1, 6, 5],
            [2, 3, 7], [2, 7, 6],
            [3, 0, 4], [3, 4, 7],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices=vertices, faces=faces, metadata={"fixture": "cube"})


def closed_cube_with_flipped_top_triangle() -> MeshDocument:
    vertices = np.array(
        [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 3, 2],
            [0, 2, 1],
            [4, 6, 5],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices=vertices, faces=faces, metadata={"fixture": "closed_cube_with_flipped_top_triangle"})


def box(size_x: float, size_y: float, size_z: float, center: tuple[float, float, float] = (0.0, 0.0, 0.0)) -> MeshDocument:
    base = cube(1.0)
    scale = np.array([size_x, size_y, size_z], dtype=np.float64)
    offset = np.asarray(center, dtype=np.float64)
    return MeshDocument(
        vertices=base.vertices * scale + offset,
        faces=base.faces.copy(),
        metadata={"fixture": "box", "size": (float(size_x), float(size_y), float(size_z)), "center": tuple(float(x) for x in offset)},
    )


def open_cube(size: float = 1.0) -> MeshDocument:
    mesh = cube(size)
    return mesh.copy(faces=mesh.faces[:-2])


def ring(major_radius: float = 9.0, minor_radius: float = 1.2, radial_segments: int = 48, tube_segments: int = 12) -> MeshDocument:
    vertices: list[list[float]] = []
    faces: list[list[int]] = []
    for i in range(radial_segments):
        theta = 2.0 * np.pi * i / radial_segments
        radial = np.array([np.cos(theta), 0.0, np.sin(theta)], dtype=np.float64)
        tangent_y = np.array([0.0, 1.0, 0.0], dtype=np.float64)
        center = radial * major_radius
        for j in range(tube_segments):
            phi = 2.0 * np.pi * j / tube_segments
            point = center + radial * (minor_radius * np.cos(phi)) + tangent_y * (minor_radius * np.sin(phi))
            vertices.append([float(point[0]), float(point[1]), float(point[2])])
    for i in range(radial_segments):
        ni = (i + 1) % radial_segments
        for j in range(tube_segments):
            nj = (j + 1) % tube_segments
            a = i * tube_segments + j
            b = ni * tube_segments + j
            c = ni * tube_segments + nj
            d = i * tube_segments + nj
            faces.append([a, b, c])
            faces.append([a, c, d])
    return MeshDocument(np.asarray(vertices, dtype=np.float64), np.asarray(faces, dtype=np.int64), metadata={"fixture": "ring"})


def thin_wall_ring() -> MeshDocument:
    mesh = ring(major_radius=9.0, minor_radius=0.35, radial_segments=48, tube_segments=8)
    mesh.metadata["fixture"] = "thin_wall_ring"
    return mesh


def hollowed_ring() -> MeshDocument:
    outer = ring(major_radius=9.0, minor_radius=1.2, radial_segments=48, tube_segments=12)
    inner = ring(major_radius=9.0, minor_radius=0.75, radial_segments=48, tube_segments=12)
    inner_faces = inner.faces[:, [0, 2, 1]] + outer.vertex_count
    return MeshDocument(
        vertices=np.vstack([outer.vertices, inner.vertices]),
        faces=np.vstack([outer.faces, inner_faces]),
        metadata={"fixture": "hollowed_ring"},
    )


def ring_with_head(
    major_radius: float = 9.0,
    minor_radius: float = 1.2,
    head_radius: float = 2.2,
    radial_segments: int = 48,
    tube_segments: int = 12,
) -> MeshDocument:
    base = ring(major_radius=major_radius, minor_radius=minor_radius, radial_segments=radial_segments, tube_segments=tube_segments)
    vertices = base.vertices.copy()
    top_center = np.array([0.0, 0.0, major_radius + minor_radius + head_radius * 0.35], dtype=np.float64)
    start = len(vertices)
    head_vertices = [
        top_center + np.array([0.0, head_radius * 0.55, 0.0]),
        top_center + np.array([head_radius, 0.0, 0.0]),
        top_center + np.array([0.0, 0.0, head_radius]),
        top_center + np.array([-head_radius, 0.0, 0.0]),
        top_center + np.array([0.0, 0.0, -head_radius]),
        top_center + np.array([0.0, -head_radius * 0.55, 0.0]),
    ]
    head_faces = np.array(
        [
            [start, start + 1, start + 2],
            [start, start + 2, start + 3],
            [start, start + 3, start + 4],
            [start, start + 4, start + 1],
            [start + 5, start + 2, start + 1],
            [start + 5, start + 3, start + 2],
            [start + 5, start + 4, start + 3],
            [start + 5, start + 1, start + 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(
        vertices=np.vstack([vertices, np.asarray(head_vertices, dtype=np.float64)]),
        faces=np.vstack([base.faces, head_faces]),
        metadata={"fixture": "ring_with_head"},
    )


def pendant(
    radius_x: float = 5.0,
    radius_z: float = 7.0,
    thickness: float = 1.2,
    radial_segments: int = 16,
) -> MeshDocument:
    top_y = thickness / 2.0
    bottom_y = -top_y
    vertices: list[list[float]] = [[0.0, top_y, 0.0], [0.0, bottom_y, 0.0]]
    for i in range(radial_segments):
        theta = 2.0 * np.pi * i / radial_segments
        vertices.append([radius_x * np.cos(theta), top_y, radius_z * np.sin(theta)])
    for i in range(radial_segments):
        theta = 2.0 * np.pi * i / radial_segments
        vertices.append([radius_x * np.cos(theta), bottom_y, radius_z * np.sin(theta)])

    faces: list[list[int]] = []
    top_center = 0
    bottom_center = 1
    top_start = 2
    bottom_start = top_start + radial_segments
    for i in range(radial_segments):
        ni = (i + 1) % radial_segments
        top_a = top_start + i
        top_b = top_start + ni
        bottom_a = bottom_start + i
        bottom_b = bottom_start + ni
        faces.append([top_center, top_a, top_b])
        faces.append([bottom_center, bottom_b, bottom_a])
        faces.append([top_a, bottom_a, bottom_b])
        faces.append([top_a, bottom_b, top_b])
    return MeshDocument(np.asarray(vertices, dtype=np.float64), np.asarray(faces, dtype=np.int64), metadata={"fixture": "pendant"})


def crossing_triangles() -> MeshDocument:
    vertices = np.array(
        [
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -0.5, -1.0],
            [0.0, -0.5, 1.0],
            [0.0, 1.2, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 2], [3, 4, 5]], dtype=np.int64)
    return MeshDocument(vertices, faces, metadata={"fixture": "crossing_triangles"})


def meshlib_self_intersecting_torus(
    primary_radius: float = 1.0,
    secondary_radius: float = 0.2,
    primary_resolution: int = 32,
    secondary_resolution: int = 16,
) -> MeshDocument:
    vertices: list[list[float]] = []
    for i in range(secondary_resolution):
        a = 2.0 * np.pi * i / secondary_resolution
        for j in range(primary_resolution):
            b = 2.0 * np.pi * j / primary_resolution
            vertices.append(
                [
                    (primary_radius - secondary_radius * np.cos(a)) * np.cos(b),
                    (primary_radius - secondary_radius * np.cos(a)) * np.sin(b),
                    secondary_radius * np.sin(2.0 * a),
                ]
            )

    faces: list[list[int]] = []
    for i in range(secondary_resolution):
        for j in range(primary_resolution):
            faces.append(
                [
                    i * primary_resolution + j,
                    ((i + 1) % secondary_resolution) * primary_resolution + j,
                    i * primary_resolution + (j + 1) % primary_resolution,
                ]
            )
            faces.append(
                [
                    i * primary_resolution + (j + 1) % primary_resolution,
                    ((i + 1) % secondary_resolution) * primary_resolution + j,
                    ((i + 1) % secondary_resolution) * primary_resolution + (j + 1) % primary_resolution,
                ]
            )

    return MeshDocument(
        np.asarray(vertices, dtype=np.float64),
        np.asarray(faces, dtype=np.int64),
        metadata={"fixture": "meshlib_self_intersecting_torus"},
    )
