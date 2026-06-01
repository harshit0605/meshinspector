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


def nearest_distances(vertices: Any, target_indices: Any) -> np.ndarray:
    kernel = _require_rust_kernel("nearest_distances_to_indices")
    distances = kernel(
        np.asarray(vertices, dtype=np.float64),
        np.asarray(target_indices, dtype=np.int64).reshape(-1),
    )
    return np.asarray(distances, dtype=np.float64).reshape(-1)
