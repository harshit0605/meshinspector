use numpy::PyReadonlyArray1;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::read_f32_values;

#[pyfunction(signature = (
    values,
    overlay_type,
    center_value,
    threshold_mm = None,
    max_abs_value = 1_000_000.0
))]
fn scalar_overlay_payload(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    overlay_type: &str,
    center_value: f64,
    threshold_mm: Option<f64>,
    max_abs_value: f64,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let payload = py.detach(|| {
        zennah_geometry_core::scalar_overlay_payload(
            &rust_values,
            overlay_type,
            center_value,
            threshold_mm,
            max_abs_value,
        )
    });

    let output = PyDict::new(py);
    output.set_item("overlay_type", payload.overlay_type)?;
    output.set_item("values", payload.values)?;
    output.set_item("min_value", payload.min_value)?;
    output.set_item("max_value", payload.max_value)?;
    output.set_item("center_value", payload.center_value)?;
    output.set_item("threshold_mm", payload.threshold_mm)?;
    output.set_item("max_abs_value", payload.max_abs_value)?;
    output.set_item("mean_value", payload.mean_value)?;
    output.set_item("valid_count", payload.valid_count)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(scalar_overlay_payload, module)?)?;
    Ok(())
}
