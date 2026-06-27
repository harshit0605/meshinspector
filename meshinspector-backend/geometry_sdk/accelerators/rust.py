"""Stable Rust accelerator facade."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_exports as _exports
from geometry_sdk.accelerators._rust_exports import *  # noqa: F401,F403

_common = _exports._common

__all__ = [*_exports.__all__, "_common"]
