use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::read_faces;

#[pyfunction]
fn orient_faces_consistently(
    py: Python<'_>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::orient_faces_consistently(&rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let component_offsets: Vec<i64> = result
        .component_offsets
        .into_iter()
        .map(|value| value as i64)
        .collect();
    let component_faces: Vec<i64> = result
        .component_faces
        .into_iter()
        .map(|value| value as i64)
        .collect();
    let output = PyDict::new(py);
    output.set_item("faces", face_values.into_pyarray(py))?;
    output.set_item("component_offsets", component_offsets.into_pyarray(py))?;
    output.set_item("component_faces", component_faces.into_pyarray(py))?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(orient_faces_consistently, module)?)?;
    Ok(())
}
