"""Multi-object point-cloud ICP wrappers for Rust-owned registration kernels."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Literal

import numpy as np

from geometry_sdk.accelerators import rust
from geometry_sdk.point_cloud.icp import PointCloudDocument, _require_rust


@dataclass(slots=True)
class ICPObjectTransform:
    rotation: np.ndarray
    translation: np.ndarray
    transform: np.ndarray

    def apply(self, cloud: PointCloudDocument) -> PointCloudDocument:
        points = np.asarray(cloud.points, dtype=np.float64) @ self.rotation.T + self.translation
        return PointCloudDocument(points, unit=cloud.unit, metadata=dict(cloud.metadata))


@dataclass(slots=True)
class MultiwayICPRegistrationResult:
    transforms: tuple[ICPObjectTransform, ...]
    iterations: int
    mean_square_distance: float
    active_pair_count: int
    fixed_object_index: int
    method: Literal[
        "point_to_point",
        "point_to_plane",
        "combined",
        "point_to_point_all_object",
        "point_to_plane_all_object",
        "combined_all_object",
        "point_to_point_sequential_cascade",
        "point_to_plane_sequential_cascade",
        "combined_sequential_cascade",
        "point_to_point_aabb_cascade",
        "point_to_plane_aabb_cascade",
        "combined_aabb_cascade",
    ]
    mode: Literal["rigid", "translation"]


def _object_transform_from_payload(payload: dict[str, Any]) -> ICPObjectTransform:
    return ICPObjectTransform(
        rotation=np.asarray(payload["rotation"], dtype=np.float64).reshape(3, 3),
        translation=np.asarray(payload["translation"], dtype=np.float64).reshape(3),
        transform=np.asarray(payload["transform"], dtype=np.float64).reshape(4, 4),
    )


def _multiway_result_from_payload(
    payload: dict[str, Any],
    method: Literal[
        "point_to_point",
        "point_to_plane",
        "combined",
        "point_to_point_all_object",
        "point_to_plane_all_object",
        "combined_all_object",
        "point_to_point_sequential_cascade",
        "point_to_plane_sequential_cascade",
        "combined_sequential_cascade",
        "point_to_point_aabb_cascade",
        "point_to_plane_aabb_cascade",
        "combined_aabb_cascade",
    ],
    mode: Literal["rigid", "translation"],
) -> MultiwayICPRegistrationResult:
    return MultiwayICPRegistrationResult(
        transforms=tuple(
            _object_transform_from_payload(transform) for transform in payload["transforms"]
        ),
        iterations=int(payload["iterations"]),
        mean_square_distance=float(payload["mean_square_distance"]),
        active_pair_count=int(payload["active_pair_count"]),
        fixed_object_index=int(payload["fixed_object_index"]),
        method=method,
        mode=mode,
    )


def multiway_point_to_point_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_point_to_point_icp(
        tuple(cloud.points for cloud in objects),
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_point", mode)
    )
    return _require_rust(result, "multiway_point_to_point_icp")


def multiway_point_to_plane_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_point_to_plane_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_plane", mode)
    )
    return _require_rust(result, "multiway_point_to_plane_icp")


def multiway_combined_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_combined_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = None if payload is None else _multiway_result_from_payload(payload, "combined", mode)
    return _require_rust(result, "multiway_combined_icp")


def multiway_all_object_point_to_point_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_all_object_point_to_point_icp(
        tuple(cloud.points for cloud in objects),
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_point_all_object", mode)
    )
    return _require_rust(result, "multiway_all_object_point_to_point_icp")


def multiway_all_object_point_to_plane_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_all_object_point_to_plane_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_plane_all_object", mode)
    )
    return _require_rust(result, "multiway_all_object_point_to_plane_icp")


def multiway_all_object_combined_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_all_object_combined_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "combined_all_object", mode)
    )
    return _require_rust(result, "multiway_all_object_combined_icp")


def multiway_sequential_cascade_point_to_point_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_sequential_cascade_point_to_point_icp(
        tuple(cloud.points for cloud in objects),
        max_group_size=max_group_size,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_point_sequential_cascade", mode)
    )
    return _require_rust(result, "multiway_sequential_cascade_point_to_point_icp")


def multiway_sequential_cascade_point_to_plane_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_sequential_cascade_point_to_plane_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_group_size=max_group_size,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_plane_sequential_cascade", mode)
    )
    return _require_rust(result, "multiway_sequential_cascade_point_to_plane_icp")


def multiway_sequential_cascade_combined_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_sequential_cascade_combined_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_group_size=max_group_size,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "combined_sequential_cascade", mode)
    )
    return _require_rust(result, "multiway_sequential_cascade_combined_icp")


def multiway_aabb_cascade_point_to_point_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_aabb_cascade_point_to_point_icp(
        tuple(cloud.points for cloud in objects),
        max_group_size=max_group_size,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_point_aabb_cascade", mode)
    )
    return _require_rust(result, "multiway_aabb_cascade_point_to_point_icp")


def multiway_aabb_cascade_point_to_plane_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_aabb_cascade_point_to_plane_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_group_size=max_group_size,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "point_to_plane_aabb_cascade", mode)
    )
    return _require_rust(result, "multiway_aabb_cascade_point_to_plane_icp")


def multiway_aabb_cascade_combined_icp(
    objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
    normals: tuple[np.ndarray, ...] | list[np.ndarray],
    *,
    max_group_size: int = 64,
    max_iterations: int = 20,
    tolerance: float = 1e-8,
    mode: Literal["rigid", "translation"] = "rigid",
    fixed_object_index: int | None = None,
) -> MultiwayICPRegistrationResult:
    payload = rust.multiway_aabb_cascade_combined_icp(
        tuple(cloud.points for cloud in objects),
        tuple(np.asarray(values, dtype=np.float64) for values in normals),
        max_group_size=max_group_size,
        max_iterations=max_iterations,
        tolerance=tolerance,
        mode=mode,
        fixed_object_index=fixed_object_index,
    )
    result = (
        None
        if payload is None
        else _multiway_result_from_payload(payload, "combined_aabb_cascade", mode)
    )
    return _require_rust(result, "multiway_aabb_cascade_combined_icp")
