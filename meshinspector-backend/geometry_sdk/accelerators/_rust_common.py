"""Shared Rust accelerator loading and mode policy."""

from __future__ import annotations

import os
from importlib import import_module

try:
    _rs = import_module("geometry_sdk._zennah_geometry_rs")
except ImportError:  # pragma: no cover - exercised through missing-extension behavior
    _rs = None

VALID_MODES = {"rust", "auto"}
SDF_BOOLEAN_OPERATIONS = {"union", "intersection", "difference"}
VOXEL_BINARY_OPERATIONS = {
    "union",
    "intersection",
    "difference",
    "max",
    "min",
    "sum",
    "multiply",
    "divide",
}
RAW_VOXEL_SCALAR_TYPES = {
    "uint8",
    "int8",
    "uint16",
    "int16",
    "uint32",
    "int32",
    "uint64",
    "int64",
    "float32",
    "float64",
    "float32_4",
}
BRUSH_OPERATION_CODES = {"thicken": 0, "scoop": 1, "smooth": 2}


def accelerator_mode() -> str:
    mode = os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower()
    if mode not in VALID_MODES:
        raise ValueError("GEOMETRY_SDK_ACCELERATOR must be one of: rust, auto")
    return "rust"


def available() -> bool:
    return _rs is not None


def backend_name() -> str:
    accelerator_mode()
    if _rs is None:
        raise RuntimeError("Rust geometry backend requires _zennah_geometry_rs, but it is not installed")
    return "rust"
