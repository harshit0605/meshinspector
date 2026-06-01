use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_f32_values, read_faces, read_vertices};

#[pyfunction(signature = (values, threshold_mm = 0.6))]
fn summarize_thickness(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    threshold_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let summary =
        py.detach(|| zennah_geometry_core::summarize_thickness(&rust_values, threshold_mm));

    let output = PyDict::new(py);
    output.set_item("min_mm", summary.min_mm)?;
    output.set_item("avg_mm", summary.avg_mm)?;
    output.set_item("max_mm", summary.max_mm)?;
    output.set_item("valid_vertex_count", summary.valid_vertex_count)?;
    output.set_item("violation_count", summary.violation_count)?;
    Ok(output.unbind())
}

fn distance_summary_dict(
    py: Python<'_>,
    summary: zennah_geometry_core::DistanceSummary,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("min_mm", summary.min_mm)?;
    output.set_item("max_mm", summary.max_mm)?;
    output.set_item("mean_mm", summary.mean_mm)?;
    Ok(output.unbind())
}

#[pyfunction]
fn nearest_vertex_distances(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    target_vertices: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_target_vertices = read_vertices(target_vertices)?;
    let distances = py.detach(|| {
        zennah_geometry_core::nearest_vertex_distances(&rust_source_vertices, &rust_target_vertices)
    });
    Ok(distances.into_pyarray(py).unbind())
}

#[pyfunction]
fn nearest_surface_distances(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    target_vertices: PyReadonlyArray2<'_, f64>,
    target_faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_target_vertices = read_vertices(target_vertices)?;
    let rust_target_faces = read_faces(target_faces)?;
    let distances = py
        .detach(|| {
            zennah_geometry_core::nearest_surface_distances(
                &rust_source_vertices,
                &rust_target_vertices,
                &rust_target_faces,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(distances.into_pyarray(py).unbind())
}

#[pyfunction(signature = (
    source_vertices,
    target_vertices,
    target_faces,
    winding_threshold = 0.5,
    reject_self_intersections = true,
    max_self_intersection_faces = Some(50000),
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn signed_surface_distances(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    target_vertices: PyReadonlyArray2<'_, f64>,
    target_faces: PyReadonlyArray2<'_, i64>,
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_target_vertices = read_vertices(target_vertices)?;
    let rust_target_faces = read_faces(target_faces)?;
    let distances = py
        .detach(|| {
            zennah_geometry_core::signed_surface_distances(
                &rust_source_vertices,
                &rust_target_vertices,
                &rust_target_faces,
                winding_threshold,
                reject_self_intersections,
                max_self_intersection_faces,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(distances.into_pyarray(py).unbind())
}

#[pyfunction(signature = (
    source_vertices,
    target_vertices,
    target_faces,
    winding_threshold = 0.5,
    reject_self_intersections = true,
    max_self_intersection_faces = Some(50000),
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn version_compare_distances(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    target_vertices: PyReadonlyArray2<'_, f64>,
    target_faces: PyReadonlyArray2<'_, i64>,
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_target_vertices = read_vertices(target_vertices)?;
    let rust_target_faces = read_faces(target_faces)?;
    let distances = py
        .detach(|| {
            zennah_geometry_core::version_compare_distances(
                &rust_source_vertices,
                &rust_target_vertices,
                &rust_target_faces,
                zennah_geometry_core::SignedCompareOptions {
                    winding_threshold,
                    reject_self_intersections,
                    max_self_intersection_faces,
                    epsilon,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(distances.into_pyarray(py).unbind())
}

#[pyfunction]
fn compare_summary(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    target_vertices: PyReadonlyArray2<'_, f64>,
    target_faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_target_vertices = read_vertices(target_vertices)?;
    let rust_target_faces = read_faces(target_faces)?;
    let summary = py
        .detach(|| {
            zennah_geometry_core::compare_summary(
                &rust_source_vertices,
                &rust_target_vertices,
                &rust_target_faces,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    distance_summary_dict(py, summary)
}

#[pyfunction(signature = (
    source_vertices,
    target_vertices,
    target_faces,
    winding_threshold = 0.5,
    reject_self_intersections = true,
    max_self_intersection_faces = Some(50000),
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn signed_compare_summary(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    target_vertices: PyReadonlyArray2<'_, f64>,
    target_faces: PyReadonlyArray2<'_, i64>,
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_target_vertices = read_vertices(target_vertices)?;
    let rust_target_faces = read_faces(target_faces)?;
    let summary = py
        .detach(|| {
            zennah_geometry_core::signed_compare_summary(
                &rust_source_vertices,
                &rust_target_vertices,
                &rust_target_faces,
                winding_threshold,
                reject_self_intersections,
                max_self_intersection_faces,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    distance_summary_dict(py, summary)
}

#[pyfunction(signature = (
    source_vertices,
    source_faces,
    target_vertices,
    target_faces,
    winding_threshold = 0.5,
    reject_self_intersections = true,
    max_self_intersection_faces = Some(50000),
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn version_compare_summary(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    source_faces: PyReadonlyArray2<'_, i64>,
    target_vertices: PyReadonlyArray2<'_, f64>,
    target_faces: PyReadonlyArray2<'_, i64>,
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_source_faces = read_faces(source_faces)?;
    let rust_target_vertices = read_vertices(target_vertices)?;
    let rust_target_faces = read_faces(target_faces)?;
    let summary = py
        .detach(|| {
            zennah_geometry_core::version_compare_summary(
                &rust_source_vertices,
                &rust_source_faces,
                &rust_target_vertices,
                &rust_target_faces,
                zennah_geometry_core::SignedCompareOptions {
                    winding_threshold,
                    reject_self_intersections,
                    max_self_intersection_faces,
                    epsilon,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("volume_delta_mm3", summary.volume_delta_mm3)?;
    output.set_item("bbox_delta_mm", summary.bbox_delta_mm)?;
    output.set_item("min_signed_distance_mm", summary.min_signed_distance_mm)?;
    output.set_item("max_signed_distance_mm", summary.max_signed_distance_mm)?;
    output.set_item("mean_signed_distance_mm", summary.mean_signed_distance_mm)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(summarize_thickness, module)?)?;
    module.add_function(wrap_pyfunction!(nearest_vertex_distances, module)?)?;
    module.add_function(wrap_pyfunction!(nearest_surface_distances, module)?)?;
    module.add_function(wrap_pyfunction!(signed_surface_distances, module)?)?;
    module.add_function(wrap_pyfunction!(version_compare_distances, module)?)?;
    module.add_function(wrap_pyfunction!(compare_summary, module)?)?;
    module.add_function(wrap_pyfunction!(signed_compare_summary, module)?)?;
    module.add_function(wrap_pyfunction!(version_compare_summary, module)?)?;
    Ok(())
}
