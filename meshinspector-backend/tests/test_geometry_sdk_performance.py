from __future__ import annotations

from collections import defaultdict, deque
from dataclasses import asdict, is_dataclass
import os
from pathlib import Path

import numpy as np
import pytest

from geometry_sdk.accelerators import rust
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.core.mesh import safe_normalize, vertex_neighbors, vertex_normals
from geometry_sdk.analysis.thickness import ray_thickness_at_vertices
from geometry_sdk.deform.brushes import apply_brush_strokes
from geometry_sdk.deform._distance import nearest_distances
from geometry_sdk.deform.local import falloff_weights, local_scoop, local_thicken, smooth
from geometry_sdk.spatial.closest_point import closest_points_on_mesh, point_mesh_distances
from geometry_sdk.spatial.intersections import self_intersecting_faces
from geometry_sdk.spatial.raycast import RayHit, first_ray_hit, first_ray_hits
from geometry_sdk.spatial.aabb_tree import build_aabb_tree, overlapping_face_pairs, ray_candidate_faces
from geometry_sdk.spatial.signed_distance import signed_point_mesh_distances, winding_numbers
from geometry_sdk.testing.fixtures import crossing_triangles, cube, ring
from geometry_sdk.testing.performance import best_of, compare_accelerator_modes
from geometry_sdk.testing.uploaded_fragments import load_npz_mesh
from geometry_sdk.types import BrushStroke, MeshDocument, MeshStats
from geometry_sdk.voxel.marching import extract_marching_tetrahedra
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh, voxel_offset_mesh, voxel_shell_mesh
from geometry_sdk.voxel.refine import laplacian_smooth_vertices, project_vertices_to_sdf, refine_sdf_mesh
from geometry_sdk.voxel.sdf import SDFGrid, sample_sdf_grid


RUST_SPEED_RATIO_BUDGET = 0.75
BACKEND_ROOT = Path(__file__).resolve().parents[1]
UPLOADED_FRAGMENT_DIR = BACKEND_ROOT / "geometry_sdk" / "testing" / "golden_data" / "uploaded_fragments"
UPLOADED_RING_FRAGMENT = "uploaded_ring_processed_component_rank_2.npz"
UPLOADED_PENDANT_FRAGMENT = "uploaded_pendant_processed_component_rank_2.npz"


def _query_points() -> np.ndarray:
    return np.array(
        [
            [x, y, z]
            for x in np.linspace(-12.0, 12.0, 10)
            for y in np.linspace(-2.0, 2.0, 5)
            for z in np.linspace(-12.0, 12.0, 10)
        ],
        dtype=np.float64,
    )


