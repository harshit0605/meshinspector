from __future__ import annotations

import numpy as np
import pytest

from geometry_sdk.accelerators import _rust_common, rust
from geometry_sdk import GeometrySDK, default_sdk
from geometry_sdk.analysis.compare import (
    compare_summary,
    nearest_surface_distances,
    nearest_vertex_distances,
    service_compare_distances,
    service_compare_summary,
    signed_compare_summary,
    signed_surface_distances,
    version_compare_distances,
    version_compare_summary,
)
from geometry_sdk.spatial.aabb_tree import build_aabb_tree, closest_candidate_faces, overlapping_face_pairs, ray_candidate_faces
from geometry_sdk.spatial.closest_point import closest_point_on_triangle, closest_points_on_mesh, point_mesh_distances
from geometry_sdk.spatial.intersections import exact_mesh_intersections, self_intersecting_faces, triangles_intersect
from geometry_sdk.spatial.raycast import RayHit, first_ray_hit, first_ray_hits, ray_triangle_hits
from geometry_sdk.spatial.signed_distance import (
    point_inside_mesh,
    point_inside_mesh_winding,
    signed_point_mesh_distances,
    supports_winding_sign,
    winding_numbers,
)
from geometry_sdk.testing.fixtures import crossing_triangles, cube, meshlib_self_intersecting_torus, open_cube, ring_with_head
from geometry_sdk.types import MeshDocument


def _overlapping_closed_cubes() -> MeshDocument:
    left = cube(size=2.0)
    right = cube(size=2.0).copy(vertices=cube(size=2.0).vertices + np.array([0.75, 0.0, 0.0], dtype=np.float64))
    return MeshDocument(
        vertices=np.vstack([left.vertices, right.vertices]),
        faces=np.vstack([left.faces, right.faces + left.vertex_count]),
        metadata={"fixture": "overlapping_closed_cubes"},
    )


def _inside_tetrahedron() -> MeshDocument:
    return MeshDocument(
        vertices=np.array(
            [
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.0, 0.5, -0.5],
                [0.0, 0.0, 0.75],
            ],
            dtype=np.float64,
        ),
        faces=np.array(
            [
                [0, 2, 1],
                [0, 1, 3],
                [1, 2, 3],
                [2, 0, 3],
            ],
            dtype=np.int64,
        ),
    )


