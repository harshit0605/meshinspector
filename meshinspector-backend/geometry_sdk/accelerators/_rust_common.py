"""Shared Rust accelerator loading and mode policy."""

from __future__ import annotations

import os
from importlib import import_module

try:
    _rs = import_module("geometry_sdk._zennah_geometry_rs")
except ImportError:  # pragma: no cover - exercised through fallback behavior
    _rs = None

VALID_MODES = {"python", "rust", "auto"}
SDF_BOOLEAN_OPERATIONS = {"union", "intersection", "difference"}
BRUSH_OPERATION_CODES = {"thicken": 0, "scoop": 1, "smooth": 2}


def accelerator_mode() -> str:
    mode = os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower()
    if mode not in VALID_MODES:
        raise ValueError("GEOMETRY_SDK_ACCELERATOR must be one of: python, rust, auto")
    return mode


def available() -> bool:
    return _rs is not None


def backend_name() -> str:
    mode = accelerator_mode()
    if mode == "python":
        return "python"
    if _rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return "python"
    return "rust"
