use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vertices};

fn mesh_health_dict(
    py: Python<'_>,
    health: zennah_geometry_core::MeshHealth,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("is_closed", health.is_closed)?;
    output.set_item("holes_count", health.holes_count)?;
    output.set_item("boundary_edge_count", health.boundary_edge_count)?;
    output.set_item("nonmanifold_edge_count", health.nonmanifold_edge_count)?;
    output.set_item("self_intersections", health.self_intersections)?;
    output.set_item(
        "self_intersections_available",
        health.self_intersections_available,
    )?;
    Ok(output.unbind())
}

fn mesh_stats_dict(py: Python<'_>, stats: zennah_geometry_core::MeshStats) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("bbox_min", stats.bbox_min.to_vec())?;
    output.set_item("bbox_max", stats.bbox_max.to_vec())?;
    output.set_item("bbox_size", stats.bbox_size.to_vec())?;
    output.set_item("surface_area_mm2", stats.surface_area_mm2)?;
    output.set_item("volume_mm3", stats.volume_mm3)?;
    output.set_item("vertex_count", stats.vertex_count)?;
    output.set_item("face_count", stats.face_count)?;
    output.set_item("connected_components", stats.connected_components)?;
    output.set_item("boundary_edge_count", stats.boundary_edge_count)?;
    Ok(output.unbind())
}

fn thickness_dict(
    py: Python<'_>,
    thickness: zennah_geometry_core::ThicknessSummary,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("min_mm", thickness.min_mm)?;
    output.set_item("avg_mm", thickness.avg_mm)?;
    output.set_item("max_mm", thickness.max_mm)?;
    output.set_item("valid_vertex_count", thickness.valid_vertex_count)?;
    output.set_item("violation_count", thickness.violation_count)?;
    Ok(output.unbind())
}

fn ring_measurement_dict(
    py: Python<'_>,
    measurement: zennah_geometry_core::RingMeasurement,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("ring_axis", measurement.ring_axis.to_vec())?;
    output.set_item("ring_axis_confidence", measurement.ring_axis_confidence)?;
    output.set_item("estimated_ring_size_us", measurement.estimated_ring_size_us)?;
    output.set_item("inner_diameter_mm", measurement.inner_diameter_mm)?;
    output.set_item("band_width_min_mm", measurement.band_width_min_mm)?;
    output.set_item("band_width_max_mm", measurement.band_width_max_mm)?;
    output.set_item("head_height_mm", measurement.head_height_mm)?;
    output.set_item("bbox_mm", measurement.bbox_mm.to_vec())?;
    output.set_item(
        "needs_axis_confirmation",
        measurement.needs_axis_confirmation,
    )?;
    Ok(output.unbind())
}

fn region_dict(py: Python<'_>, region: zennah_geometry_core::RegionEntry) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("region_id", region.region_id)?;
    output.set_item("label", region.label)?;
    output.set_item("vertex_indices", region.vertex_indices.into_pyarray(py))?;
    output.set_item("coverage_pct", region.coverage_pct)?;
    output.set_item("protected_by_default", region.protected_by_default)?;
    output.set_item("allowed_operations", region.allowed_operations)?;
    output.set_item("min_thickness_mm", region.min_thickness_mm)?;
    output.set_item("avg_thickness_mm", region.avg_thickness_mm)?;
    output.set_item("violation_count", region.violation_count)?;
    output.set_item(
        "centroid_mm",
        region.centroid_mm.map(|centroid| centroid.to_vec()),
    )?;
    Ok(output.unbind())
}

fn material_weights_dict(
    py: Python<'_>,
    weights: Vec<(String, zennah_geometry_core::MaterialWeightEntry)>,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    for (material, entry) in weights {
        let payload = PyDict::new(py);
        payload.set_item("volume_mm3", entry.volume_mm3)?;
        payload.set_item("weight_g", entry.weight_g)?;
        output.set_item(material, payload)?;
    }
    Ok(output.unbind())
}

