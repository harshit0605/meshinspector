use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_i64_values, read_vertices};

#[pyfunction]
fn nearest_distances_to_indices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    target_indices: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_target_indices = read_i64_values(target_indices);
    let distances = py
        .detach(|| {
            zennah_geometry_core::nearest_distances_to_indices(&rust_vertices, &rust_target_indices)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(distances.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(nearest_distances_to_indices, module)?)?;
    Ok(())
}
