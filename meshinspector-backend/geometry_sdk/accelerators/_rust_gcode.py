from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_common as _common


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def parse_gcode_paths(source: str, settings: dict[str, Any] | None = None) -> dict[str, Any]:
    kernel = _require_rust_kernel("parse_gcode_paths")
    if settings is None:
        return kernel(str(source))
    return kernel(str(source), settings)


def load_gcode_source(path: str) -> list[str]:
    kernel = _require_rust_kernel("load_gcode_source")
    return [str(frame) for frame in kernel(str(path))]


def write_gcode_source(source: list[str], path: str) -> None:
    kernel = _require_rust_kernel("write_gcode_source")
    kernel([str(frame) for frame in source], str(path))


def parse_gcode_file_paths(path: str, settings: dict[str, Any] | None = None) -> dict[str, Any]:
    kernel = _require_rust_kernel("parse_gcode_file_paths")
    if settings is None:
        return kernel(str(path))
    return kernel(str(path), settings)


def gcode_machine_settings_to_meshlib_json(settings: dict[str, Any]) -> dict[str, Any]:
    kernel = _require_rust_kernel("gcode_machine_settings_to_meshlib_json")
    return dict(kernel(dict(settings)))


def gcode_machine_settings_from_meshlib_json(settings_json: dict[str, Any]) -> dict[str, Any]:
    kernel = _require_rust_kernel("gcode_machine_settings_from_meshlib_json")
    return dict(kernel(dict(settings_json)))
