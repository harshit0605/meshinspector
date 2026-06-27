pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(
        gcode_machine_settings_to_meshlib_json,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        gcode_machine_settings_from_meshlib_json,
        module
    )?)?;
    Ok(())
}