def _crossing_triangle_pair() -> tuple[MeshDocument, MeshDocument]:
    first = MeshDocument(
        vertices=np.array([[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={"fixture": "collision_first_triangle"},
    )
    second = MeshDocument(
        vertices=np.array([[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [3.0, 0.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={"fixture": "collision_second_triangle"},
    )
    return first, second


def test_exact_mesh_intersections_exposes_meshlib_style_collision_face_pairs() -> None:
    first, second = _crossing_triangle_pair()

    result = exact_mesh_intersections(first, second)

    assert result.colliding is True
    assert result.pair_count == 1
    assert result.first_face_indices == [0]
    assert result.second_face_indices == [0]
    assert result.pairs[0].first_face == 0
    assert result.pairs[0].second_face == 0
    assert result.pairs[0].intersection_count > 0


def _python_nearest_vertex_distances(source: MeshDocument, target: MeshDocument, *, chunk_size: int = 4096) -> np.ndarray:
    if source.vertex_count == 0 or target.vertex_count == 0:
        return np.zeros(source.vertex_count, dtype=np.float32)

    output = np.empty(source.vertex_count, dtype=np.float64)
    target_vertices = target.vertices
    for start in range(0, source.vertex_count, chunk_size):
        points = source.vertices[start : start + chunk_size]
        diff = points[:, None, :] - target_vertices[None, :, :]
        output[start : start + len(points)] = np.sqrt(np.min(np.einsum("ijk,ijk->ij", diff, diff), axis=1))
    return output.astype(np.float32)


def _python_nearest_surface_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray:
    if source.vertex_count == 0:
        return np.zeros(0, dtype=np.float32)
    return _python_point_mesh_distances(source.vertices, target)


def _python_closest_point_on_triangle(point: np.ndarray, triangle: np.ndarray) -> np.ndarray:
    p = np.asarray(point, dtype=np.float64)
    a, b, c = np.asarray(triangle, dtype=np.float64)
    ab = b - a
    ac = c - a
    ap = p - a

    d1 = float(np.dot(ab, ap))
    d2 = float(np.dot(ac, ap))
    if d1 <= 0.0 and d2 <= 0.0:
        return a.copy()

    bp = p - b
    d3 = float(np.dot(ab, bp))
    d4 = float(np.dot(ac, bp))
    if d3 >= 0.0 and d4 <= d3:
        return b.copy()

    vc = d1 * d4 - d3 * d2
    if vc <= 0.0 and d1 >= 0.0 and d3 <= 0.0:
        v = d1 / (d1 - d3)
        return a + v * ab

    cp = p - c
    d5 = float(np.dot(ab, cp))
    d6 = float(np.dot(ac, cp))
    if d6 >= 0.0 and d5 <= d6:
        return c.copy()

    vb = d5 * d2 - d1 * d6
    if vb <= 0.0 and d2 >= 0.0 and d6 <= 0.0:
        w = d2 / (d2 - d6)
        return a + w * ac

    va = d3 * d6 - d5 * d4
    if va <= 0.0 and (d4 - d3) >= 0.0 and (d5 - d6) >= 0.0:
        w = (d4 - d3) / ((d4 - d3) + (d5 - d6))
        return b + w * (c - b)

    denom = 1.0 / (va + vb + vc)
    v = vb * denom
    w = vc * denom
    return a + ab * v + ac * w


def _python_point_mesh_distances(points: np.ndarray, mesh: MeshDocument) -> np.ndarray:
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    if mesh.face_count == 0:
        return np.full(query.shape[0], np.inf, dtype=np.float32)
    triangles = mesh.vertices[mesh.faces]
    output = np.empty(query.shape[0], dtype=np.float64)
    for point_index, point in enumerate(query):
        best = np.inf
        for triangle in triangles:
            closest = _python_closest_point_on_triangle(point, triangle)
            best = min(best, float(np.linalg.norm(point - closest)))
        output[point_index] = best
    return output.astype(np.float32)


def _python_closest_points_on_mesh(points: np.ndarray, mesh: MeshDocument) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    closest_points = np.zeros_like(query)
    distances = np.full(query.shape[0], np.inf, dtype=np.float64)
    face_indices = np.full(query.shape[0], -1, dtype=np.int64)
    if mesh.face_count == 0:
        return closest_points, distances, face_indices
    triangles = mesh.vertices[mesh.faces]
    for point_index, point in enumerate(query):
        for face_index, triangle in enumerate(triangles):
            closest = _python_closest_point_on_triangle(point, triangle)
            distance = float(np.linalg.norm(point - closest))
            if distance < distances[point_index]:
                closest_points[point_index] = closest
                distances[point_index] = distance
                face_indices[point_index] = face_index
    return closest_points, distances, face_indices


def _python_triangle_solid_angles(points: np.ndarray, triangle: np.ndarray) -> np.ndarray:
    a = triangle[0] - points
    b = triangle[1] - points
    c = triangle[2] - points
    la = np.linalg.norm(a, axis=1)
    lb = np.linalg.norm(b, axis=1)
    lc = np.linalg.norm(c, axis=1)
    numerator = np.einsum("ij,ij->i", a, np.cross(b, c))
    denominator = (
        la * lb * lc
        + np.einsum("ij,ij->i", a, b) * lc
        + np.einsum("ij,ij->i", b, c) * la
        + np.einsum("ij,ij->i", c, a) * lb
    )
    return 2.0 * np.arctan2(numerator, denominator)


def _python_winding_numbers(points: np.ndarray, mesh: MeshDocument) -> np.ndarray:
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    output = np.zeros(query.shape[0], dtype=np.float64)
    for triangle in mesh.vertices[mesh.faces]:
        output += _python_triangle_solid_angles(query, triangle)
    return output / (4.0 * np.pi)


def _python_supports_winding_sign(mesh: MeshDocument) -> bool:
    if mesh.face_count == 0:
        return False
    edge_faces: dict[tuple[int, int], list[int]] = {}
    for face_index, face in enumerate(mesh.faces):
        for start, end in ((face[0], face[1]), (face[1], face[2]), (face[2], face[0])):
            edge = tuple(sorted((int(start), int(end))))
            edge_faces.setdefault(edge, []).append(face_index)
    if not all(len(face_ids) == 2 for face_ids in edge_faces.values()):
        return False
    return not _python_self_intersecting_faces(mesh)


def _python_signed_point_mesh_distances(points: np.ndarray, mesh: MeshDocument, *, sign_method: str = "auto") -> np.ndarray:
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    method = "winding" if sign_method == "auto" and _python_supports_winding_sign(mesh) else sign_method
    if method == "auto":
        method = "unsigned"
    distances = _python_point_mesh_distances(query, mesh)
    if method == "unsigned":
        return distances
    if method == "winding":
        signs = np.where(np.abs(_python_winding_numbers(query, mesh)) >= 0.5, -1.0, 1.0).astype(np.float32)
        return distances * signs
    if method == "ray":
        signs = np.ones(query.shape[0], dtype=np.float32)
        for index, point in enumerate(query):
            hits = _python_ray_triangle_hits(mesh, point, (1.0, 0.371, 0.219), epsilon=1e-7)
            unique: list[float] = []
            for hit in hits:
                if hit.distance <= 1e-7:
                    continue
                if not unique or abs(hit.distance - unique[-1]) > 1e-5:
                    unique.append(hit.distance)
            if len(unique) % 2 == 1:
                signs[index] = -1.0
        return distances * signs
    raise ValueError("sign_method must be 'auto', 'winding', 'ray', or 'unsigned'")


def _python_signed_surface_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray:
    if source.vertex_count == 0:
        return np.zeros(0, dtype=np.float32)
    return _python_signed_point_mesh_distances(source.vertices, target)


def _python_point_inside_mesh(mesh: MeshDocument, point: np.ndarray | tuple[float, float, float]) -> bool:
    hits = _python_ray_triangle_hits(mesh, point, (1.0, 0.371, 0.219), epsilon=1e-7)
    unique: list[float] = []
    for hit in hits:
        if hit.distance <= 1e-7:
            continue
        if not unique or abs(hit.distance - unique[-1]) > 1e-5:
            unique.append(hit.distance)
    return len(unique) % 2 == 1


def _python_compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
    distances = _python_nearest_surface_distances(source, target)
    if distances.size == 0:
        return {"min_distance_mm": None, "max_distance_mm": None, "mean_distance_mm": None}
    return {
        "min_distance_mm": float(np.min(distances)),
        "max_distance_mm": float(np.max(distances)),
        "mean_distance_mm": float(np.mean(distances, dtype=np.float64)),
    }


def _python_signed_compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
    distances = _python_signed_surface_distances(source, target)
    finite = distances[np.isfinite(distances)]
    if finite.size == 0:
        return {"min_signed_distance_mm": None, "max_signed_distance_mm": None, "mean_signed_distance_mm": None}
    return {
        "min_signed_distance_mm": float(np.min(finite)),
        "max_signed_distance_mm": float(np.max(finite)),
        "mean_signed_distance_mm": float(np.mean(finite, dtype=np.float64)),
    }


def _python_aabb_overlap(a: np.ndarray, b: np.ndarray, *, epsilon: float) -> bool:
    return bool(np.all(a.min(axis=0) <= b.max(axis=0) + epsilon) and np.all(b.min(axis=0) <= a.max(axis=0) + epsilon))


def _python_segment_intersects_triangle(p0: np.ndarray, p1: np.ndarray, tri: np.ndarray, *, epsilon: float) -> bool:
    direction = p1 - p0
    a, b, c = tri
    edge1 = b - a
    edge2 = c - a
    h = np.cross(direction, edge2)
    det = float(np.dot(edge1, h))
    if abs(det) < epsilon:
        return False
    inv_det = 1.0 / det
    s = p0 - a
    u = inv_det * float(np.dot(s, h))
    if u < -epsilon or u > 1.0 + epsilon:
        return False
    q = np.cross(s, edge1)
    v = inv_det * float(np.dot(direction, q))
    if v < -epsilon or u + v > 1.0 + epsilon:
        return False
    t = inv_det * float(np.dot(edge2, q))
    return bool(-epsilon <= t <= 1.0 + epsilon)


def _python_point_in_triangle(point: np.ndarray, tri: np.ndarray, *, epsilon: float) -> bool:
    a, b, c = tri
    normal = np.cross(b - a, c - a)
    if np.linalg.norm(normal) < epsilon:
        return False
    if abs(float(np.dot(point - a, normal))) > epsilon * max(np.linalg.norm(normal), 1.0):
        return False
    v0 = c - a
    v1 = b - a
    v2 = point - a
    dot00 = float(np.dot(v0, v0))
    dot01 = float(np.dot(v0, v1))
    dot02 = float(np.dot(v0, v2))
    dot11 = float(np.dot(v1, v1))
    dot12 = float(np.dot(v1, v2))
    denom = dot00 * dot11 - dot01 * dot01
    if abs(denom) < epsilon:
        return False
    inv = 1.0 / denom
    u = (dot11 * dot02 - dot01 * dot12) * inv
    v = (dot00 * dot12 - dot01 * dot02) * inv
    return bool(u >= -epsilon and v >= -epsilon and u + v <= 1.0 + epsilon)


def _python_triangles_intersect(triangle_a: np.ndarray, triangle_b: np.ndarray, *, epsilon: float = 1e-8) -> bool:
    a = np.asarray(triangle_a, dtype=np.float64)
    b = np.asarray(triangle_b, dtype=np.float64)
    if not _python_aabb_overlap(a, b, epsilon=epsilon):
        return False
    for index in range(3):
        if _python_segment_intersects_triangle(a[index], a[(index + 1) % 3], b, epsilon=epsilon):
            return True
        if _python_segment_intersects_triangle(b[index], b[(index + 1) % 3], a, epsilon=epsilon):
            return True
    return any(_python_point_in_triangle(point, b, epsilon=epsilon) for point in a) or any(
        _python_point_in_triangle(point, a, epsilon=epsilon) for point in b
    )


def _python_self_intersecting_faces(mesh: MeshDocument, *, epsilon: float = 1e-8) -> set[int]:
    if mesh.face_count < 2:
        return set()
    triangles = mesh.vertices[mesh.faces]
    tree = build_aabb_tree(mesh, leaf_size=16)
    intersecting: set[int] = set()
    face_vertex_sets = [set(int(vertex_id) for vertex_id in face) for face in mesh.faces]
    for face_a, face_b in overlapping_face_pairs(tree, epsilon=epsilon):
        if face_vertex_sets[face_a] & face_vertex_sets[face_b]:
            continue
        if _python_triangles_intersect(triangles[face_a], triangles[face_b], epsilon=epsilon):
            intersecting.add(face_a)
            intersecting.add(face_b)
    return intersecting


def _python_ray_triangle_distance(origin: np.ndarray, direction: np.ndarray, triangle: np.ndarray, epsilon: float) -> float | None:
    a, b, c = triangle
    edge1 = b - a
    edge2 = c - a
    h = np.cross(direction, edge2)
    det = float(np.dot(edge1, h))
    if -epsilon < det < epsilon:
        return None

    inv_det = 1.0 / det
    s = origin - a
    u = inv_det * float(np.dot(s, h))
    if u < -epsilon or u > 1.0 + epsilon:
        return None

    q = np.cross(s, edge1)
    v = inv_det * float(np.dot(direction, q))
    if v < -epsilon or u + v > 1.0 + epsilon:
        return None

    distance = inv_det * float(np.dot(edge2, q))
    if distance <= epsilon:
        return None
    return distance


def _python_ray_triangle_hits(
    mesh: MeshDocument,
    origin: np.ndarray | tuple[float, float, float],
    direction: np.ndarray | tuple[float, float, float],
    *,
    epsilon: float = 1e-8,
    ignore_faces: set[int] | np.ndarray | None = None,
) -> list[RayHit]:
    ray_origin = np.asarray(origin, dtype=np.float64)
    ray_direction = np.asarray(direction, dtype=np.float64)
    ray_direction = ray_direction / np.linalg.norm(ray_direction)
    ignored = set(int(face_id) for face_id in ignore_faces) if ignore_faces is not None else set()
    hits: list[RayHit] = []
    for face_index, face in enumerate(mesh.faces):
        if face_index in ignored:
            continue
        distance = _python_ray_triangle_distance(ray_origin, ray_direction, mesh.vertices[face], epsilon)
        if distance is None:
            continue
        point = ray_origin + ray_direction * distance
        hits.append(
            RayHit(
                face_index=face_index,
                distance=float(distance),
                point=tuple(float(value) for value in point),
            )
        )
    return sorted(hits, key=lambda hit: hit.distance)


def _python_first_ray_hits(mesh: MeshDocument, origins: np.ndarray, directions: np.ndarray) -> list[RayHit | None]:
    output: list[RayHit | None] = []
    for origin, direction in zip(origins, directions):
        hits = _python_ray_triangle_hits(mesh, origin=origin, direction=direction)
        output.append(hits[0] if hits else None)
    return output


def test_closest_point_on_triangle_projects_to_face() -> None:
    triangle = np.array([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]], dtype=np.float64)
    point = np.array([0.5, 0.5, 2.0], dtype=np.float64)

    closest = closest_point_on_triangle(point, triangle)

    assert np.allclose(closest, [0.5, 0.5, 0.0])
    assert np.allclose(closest, _python_closest_point_on_triangle(point, triangle))


def test_point_mesh_distance_to_cube_surface() -> None:
    mesh = cube(size=2.0)
    distances = point_mesh_distances(np.array([[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]], dtype=np.float64), mesh)

    assert np.allclose(distances, [1.0, 1.0])


def test_closest_points_on_mesh_returns_points_distances_and_faces() -> None:
    mesh = cube(size=2.0)
    closest, distances, face_indices = closest_points_on_mesh(
        np.array([[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]], dtype=np.float64),
        mesh,
    )

    assert np.allclose(closest[0], [1.0, 0.0, 0.0])
    assert np.allclose(distances, [1.0, 1.0])
    assert face_indices.shape == (2,)


def test_ray_hits_cube_front_face() -> None:
    mesh = cube(size=2.0)
    hit = first_ray_hit(mesh, origin=(0.0, 0.0, 3.0), direction=(0.0, 0.0, -1.0))

    assert hit is not None
    assert np.isclose(hit.distance, 2.0)
    assert np.allclose(hit.point, [0.0, 0.0, 1.0])


def test_first_ray_hits_returns_hits_and_misses() -> None:
    mesh = cube(size=2.0)
    hits = first_ray_hits(
        mesh,
        origins=np.array([[0.0, 0.0, 3.0], [4.0, 4.0, 4.0]], dtype=np.float64),
        directions=np.array([[0.0, 0.0, -1.0], [1.0, 0.0, 0.0]], dtype=np.float64),
    )

    assert hits[0] is not None
    assert np.isclose(hits[0].distance, 2.0)
    assert np.allclose(hits[0].point, [0.0, 0.0, 1.0])
    assert hits[1] is None


def test_aabb_tree_returns_relevant_ray_candidates() -> None:
    mesh = cube(size=2.0)
    tree = build_aabb_tree(mesh, leaf_size=2)
    candidates = ray_candidate_faces(tree, np.array([0.0, 0.0, 3.0]), np.array([0.0, 0.0, -1.0]))

    assert candidates.size > 0
    assert candidates.size < mesh.face_count


def test_aabb_tree_returns_overlapping_face_pairs_for_intersection_broad_phase() -> None:
    tree = build_aabb_tree(crossing_triangles(), leaf_size=1)

    assert overlapping_face_pairs(tree) == [(0, 1)]


def test_aabb_tree_module_is_rust_owned(monkeypatch) -> None:
    tree = build_aabb_tree(cube(size=2.0), leaf_size=2)

    assert ray_candidate_faces(tree, np.array([0.0, 0.0, 3.0]), np.array([0.0, 0.0, -1.0])).size > 0
    assert closest_candidate_faces(tree, np.array([0.0, 0.0, 3.0]), 100.0).size > 0

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel aabb_ray_candidate_faces is required"):
        ray_candidate_faces(tree, np.array([0.0, 0.0, 3.0]), np.array([0.0, 0.0, -1.0]))


def test_compare_uses_surface_distance_summary() -> None:
    source = cube(size=2.0)
    target = cube(size=4.0)
    distances = nearest_surface_distances(source, target)
    summary = compare_summary(source, target)

    assert np.allclose(distances, 1.0)
    assert np.isclose(summary["mean_distance_mm"], 1.0)


def test_signed_distance_classifies_inside_and_outside_points() -> None:
    mesh = cube(size=2.0)
    points = np.array([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]], dtype=np.float64)
    distances = signed_point_mesh_distances(points, mesh)

    assert point_inside_mesh(mesh, points[0])
    assert point_inside_mesh_winding(mesh, points[0])
    assert not point_inside_mesh(mesh, points[1])
    assert not point_inside_mesh_winding(mesh, points[1])
    assert np.allclose(distances, [-1.0, 1.0])


def test_signed_distance_defaults_to_unsigned_for_open_meshes() -> None:
    mesh = open_cube(size=2.0)
    points = np.array([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]], dtype=np.float64)

    assert not supports_winding_sign(mesh)
    assert not point_inside_mesh_winding(mesh, points[0])
    assert point_inside_mesh_winding(mesh, points[0], require_closed=False)
    assert np.allclose(signed_point_mesh_distances(points, mesh), [1.0, 1.0])
    assert np.allclose(signed_point_mesh_distances(points, mesh, sign_method="winding"), [-1.0, 1.0])


def test_signed_distance_defaults_to_unsigned_for_nonmanifold_meshes() -> None:
    source = cube(size=2.0)
    mesh = source.copy(faces=np.vstack([source.faces, source.faces[0]]))

    assert not supports_winding_sign(mesh)
    distances = signed_point_mesh_distances(np.array([[0.0, 0.0, 0.0]], dtype=np.float64), mesh)

    assert np.all(distances >= 0.0)


def test_signed_distance_defaults_to_unsigned_for_self_intersecting_closed_meshes() -> None:
    mesh = _overlapping_closed_cubes()
    points = np.array([[0.0, 0.0, 0.0], [3.0, 0.0, 0.0]], dtype=np.float64)

    assert not supports_winding_sign(mesh)
    assert supports_winding_sign(mesh, reject_self_intersections=False)
    assert np.allclose(signed_point_mesh_distances(points, mesh), [0.25, 1.25])
    assert np.allclose(signed_point_mesh_distances(points, mesh, sign_method="winding"), [-0.25, 1.25])


def test_winding_number_is_stable_for_cube_points() -> None:
    mesh = cube(size=2.0)
    values = winding_numbers(np.array([[0.0, 0.0, 0.0], [3.0, 0.0, 0.0]], dtype=np.float64), mesh)

    assert np.isclose(abs(values[0]), 1.0)
    assert np.isclose(values[1], 0.0)


def test_signed_compare_reports_negative_when_source_inside_target() -> None:
    source = cube(size=2.0)
    target = cube(size=4.0)
    distances = signed_surface_distances(source, target)
    summary = signed_compare_summary(source, target)

    assert np.allclose(distances, -1.0)
    assert np.isclose(summary["mean_signed_distance_mm"], -1.0)


def test_version_compare_summary_matches_service_compare_contract() -> None:
    source = cube(size=2.0)
    target = cube(size=4.0)

    summary = version_compare_summary(source, target)

    assert np.isclose(summary.volume_delta_mm3, -56.0)
    assert summary.bbox_delta_mm == (-2.0, -2.0, -2.0)
    assert np.isclose(summary.min_signed_distance_mm, -1.0)
    assert np.isclose(summary.max_signed_distance_mm, -1.0)
    assert np.isclose(summary.mean_signed_distance_mm, -1.0)
    assert summary.weight_delta_g == 0.0


def test_version_compare_distances_apply_service_outlier_filter() -> None:
    source = cube(size=2.0)
    target = cube(size=2.0).copy(vertices=cube(size=2.0).vertices + np.array([100.0, 0.0, 0.0], dtype=np.float64))

    values = version_compare_distances(source, target)
    summary = version_compare_summary(source, target)

    assert values.shape == (source.vertex_count,)
    assert np.all(np.isnan(values))
    assert summary.min_signed_distance_mm is None
    assert summary.max_signed_distance_mm is None
    assert summary.mean_signed_distance_mm is None


def test_service_compare_matches_meshlib_reference_mesh_direction(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    source = cube(size=2.0)
    other = _inside_tetrahedron()

    service_field = service_compare_distances(source, other)
    expected_field = version_compare_distances(other, source)
    service_summary = service_compare_summary(source, other)

    assert service_field.shape == (other.vertex_count,)
    assert np.array_equal(service_field, expected_field)
    assert service_summary.volume_delta_mm3 > 0.0
    assert service_summary.bbox_delta_mm == pytest.approx((1.0, 1.0, 0.75))
    assert service_summary.min_signed_distance_mm == pytest.approx(float(np.nanmin(service_field)))

    sdk = GeometrySDK()
    assert np.array_equal(sdk.service_compare_field(source, other), service_field)
    assert sdk.service_compare(source, other).bbox_delta_mm == pytest.approx((1.0, 1.0, 0.75))

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel service_compare_distances is required"):
        service_compare_distances(source, other)


def test_compare_module_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    source = cube(size=2.0)
    target = cube(size=4.0)
    open_target = open_cube(size=2.0)

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    reference_vertex_distances = _python_nearest_vertex_distances(source, target)
    reference_surface_distances = _python_nearest_surface_distances(source, target)
    reference_signed_distances = _python_signed_surface_distances(source, target)
    reference_open_signed_distances = _python_signed_surface_distances(source, open_target)
    reference_summary = _python_compare_summary(source, target)
    reference_signed_summary = _python_signed_compare_summary(source, target)
    reference_version_distances = version_compare_distances(source, target)
    reference_version_summary = version_compare_summary(source, target)

    rust_vertex_distances = nearest_vertex_distances(source, target)
    rust_surface_distances = nearest_surface_distances(source, target)
    rust_signed_distances = signed_surface_distances(source, target)
    rust_open_signed_distances = signed_surface_distances(source, open_target)
    rust_summary = compare_summary(source, target)
    rust_signed_summary = signed_compare_summary(source, target)
    rust_version_distances = version_compare_distances(source, target)
    rust_version_summary = version_compare_summary(source, target)

    assert np.allclose(rust_vertex_distances, reference_vertex_distances, atol=1e-6)
    assert np.allclose(rust_surface_distances, reference_surface_distances, atol=1e-6)
    assert np.allclose(rust_signed_distances, reference_signed_distances, atol=1e-6)
    assert np.allclose(rust_open_signed_distances, reference_open_signed_distances, atol=1e-6)
    assert rust_summary == reference_summary
    assert rust_signed_summary == reference_signed_summary
    assert np.allclose(rust_version_distances, reference_version_distances, equal_nan=True)
    assert rust_version_summary == reference_version_summary

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel compare_summary is required"):
        compare_summary(source, target)
    with pytest.raises(RuntimeError, match="Rust kernel version_compare_summary is required"):
        version_compare_summary(source, target)
    with pytest.raises(RuntimeError, match="Rust kernel version_compare_distances is required"):
        version_compare_distances(source, target)


def test_triangle_intersection_detects_crossing_faces() -> None:
    mesh = crossing_triangles()
    triangles = mesh.vertices[mesh.faces]

    assert triangles_intersect(triangles[0], triangles[1])
    assert triangles_intersect(triangles[0], triangles[1]) == _python_triangles_intersect(triangles[0], triangles[1])
    assert self_intersecting_faces(mesh) == {0, 1}
    assert default_sdk.self_intersecting_faces(mesh) == {0, 1}


def test_self_intersections_support_meshlib_no_touch_mode() -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = meshlib_self_intersecting_torus()

    assert mesh.face_count == 1024
    assert len(self_intersecting_faces(mesh)) == 256
    assert len(self_intersecting_faces(mesh, touch_is_intersection=False)) == 128


@pytest.mark.parametrize("mesh_factory", [cube, crossing_triangles, ring_with_head])
def test_intersections_module_is_rust_owned_and_matches_reference(monkeypatch, mesh_factory) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = mesh_factory()
    reference_faces = _python_self_intersecting_faces(mesh)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_faces = self_intersecting_faces(mesh)

    assert rust_faces == reference_faces

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel self_intersecting_faces is required"):
        self_intersecting_faces(mesh)


@pytest.mark.parametrize("mesh_factory", [cube, ring_with_head])
def test_closest_point_distances_are_rust_owned_and_match_reference(monkeypatch, mesh_factory) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = mesh_factory()
    points = np.vstack([mesh.vertices[:5], mesh.vertices[:5] + np.array([0.25, 0.1, -0.2])])

    python_distances = _python_point_mesh_distances(points, mesh)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_distances = point_mesh_distances(points, mesh)

    assert np.allclose(rust_distances, python_distances, atol=1e-6)

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel point_mesh_distances is required"):
        point_mesh_distances(points, mesh)


@pytest.mark.parametrize("mesh_factory", [cube, ring_with_head])
def test_closest_points_module_is_rust_owned_and_matches_reference(monkeypatch, mesh_factory) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = mesh_factory()
    points = np.vstack([mesh.vertices[:5], mesh.vertices[:5] + np.array([0.25, 0.1, -0.2])])

    python_closest, python_distances, _python_faces = _python_closest_points_on_mesh(points, mesh)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_closest, rust_distances, rust_faces = closest_points_on_mesh(points, mesh)

    assert np.allclose(rust_closest, python_closest, atol=1e-6)
    assert np.allclose(rust_distances, python_distances, atol=1e-6)
    assert rust_faces.shape == (points.shape[0],)


@pytest.mark.parametrize("mesh_factory", [cube, ring_with_head])
def test_signed_distance_winding_numbers_are_rust_owned_and_match_reference(monkeypatch, mesh_factory) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = mesh_factory()
    center = mesh.vertices.mean(axis=0)
    points = np.vstack([center, mesh.vertices[:5] + np.array([0.2, -0.1, 0.15])])

    python_values = _python_winding_numbers(points, mesh)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_values = winding_numbers(points, mesh)

    assert np.allclose(rust_values, python_values, atol=1e-8)


@pytest.mark.parametrize("mesh_factory", [cube, ring_with_head])
def test_signed_distance_module_is_rust_owned_and_matches_reference(monkeypatch, mesh_factory) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = mesh_factory()
    center = mesh.vertices.mean(axis=0)
    points = np.vstack([center, mesh.vertices[:5] + np.array([0.2, -0.1, 0.15])])

    python_values = _python_signed_point_mesh_distances(points, mesh)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_values = signed_point_mesh_distances(points, mesh)

    assert np.allclose(rust_values, python_values, atol=1e-6)

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel signed_point_mesh_distances_with_method is required"):
        signed_point_mesh_distances(points, mesh)


def test_raycast_module_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = cube(size=2.0)
    origin = (0.0, 0.0, 3.0)
    direction = (0.0, 0.0, -1.0)
    reference_hits = _python_ray_triangle_hits(mesh, origin=origin, direction=direction)

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_hits = ray_triangle_hits(mesh, origin=origin, direction=direction)
    rust_hit = first_ray_hit(mesh, origin=origin, direction=direction)

    assert len(rust_hits) == len(reference_hits)
    assert np.allclose([hit.distance for hit in rust_hits], [hit.distance for hit in reference_hits])
    assert np.allclose([hit.point for hit in rust_hits], [hit.point for hit in reference_hits])

    assert rust_hit is not None
    assert np.isclose(rust_hit.distance, reference_hits[0].distance)
    assert np.allclose(rust_hit.point, reference_hits[0].point)

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel first_ray_hit is required"):
        first_ray_hit(mesh, origin=origin, direction=direction)


def test_rust_owned_first_ray_hits_match_reference(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = ring_with_head()
    origins = np.array(
        [
            [0.0, 0.0, 16.0],
            [12.0, 0.0, 0.0],
            [30.0, 30.0, 30.0],
        ],
        dtype=np.float64,
    )
    directions = np.array(
        [
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        dtype=np.float64,
    )

    reference_hits = _python_first_ray_hits(mesh, origins, directions)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_hits = first_ray_hits(mesh, origins=origins, directions=directions)

    assert len(rust_hits) == len(reference_hits)
    for rust_hit, reference_hit in zip(rust_hits, reference_hits):
        if reference_hit is None:
            assert rust_hit is None
        else:
            assert rust_hit is not None
            assert np.isclose(rust_hit.distance, reference_hit.distance, atol=1e-8)
            assert np.allclose(rust_hit.point, reference_hit.point, atol=1e-8)
