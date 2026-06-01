use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pyfunction]
fn material_densities_g_cm3(py: Python<'_>) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    for (material, density) in zennah_geometry_core::MATERIAL_DENSITIES_G_CM3 {
        output.set_item(material, density)?;
    }
    Ok(output.unbind())
}

#[pyfunction]
fn mm3_to_grams(volume_mm3: f64, material: &str) -> f64 {
    zennah_geometry_core::mm3_to_grams(volume_mm3, material)
}

#[pyfunction]
fn grams_to_mm3(weight_g: f64, material: &str) -> f64 {
    zennah_geometry_core::grams_to_mm3(weight_g, material)
}

#[pyfunction]
fn material_weight_table(py: Python<'_>, volume_mm3: f64) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    for (material, entry) in zennah_geometry_core::material_weight_table(volume_mm3) {
        let payload = PyDict::new(py);
        payload.set_item("volume_mm3", entry.volume_mm3)?;
        payload.set_item("weight_g", entry.weight_g)?;
        output.set_item(material, payload)?;
    }
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(material_densities_g_cm3, module)?)?;
    module.add_function(wrap_pyfunction!(mm3_to_grams, module)?)?;
    module.add_function(wrap_pyfunction!(grams_to_mm3, module)?)?;
    module.add_function(wrap_pyfunction!(material_weight_table, module)?)?;
    Ok(())
}
