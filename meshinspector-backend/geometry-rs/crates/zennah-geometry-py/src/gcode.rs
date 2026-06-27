mod settings;

use numpy::IntoPyArray;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use settings::machine_settings_from_dict;

#[pyfunction]
#[pyo3(signature = (source, settings=None))]
fn parse_gcode_paths(
    py: Python<'_>,
    source: &str,
    settings: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyDict>> {
    let machine_settings = settings.map(machine_settings_from_dict).transpose()?;
    let parsed = py
        .detach(|| match machine_settings {
            Some(settings) => {
                zennah_geometry_core::gcode::parse_gcode_paths_with_settings(source, &settings)
            }
            None => zennah_geometry_core::gcode::parse_gcode_paths(source),
        })
        .map_err(PyValueError::new_err)?;
    gcode_result_to_dict(py, parsed)
}

#[pyfunction]
fn load_gcode_source(py: Python<'_>, path: &str) -> PyResult<Vec<String>> {
    py.detach(|| zennah_geometry_core::gcode::load_gcode_source(path))
        .map_err(PyValueError::new_err)
}

#[pyfunction]
fn write_gcode_source(py: Python<'_>, source: Vec<String>, path: &str) -> PyResult<()> {
    py.detach(|| zennah_geometry_core::gcode::write_gcode_source(&source, path))
        .map_err(PyValueError::new_err)
}

#[pyfunction]
#[pyo3(signature = (path, settings=None))]
fn parse_gcode_file_paths(
    py: Python<'_>,
    path: &str,
    settings: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyDict>> {
    let machine_settings = settings.map(machine_settings_from_dict).transpose()?;
    let parsed = py
        .detach(|| match machine_settings {
            Some(settings) => {
                zennah_geometry_core::gcode::parse_gcode_file_paths_with_settings(path, &settings)
            }
            None => zennah_geometry_core::gcode::parse_gcode_file_paths(path),
        })
        .map_err(PyValueError::new_err)?;
    gcode_result_to_dict(py, parsed)
}

fn gcode_result_to_dict(
    py: Python<'_>,
    parsed: zennah_geometry_core::gcode::GcodePathResult,
) -> PyResult<Py<PyDict>> {
    let segments = parsed
        .segments
        .iter()
        .flat_map(|segment| segment.start.into_iter().chain(segment.end))
        .collect::<Vec<_>>();
    let tool_directions = parsed
        .segments
        .iter()
        .flat_map(|segment| {
            segment
                .tool_direction_start
                .into_iter()
                .chain(segment.tool_direction_end)
        })
        .collect::<Vec<_>>();
    let source_frame_indices = parsed
        .segments
        .iter()
        .map(|segment| segment.source_frame_index as i64)
        .collect::<Vec<_>>();
    let idle = parsed
        .segments
        .iter()
        .map(|segment| segment.idle)
        .collect::<Vec<_>>();
    let feedrates = parsed
        .segments
        .iter()
        .map(|segment| segment.feedrate)
        .collect::<Vec<_>>();

    let output = PyDict::new(py);
    output.set_item("segments", segments.into_pyarray(py))?;
    output.set_item("tool_directions", tool_directions.into_pyarray(py))?;
    output.set_item(
        "source_frame_indices",
        source_frame_indices.into_pyarray(py),
    )?;
    output.set_item("idle", idle.into_pyarray(py))?;
    output.set_item("feedrates", feedrates.into_pyarray(py))?;
    output.set_item("frame_count", parsed.frame_count)?;
    output.set_item("command_count", parsed.command_count)?;
    output.set_item("max_feedrate", parsed.max_feedrate)?;
    output.set_item("warnings", parsed.warnings)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(parse_gcode_paths, module)?)?;
    module.add_function(wrap_pyfunction!(load_gcode_source, module)?)?;
    module.add_function(wrap_pyfunction!(write_gcode_source, module)?)?;
    module.add_function(wrap_pyfunction!(parse_gcode_file_paths, module)?)?;
    settings::register(module)?;
    Ok(())
}
