use numpy::PyReadonlyArray2;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_faces, read_vertices};

#[pyfunction]
fn decimate_output_failures(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    source_faces: PyReadonlyArray2<'_, i64>,
    output_vertices: PyReadonlyArray2<'_, f64>,
    output_faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<String>> {
    let sv = read_vertices(source_vertices)?;
    let sf = read_faces(source_faces)?;
    let ov = read_vertices(output_vertices)?;
    let of = read_faces(output_faces)?;
    py.detach(|| zennah_geometry_core::decimate_output_failures(&sv, &sf, &ov, &of))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn hollow_output_failures(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    source_faces: PyReadonlyArray2<'_, i64>,
    output_vertices: PyReadonlyArray2<'_, f64>,
    output_faces: PyReadonlyArray2<'_, i64>,
    wall_thickness_mm: f64,
) -> PyResult<Vec<String>> {
    let sv = read_vertices(source_vertices)?;
    let sf = read_faces(source_faces)?;
    let ov = read_vertices(output_vertices)?;
    let of = read_faces(output_faces)?;
    py.detach(|| {
        zennah_geometry_core::hollow_output_failures(&sv, &sf, &ov, &of, wall_thickness_mm)
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn offset_shell_failures(
    py: Python<'_>,
    source_vertices: PyReadonlyArray2<'_, f64>,
    source_faces: PyReadonlyArray2<'_, i64>,
    output_vertices: PyReadonlyArray2<'_, f64>,
    output_faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<String>> {
    let sv = read_vertices(source_vertices)?;
    let sf = read_faces(source_faces)?;
    let ov = read_vertices(output_vertices)?;
    let of = read_faces(output_faces)?;
    py.detach(|| zennah_geometry_core::offset_shell_failures(&sv, &sf, &ov, &of))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn boolean_output_failures(
    py: Python<'_>,
    output_vertices: PyReadonlyArray2<'_, f64>,
    output_faces: PyReadonlyArray2<'_, i64>,
    operation: &str,
    source_volume_mm3: f64,
    target_volume_mm3: f64,
) -> PyResult<Vec<String>> {
    let ov = read_vertices(output_vertices)?;
    let of = read_faces(output_faces)?;
    py.detach(|| {
        zennah_geometry_core::boolean_output_failures(
            &ov,
            &of,
            operation,
            source_volume_mm3,
            target_volume_mm3,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(decimate_output_failures, module)?)?;
    module.add_function(wrap_pyfunction!(hollow_output_failures, module)?)?;
    module.add_function(wrap_pyfunction!(offset_shell_failures, module)?)?;
    module.add_function(wrap_pyfunction!(boolean_output_failures, module)?)?;
    Ok(())
}
