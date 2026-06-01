use crate::types::MaterialWeightEntry;

pub const DEFAULT_MATERIAL: &str = "gold_18k";

pub const MATERIAL_DENSITIES_G_CM3: [(&str, f64); 7] = [
    ("gold_24k", 19.32),
    ("gold_22k", 17.54),
    ("gold_18k", 15.58),
    ("gold_14k", 13.57),
    ("gold_10k", 11.57),
    ("silver_925", 10.36),
    ("platinum", 21.45),
];

pub fn material_density_g_cm3(material: &str) -> f64 {
    MATERIAL_DENSITIES_G_CM3
        .iter()
        .find_map(|(name, density)| (*name == material).then_some(*density))
        .unwrap_or_else(|| material_density_g_cm3(DEFAULT_MATERIAL))
}

pub fn mm3_to_grams(volume_mm3: f64, material: &str) -> f64 {
    (volume_mm3 / 1000.0) * material_density_g_cm3(material)
}

pub fn grams_to_mm3(weight_g: f64, material: &str) -> f64 {
    (weight_g / material_density_g_cm3(material)) * 1000.0
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round_ties_even() / 1000.0
}

pub fn material_weight_table(volume_mm3: f64) -> Vec<(&'static str, MaterialWeightEntry)> {
    MATERIAL_DENSITIES_G_CM3
        .iter()
        .map(|(material, _)| {
            (
                *material,
                MaterialWeightEntry {
                    volume_mm3: round3(volume_mm3),
                    weight_g: round3(mm3_to_grams(volume_mm3, material)),
                },
            )
        })
        .collect()
}
