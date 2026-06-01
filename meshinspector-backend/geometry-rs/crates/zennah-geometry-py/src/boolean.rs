use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

mod diagnostics;
mod near_stitch;
mod paired_coplanar;
mod prepared_base;
mod replay;
mod rewrite;

use diagnostics::exact_boolean_diagnostics_dict;

use crate::convert::{read_faces, read_vertices};

fn parse_exact_boolean_operation(
    operation: &str,
) -> PyResult<zennah_geometry_core::ExactBooleanOperation> {
    match operation {
        "union" => Ok(zennah_geometry_core::ExactBooleanOperation::Union),
        "intersection" => Ok(zennah_geometry_core::ExactBooleanOperation::Intersection),
        "difference" | "difference_ab" => {
            Ok(zennah_geometry_core::ExactBooleanOperation::DifferenceAB)
        }
        "difference_ba" => Ok(zennah_geometry_core::ExactBooleanOperation::DifferenceBA),
        "inside_a" => Ok(zennah_geometry_core::ExactBooleanOperation::InsideA),
        "inside_b" => Ok(zennah_geometry_core::ExactBooleanOperation::InsideB),
        "outside_a" => Ok(zennah_geometry_core::ExactBooleanOperation::OutsideA),
        "outside_b" => Ok(zennah_geometry_core::ExactBooleanOperation::OutsideB),
        _ => Err(PyValueError::new_err(
            "operation must be 'union', 'intersection', 'difference', 'difference_ab', \
             'difference_ba', 'inside_a', 'inside_b', 'outside_a', or 'outside_b'",
        )),
    }
}

#[pyfunction(signature = (
    first_vertices,
    first_faces,
    second_vertices,
    second_faces,
    operation,
    leaf_size = 16,
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn exact_boolean_mesh(
    py: Python<'_>,
    first_vertices: PyReadonlyArray2<'_, f64>,
    first_faces: PyReadonlyArray2<'_, i64>,
    second_vertices: PyReadonlyArray2<'_, f64>,
    second_faces: PyReadonlyArray2<'_, i64>,
    operation: &str,
    leaf_size: usize,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_first_vertices = read_vertices(first_vertices)?;
    let rust_first_faces = read_faces(first_faces)?;
    let rust_second_vertices = read_vertices(second_vertices)?;
    let rust_second_faces = read_faces(second_faces)?;
    let rust_operation = parse_exact_boolean_operation(operation)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::exact_boolean_from_meshes(
                &rust_first_vertices,
                &rust_first_faces,
                &rust_second_vertices,
                &rust_second_faces,
                rust_operation,
                leaf_size,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertices: Vec<f64> = result.assembly.vertices.iter().flatten().copied().collect();
    let faces: Vec<i64> = result.assembly.faces.iter().flatten().copied().collect();
    let diagnostics = exact_boolean_diagnostics_dict(py, &result)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("diagnostics", diagnostics)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(exact_boolean_mesh, module)?)?;
    Ok(())
}