fn report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::ManufacturabilityReport,
) -> PyResult<Py<PyDict>> {
    let regions: PyResult<Vec<Py<PyDict>>> = report
        .regions
        .into_iter()
        .map(|region| region_dict(py, region))
        .collect();
    let output = PyDict::new(py);
    output.set_item("health", mesh_health_dict(py, report.health)?)?;
    output.set_item("stats", mesh_stats_dict(py, report.stats)?)?;
    output.set_item(
        "ring_measurement",
        ring_measurement_dict(py, report.ring_measurement)?,
    )?;
    output.set_item("thickness", thickness_dict(py, report.thickness)?)?;
    output.set_item("regions", regions?)?;
    output.set_item(
        "material_weights",
        material_weights_dict(py, report.material_weights)?,
    )?;
    output.set_item("recommendations", report.recommendations)?;
    output.set_item("export_ready", report.export_ready)?;
    output.set_item("health_score", report.health_score)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    is_closed,
    holes_count,
    boundary_edge_count,
    nonmanifold_edge_count,
    self_intersections = None,
    self_intersections_available = true
))]
fn health_score(
    is_closed: bool,
    holes_count: usize,
    boundary_edge_count: usize,
    nonmanifold_edge_count: usize,
    self_intersections: Option<usize>,
    self_intersections_available: bool,
) -> usize {
    zennah_geometry_core::health_score(&zennah_geometry_core::MeshHealth {
        is_closed,
        holes_count,
        boundary_edge_count,
        nonmanifold_edge_count,
        self_intersections,
        self_intersections_available,
    })
}

#[pyfunction(signature = (
    is_closed,
    holes_count,
    boundary_edge_count,
    nonmanifold_edge_count,
    self_intersections,
    needs_axis_confirmation,
    min_thickness_mm,
    protected_violation_labels,
    threshold_mm = 0.6
))]
#[allow(clippy::too_many_arguments)]
fn build_recommendations(
    is_closed: bool,
    holes_count: usize,
    boundary_edge_count: usize,
    nonmanifold_edge_count: usize,
    self_intersections: Option<usize>,
    needs_axis_confirmation: bool,
    min_thickness_mm: Option<f64>,
    protected_violation_labels: Vec<String>,
    threshold_mm: f64,
) -> Vec<String> {
    let health = zennah_geometry_core::MeshHealth {
        is_closed,
        holes_count,
        boundary_edge_count,
        nonmanifold_edge_count,
        self_intersections,
        self_intersections_available: true,
    };
    let measurement = zennah_geometry_core::RingMeasurement {
        ring_axis: [0.0, 1.0, 0.0],
        ring_axis_confidence: 1.0,
        estimated_ring_size_us: None,
        inner_diameter_mm: None,
        band_width_min_mm: None,
        band_width_max_mm: None,
        head_height_mm: None,
        bbox_mm: [0.0, 0.0, 0.0],
        needs_axis_confirmation,
    };
    let thickness = zennah_geometry_core::ThicknessSummary {
        min_mm: min_thickness_mm,
        avg_mm: None,
        max_mm: None,
        valid_vertex_count: usize::from(min_thickness_mm.is_some()),
        violation_count: usize::from(min_thickness_mm.is_some_and(|value| value < threshold_mm)),
    };
    let regions: Vec<zennah_geometry_core::RegionEntry> = protected_violation_labels
        .into_iter()
        .map(|label| zennah_geometry_core::RegionEntry {
            region_id: label.clone(),
            label,
            vertex_indices: Vec::new(),
            coverage_pct: 0.0,
            protected_by_default: true,
            allowed_operations: Vec::new(),
            min_thickness_mm: None,
            avg_thickness_mm: None,
            violation_count: 1,
            centroid_mm: None,
        })
        .collect();
    zennah_geometry_core::build_recommendations(
        &health,
        &measurement,
        &thickness,
        &regions,
        threshold_mm,
    )
}

#[pyfunction(signature = (vertices, faces, threshold_mm = 0.6))]
fn compute_manufacturability_report(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    threshold_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let report = py
        .detach(|| {
            zennah_geometry_core::compute_manufacturability_report(
                &rust_vertices,
                &rust_faces,
                threshold_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    report_dict(py, report)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(health_score, module)?)?;
    module.add_function(wrap_pyfunction!(build_recommendations, module)?)?;
    module.add_function(wrap_pyfunction!(compute_manufacturability_report, module)?)?;
    Ok(())
}
