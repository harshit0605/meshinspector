"""Ring measurement compatibility wrapper for the Rust-owned SDK module."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import rust
from geometry_sdk.types import MeshDocument, RingMeasurement


RING_SIZE_CHART: dict[float, float] = {
    3.0: 14.05,
    3.5: 14.45,
    4.0: 14.86,
    4.5: 15.27,
    5.0: 15.67,
    5.5: 16.08,
    6.0: 16.48,
    6.5: 16.89,
    7.0: 17.30,
    7.5: 17.70,
    8.0: 18.11,
    8.5: 18.51,
    9.0: 18.92,
    9.5: 19.33,
    10.0: 19.73,
    10.5: 20.14,
    11.0: 20.54,
    11.5: 20.95,
    12.0: 21.35,
    12.5: 21.76,
    13.0: 22.16,
}


def ring_diameter_for_size(size: float) -> float:
    accelerated = rust.ring_diameter_for_size(size)
    return _require_rust(accelerated, "ring_diameter_for_size")


def closest_ring_size(inner_diameter_mm: float | None) -> float | None:
    if inner_diameter_mm is None:
        return None
    accelerated = rust.closest_ring_size(inner_diameter_mm)
    return _require_rust(accelerated, "closest_ring_size")


def measure_ring(mesh: MeshDocument, axis_override: Any = None) -> RingMeasurement:
    accelerated = rust.measure_ring(mesh, axis_override=axis_override)
    return _require_rust(accelerated, "measure_ring")


def _require_rust(value: Any, kernel_name: str) -> Any:
    if value is None:
        raise RuntimeError(
            f"Rust kernel {kernel_name} is required for geometry_sdk.jewelry.ring_measurement. "
            "Build the extension with `uv tool run maturin develop --manifest-path "
            "geometry-rs/crates/zennah-geometry-py/Cargo.toml`."
        )
    return value
