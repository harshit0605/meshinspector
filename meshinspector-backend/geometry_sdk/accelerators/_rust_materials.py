from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MaterialWeightEntry


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def material_densities_g_cm3() -> dict[str, float]:
    kernel = _require_rust_kernel("material_densities_g_cm3")
    return {str(material): float(density) for material, density in kernel().items()}


def mm3_to_grams(volume_mm3: float, material: str = "gold_18k") -> float:
    kernel = _require_rust_kernel("mm3_to_grams")
    return float(kernel(float(volume_mm3), str(material)))


def grams_to_mm3(weight_g: float, material: str = "gold_18k") -> float:
    kernel = _require_rust_kernel("grams_to_mm3")
    return float(kernel(float(weight_g), str(material)))


def material_weight_table(volume_mm3: float) -> dict[str, MaterialWeightEntry]:
    kernel = _require_rust_kernel("material_weight_table")
    payload: dict[str, dict[str, Any]] = kernel(float(volume_mm3))
    return {
        str(material): MaterialWeightEntry(
            volume_mm3=float(entry["volume_mm3"]),
            weight_g=float(entry["weight_g"]),
        )
        for material, entry in payload.items()
    }
