from __future__ import annotations

from dataclasses import dataclass, field
from collections.abc import Sequence
from pathlib import Path
from typing import Any

import numpy as np

from geometry_sdk.accelerators import rust


_AXIS_NAME_TO_INDEX = {"A": 0, "B": 1, "C": 2}
_AXIS_INDEX_TO_NAME = {0: "A", 1: "B", 2: "C"}


@dataclass(slots=True)
class GcodeMachineSettings:
    home_position: Sequence[float] = (0.0, 0.0, 0.0)
    feedrate_idle: float = 10_000.0
    rotation_axes: Sequence[Sequence[float]] = (
        (-1.0, 0.0, 0.0),
        (0.0, -1.0, 0.0),
        (0.0, 0.0, 1.0),
    )
    rotation_order: Sequence[str | int] = ("A", "B", "C")
    rotation_limits: Sequence[Sequence[float] | None] = (None, None, None)

    def to_payload(self) -> dict[str, Any]:
        axes = list(self.rotation_axes)
        if len(axes) != 3:
            raise ValueError("rotation_axes must contain exactly three axes")
        rotation_limits = list(self.rotation_limits)
        if len(rotation_limits) != 3:
            raise ValueError("rotation_limits must contain exactly three axis entries")
        return {
            "home_position": _vector3(self.home_position, "home_position"),
            "feedrate_idle": float(self.feedrate_idle),
            "rotation_axes": [
                _vector3(axes[0], "rotation_axes[0]"),
                _vector3(axes[1], "rotation_axes[1]"),
                _vector3(axes[2], "rotation_axes[2]"),
            ],
            "rotation_order": [_axis_index(axis) for axis in self.rotation_order],
            "rotation_limits": [
                _optional_vector2(rotation_limits[0], "rotation_limits[0]"),
                _optional_vector2(rotation_limits[1], "rotation_limits[1]"),
                _optional_vector2(rotation_limits[2], "rotation_limits[2]"),
            ],
        }

    def to_meshlib_json(self) -> dict[str, Any]:
        return rust.gcode_machine_settings_to_meshlib_json(self.to_payload())

    @classmethod
    def from_meshlib_json(cls, value: dict[str, Any]) -> GcodeMachineSettings:
        payload = rust.gcode_machine_settings_from_meshlib_json(dict(value))
        return cls(
            home_position=tuple(payload["home_position"]),
            feedrate_idle=float(payload["feedrate_idle"]),
            rotation_axes=tuple(tuple(axis) for axis in payload["rotation_axes"]),
            rotation_order=tuple(
                _AXIS_INDEX_TO_NAME[int(axis)] for axis in payload["rotation_order"]
            ),
            rotation_limits=tuple(
                None if limits is None else tuple(limits)
                for limits in payload["rotation_limits"]
            ),
        )


@dataclass(slots=True)
class GcodePathDocument:
    segments: np.ndarray
    tool_directions: np.ndarray
    source_frame_indices: np.ndarray
    idle: np.ndarray
    feedrates: np.ndarray
    frame_count: int
    command_count: int
    max_feedrate: float
    warnings: list[str] = field(default_factory=list)
    unit: str = "mm"
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        segments = np.asarray(self.segments, dtype=np.float64)
        if segments.size == 0:
            self.segments = segments.reshape(0, 2, 3)
        elif segments.size % 6 == 0:
            self.segments = segments.reshape(-1, 2, 3)
        else:
            raise ValueError("G-code path segments must be groups of two 3D points")

        segment_count = int(self.segments.shape[0])
        tool_directions = np.asarray(self.tool_directions, dtype=np.float64)
        if tool_directions.size == 0:
            self.tool_directions = tool_directions.reshape(0, 2, 3)
        elif tool_directions.size % 6 == 0:
            self.tool_directions = tool_directions.reshape(-1, 2, 3)
        else:
            raise ValueError("G-code tool directions must be groups of two 3D vectors")

        self.source_frame_indices = np.asarray(self.source_frame_indices, dtype=np.int64).reshape(-1)
        self.idle = np.asarray(self.idle, dtype=np.bool_).reshape(-1)
        self.feedrates = np.asarray(self.feedrates, dtype=np.float64).reshape(-1)
        if self.tool_directions.shape[0] != segment_count:
            raise ValueError("tool_directions must match G-code segment count")
        if self.source_frame_indices.size != segment_count:
            raise ValueError("source_frame_indices must match G-code segment count")
        if self.idle.size != segment_count:
            raise ValueError("idle flags must match G-code segment count")
        if self.feedrates.size != segment_count:
            raise ValueError("feedrates must match G-code segment count")

    @property
    def segment_count(self) -> int:
        return int(self.segments.shape[0])


