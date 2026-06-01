use numpy::{IntoPyArray, PyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vertices};

#[pyfunction(signature = (
    source_vertices,
    source_faces,
    other_vertices,
    winding_threshold = 0.5,
    reject_self_intersections = true,
    max_self_intersection_faces = Some(50000),
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn service_compare_distances(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    source_faces: PyReadonlyArray2<'_, i64>,
    other_vertices: PyReadonlyArray2<'_, f64>,
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_source_faces = read_faces(source_faces)?;
    let rust_other_vertices = read_vertices(other_vertices)?;
    let distances = py
        .detach(|| {
            zennah_geometry_core::service_compare_distances(
                &rust_source_vertices,
                &rust_source_faces,
                &rust_other_vertices,
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

#[pyfunction(signature = (
    source_vertices,
    source_faces,
    other_vertices,
    other_faces,
    winding_threshold = 0.5,
    reject_self_intersections = true,
    max_self_intersection_faces = Some(50000),
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn service_compare_summary(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    source_faces: PyReadonlyArray2<'_, i64>,
    other_vertices: PyReadonlyArray2<'_, f64>,
    other_faces: PyReadonlyArray2<'_, i64>,
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_source_vertices = read_vertices(source_vertices)?;
    let rust_source_faces = read_faces(source_faces)?;
    let rust_other_vertices = read_vertices(other_vertices)?;
    let rust_other_faces = read_faces(other_faces)?;
    let summary = py
        .detach(|| {
            zennah_geometry_core::service_compare_summary(
                &rust_source_vertices,
                &rust_source_faces,
                &rust_other_vertices,
                &rust_other_faces,
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
    module.add_function(wrap_pyfunction!(service_compare_distances, module)?)?;
    module.add_function(wrap_pyfunction!(service_compare_summary, module)?)?;
    Ok(())
}
