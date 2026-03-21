"""Unit conversions and material constants."""

from typing import Dict

# Material densities in g/cm³
MATERIAL_DENSITIES: Dict[str, float] = {
    "gold_24k": 19.32,
    "gold_22k": 17.54,
    "gold_18k": 15.58,
    "gold_14k": 13.57,
    "gold_10k": 11.57,
    "silver_925": 10.36,
    "platinum": 21.45,
}

# US Ring Size to inner diameter (mm)
# Formula: circumference = 40 + (size * 2.55), diameter = circumference / π
RING_SIZE_CHART: Dict[float, float] = {
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


def get_ring_diameter(size: float) -> float:
    """Get ring inner diameter for a given US size."""
    if size in RING_SIZE_CHART:
        return RING_SIZE_CHART[size]
    # Interpolate for half sizes not in chart
    import math
    circumference = 40.0 + (size * 2.55)
    return circumference / math.pi


def mm3_to_grams(volume_mm3: float, material: str) -> float:
    """Convert volume in mm³ to weight in grams."""
    density = MATERIAL_DENSITIES.get(material, MATERIAL_DENSITIES["gold_18k"])
    volume_cm3 = volume_mm3 / 1000.0
    return volume_cm3 * density


def grams_to_mm3(weight_g: float, material: str) -> float:
    """Convert weight in grams to volume in mm³."""
    density = MATERIAL_DENSITIES.get(material, MATERIAL_DENSITIES["gold_18k"])
    volume_cm3 = weight_g / density
    return volume_cm3 * 1000.0
