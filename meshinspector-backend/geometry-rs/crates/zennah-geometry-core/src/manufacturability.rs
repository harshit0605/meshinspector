use crate::{
    detect_ring_regions, material_weight_table, measure_ring, mesh_health, mesh_stats,
    ray_thickness_at_vertices, summarize_thickness, GeometryError, ManufacturabilityReport,
    MeshHealth, RegionEntry, RingMeasurement, ThicknessSummary,
};

pub fn health_score(health: &MeshHealth) -> usize {
    let mut score = 100usize;
    if !health.is_closed {
        score = score.saturating_sub(30);
    }
    if let Some(self_intersections) = health.self_intersections {
        score = score.saturating_sub((self_intersections * 2).min(40));
    }
    if health.holes_count > 0 {
        score = score.saturating_sub((health.holes_count * 5).min(20));
    }
    if health.nonmanifold_edge_count > 0 {
        score = score.saturating_sub((health.nonmanifold_edge_count * 4).min(20));
    }
    score
}

pub fn build_recommendations(
    health: &MeshHealth,
    measurement: &RingMeasurement,
    thickness: &ThicknessSummary,
    regions: &[RegionEntry],
    threshold_mm: f64,
) -> Vec<String> {
    let mut recommendations = Vec::new();
    if !health.is_closed || health.holes_count > 0 {
        recommendations
            .push("Run auto repair before any hollowing or boolean operations.".to_string());
    }
    if health.self_intersections.is_some_and(|count| count > 0) {
        recommendations.push("Repair self-intersections before export.".to_string());
    }
    match thickness.min_mm {
        None => recommendations
            .push("Thickness analysis failed; inspect the mesh manually.".to_string()),
        Some(min_mm) if min_mm < threshold_mm => {
            recommendations.push("Fix thin regions before casting.".to_string());
        }
        Some(_) => {}
    }
    if measurement.needs_axis_confirmation {
        recommendations.push("Confirm the detected ring axis before resizing.".to_string());
    }
    let protected_violations: Vec<&str> = regions
        .iter()
        .filter(|region| region.protected_by_default && region.violation_count > 0)
        .map(|region| region.label.as_str())
        .collect();
    if !protected_violations.is_empty() {
        recommendations.push(format!(
            "Protected detail regions need attention: {}.",
            protected_violations.join(", ")
        ));
    }
    if recommendations.is_empty() {
        recommendations.push("Mesh is ready for guided manufacturing workflows.".to_string());
    }
    recommendations
}

pub fn compute_manufacturability_report(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    threshold_mm: f64,
) -> Result<ManufacturabilityReport, GeometryError> {
    let stats = mesh_stats(vertices, faces)?;
    let health = mesh_health(vertices, faces, true, Some(50_000), 1e-8)?;
    let measurement = measure_ring(vertices, None)?;
    let thickness_field = ray_thickness_at_vertices(vertices, faces, 1e-5)?;
    let thickness_values: Vec<f32> = thickness_field.iter().map(|value| *value as f32).collect();
    let thickness = summarize_thickness(&thickness_values, threshold_mm);
    let regions = detect_ring_regions(
        vertices,
        faces,
        measurement.ring_axis,
        Some(&thickness_values),
        threshold_mm,
    )?;
    let recommendations =
        build_recommendations(&health, &measurement, &thickness, &regions, threshold_mm);
    let export_ready = health.is_closed
        && health.self_intersections.unwrap_or(0) == 0
        && thickness
            .min_mm
            .is_some_and(|min_mm| min_mm >= threshold_mm);
    let material_weights = material_weight_table(stats.volume_mm3)
        .into_iter()
        .map(|(name, entry)| (name.to_string(), entry))
        .collect();
    let score = health_score(&health);

    Ok(ManufacturabilityReport {
        health,
        stats,
        ring_measurement: measurement,
        thickness,
        regions,
        material_weights,
        recommendations,
        export_ready,
        health_score: score,
    })
}
