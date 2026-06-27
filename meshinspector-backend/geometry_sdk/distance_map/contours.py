"""Contour distance-map compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal, Sequence

import numpy as np

from geometry_sdk.accelerators import rust
from geometry_sdk.types import MeshDocument

Point2 = Sequence[float]
Contour2 = Sequence[Point2]


@dataclass(slots=True)
class DistanceMapDocument:
    width: int
    height: int
    origin: tuple[float, float]
    pixel_size: tuple[float, float]
    values: np.ndarray
    valid_count: int
    min_value: float
    max_value: float
    model_transform: tuple[float, ...] | None = None
    unit: str = "mm"
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        values = np.asarray(self.values, dtype=np.float32)
        if values.size != self.width * self.height:
            raise ValueError("distance-map values must match width * height")
        self.values = values.reshape(self.height, self.width)
        if self.model_transform is not None:
            transform = np.asarray(self.model_transform, dtype=np.float64).reshape(-1)
            if transform.size != 16:
                raise ValueError("model_transform must contain 16 row-major values")
            if not np.isfinite(transform).all():
                raise ValueError("model_transform values must be finite")
            self.model_transform = tuple(float(value) for value in transform)


@dataclass(slots=True)
class IsoLineSegmentsDocument:
    iso_value: float
    segments: np.ndarray
    unit: str = "mm"
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        segments = np.asarray(self.segments, dtype=np.float64)
        if segments.size == 0:
            self.segments = segments.reshape(0, 2, 2)
            return
        if segments.size % 4 != 0:
            raise ValueError("iso-line segment coordinates must be groups of two 2D points")
        self.segments = segments.reshape(-1, 2, 2)

    @property
    def segment_count(self) -> int:
        return int(self.segments.shape[0])


def _flatten_contours(contours: Sequence[Contour2]) -> tuple[np.ndarray, np.ndarray]:
    points: list[tuple[float, float]] = []
    offsets = [0]
    for contour in contours:
        contour_points = [(float(point[0]), float(point[1])) for point in contour]
        points.extend(contour_points)
        offsets.append(len(points))
    return (
        np.asarray(points, dtype=np.float64).reshape(-1, 2),
        np.asarray(offsets, dtype=np.int64),
    )


def _normalize_pixel_size(pixel_size: float | Sequence[float]) -> tuple[float, float]:
    if isinstance(pixel_size, (int, float)):
        value = float(pixel_size)
        return value, value
    return float(pixel_size[0]), float(pixel_size[1])


def _result_from_payload(payload: dict[str, Any]) -> DistanceMapDocument:
    return DistanceMapDocument(
        width=int(payload["width"]),
        height=int(payload["height"]),
        origin=tuple(float(value) for value in payload["origin"]),
        pixel_size=tuple(float(value) for value in payload["pixel_size"]),
        values=np.asarray(payload["values"], dtype=np.float32),
        valid_count=int(payload["valid_count"]),
        min_value=float(payload["min_value"]),
        max_value=float(payload["max_value"]),
        model_transform=(
            None
            if payload.get("model_transform") is None
            else tuple(float(value) for value in payload["model_transform"])
        ),
    )


def _iso_result_from_payload(payload: dict[str, Any]) -> IsoLineSegmentsDocument:
    return IsoLineSegmentsDocument(
        iso_value=float(payload["iso_value"]),
        segments=np.asarray(payload["segments"], dtype=np.float64),
        metadata={key: payload[key] for key in ("mode",) if key in payload},
    )


def _require_rust(value, operation: str):
    if value is None:
        raise RuntimeError(f"Rust {operation} kernel is unavailable")
    return value


def distance_map_from_contours(
    contours: Sequence[Contour2],
    *,
    width: int,
    height: int,
    origin: tuple[float, float] = (0.0, 0.0),
    pixel_size: float | tuple[float, float] = 1.0,
    signed: bool = False,
) -> DistanceMapDocument:
    contour_points, contour_offsets = _flatten_contours(contours)
    normalized_pixel_size = _normalize_pixel_size(pixel_size)
    payload = rust.distance_map_from_contours(
        contour_points,
        contour_offsets,
        width=width,
        height=height,
        origin=(float(origin[0]), float(origin[1])),
        pixel_size=normalized_pixel_size,
        signed=signed,
    )
    result = None if payload is None else _result_from_payload(payload)
    return _require_rust(result, "distance_map_from_contours")


def distance_map_from_mesh(
    mesh: MeshDocument,
    *,
    width: int,
    height: int,
    origin: tuple[float, float, float],
    x_range: tuple[float, float, float],
    y_range: tuple[float, float, float],
    direction: tuple[float, float, float],
    epsilon: float = 1e-8,
) -> DistanceMapDocument:
    payload = rust.distance_map_from_mesh(
        mesh.vertices,
        mesh.faces,
        width=width,
        height=height,
        origin=origin,
        x_range=x_range,
        y_range=y_range,
        direction=direction,
        epsilon=epsilon,
    )
    result = None if payload is None else _result_from_payload(payload)
    return _require_rust(result, "distance_map_from_mesh")


def distance_map_from_tiff(path: str | Path) -> DistanceMapDocument:
    payload = rust.distance_map_from_tiff(str(path))
    result = None if payload is None else _result_from_payload(payload)
    result = _require_rust(result, "distance_map_from_tiff")
    result.metadata["source"] = "MeshLib-style TIFF distance-map import"
    return result


def distance_map_to_tiff(distance_map: DistanceMapDocument, path: str | Path) -> Path:
    output_path = Path(path)
    rust.distance_map_to_tiff(
        distance_map.values,
        origin=distance_map.origin,
        pixel_size=distance_map.pixel_size,
        model_transform=distance_map.model_transform,
        path=str(output_path),
    )
    return output_path


def distance_map_to_iso_segments(
    distance_map: DistanceMapDocument,
    *,
    iso_value: float,
) -> IsoLineSegmentsDocument:
    payload = rust.distance_map_to_iso_segments(
        distance_map.values,
        origin=distance_map.origin,
        pixel_size=distance_map.pixel_size,
        iso_value=float(iso_value),
    )
    result = None if payload is None else _iso_result_from_payload(payload)
    return _require_rust(result, "distance_map_to_iso_segments")


def distance_map_merge(
    left: DistanceMapDocument,
    right: DistanceMapDocument,
    *,
    mode: Literal["min", "max", "subtract"],
) -> DistanceMapDocument:
    payload = rust.distance_map_merge(
        left.values,
        right.values,
        left_origin=left.origin,
        left_pixel_size=left.pixel_size,
        right_origin=right.origin,
        right_pixel_size=right.pixel_size,
        mode=mode,
    )
    result = None if payload is None else _result_from_payload(payload)
    return _require_rust(result, "distance_map_merge")


def distance_map_contour_boolean(
    contours_a: Sequence[Contour2],
    contours_b: Sequence[Contour2],
    *,
    mode: Literal["union", "intersection", "subtract"],
    width: int,
    height: int,
    origin: tuple[float, float] = (0.0, 0.0),
    pixel_size: float | tuple[float, float] = 1.0,
    iso_value: float = 0.0,
) -> IsoLineSegmentsDocument:
    points_a, offsets_a = _flatten_contours(contours_a)
    points_b, offsets_b = _flatten_contours(contours_b)
    normalized_pixel_size = _normalize_pixel_size(pixel_size)
    payload = rust.distance_map_contour_boolean(
        points_a,
        offsets_a,
        points_b,
        offsets_b,
        mode=mode,
        width=width,
        height=height,
        origin=(float(origin[0]), float(origin[1])),
        pixel_size=normalized_pixel_size,
        iso_value=float(iso_value),
    )
    result = None if payload is None else _iso_result_from_payload(payload)
    return _require_rust(result, "distance_map_contour_boolean")
