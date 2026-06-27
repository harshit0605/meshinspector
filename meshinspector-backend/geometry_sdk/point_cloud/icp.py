"""Point-cloud ICP compatibility wrappers for Rust-owned registration kernels."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal, TypeVar

import numpy as np

from geometry_sdk.accelerators import rust
from geometry_sdk.types import MeshDocument

T = TypeVar("T")


@dataclass(slots=True)
class PointCloudDocument:
    points: np.ndarray
    unit: Literal["mm"] = "mm"
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        points = np.asarray(self.points, dtype=np.float64)
        if points.ndim != 2 or points.shape[1] != 3:
            raise ValueError("points must have shape (n, 3)")
        if not np.all(np.isfinite(points)):
            raise ValueError("points must be finite")
        self.points = np.ascontiguousarray(points)

    @property
    def point_count(self) -> int:
        return int(self.points.shape[0])


@dataclass(slots=True)
class ICPRegistrationResult:
    rotation: np.ndarray
    translation: np.ndarray
    transform: np.ndarray
    iterations: int
    mean_square_distance: float
    active_pair_count: int
    method: Literal["point_to_point", "point_to_plane"]
    mode: Literal["rigid", "translation"]

    def apply(self, cloud: PointCloudDocument) -> PointCloudDocument:
        points = np.asarray(cloud.points, dtype=np.float64) @ self.rotation.T + self.translation
        return PointCloudDocument(points, unit=cloud.unit, metadata=dict(cloud.metadata))


@dataclass(slots=True)
class PointCloudProjectionResult:
    points: np.ndarray
    squared_distances: np.ndarray
    vertex_indices: np.ndarray

    @property
    def distances(self) -> np.ndarray:
        return np.sqrt(self.squared_distances)


@dataclass(slots=True)
class PointCloudMeshProjectionResult:
    points: np.ndarray
    squared_distances: np.ndarray
    face_indices: np.ndarray
    vertex_indices: np.ndarray
    normals: np.ndarray
    boundary_flags: np.ndarray

    @property
    def distances(self) -> np.ndarray:
        return np.sqrt(self.squared_distances)


@dataclass(slots=True)
class PointCloudClosestPair:
    vertex_indices: np.ndarray
    squared_distance: float

    @property
    def distance(self) -> float:
        return float(np.sqrt(self.squared_distance))


@dataclass(slots=True)
class PointCloudLocalFan:
    neighbors: np.ndarray
    boundary_neighbor: int
    actual_radius: float
    removed_count: int


@dataclass(slots=True)
class PointCloudLocalFanTriangles:
    triangles: np.ndarray
    boundary_neighbor: int
    actual_radius: float
    removed_count: int


@dataclass(slots=True)
class PointCloudLocalTriangulationRepetitions:
    repetition_counts: np.ndarray
    repeated_3: np.ndarray
    repeated_2: np.ndarray


def _result_from_payload(
    payload: dict[str, Any],
    method: Literal["point_to_point", "point_to_plane"],
    mode: Literal["rigid", "translation"],
) -> ICPRegistrationResult:
    return ICPRegistrationResult(
        rotation=np.asarray(payload["rotation"], dtype=np.float64).reshape(3, 3),
        translation=np.asarray(payload["translation"], dtype=np.float64).reshape(3),
        transform=np.asarray(payload["transform"], dtype=np.float64).reshape(4, 4),
        iterations=int(payload["iterations"]),
        mean_square_distance=float(payload["mean_square_distance"]),
        active_pair_count=int(payload["active_pair_count"]),
        method=method,
        mode=mode,
    )


def _require_rust(value: T | None, operation: str) -> T:
    if value is None:
        raise RuntimeError(f"Rust {operation} kernel is unavailable")
    return value


def point_cloud_from_ply(source: bytes | bytearray) -> PointCloudDocument:
    payload = _require_rust(rust.point_cloud_from_ply(bytes(source)), "point_cloud_from_ply")
    metadata: dict[str, Any] = {
        "source": "rust_point_cloud_from_ply",
        "meshlib_reference": "MR::PointsLoad",
        "meshlib_source": "MeshLib/source/MRMesh/MRPointsLoad.*",
    }
    normals = np.asarray(payload["normals"], dtype=np.float64).reshape((-1, 3))
    colors = np.asarray(payload["colors"], dtype=np.uint8).reshape((-1, 3))
    if normals.size:
        metadata["normals"] = normals.tolist()
    if colors.size:
        metadata["point_colors"] = colors.tolist()
    return PointCloudDocument(
        np.asarray(payload["points"], dtype=np.float64).reshape((-1, 3)),
        metadata=metadata,
    )


def load_point_cloud_ply(path: str | Path) -> PointCloudDocument:
    source_path = Path(path)
    cloud = point_cloud_from_ply(source_path.read_bytes())
    cloud.metadata["source_path"] = str(source_path)
    return cloud


def point_cloud_to_ply(cloud: PointCloudDocument) -> bytes:
    payload = rust.point_cloud_to_ply(
        cloud.points,
        normals=_point_cloud_metadata_array(cloud, "normals", dtype=np.float64),
        colors=_point_cloud_color_array(cloud),
    )
    return _require_rust(payload, "point_cloud_to_ply")


def save_point_cloud_ply(cloud: PointCloudDocument, path: str | Path) -> Path:
    output_path = Path(path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(point_cloud_to_ply(cloud))
    return output_path


def _point_cloud_metadata_array(cloud: PointCloudDocument, key: str, *, dtype) -> np.ndarray | None:
    value = cloud.metadata.get(key)
    if value is None:
        return None
    array = np.asarray(value, dtype=dtype)
    if array.shape != (cloud.point_count, 3):
        return None
    return np.ascontiguousarray(array)


def _point_cloud_color_array(cloud: PointCloudDocument) -> np.ndarray | None:
    for key in ("point_colors", "colors", "vertex_colors", "verts_color_map"):
        colors = _point_cloud_metadata_array(cloud, key, dtype=np.uint8)
        if colors is not None:
            return colors
    return None


def _points_from_query(points: PointCloudDocument | np.ndarray) -> np.ndarray:
    if isinstance(points, PointCloudDocument):
        return points.points
    query = np.asarray(points, dtype=np.float64)
    if query.ndim != 2 or query.shape[1] != 3:
        raise ValueError("query points must have shape (n, 3)")
    if not np.all(np.isfinite(query)):
        raise ValueError("query points must be finite")
    return np.ascontiguousarray(query)


def _normals_for_cloud(cloud: PointCloudDocument, normals: np.ndarray | None) -> np.ndarray | None:
    if normals is None:
        return None
    sample_normals = np.asarray(normals, dtype=np.float64)
    if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
        raise ValueError("normals must have shape matching cloud.points")
    if not np.all(np.isfinite(sample_normals)):
        raise ValueError("normals must be finite")
    return np.ascontiguousarray(sample_normals)


def _clone_region_metadata(
    cloud: PointCloudDocument,
    source_point_indices: np.ndarray,
) -> dict[str, Any]:
    metadata = dict(cloud.metadata)
    point_attribute_keys = {
        "colors",
        "normals",
        "point_colors",
        "point_normals",
        "vertex_colors",
        "verts_color_map",
    }
    for key in point_attribute_keys:
        if key not in metadata:
            continue
        values = np.asarray(metadata[key])
        if values.ndim >= 1 and values.shape[0] == cloud.point_count:
            metadata[key] = np.ascontiguousarray(values[source_point_indices])
    metadata["meshlib_operation"] = "ObjectPoints::cloneRegion"
    metadata["source_point_indices"] = [int(index) for index in source_point_indices]
    return metadata


def _projection_result_from_payload(payload: dict[str, Any]) -> PointCloudProjectionResult:
    vertex_indices = np.asarray(payload["vertex_indices"], dtype=np.int64).reshape(-1)
    return PointCloudProjectionResult(
        points=np.asarray(payload["points"], dtype=np.float64).reshape(vertex_indices.shape[0], 3),
        squared_distances=np.asarray(payload["squared_distances"], dtype=np.float64).reshape(-1),
        vertex_indices=vertex_indices,
    )


def _mesh_projection_result_from_payload(payload: dict[str, Any]) -> PointCloudMeshProjectionResult:
    face_indices = np.asarray(payload["face_indices"], dtype=np.int64).reshape(-1)
    return PointCloudMeshProjectionResult(
        points=np.asarray(payload["points"], dtype=np.float64).reshape(face_indices.shape[0], 3),
        squared_distances=np.asarray(payload["squared_distances"], dtype=np.float64).reshape(-1),
        face_indices=face_indices,
        vertex_indices=np.asarray(payload["vertex_indices"], dtype=np.int64).reshape(-1),
        normals=np.asarray(payload["normals"], dtype=np.float64).reshape(face_indices.shape[0], 3),
        boundary_flags=np.asarray(payload["boundary_flags"], dtype=np.bool_).reshape(-1),
    )


def point_cloud_nearest_projections(
    query_points: PointCloudDocument | np.ndarray,
    reference: PointCloudDocument,
    *,
    up_dist_limit_sq: float = np.inf,
    lo_dist_limit_sq: float = 0.0,
    skip_same_index: bool = False,
) -> PointCloudProjectionResult:
    payload = rust.point_cloud_nearest_projections(
        _points_from_query(query_points),
        reference.points,
        up_dist_limit_sq=up_dist_limit_sq,
        lo_dist_limit_sq=lo_dist_limit_sq,
        skip_same_index=skip_same_index,
    )
    return _projection_result_from_payload(
        _require_rust(payload, "point_cloud_nearest_projections")
    )


def point_cloud_project_to_mesh(
    query_points: PointCloudDocument | np.ndarray,
    mesh: MeshDocument,
    *,
    up_dist_limit_sq: float = np.finfo(np.float64).max,
    lo_dist_limit_sq: float = 0.0,
    point_transform: np.ndarray | None = None,
    mesh_transform: np.ndarray | None = None,
    face_mask: np.ndarray | None = None,
) -> PointCloudMeshProjectionResult:
    payload = rust.point_cloud_project_to_mesh(
        _points_from_query(query_points),
        mesh.vertices,
        mesh.faces,
        up_dist_limit_sq=up_dist_limit_sq,
        lo_dist_limit_sq=lo_dist_limit_sq,
        point_transform=point_transform,
        mesh_transform=mesh_transform,
        face_mask=face_mask,
    )
    return _mesh_projection_result_from_payload(
        _require_rust(payload, "point_cloud_project_to_mesh")
    )


def point_cloud_n_closest_neighbors(
    cloud: PointCloudDocument,
    *,
    num_neighbors: int,
    up_dist_limit_sq: float = np.finfo(np.float64).max,
) -> np.ndarray:
    payload = rust.point_cloud_n_closest_neighbors(
        cloud.points,
        num_neighbors=num_neighbors,
        up_dist_limit_sq=up_dist_limit_sq,
    )
    values = _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64),
        "point_cloud_n_closest_neighbors",
    )
    return values.reshape(cloud.point_count, int(num_neighbors))


def point_cloud_two_closest_points(cloud: PointCloudDocument) -> PointCloudClosestPair:
    payload = _require_rust(
        rust.point_cloud_two_closest_points(cloud.points),
        "point_cloud_two_closest_points",
    )
    return PointCloudClosestPair(
        vertex_indices=np.asarray(payload["vertex_indices"], dtype=np.int64).reshape(2),
        squared_distance=float(payload["squared_distance"]),
    )


def point_cloud_neighbors_in_radius(
    cloud: PointCloudDocument,
    *,
    center_index: int,
    radius: float,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> np.ndarray:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = rust.point_cloud_neighbors_in_radius(
        cloud.points,
        center_index=center_index,
        radius=radius,
        normals=sample_normals,
        untrusted_indices=untrusted_indices,
    )
    return _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64),
        "point_cloud_neighbors_in_radius",
    )


def point_cloud_select_by_screen_polygon(
    cloud: PointCloudDocument,
    view_projection_4x4,
    polygon_xy,
    *,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> np.ndarray:
    payload = rust.point_cloud_select_by_screen_polygon(
        cloud.points,
        np.asarray(view_projection_4x4, dtype=np.float64),
        np.asarray(polygon_xy, dtype=np.float64),
        normals=_normals_for_cloud(cloud, normals),
        include_backfaces=include_backfaces,
        visible_only=visible_only,
    )
    return _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64).reshape(-1),
        "point_cloud_select_by_screen_polygon",
    )


def point_cloud_select_by_screen_rect(
    cloud: PointCloudDocument,
    view_projection_4x4,
    rect_min_xy,
    rect_max_xy,
    *,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> np.ndarray:
    payload = rust.point_cloud_select_by_screen_rect(
        cloud.points,
        np.asarray(view_projection_4x4, dtype=np.float64),
        np.asarray(rect_min_xy, dtype=np.float64),
        np.asarray(rect_max_xy, dtype=np.float64),
        normals=_normals_for_cloud(cloud, normals),
        include_backfaces=include_backfaces,
        visible_only=visible_only,
    )
    return _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64).reshape(-1),
        "point_cloud_select_by_screen_rect",
    )


def point_cloud_select_by_screen_brush(
    cloud: PointCloudDocument,
    view_projection_4x4,
    brush_path_xy,
    *,
    radius_px: float,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> np.ndarray:
    payload = rust.point_cloud_select_by_screen_brush(
        cloud.points,
        np.asarray(view_projection_4x4, dtype=np.float64),
        np.asarray(brush_path_xy, dtype=np.float64),
        radius_px=radius_px,
        normals=_normals_for_cloud(cloud, normals),
        include_backfaces=include_backfaces,
        visible_only=visible_only,
    )
    return _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64).reshape(-1),
        "point_cloud_select_by_screen_brush",
    )


def point_cloud_pick_by_ray(
    cloud: PointCloudDocument,
    ray_origin,
    ray_direction,
    *,
    max_distance_to_ray: float,
    max_depth: float = np.inf,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
) -> np.ndarray:
    payload = rust.point_cloud_pick_by_ray(
        cloud.points,
        np.asarray(ray_origin, dtype=np.float64),
        np.asarray(ray_direction, dtype=np.float64),
        max_distance_to_ray=max_distance_to_ray,
        max_depth=max_depth,
        normals=_normals_for_cloud(cloud, normals),
        include_backfaces=include_backfaces,
    )
    return _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64).reshape(-1),
        "point_cloud_pick_by_ray",
    )


def point_cloud_extract_selected_points_as_object(
    cloud: PointCloudDocument,
    selected_point_ids,
) -> PointCloudDocument:
    payload = rust.point_cloud_extract_selected_points_as_object(
        cloud.points,
        selected_point_ids,
    )
    payload = _require_rust(payload, "point_cloud_extract_selected_points_as_object")
    source_point_indices = np.asarray(payload["source_point_indices"], dtype=np.int64).reshape(-1)
    return PointCloudDocument(
        np.asarray(payload["points"], dtype=np.float64).reshape((-1, 3)),
        unit=cloud.unit,
        metadata=_clone_region_metadata(cloud, source_point_indices),
    )


def point_cloud_local_neighbor_fan(
    cloud: PointCloudDocument,
    *,
    center_index: int,
    radius: float,
    num_neighbors: int = 0,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 0,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> PointCloudLocalFan:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = _require_rust(
        rust.point_cloud_local_neighbor_fan(
            cloud.points,
            center_index=center_index,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=sample_normals,
            untrusted_indices=untrusted_indices,
        ),
        "point_cloud_local_neighbor_fan",
    )
    return PointCloudLocalFan(
        neighbors=np.asarray(payload["neighbors"], dtype=np.int64).reshape(-1),
        boundary_neighbor=int(payload["boundary_neighbor"]),
        actual_radius=float(payload["actual_radius"]),
        removed_count=int(payload["removed_count"]),
    )


def point_cloud_local_fan_triangles(
    cloud: PointCloudDocument,
    *,
    center_index: int,
    radius: float,
    num_neighbors: int = 0,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 0,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> PointCloudLocalFanTriangles:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = _require_rust(
        rust.point_cloud_local_fan_triangles(
            cloud.points,
            center_index=center_index,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=sample_normals,
            untrusted_indices=untrusted_indices,
        ),
        "point_cloud_local_fan_triangles",
    )
    return PointCloudLocalFanTriangles(
        triangles=np.asarray(payload["triangles"], dtype=np.int64).reshape(-1, 3),
        boundary_neighbor=int(payload["boundary_neighbor"]),
        actual_radius=float(payload["actual_radius"]),
        removed_count=int(payload["removed_count"]),
    )


def point_cloud_local_triangulation_repetitions(
    cloud: PointCloudDocument,
    *,
    radius: float,
    num_neighbors: int = 0,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 0,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> PointCloudLocalTriangulationRepetitions:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = _require_rust(
        rust.point_cloud_local_triangulation_repetitions(
            cloud.points,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=sample_normals,
            untrusted_indices=untrusted_indices,
        ),
        "point_cloud_local_triangulation_repetitions",
    )
    return PointCloudLocalTriangulationRepetitions(
        repetition_counts=np.asarray(payload["repetition_counts"], dtype=np.int64).reshape(4),
        repeated_3=np.asarray(payload["repeated_3"], dtype=np.int64).reshape(-1, 3),
        repeated_2=np.asarray(payload["repeated_2"], dtype=np.int64).reshape(-1, 3),
    )


def _candidate_mesh_from_payload(
    cloud: PointCloudDocument,
    payload: dict[str, Any],
    source: str,
) -> MeshDocument:
    metadata = {
        **cloud.metadata,
        "source": source,
        "repetition_counts": np.asarray(
            payload["repetition_counts"],
            dtype=np.int64,
        ).reshape(4).tolist(),
        "repeated_3_count": int(payload["repeated_3_count"]),
        "repeated_2_count": int(payload["repeated_2_count"]),
    }
    for key in (
        "candidate_face_count",
        "input_face_count",
        "topology_skipped_face_count",
        "topology_degenerate_face_count",
        "topology_nonmanifold_edge_face_count",
        "topology_nonmanifold_vertex_face_count",
        "topology_unsafe_retry_face_count",
        "removed_hole_complicating_face_count",
        "output_repeated_boundary_vertex_count",
        "input_hole_count",
        "filled_hole_count",
        "skipped_hole_count",
        "added_fill_face_count",
    ):
        if key in payload:
            metadata[key] = int(payload[key])
    if "max_hole_perimeter" in payload:
        metadata["max_hole_perimeter"] = float(payload["max_hole_perimeter"])
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        faces=np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        unit=cloud.unit,
        metadata=metadata,
    )


def point_cloud_triangulate_candidate_mesh(
    cloud: PointCloudDocument,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> MeshDocument:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = _require_rust(
        rust.point_cloud_triangulate_candidate_mesh(
            cloud.points,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=sample_normals,
            untrusted_indices=untrusted_indices,
        ),
        "point_cloud_triangulate_candidate_mesh",
    )
    return _candidate_mesh_from_payload(
        cloud,
        payload,
        "point_cloud_triangulate_candidate_mesh",
    )


def point_cloud_triangulate_cleaned_candidate_mesh(
    cloud: PointCloudDocument,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> MeshDocument:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = _require_rust(
        rust.point_cloud_triangulate_cleaned_candidate_mesh(
            cloud.points,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=sample_normals,
            untrusted_indices=untrusted_indices,
        ),
        "point_cloud_triangulate_cleaned_candidate_mesh",
    )
    return _candidate_mesh_from_payload(
        cloud,
        payload,
        "point_cloud_triangulate_cleaned_candidate_mesh",
    )


def point_cloud_triangulate_topology_candidate_mesh(
    cloud: PointCloudDocument,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> MeshDocument:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = _require_rust(
        rust.point_cloud_triangulate_topology_candidate_mesh(
            cloud.points,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=sample_normals,
            untrusted_indices=untrusted_indices,
        ),
        "point_cloud_triangulate_topology_candidate_mesh",
    )
    return _candidate_mesh_from_payload(
        cloud,
        payload,
        "point_cloud_triangulate_topology_candidate_mesh",
    )


def point_cloud_triangulate_filled_candidate_mesh(
    cloud: PointCloudDocument,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    crit_hole_length: float = -1.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> MeshDocument:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = _require_rust(
        rust.point_cloud_triangulate_filled_candidate_mesh(
            cloud.points,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            crit_hole_length=crit_hole_length,
            normals=sample_normals,
            untrusted_indices=untrusted_indices,
        ),
        "point_cloud_triangulate_filled_candidate_mesh",
    )
    return _candidate_mesh_from_payload(
        cloud,
        payload,
        "point_cloud_triangulate_filled_candidate_mesh",
    )


def point_cloud_grid_sample(
    cloud: PointCloudDocument,
    *,
    voxel_size: float,
    max_voxels: int = 500_000,
    return_indices: bool = False,
) -> PointCloudDocument | tuple[PointCloudDocument, np.ndarray]:
    payload = rust.point_cloud_grid_sample_indices(
        cloud.points,
        voxel_size=voxel_size,
        max_voxels=max_voxels,
    )
    indices = _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64),
        "point_cloud_grid_sample_indices",
    )
    sampled = PointCloudDocument(
        cloud.points[indices],
        unit=cloud.unit,
        metadata=dict(cloud.metadata),
    )
    if return_indices:
        return sampled, indices
    return sampled


def point_cloud_uniform_sample(
    cloud: PointCloudDocument,
    *,
    distance: float,
    min_normal_dot: float = 0.0,
    lexicographical_order: bool = True,
    normals: np.ndarray | None = None,
    return_indices: bool = False,
) -> PointCloudDocument | tuple[PointCloudDocument, np.ndarray]:
    sample_normals = None
    if normals is not None:
        sample_normals = np.asarray(normals, dtype=np.float64)
        if sample_normals.ndim != 2 or sample_normals.shape != cloud.points.shape:
            raise ValueError("normals must have shape matching cloud.points")
    payload = rust.point_cloud_uniform_sample_indices(
        cloud.points,
        distance=distance,
        min_normal_dot=min_normal_dot,
        lexicographical_order=lexicographical_order,
        normals=sample_normals,
    )
    indices = _require_rust(
        None if payload is None else np.asarray(payload, dtype=np.int64),
        "point_cloud_uniform_sample_indices",
    )
    sampled = PointCloudDocument(
        cloud.points[indices],
        unit=cloud.unit,
        metadata=dict(cloud.metadata),
    )
    if return_indices:
        return sampled, indices
    return sampled


def pairwise_point_to_point_icp(
    floating: PointCloudDocument,
    reference: PointCloudDocument,
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
) -> ICPRegistrationResult:
    payload = rust.pairwise_point_to_point_icp(
        floating.points,
        reference.points,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
    )
    result = None if payload is None else _result_from_payload(payload, "point_to_point", mode)
    return _require_rust(result, "pairwise_point_to_point_icp")


def pairwise_point_to_plane_icp(
    floating: PointCloudDocument,
    reference: PointCloudDocument,
    reference_normals: np.ndarray,
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    floating_normals: np.ndarray | None = None,
    max_pair_distance: float | None = None,
    cos_threshold: float | None = None,
    far_dist_factor: float | None = None,
    mutual_closest: bool = False,
) -> ICPRegistrationResult:
    normals = np.asarray(reference_normals, dtype=np.float64)
    if normals.ndim != 2 or normals.shape != reference.points.shape:
        raise ValueError("reference_normals must have shape matching reference.points")
    source_normals = None
    if floating_normals is not None:
        source_normals = np.asarray(floating_normals, dtype=np.float64)
        if source_normals.ndim != 2 or source_normals.shape != floating.points.shape:
            raise ValueError("floating_normals must have shape matching floating.points")
    payload = rust.pairwise_point_to_plane_icp(
        floating.points,
        reference.points,
        normals,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        floating_normals=source_normals,
        max_pair_distance=max_pair_distance,
        cos_threshold=cos_threshold,
        far_dist_factor=far_dist_factor,
        mutual_closest=mutual_closest,
    )
    result = None if payload is None else _result_from_payload(payload, "point_to_plane", mode)
    return _require_rust(result, "pairwise_point_to_plane_icp")
