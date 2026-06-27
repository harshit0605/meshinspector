from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def point_cloud_from_ply(source: bytes | bytearray) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_from_ply")
    return kernel(bytes(source))


def point_cloud_to_ply(points: np.ndarray, *, normals: np.ndarray | None = None, colors: np.ndarray | None = None) -> bytes | None:
    kernel = _require_rust_kernel("point_cloud_to_ply")
    return bytes(
        kernel(
            np.asarray(points, dtype=np.float64),
            None if normals is None else np.asarray(normals, dtype=np.float64),
            None if colors is None else np.asarray(colors, dtype=np.uint8),
        )
    )