def _crossing_triangle_pairs(count: int) -> MeshDocument:
    source = crossing_triangles()
    vertices = []
    faces = []
    vertex_offset = 0
    grid_width = int(np.ceil(np.sqrt(count)))
    for index in range(count):
        translation = np.array([(index % grid_width) * 3.0, (index // grid_width) * 3.0, 0.0], dtype=np.float64)
        vertices.append(source.vertices + translation)
        faces.append(source.faces + vertex_offset)
        vertex_offset += source.vertex_count
    return MeshDocument(np.vstack(vertices), np.vstack(faces))


def _uploaded_fragment(name: str) -> MeshDocument:
    return load_npz_mesh(UPLOADED_FRAGMENT_DIR / name)


def _python_nearest_distances(vertices: np.ndarray, target_indices: np.ndarray, chunk_size: int = 4096) -> np.ndarray:
    targets = vertices[target_indices]
    distances = np.empty(len(vertices), dtype=np.float64)
    for start in range(0, len(vertices), chunk_size):
        points = vertices[start : start + chunk_size]
        diff = points[:, None, :] - targets[None, :, :]
        distances[start : start + len(points)] = np.sqrt(np.min(np.einsum("ijk,ijk->ij", diff, diff), axis=1))
    return distances


def _python_mesh_stats(mesh: MeshDocument) -> MeshStats:
    def edge_face_map() -> dict[tuple[int, int], list[int]]:
        edges: dict[tuple[int, int], list[int]] = defaultdict(list)
        for face_index, (a, b, c) in enumerate(mesh.faces):
            for u, v in ((int(a), int(b)), (int(b), int(c)), (int(c), int(a))):
                edges[(min(u, v), max(u, v))].append(face_index)
        return dict(edges)

    def boundary_edges() -> list[tuple[int, int]]:
        return [edge for edge, face_ids in edge_face_map().items() if len(face_ids) == 1]

    def face_adjacency() -> list[list[int]]:
        adjacency = [[] for _ in range(mesh.face_count)]
        for face_ids in edge_face_map().values():
            if len(face_ids) < 2:
                continue
            for i, face_a in enumerate(face_ids):
                for face_b in face_ids[i + 1 :]:
                    adjacency[face_a].append(face_b)
                    adjacency[face_b].append(face_a)
        return adjacency

    def connected_face_components() -> list[list[int]]:
        adjacency = face_adjacency()
        seen = np.zeros(mesh.face_count, dtype=bool)
        components: list[list[int]] = []
        for start in range(mesh.face_count):
            if seen[start]:
                continue
            queue: deque[int] = deque([start])
            seen[start] = True
            component: list[int] = []
            while queue:
                face_id = queue.popleft()
                component.append(face_id)
                for neighbor in adjacency[face_id]:
                    if not seen[neighbor]:
                        seen[neighbor] = True
                        queue.append(neighbor)
            components.append(component)
        return components

    def surface_area() -> float:
        if mesh.face_count == 0:
            return 0.0
        triangles = mesh.vertices[mesh.faces]
        cross = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
        return float(np.sum(np.linalg.norm(cross, axis=1) * 0.5, dtype=np.float64))

    def signed_volume() -> float:
        if mesh.face_count == 0:
            return 0.0
        triangles = mesh.vertices[mesh.faces]
        volumes = np.einsum("ij,ij->i", triangles[:, 0], np.cross(triangles[:, 1], triangles[:, 2])) / 6.0
        return float(np.sum(volumes, dtype=np.float64))

    bbox_min = np.zeros(3, dtype=np.float64) if mesh.vertex_count == 0 else mesh.vertices.min(axis=0)
    bbox_max = np.zeros(3, dtype=np.float64) if mesh.vertex_count == 0 else mesh.vertices.max(axis=0)
    bbox_size = bbox_max - bbox_min
    return MeshStats(
        bbox_min=tuple(float(x) for x in bbox_min),
        bbox_max=tuple(float(x) for x in bbox_max),
        bbox_size=tuple(float(x) for x in bbox_size),
        surface_area_mm2=surface_area(),
        volume_mm3=abs(signed_volume()),
        vertex_count=mesh.vertex_count,
        face_count=mesh.face_count,
        connected_components=len(connected_face_components()) if mesh.face_count else 0,
        boundary_edge_count=len(boundary_edges()),
    )

def _bbox_query_points(mesh: MeshDocument, *, x_count: int, y_count: int, z_count: int) -> np.ndarray:
    minimum = mesh.vertices.min(axis=0)
    maximum = mesh.vertices.max(axis=0)
    return np.array(
        [
            [x, y, z]
            for x in np.linspace(minimum[0] - 0.5, maximum[0] + 0.5, x_count)
            for y in np.linspace(minimum[1] - 0.3, maximum[1] + 0.3, y_count)
            for z in np.linspace(minimum[2] - 0.5, maximum[2] + 0.5, z_count)
        ],
        dtype=np.float64,
    )


def _bbox_ray_grid(mesh: MeshDocument, *, samples_per_axis: int) -> tuple[np.ndarray, np.ndarray]:
    minimum = mesh.vertices.min(axis=0)
    maximum = mesh.vertices.max(axis=0)
    span = maximum - minimum
    origins: list[list[float]] = []
    directions: list[list[float]] = []
    for y in np.linspace(minimum[1] - 0.2, maximum[1] + 0.2, samples_per_axis):
        for z in np.linspace(minimum[2] - 0.5, maximum[2] + 0.5, samples_per_axis):
            origins.append([maximum[0] + span[0] * 0.25 + 1.0, y, z])
            directions.append([-1.0, 0.0, 0.0])
    for x in np.linspace(minimum[0] - 0.5, maximum[0] + 0.5, samples_per_axis):
        for y in np.linspace(minimum[1] - 0.2, maximum[1] + 0.2, samples_per_axis):
            origins.append([x, y, maximum[2] + span[2] * 0.25 + 1.0])
            directions.append([0.0, 0.0, -1.0])
    return np.asarray(origins, dtype=np.float64), np.asarray(directions, dtype=np.float64)


def _ray_hit_signature(hits: list[RayHit | None]) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    mask = np.asarray([hit is not None for hit in hits], dtype=bool)
    distances = np.asarray([hit.distance if hit is not None else np.inf for hit in hits], dtype=np.float64)
    points = np.asarray(
        [hit.point if hit is not None else (np.nan, np.nan, np.nan) for hit in hits],
        dtype=np.float64,
    )
    return mask, distances, points


def _assert_result_close(left, right) -> None:
    if isinstance(left, SDFGrid):
        assert isinstance(right, SDFGrid)
        assert right.origin == left.origin
        assert right.shape == left.shape
        assert np.allclose(right.values, left.values, atol=1e-6)
        return
    if is_dataclass(left):
        assert is_dataclass(right)
        _assert_result_close(asdict(left), asdict(right))
        return
    if isinstance(left, dict):
        assert isinstance(right, dict)
        assert set(right) >= set(left)
        for key, value in left.items():
            _assert_result_close(value, right[key])
        return
    if isinstance(left, tuple):
        assert isinstance(right, tuple)
        assert len(right) == len(left)
        for left_value, right_value in zip(left, right):
            _assert_result_close(left_value, right_value)
        return
    if isinstance(left, list):
        assert right == left
        return
    if isinstance(left, set):
        assert right == left
        return
    if left is None:
        assert right is None
        return
    if isinstance(left, str):
        assert right == left
        return
    assert np.allclose(right, left, equal_nan=True, atol=1e-6)


def _assert_rust_performance_budget(
    name: str,
    callback,
    *,
    max_ratio: float = RUST_SPEED_RATIO_BUDGET,
    repeats: int = 3,
) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    sample, python_value, rust_value = compare_accelerator_modes(callback, repeats=repeats)

    _assert_result_close(python_value, rust_value)
    assert sample.ratio <= max_ratio, (
        f"{name} Rust path should be faster than Python fallback: "
        f"python={sample.python_seconds:.6f}s rust={sample.rust_seconds:.6f}s ratio={sample.ratio:.3f}"
    )


def _assert_rust_owned_latency_budget(
    name: str,
    callback,
    *,
    max_seconds: float,
    repeats: int = 3,
) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, result = best_of(repeats, callback)
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    if isinstance(result, MeshDocument):
        assert result.vertex_count > 0
        assert result.face_count > 0
    assert rust_seconds <= max_seconds, (
        f"{name} Rust-owned path exceeded latency budget: "
        f"rust={rust_seconds:.6f}s budget={max_seconds:.6f}s"
    )


def _assert_rust_owned_stats_budget(name: str, mesh: MeshDocument, *, max_ratio: float = 0.95, repeats: int = 3) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(repeats, lambda: _python_mesh_stats(mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(repeats, lambda: compute_mesh_stats(mesh))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= max_ratio, (
        f"{name} Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def _assert_rust_owned_callback_budget(
    name: str,
    python_call,
    rust_call,
    *,
    max_ratio: float = RUST_SPEED_RATIO_BUDGET,
    repeats: int = 3,
) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(repeats, python_call)
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(repeats, rust_call)
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= max_ratio, (
        f"{name} Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def _python_outward_directions(mesh: MeshDocument) -> np.ndarray:
    vertices = mesh.vertices
    normals = safe_normalize(vertex_normals(mesh))
    center = vertices.mean(axis=0)
    toward_center = safe_normalize(center - vertices)
    return np.where((np.einsum("ij,ij->i", normals, toward_center) >= 0.0)[:, None], -normals, normals)


def _python_falloff_weights(mesh: MeshDocument, seed_indices: np.ndarray, falloff_mm: float) -> np.ndarray:
    seeds = np.unique(np.asarray(seed_indices, dtype=np.int32))
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    distances = _python_nearest_distances(mesh.vertices, seeds)
    weights = np.exp(-0.5 * np.square(distances / max(falloff_mm, 1e-3)))
    weights[distances > falloff_mm * 3.0] = 0.0
    return weights.astype(np.float32)


def _python_local_offset(mesh: MeshDocument, seed_indices: np.ndarray, amount_mm: float, falloff_mm: float) -> MeshDocument:
    weights = _python_falloff_weights(mesh, seed_indices, falloff_mm)
    displaced = mesh.vertices + _python_outward_directions(mesh) * (amount_mm * weights[:, None])
    return mesh.copy(vertices=displaced)


def _python_stroke_weights(mesh: MeshDocument, stroke: BrushStroke) -> np.ndarray:
    weights = _python_falloff_weights(mesh, stroke.seed_indices, stroke.falloff_mm)
    if stroke.mask_indices is not None:
        mask = np.zeros(mesh.vertex_count, dtype=bool)
        mask[np.asarray(stroke.mask_indices, dtype=np.int64)] = True
        weights = np.where(mask, weights, 0.0).astype(np.float32)
    if stroke.protected_indices is not None:
        weights = weights.copy()
        weights[np.asarray(stroke.protected_indices, dtype=np.int64)] = 0.0
    return weights


def _python_offset_with_weights(mesh: MeshDocument, weights: np.ndarray, amount_mm: float) -> MeshDocument:
    displaced = mesh.vertices + _python_outward_directions(mesh) * (float(amount_mm) * weights[:, None])
    return mesh.copy(vertices=displaced)


def _python_smooth_with_weights(mesh: MeshDocument, weights: np.ndarray, *, iterations: int, strength: float) -> MeshDocument:
    vertices = mesh.vertices.copy()
    neighbors = vertex_neighbors(mesh)
    active = np.flatnonzero(weights > 0.02)
    for _ in range(max(1, int(iterations))):
        updated = vertices.copy()
        for index in active:
            neighbor_ids = neighbors[int(index)]
            if not neighbor_ids:
                continue
            neighbor_mean = vertices[np.asarray(neighbor_ids, dtype=np.int32)].mean(axis=0)
            updated[index] = vertices[index] + (neighbor_mean - vertices[index]) * float(strength) * float(weights[index])
        vertices = updated
    return mesh.copy(vertices=vertices)


def _python_apply_brush_strokes(mesh: MeshDocument, strokes: list[BrushStroke]) -> MeshDocument:
    output = mesh
    for stroke in strokes:
        weights = _python_stroke_weights(output, stroke)
        if stroke.operation == "thicken":
            output = _python_offset_with_weights(output, weights, stroke.amount_mm)
        elif stroke.operation == "scoop":
            output = _python_offset_with_weights(output, weights, -stroke.amount_mm)
        else:
            output = _python_smooth_with_weights(
                output,
                weights,
                iterations=stroke.iterations,
                strength=stroke.strength,
            )
    return output


def _python_smooth(
    mesh: MeshDocument,
    *,
    iterations: int = 5,
    strength: float = 0.5,
    seed_indices: np.ndarray | None = None,
    falloff_mm: float = 1.8,
) -> MeshDocument:
    iterations = max(1, int(iterations))
    strength = float(np.clip(strength, 0.0, 1.0))
    if seed_indices is None:
        return _python_taubin_smooth(mesh, iterations=iterations, lamb=strength, nu=-0.53)
    weights = _python_falloff_weights(mesh, seed_indices, falloff_mm)
    vertices = mesh.vertices.copy()
    neighbors = vertex_neighbors(mesh)
    active = np.flatnonzero(weights > 0.02)
    for _ in range(iterations):
        updated = vertices.copy()
        for index in active:
            neighbor_ids = neighbors[int(index)]
            if not neighbor_ids:
                continue
            neighbor_mean = vertices[np.asarray(neighbor_ids, dtype=np.int32)].mean(axis=0)
            updated[index] = vertices[index] + (neighbor_mean - vertices[index]) * strength * float(weights[index])
        vertices = updated
    return mesh.copy(vertices=vertices)


def _python_taubin_smooth(mesh: MeshDocument, *, iterations: int, lamb: float, nu: float) -> MeshDocument:
    vertices = mesh.vertices.copy()
    neighbors = vertex_neighbors(mesh)
    for pass_index in range(max(1, int(iterations))):
        updated = vertices.copy()
        factor = float(np.clip(lamb, 0.0, 1.0)) if pass_index % 2 == 0 else -float(nu)
        for index, neighbor_ids in enumerate(neighbors):
            if not neighbor_ids:
                continue
            neighbor_mean = vertices[np.asarray(neighbor_ids, dtype=np.int32)].mean(axis=0)
            updated[index] = vertices[index] + (neighbor_mean - vertices[index]) * factor
        vertices = updated
    return mesh.copy(vertices=vertices)


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


def _python_signed_point_mesh_distances(points: np.ndarray, mesh: MeshDocument) -> np.ndarray:
    distances = _python_point_mesh_distances(points, mesh)
    signs = np.where(np.abs(_python_winding_numbers(points, mesh)) >= 0.5, -1.0, 1.0).astype(np.float32)
    return distances * signs


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


def _python_first_ray_hit(
    mesh: MeshDocument,
    origin: np.ndarray,
    direction: np.ndarray,
    *,
    epsilon: float,
    ignore_faces: np.ndarray,
    tree,
) -> RayHit | None:
    ray_origin = np.asarray(origin, dtype=np.float64)
    ray_direction = safe_normalize(np.asarray(direction, dtype=np.float64))
    ignored = set(int(face_id) for face_id in ignore_faces)
    best: RayHit | None = None
    for face_index in ray_candidate_faces(tree, ray_origin, ray_direction):
        face_index = int(face_index)
        if face_index in ignored:
            continue
        distance = _python_ray_triangle_distance(ray_origin, ray_direction, tree.triangles[face_index], epsilon)
        if distance is None:
            continue
        if best is not None and distance >= best.distance:
            continue
        point = ray_origin + ray_direction * distance
        best = RayHit(
            face_index=face_index,
            distance=float(distance),
            point=tuple(float(value) for value in point),
        )
    return best


def _python_first_ray_hit_with_tree(
    mesh: MeshDocument,
    origin: np.ndarray | tuple[float, float, float],
    direction: np.ndarray | tuple[float, float, float],
    *,
    epsilon: float = 1e-8,
    ignore_faces: np.ndarray | None = None,
) -> RayHit | None:
    ignored = np.zeros(0, dtype=np.int64) if ignore_faces is None else ignore_faces
    return _python_first_ray_hit(
        mesh,
        np.asarray(origin, dtype=np.float64),
        np.asarray(direction, dtype=np.float64),
        epsilon=epsilon,
        ignore_faces=ignored,
        tree=build_aabb_tree(mesh),
    )


def _python_first_ray_hits(mesh: MeshDocument, origins: np.ndarray, directions: np.ndarray, *, epsilon: float = 1e-8) -> list[RayHit | None]:
    tree = build_aabb_tree(mesh)
    ignored = np.zeros(0, dtype=np.int64)
    return [
        _python_first_ray_hit(mesh, origin, direction, epsilon=epsilon, ignore_faces=ignored, tree=tree)
        for origin, direction in zip(origins, directions)
    ]


def _python_ray_thickness_at_vertices(mesh: MeshDocument, *, epsilon: float = 1e-5) -> np.ndarray:
    if mesh.vertex_count == 0 or mesh.face_count == 0:
        return np.zeros(mesh.vertex_count, dtype=np.float32)
    normals = safe_normalize(vertex_normals(mesh))
    face_ids_by_vertex: list[list[int]] = [[] for _ in range(mesh.vertex_count)]
    for face_index, face in enumerate(mesh.faces):
        for vertex_id in face:
            face_ids_by_vertex[int(vertex_id)].append(face_index)

    thickness = np.full(mesh.vertex_count, np.nan, dtype=np.float32)
    tree = build_aabb_tree(mesh)
    for vertex_id, vertex in enumerate(mesh.vertices):
        normal = normals[vertex_id]
        if np.linalg.norm(normal) < 1e-8:
            continue
        candidates: list[float] = []
        ignored = np.asarray(face_ids_by_vertex[vertex_id], dtype=np.int64)
        for direction in (normal, -normal):
            origin = vertex + direction * epsilon
            hit = _python_first_ray_hit(mesh, origin, direction, epsilon=epsilon * 0.1, ignore_faces=ignored, tree=tree)
            if hit is not None and np.isfinite(hit.distance):
                candidates.append(float(hit.distance + epsilon))
        if candidates:
            thickness[vertex_id] = min(candidates)
    return thickness


def _assert_rust_owned_ray_thickness_budget(
    name: str,
    mesh: MeshDocument,
    *,
    max_ratio: float = RUST_SPEED_RATIO_BUDGET,
    repeats: int = 3,
) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(repeats, lambda: _python_ray_thickness_at_vertices(mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(repeats, lambda: ray_thickness_at_vertices(mesh))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= max_ratio, (
        f"{name} Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def _python_triangles_intersect(triangle_a: np.ndarray, triangle_b: np.ndarray, *, epsilon: float = 1e-8) -> bool:
    a = np.asarray(triangle_a, dtype=np.float64)
    b = np.asarray(triangle_b, dtype=np.float64)
    if not bool(np.all(a.min(axis=0) <= b.max(axis=0) + epsilon) and np.all(b.min(axis=0) <= a.max(axis=0) + epsilon)):
        return False

    def segment_intersects_triangle(p0: np.ndarray, p1: np.ndarray, tri: np.ndarray) -> bool:
        direction = p1 - p0
        x, y, z = tri
        edge1 = y - x
        edge2 = z - x
        h = np.cross(direction, edge2)
        det = float(np.dot(edge1, h))
        if abs(det) < epsilon:
            return False
        inv_det = 1.0 / det
        s = p0 - x
        u = inv_det * float(np.dot(s, h))
        if u < -epsilon or u > 1.0 + epsilon:
            return False
        q = np.cross(s, edge1)
        v = inv_det * float(np.dot(direction, q))
        if v < -epsilon or u + v > 1.0 + epsilon:
            return False
        t = inv_det * float(np.dot(edge2, q))
        return bool(-epsilon <= t <= 1.0 + epsilon)

    def point_in_triangle(point: np.ndarray, tri: np.ndarray) -> bool:
        x, y, z = tri
        normal = np.cross(y - x, z - x)
        if np.linalg.norm(normal) < epsilon:
            return False
        if abs(float(np.dot(point - x, normal))) > epsilon * max(np.linalg.norm(normal), 1.0):
            return False
        v0 = z - x
        v1 = y - x
        v2 = point - x
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

    for index in range(3):
        if segment_intersects_triangle(a[index], a[(index + 1) % 3], b):
            return True
        if segment_intersects_triangle(b[index], b[(index + 1) % 3], a):
            return True
    return any(point_in_triangle(point, b) for point in a) or any(point_in_triangle(point, a) for point in b)


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


def _assert_rust_owned_self_intersection_budget(name: str, mesh: MeshDocument, *, max_ratio: float = RUST_SPEED_RATIO_BUDGET) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(3, lambda: _python_self_intersecting_faces(mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(3, lambda: self_intersecting_faces(mesh))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= max_ratio, (
        f"{name} Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def test_rust_mesh_stats_performance_budget() -> None:
    mesh = ring(radial_segments=128, tube_segments=32)

    _assert_rust_owned_stats_budget("mesh_stats", mesh, max_ratio=0.95)


@pytest.mark.parametrize("fragment_name", [UPLOADED_RING_FRAGMENT, UPLOADED_PENDANT_FRAGMENT])
def test_rust_uploaded_fragment_mesh_stats_performance_budget(fragment_name: str) -> None:
    mesh = _uploaded_fragment(fragment_name)

    _assert_rust_owned_stats_budget(
        f"uploaded fragment mesh_stats {fragment_name}",
        mesh,
        max_ratio=0.95,
        repeats=2,
    )


def test_rust_self_intersection_performance_budget() -> None:
    mesh = _crossing_triangle_pairs(128)

    _assert_rust_owned_self_intersection_budget("self_intersecting_faces", mesh)


def test_rust_point_distance_performance_budget() -> None:
    mesh = ring(radial_segments=32, tube_segments=8)
    points = _query_points()

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(3, lambda: _python_point_mesh_distances(points, mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(3, lambda: point_mesh_distances(points, mesh))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= RUST_SPEED_RATIO_BUDGET, (
        "point_mesh_distances Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def test_rust_closest_points_performance_budget() -> None:
    mesh = ring(radial_segments=32, tube_segments=8)
    points = _query_points()

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(3, lambda: _python_closest_points_on_mesh(points, mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(3, lambda: closest_points_on_mesh(points, mesh))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value[1], rust_value[1])
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= RUST_SPEED_RATIO_BUDGET, (
        "closest_points_on_mesh Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def test_rust_uploaded_fragment_closest_points_performance_budget() -> None:
    mesh = _uploaded_fragment(UPLOADED_PENDANT_FRAGMENT)
    points = _bbox_query_points(mesh, x_count=7, y_count=4, z_count=7)

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(2, lambda: _python_closest_points_on_mesh(points, mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(2, lambda: closest_points_on_mesh(points, mesh))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value[1], rust_value[1])
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= 0.9, (
        "uploaded fragment closest_points_on_mesh Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def test_rust_winding_number_performance_budget() -> None:
    mesh = ring(radial_segments=32, tube_segments=8)
    points = _query_points()[:200]

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(3, lambda: _python_winding_numbers(points, mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(3, lambda: winding_numbers(points, mesh))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= RUST_SPEED_RATIO_BUDGET, (
        "winding_numbers Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def test_rust_signed_distance_performance_budget() -> None:
    mesh = ring(radial_segments=32, tube_segments=8)
    points = _query_points()[:200]

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(3, lambda: _python_signed_point_mesh_distances(points, mesh))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(3, lambda: signed_point_mesh_distances(points, mesh, sign_method="winding"))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= RUST_SPEED_RATIO_BUDGET, (
        "signed_point_mesh_distances Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )


def test_rust_ray_thickness_performance_budget() -> None:
    mesh = ring(radial_segments=16, tube_segments=8)

    _assert_rust_owned_ray_thickness_budget("ray_thickness_at_vertices", mesh)


def test_rust_uploaded_fragment_ray_thickness_performance_budget() -> None:
    mesh = _uploaded_fragment(UPLOADED_RING_FRAGMENT)

    _assert_rust_owned_ray_thickness_budget(
        "uploaded fragment ray_thickness_at_vertices",
        mesh,
        max_ratio=0.2,
        repeats=1,
    )


def test_rust_sdf_grid_performance_budget() -> None:
    mesh = cube(size=2.0)

    _assert_rust_owned_latency_budget(
        "sample_sdf_grid",
        lambda: sample_sdf_grid(mesh, voxel_size_mm=0.25, padding_mm=0.5),
        max_seconds=0.12,
    )


def test_rust_marching_tetrahedra_performance_budget() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.2, padding_mm=0.4)

    _assert_rust_owned_latency_budget(
        "extract_marching_tetrahedra",
        lambda: extract_marching_tetrahedra(grid),
        max_seconds=0.08,
    )


def test_rust_voxel_boolean_mesh_performance_budget() -> None:
    a = cube(size=2.0)
    b = cube(size=2.0).copy(vertices=cube(size=2.0).vertices + np.array([1.0, 0.0, 0.0]))

    _assert_rust_owned_latency_budget(
        "voxel_boolean_mesh",
        lambda: voxel_boolean_mesh(a, b, operation="difference", voxel_size_mm=0.25, refine=False),
        max_seconds=0.12,
    )


def test_rust_voxel_offset_mesh_performance_budget() -> None:
    _assert_rust_owned_latency_budget(
        "voxel_offset_mesh",
        lambda: voxel_offset_mesh(cube(size=2.0), offset_mm=0.5, voxel_size_mm=0.35, refine=False),
        max_seconds=0.12,
    )


def test_rust_voxel_shell_mesh_performance_budget() -> None:
    _assert_rust_owned_latency_budget(
        "voxel_shell_mesh",
        lambda: voxel_shell_mesh(cube(size=4.0), wall_thickness_mm=1.0, voxel_size_mm=0.35, refine=False),
        max_seconds=0.25,
    )


def test_rust_sdf_projection_performance_budget() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.14, padding_mm=0.4)
    mesh = extract_marching_tetrahedra(grid)
    moved = mesh.copy(vertices=mesh.vertices * 0.93)

    _assert_rust_owned_latency_budget(
        "project_vertices_to_sdf",
        lambda: project_vertices_to_sdf(moved, grid, iterations=3),
        max_seconds=0.08,
    )


def test_rust_laplacian_smoothing_performance_budget() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.2, padding_mm=0.4)
    mesh = extract_marching_tetrahedra(grid)

    _assert_rust_owned_latency_budget(
        "laplacian_smooth_vertices",
        lambda: laplacian_smooth_vertices(mesh, iterations=2, strength=0.25),
        max_seconds=0.08,
    )


def test_rust_refine_sdf_mesh_performance_budget() -> None:
    grid = sample_sdf_grid(cube(size=2.0), voxel_size_mm=0.16, padding_mm=0.4)
    mesh = extract_marching_tetrahedra(grid)
    moved = mesh.copy(vertices=mesh.vertices * 0.93)

    _assert_rust_owned_latency_budget(
        "refine_sdf_mesh",
        lambda: refine_sdf_mesh(moved, grid, smooth_iterations=2, smooth_strength=0.25, projection_iterations=3),
        max_seconds=0.12,
    )


def test_rust_global_smooth_performance_budget() -> None:
    mesh = ring(radial_segments=64, tube_segments=16)

    _assert_rust_owned_callback_budget(
        "smooth",
        lambda: _python_smooth(mesh, iterations=2, strength=0.25),
        lambda: smooth(mesh, iterations=2, strength=0.25),
    )


def test_rust_seeded_smooth_performance_budget() -> None:
    mesh = ring(radial_segments=128, tube_segments=24)
    seed = np.arange(0, 64, dtype=np.int32)

    _assert_rust_owned_callback_budget(
        "seeded smooth",
        lambda: _python_smooth(mesh, iterations=2, strength=0.35, seed_indices=seed, falloff_mm=2.0),
        lambda: smooth(mesh, iterations=2, strength=0.35, seed_indices=seed, falloff_mm=2.0),
        max_ratio=0.9,
    )


def test_rust_local_thicken_performance_budget() -> None:
    mesh = ring(radial_segments=768, tube_segments=48)
    seed = np.arange(0, 192, dtype=np.int32)

    _assert_rust_owned_callback_budget(
        "local_thicken",
        lambda: _python_local_offset(mesh, seed, amount_mm=0.18, falloff_mm=2.0),
        lambda: local_thicken(mesh, seed, amount_mm=0.18, falloff_mm=2.0),
        max_ratio=0.97,
    )


def test_rust_local_scoop_performance_budget() -> None:
    mesh = ring(radial_segments=768, tube_segments=48)
    seed = np.arange(0, 192, dtype=np.int32)

    _assert_rust_owned_callback_budget(
        "local_scoop",
        lambda: _python_local_offset(mesh, seed, amount_mm=-0.18, falloff_mm=2.0),
        lambda: local_scoop(mesh, seed, depth_mm=0.18, falloff_mm=2.0),
        max_ratio=0.97,
    )


def test_rust_brush_composition_performance_budget() -> None:
    mesh = ring(radial_segments=256, tube_segments=32)
    seed = np.arange(0, 128, dtype=np.int32)
    strokes = [
        BrushStroke("thicken", seed, amount_mm=0.18, falloff_mm=2.0),
        BrushStroke("scoop", seed + 32, amount_mm=0.07, falloff_mm=1.5),
        BrushStroke("smooth", seed, falloff_mm=2.0, iterations=2, strength=0.25),
    ]

    _assert_rust_owned_callback_budget(
        "apply_brush_strokes",
        lambda: _python_apply_brush_strokes(mesh, strokes),
        lambda: apply_brush_strokes(mesh, strokes),
        max_ratio=0.75,
    )


def test_rust_masked_brush_composition_performance_budget() -> None:
    mesh = ring(radial_segments=256, tube_segments=32)
    seed = np.arange(0, 128, dtype=np.int32)
    mask = np.arange(0, 512, dtype=np.int32)
    protected = np.arange(96, 192, dtype=np.int32)
    strokes = [
        BrushStroke(
            "thicken",
            seed,
            amount_mm=0.18,
            falloff_mm=2.0,
            mask_indices=mask,
            protected_indices=protected,
        ),
        BrushStroke(
            "smooth",
            seed,
            falloff_mm=2.0,
            iterations=2,
            strength=0.25,
            mask_indices=mask,
            protected_indices=protected,
        ),
    ]

    _assert_rust_owned_callback_budget(
        "masked apply_brush_strokes",
        lambda: _python_apply_brush_strokes(mesh, strokes),
        lambda: apply_brush_strokes(mesh, strokes),
        max_ratio=0.85,
    )


def test_rust_falloff_weights_performance_budget() -> None:
    mesh = ring(radial_segments=256, tube_segments=32)
    seed = np.arange(0, 128, dtype=np.int32)

    _assert_rust_owned_callback_budget(
        "falloff_weights",
        lambda: _python_falloff_weights(mesh, seed, 2.0),
        lambda: falloff_weights(mesh, seed, 2.0),
        max_ratio=0.75,
    )


def test_rust_nearest_distances_performance_budget() -> None:
    mesh = ring(radial_segments=512, tube_segments=32)
    targets = np.arange(0, 256, dtype=np.int64)

    _assert_rust_owned_callback_budget(
        "nearest_distances",
        lambda: _python_nearest_distances(mesh.vertices, targets),
        lambda: nearest_distances(mesh.vertices, targets),
        max_ratio=0.8,
    )


def test_rust_first_ray_hit_matches_python_reference() -> None:
    mesh = cube(size=2.0)

    python_hit = _python_first_ray_hit_with_tree(mesh, (0.0, 0.0, 3.0), (0.0, 0.0, -1.0))
    rust_hit = first_ray_hit(mesh, (0.0, 0.0, 3.0), (0.0, 0.0, -1.0))

    assert python_hit is not None
    assert rust_hit is not None
    assert np.isclose(rust_hit.distance, python_hit.distance)
    assert np.allclose(rust_hit.point, python_hit.point)


def test_rust_uploaded_fragment_first_ray_hits_performance_budget() -> None:
    mesh = _uploaded_fragment(UPLOADED_RING_FRAGMENT)
    origins, directions = _bbox_ray_grid(mesh, samples_per_axis=16)

    previous = os.environ.get("GEOMETRY_SDK_ACCELERATOR")
    try:
        python_seconds, python_value = best_of(2, lambda: _ray_hit_signature(_python_first_ray_hits(mesh, origins, directions)))
        os.environ["GEOMETRY_SDK_ACCELERATOR"] = "rust"
        rust_seconds, rust_value = best_of(2, lambda: _ray_hit_signature(first_ray_hits(mesh, origins, directions)))
    finally:
        if previous is None:
            os.environ.pop("GEOMETRY_SDK_ACCELERATOR", None)
        else:
            os.environ["GEOMETRY_SDK_ACCELERATOR"] = previous

    _assert_result_close(python_value, rust_value)
    ratio = rust_seconds / max(python_seconds, 1e-12)
    assert ratio <= 0.75, (
        "uploaded fragment first_ray_hits Rust-owned path should be faster than Python reference: "
        f"python={python_seconds:.6f}s rust={rust_seconds:.6f}s ratio={ratio:.3f}"
    )
