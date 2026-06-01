"""Material constants and weight conversion compatibility wrappers."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_materials
from geometry_sdk.types import MaterialName, MaterialWeightEntry


MATERIAL_DENSITIES_G_CM3: dict[MaterialName, float] = _rust_materials.material_densities_g_cm3()  # type: ignore[assignment]


def mm3_to_grams(volume_mm3: float, material: str = "gold_18k") -> float:
    return _rust_materials.mm3_to_grams(volume_mm3, material)


def grams_to_mm3(weight_g: float, material: str = "gold_18k") -> float:
    return _rust_materials.grams_to_mm3(weight_g, material)


def material_weight_table(volume_mm3: float) -> dict[str, MaterialWeightEntry]:
    return _rust_materials.material_weight_table(volume_mm3)
