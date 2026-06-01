use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::convert::{read_f32_values, read_faces, read_vec3, read_vertices};

#[pyfunction]
fn ring_diameter_for_size(size: f64) -> f64 {
    zennah_geometry_core::ring_diameter_for_size(size)
}

#[pyfunction]
fn closest_ring_size(inner_diameter_mm: Option<f64>) -> Option<f64> {
    zennah_geometry_core::closest_ring_size(inner_diameter_mm)
}

#[pyfunction(signature = (vertices, axis_override = None))]
fn measure_ring(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    axis_override: Option<PyReadonlyArray1<'_, f64>>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_axis_override = axis_override
        .map(|axis| read_vec3("axis_override", axis))
        .transpose()?;
    let measurement = py
        .detach(|| zennah_geometry_core::measure_ring(&rust_vertices, rust_axis_override))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    output.set_item("ring_axis", measurement.ring_axis)?;
    output.set_item("ring_axis_confidence", measurement.ring_axis_confidence)?;
    output.set_item("estimated_ring_size_us", measurement.estimated_ring_size_us)?;
    output.set_item("inner_diameter_mm", measurement.inner_diameter_mm)?;
    output.set_item("band_width_min_mm", measurement.band_width_min_mm)?;
    output.set_item("band_width_max_mm", measurement.band_width_max_mm)?;
    output.set_item("head_height_mm", measurement.head_height_mm)?;
    output.set_item("bbox_mm", measurement.bbox_mm)?;
    output.set_item(
        "needs_axis_confirmation",
        measurement.needs_axis_confirmation,
    )?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, ring_axis, thickness = None, threshold_mm = 0.6))]
fn detect_ring_regions(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    ring_axis: PyReadonlyArray1<'_, f64>,
    thickness: Option<PyReadonlyArray1<'_, f32>>,
    threshold_mm: f64,
) -> PyResult<Py<PyList>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_ring_axis = read_vec3("ring_axis", ring_axis)?;
    let rust_thickness = thickness.map(read_f32_values);
    let regions = py
        .detach(|| {
            zennah_geometry_core::detect_ring_regions(
                &rust_vertices,
                &rust_faces,
                rust_ring_axis,
                rust_thickness.as_deref(),
                threshold_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyList::empty(py);
    for region in regions {
        let item = PyDict::new(py);
        item.set_item("region_id", region.region_id)?;
        item.set_item("label", region.label)?;
        item.set_item("vertex_indices", region.vertex_indices.into_pyarray(py))?;
        item.set_item("coverage_pct", region.coverage_pct)?;
        item.set_item("protected_by_default", region.protected_by_default)?;
        item.set_item("allowed_operations", region.allowed_operations)?;
        item.set_item("min_thickness_mm", region.min_thickness_mm)?;
        item.set_item("avg_thickness_mm", region.avg_thickness_mm)?;
        item.set_item("violation_count", region.violation_count)?;
        item.set_item("centroid_mm", region.centroid_mm)?;
        output.append(item)?;
    }
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(ring_diameter_for_size, module)?)?;
    module.add_function(wrap_pyfunction!(closest_ring_size, module)?)?;
    module.add_function(wrap_pyfunction!(measure_ring, module)?)?;
    module.add_function(wrap_pyfunction!(detect_ring_regions, module)?)?;
    Ok(())
}