def _require_rust(value, operation: str):
    if value is None:
        raise RuntimeError(f"Rust {operation} kernel is unavailable")
    return value


def _vector3(value: Sequence[float], name: str) -> list[float]:
    values = list(value)
    if len(values) != 3:
        raise ValueError(f"{name} must have length 3")
    return [float(values[0]), float(values[1]), float(values[2])]


def _optional_vector2(value: Sequence[float] | None, name: str) -> list[float] | None:
    if value is None:
        return None
    values = list(value)
    if len(values) != 2:
        raise ValueError(f"{name} must have length 2")
    return [float(values[0]), float(values[1])]


def _axis_index(axis: str | int) -> int:
    if isinstance(axis, str):
        try:
            return _AXIS_NAME_TO_INDEX[axis.upper()]
        except KeyError as exc:
            raise ValueError(f"unknown G-code rotation axis {axis!r}") from exc
    axis_index = int(axis)
    if axis_index < 0 or axis_index > 2:
        raise ValueError("rotation_order axis ids must be 0, 1, or 2")
    return axis_index


def _result_from_payload(payload: dict[str, Any]) -> GcodePathDocument:
    return GcodePathDocument(
        segments=np.asarray(payload["segments"], dtype=np.float64),
        tool_directions=np.asarray(payload["tool_directions"], dtype=np.float64),
        source_frame_indices=np.asarray(payload["source_frame_indices"], dtype=np.int64),
        idle=np.asarray(payload["idle"], dtype=np.bool_),
        feedrates=np.asarray(payload["feedrates"], dtype=np.float64),
        frame_count=int(payload["frame_count"]),
        command_count=int(payload["command_count"]),
        max_feedrate=float(payload["max_feedrate"]),
        warnings=[str(warning) for warning in payload.get("warnings", [])],
        metadata={"source": "MeshLib-style linear G-code path parser"},
    )


def _settings_payload(machine_settings: GcodeMachineSettings | dict[str, Any] | None) -> dict[str, Any] | None:
    if machine_settings is None:
        return None
    if isinstance(machine_settings, GcodeMachineSettings):
        return machine_settings.to_payload()
    return dict(machine_settings)


def parse_gcode_paths(
    source: str,
    *,
    machine_settings: GcodeMachineSettings | dict[str, Any] | None = None,
) -> GcodePathDocument:
    payload = rust.parse_gcode_paths(source, _settings_payload(machine_settings))
    result = None if payload is None else _result_from_payload(payload)
    return _require_rust(result, "parse_gcode_paths")


def load_gcode_source(path: str | Path) -> list[str]:
    source = rust.load_gcode_source(str(path))
    return _require_rust(source, "load_gcode_source")


def write_gcode_source(source: Sequence[str], path: str | Path) -> Path:
    output_path = Path(path)
    rust.write_gcode_source([str(frame) for frame in source], str(output_path))
    return output_path


def parse_gcode_file_paths(
    path: str | Path,
    *,
    machine_settings: GcodeMachineSettings | dict[str, Any] | None = None,
) -> GcodePathDocument:
    payload = rust.parse_gcode_file_paths(str(path), _settings_payload(machine_settings))
    result = None if payload is None else _result_from_payload(payload)
    result = _require_rust(result, "parse_gcode_file_paths")
    result.metadata["source"] = "MeshLib-style G-code file parser"
    result.metadata["source_path"] = str(path)
    return result
