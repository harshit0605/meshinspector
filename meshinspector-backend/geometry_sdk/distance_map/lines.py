"""MeshLib ObjectLines persistence wrappers for line and contour data."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Sequence

import numpy as np

from geometry_sdk.accelerators import rust

PointLike = Sequence[float]
ContourLike = Sequence[PointLike]


@dataclass(slots=True)
class ObjectLinesDocument:
    points: np.ndarray
    lines: np.ndarray
    show_points: int = 0
    smooth_connections: int = 0
    line_width: float = 1.0
    coloring_type: str = "Solid"
    line_colors: list[Any] = field(default_factory=list)
    vert_colors: list[Any] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        points = np.asarray(self.points, dtype=np.float64)
        if points.size == 0:
            self.points = points.reshape(0, 3)
        elif points.size % 3 == 0:
            self.points = points.reshape(-1, 3)
        else:
            raise ValueError("ObjectLines points must be 3D coordinates")
        lines = np.asarray(self.lines, dtype=np.int64)
        if lines.size == 0:
            self.lines = lines.reshape(0, 2)
        elif lines.size % 2 == 0:
            self.lines = lines.reshape(-1, 2)
        else:
            raise ValueError("ObjectLines lines must be vertex-index pairs")
        if self.lines.size and (self.lines.min() < 0 or self.lines.max() >= len(self.points)):
            raise ValueError("ObjectLines lines must reference existing points")
        self.show_points = int(self.show_points)
        self.smooth_connections = int(self.smooth_connections)
        self.line_width = float(self.line_width)
        if not np.isfinite(self.points).all():
            raise ValueError("ObjectLines points must be finite")
        if not np.isfinite(self.line_width) or self.line_width < 0.0:
            raise ValueError("ObjectLines line_width must be finite and non-negative")

    def to_meshlib_json(self) -> dict[str, Any]:
        return {
            "Type": ["LinesHolder", "ObjectLines"],
            "ShowPoints": self.show_points,
            "SmoothConnections": self.smooth_connections,
            "ColoringType": self.coloring_type,
            "LineColors": list(self.line_colors),
            "VertColors": list(self.vert_colors),
            "LineWidth": self.line_width,
            "Polyline": {
                "Points": self.points.tolist(),
                "Lines": self.lines.reshape(-1).astype(int).tolist(),
            },
        }

    @classmethod
    def from_meshlib_json(cls, payload: dict[str, Any]) -> "ObjectLinesDocument":
        polyline = dict(payload.get("Polyline") or {})
        return cls(
            points=np.asarray(polyline.get("Points", []), dtype=np.float64),
            lines=np.asarray(polyline.get("Lines", []), dtype=np.int64).reshape(-1, 2),
            show_points=int(payload.get("ShowPoints", 0)),
            smooth_connections=int(payload.get("SmoothConnections", 0)),
            line_width=float(payload.get("LineWidth", 1.0)),
            coloring_type=str(payload.get("ColoringType", "Solid")),
            line_colors=list(payload.get("LineColors", [])),
            vert_colors=list(payload.get("VertColors", [])),
        )


def _require_rust(value, operation: str):
    if value is None:
        raise RuntimeError(f"Rust {operation} kernel is unavailable")
    return value


def _flatten_contours3(contours: Sequence[ContourLike]) -> tuple[np.ndarray, np.ndarray]:
    points: list[tuple[float, float, float]] = []
    offsets = [0]
    for contour in contours:
        for point in contour:
            if len(point) == 2:
                points.append((float(point[0]), float(point[1]), 0.0))
            elif len(point) == 3:
                points.append((float(point[0]), float(point[1]), float(point[2])))
            else:
                raise ValueError("ObjectLines contour points must be 2D or 3D")
        offsets.append(len(points))
    return (
        np.asarray(points, dtype=np.float64).reshape(-1, 3),
        np.asarray(offsets, dtype=np.int64),
    )


def _flatten_offset_rows(offsets: Sequence[Sequence[float]]) -> tuple[np.ndarray, np.ndarray]:
    values: list[float] = []
    row_offsets = [0]
    for row in offsets:
        values.extend(float(value) for value in row)
        row_offsets.append(len(values))
    return (
        np.asarray(values, dtype=np.float64).reshape(-1),
        np.asarray(row_offsets, dtype=np.int64),
    )


def _document_from_payload(payload: dict[str, Any]) -> ObjectLinesDocument:
    metadata: dict[str, Any] = {"source": "MeshLib ObjectLines JSON"}
    if payload.get("uv_coords"):
        metadata["uv_coords"] = list(payload["uv_coords"])
    if payload.get("texture_files"):
        metadata["texture_files"] = list(payload["texture_files"])
    return ObjectLinesDocument(
        points=np.asarray(payload["points"], dtype=np.float64),
        lines=np.asarray(payload["lines"], dtype=np.int64),
        show_points=int(payload.get("show_points", 0)),
        smooth_connections=int(payload.get("smooth_connections", 0)),
        line_width=float(payload.get("line_width", 1.0)),
        coloring_type=str(payload.get("coloring_type", "Solid")),
        line_colors=list(payload.get("line_colors", [])),
        vert_colors=list(payload.get("vert_colors", [])),
        metadata=metadata,
    )


def _coerce_document(document: ObjectLinesDocument | dict[str, Any]) -> ObjectLinesDocument:
    return ObjectLinesDocument.from_meshlib_json(document) if isinstance(document, dict) else document


def object_lines_from_contours(
    contours: Sequence[ContourLike],
    *,
    line_width: float = 1.0,
    show_points: int = 0,
    smooth_connections: int = 0,
) -> ObjectLinesDocument:
    contour_points, contour_offsets = _flatten_contours3(contours)
    payload = rust.object_lines_from_contours(
        contour_points,
        contour_offsets,
        line_width=float(line_width),
        show_points=int(show_points),
        smooth_connections=int(smooth_connections),
    )
    result = None if payload is None else _document_from_payload(payload)
    return _require_rust(result, "object_lines_from_contours")


def object_lines_to_contours(document: ObjectLinesDocument | dict[str, Any]) -> list[list[tuple[float, float, float]]]:
    lines = _coerce_document(document)
    payload = rust.object_lines_to_contours(
        lines.points,
        lines.lines,
        line_width=lines.line_width,
        show_points=lines.show_points,
        smooth_connections=lines.smooth_connections,
    )
    payload = _require_rust(payload, "object_lines_to_contours")
    points = np.asarray(payload["contour_points"], dtype=np.float64).reshape(-1, 3)
    offsets = np.asarray(payload["contour_offsets"], dtype=np.int64).reshape(-1)
    contours: list[list[tuple[float, float, float]]] = []
    for start, end in zip(offsets[:-1], offsets[1:]):
        contours.append([tuple(float(value) for value in point) for point in points[int(start) : int(end)]])
    return contours


def _to_clockwise_if_closed(contour: ContourLike) -> ContourLike:
    """Normalize a CLOSED contour to clockwise winding. The Rust offset kernel only
    implements the clockwise convention (a CCW closed contour otherwise raises
    "supports clockwise contours"); MeshLib's offsetContours is winding-agnostic.
    Reversing a closed contour's point order flips its winding without changing its
    shape, and the kernel's positive offset then grows it outward consistently.
    Open contours and degenerate inputs are returned unchanged.
    """
    pts = np.asarray(contour, dtype=np.float64)
    if pts.ndim != 2 or pts.shape[0] < 4 or pts.shape[1] < 2:
        return contour
    if not np.allclose(pts[0, :2], pts[-1, :2], atol=1e-9):
        return contour  # open contour — winding is not defined
    x, y = pts[:, 0], pts[:, 1]
    signed_area2 = float(np.sum(x[:-1] * y[1:] - x[1:] * y[:-1]))
    if signed_area2 > 0.0:  # counter-clockwise -> reverse to clockwise
        return pts[::-1]
    return contour


def offset_contours(
    contours: Sequence[ContourLike],
    *,
    offset: float | None = None,
    offsets: Sequence[Sequence[float]] | None = None,
    min_angle_precision: float = float(np.pi / 9.0),
    mode: str = "offset",
    end_type: str = "round",
    corner_type: str = "round",
    max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
    z_restore: str = "default",
    z_value: float | None = None,
    z_values: Sequence[Sequence[float]] | None = None,
    relax_iterations: int = 1,
) -> list[list[tuple[float, float, float]]]:
    if offsets is None:
        # Normalize closed-contour winding to clockwise (the kernel's supported
        # convention) for the scalar-offset path. Variable per-point offsets would
        # misalign on reversal, so they keep the explicit-winding contract.
        contours = [_to_clockwise_if_closed(contour) for contour in contours]
    contour_points, contour_offsets = _flatten_contours3(contours)
    if callable(z_values):
        z_value_points, z_value_offsets = z_values, None
    else:
        z_value_points, z_value_offsets = (
            _flatten_offset_rows(z_values) if z_values is not None else (None, None)
        )
    if offsets is None:
        if offset is None:
            raise ValueError("offset_contours requires either offset or offsets")
        payload = rust.offset_contours(
            contour_points,
            contour_offsets,
            offset=float(offset),
            min_angle_precision=float(min_angle_precision),
            mode=str(mode),
            end_type=str(end_type),
            corner_type=str(corner_type),
            max_sharp_angle=float(max_sharp_angle),
            z_restore=str(z_restore),
            z_value=z_value,
            relax_iterations=int(relax_iterations),
            z_values=z_value_points,
            z_value_offsets=z_value_offsets,
        )
    else:
        point_offsets, offset_offsets = _flatten_offset_rows(offsets)
        payload = rust.offset_contours_variable(
            contour_points,
            contour_offsets,
            point_offsets,
            offset_offsets,
            min_angle_precision=float(min_angle_precision),
            mode=str(mode),
            end_type=str(end_type),
            corner_type=str(corner_type),
            max_sharp_angle=float(max_sharp_angle),
            z_restore=str(z_restore),
            z_value=z_value,
            relax_iterations=int(relax_iterations),
            z_values=z_value_points,
            z_value_offsets=z_value_offsets,
        )
    payload = _require_rust(payload, "offset_contours")
    points = np.asarray(payload["contour_points"], dtype=np.float64).reshape(-1, 3)
    offsets = np.asarray(payload["contour_offsets"], dtype=np.int64).reshape(-1)
    result: list[list[tuple[float, float, float]]] = []
    for start, end in zip(offsets[:-1], offsets[1:]):
        result.append(
            [tuple(float(value) for value in point) for point in points[int(start) : int(end)]]
        )
    return result


def offset_contours_with_origins(
    contours: Sequence[ContourLike],
    *,
    offset: float | None = None,
    offsets: Sequence[Sequence[float]] | None = None,
    min_angle_precision: float = float(np.pi / 9.0),
    mode: str = "offset",
    end_type: str = "round",
    corner_type: str = "round",
    max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
    z_restore: str = "default",
    z_value: float | None = None,
    z_values: Sequence[Sequence[float]] | None = None,
    relax_iterations: int = 1,
) -> dict[str, Any]:
    contour_points, contour_offsets = _flatten_contours3(contours)
    if callable(z_values):
        z_value_points, z_value_offsets = z_values, None
    else:
        z_value_points, z_value_offsets = (
            _flatten_offset_rows(z_values) if z_values is not None else (None, None)
        )
    if offsets is None:
        if offset is None:
            raise ValueError("offset_contours_with_origins requires either offset or offsets")
        payload = rust.offset_contours_with_origins(
            contour_points,
            contour_offsets,
            offset=float(offset),
            min_angle_precision=float(min_angle_precision),
            mode=str(mode),
            end_type=str(end_type),
            corner_type=str(corner_type),
            max_sharp_angle=float(max_sharp_angle),
            z_restore=str(z_restore),
            z_value=z_value,
            relax_iterations=int(relax_iterations),
            z_values=z_value_points,
            z_value_offsets=z_value_offsets,
        )
    else:
        point_offsets, offset_offsets = _flatten_offset_rows(offsets)
        payload = rust.offset_contours_variable_with_origins(
            contour_points,
            contour_offsets,
            point_offsets,
            offset_offsets,
            min_angle_precision=float(min_angle_precision),
            mode=str(mode),
            end_type=str(end_type),
            corner_type=str(corner_type),
            max_sharp_angle=float(max_sharp_angle),
            z_restore=str(z_restore),
            z_value=z_value,
            relax_iterations=int(relax_iterations),
            z_values=z_value_points,
            z_value_offsets=z_value_offsets,
        )
    payload = _require_rust(payload, "offset_contours_with_origins")
    points = np.asarray(payload["contour_points"], dtype=np.float64).reshape(-1, 3)
    contour_offsets_out = np.asarray(payload["contour_offsets"], dtype=np.int64).reshape(-1)
    result: list[list[tuple[float, float, float]]] = []
    for start, end in zip(contour_offsets_out[:-1], contour_offsets_out[1:]):
        result.append(
            [tuple(float(value) for value in point) for point in points[int(start) : int(end)]]
        )
    return {"contours": result, "origins": list(payload["origins"])}


def object_lines_to_pts(document: ObjectLinesDocument | dict[str, Any]) -> str:
    lines = _coerce_document(document)
    source = rust.object_lines_to_pts(
        lines.points,
        lines.lines,
        line_width=lines.line_width,
        show_points=lines.show_points,
        smooth_connections=lines.smooth_connections,
    )
    return _require_rust(source, "object_lines_to_pts")


def object_lines_from_pts(source: str) -> ObjectLinesDocument:
    payload = rust.object_lines_from_pts(str(source))
    result = None if payload is None else _document_from_payload(payload)
    return _require_rust(result, "object_lines_from_pts")


def object_lines_to_dxf(document: ObjectLinesDocument | dict[str, Any]) -> str:
    lines = _coerce_document(document)
    source = rust.object_lines_to_dxf(
        lines.points,
        lines.lines,
        line_width=lines.line_width,
        show_points=lines.show_points,
        smooth_connections=lines.smooth_connections,
    )
    return _require_rust(source, "object_lines_to_dxf")


def object_lines_to_mrlines(document: ObjectLinesDocument | dict[str, Any]) -> bytes:
    lines = _coerce_document(document)
    payload = rust.object_lines_to_mrlines(
        lines.points,
        lines.lines,
        line_width=lines.line_width,
        show_points=lines.show_points,
        smooth_connections=lines.smooth_connections,
    )
    return _require_rust(payload, "object_lines_to_mrlines")


def object_lines_from_mrlines(source: bytes) -> ObjectLinesDocument:
    payload = rust.object_lines_from_mrlines(bytes(source))
    result = None if payload is None else _document_from_payload(payload)
    return _require_rust(result, "object_lines_from_mrlines")


def object_lines_to_ply(document: ObjectLinesDocument | dict[str, Any]) -> bytes:
    lines = _coerce_document(document)
    payload = rust.object_lines_to_ply(
        lines.points,
        lines.lines,
        line_width=lines.line_width,
        show_points=lines.show_points,
        smooth_connections=lines.smooth_connections,
        vert_colors=lines.vert_colors,
    )
    return _require_rust(payload, "object_lines_to_ply")


def object_lines_from_ply(source: bytes) -> ObjectLinesDocument:
    payload = rust.object_lines_from_ply(bytes(source))
    result = None if payload is None else _document_from_payload(payload)
    return _require_rust(result, "object_lines_from_ply")


def object_lines_from_svg(source: str) -> ObjectLinesDocument:
    payload = rust.object_lines_from_svg(str(source))
    result = None if payload is None else _document_from_payload(payload)
    if result is not None:
        result.metadata["source"] = "MeshLib SVG ObjectLines"
    return _require_rust(result, "object_lines_from_svg")
