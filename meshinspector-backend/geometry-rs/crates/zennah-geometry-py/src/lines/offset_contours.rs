use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

mod helpers;

use helpers::{
    call_z_callback, contours_result_to_dict, parse_relax_iterations, parse_z_options,
    read_contour_offsets,
};

#[pyfunction(signature = (contour_points, contour_offsets, offset, min_angle_precision, mode = "offset", end_type = "round", corner_type = "round", max_sharp_angle = std::f64::consts::PI * 2.0 / 3.0, z_restore = "default", z_value = None, relax_iterations = 1, z_values = None, z_value_offsets = None, z_callback = None))]
fn offset_contours(
    py: Python<'_>,
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
    offset: f64,
    min_angle_precision: f64,
    mode: &str,
    end_type: &str,
    corner_type: &str,
    max_sharp_angle: f64,
    z_restore: &str,
    z_value: Option<f64>,
    relax_iterations: i64,
    z_values: Option<PyReadonlyArray1<'_, f64>>,
    z_value_offsets: Option<PyReadonlyArray1<'_, i64>>,
    z_callback: Option<Py<PyAny>>,
) -> PyResult<Py<pyo3::types::PyDict>> {
    let contours = super::read_contours3(contour_points, contour_offsets)?;
    let mode = zennah_geometry_core::lines::OffsetContoursMode::from_str(mode)
        .map_err(PyValueError::new_err)?;
    let end_type = zennah_geometry_core::lines::OffsetContoursEndType::from_str(end_type)
        .map_err(PyValueError::new_err)?;
    let corner_type = zennah_geometry_core::lines::OffsetContoursCornerType::from_str(corner_type)
        .map_err(PyValueError::new_err)?;
    let options = zennah_geometry_core::lines::OffsetContoursOptions {
        mode,
        end_type,
        corner_type,
        min_angle_precision,
        max_sharp_angle,
    };
    let contours = if let Some(callback) = z_callback {
        if z_values.is_some() || z_value_offsets.is_some() {
            return Err(PyValueError::new_err(
                "z_values and z_value_offsets are not accepted when z_callback is provided",
            ));
        }
        zennah_geometry_core::lines::offset_contours_with_options_and_z_callback(
            &contours,
            offset,
            options,
            parse_relax_iterations(relax_iterations),
            |offset_contours, offset_index, origin| {
                call_z_callback(py, &callback, offset_contours, offset_index, origin)
            },
        )
        .map_err(PyValueError::new_err)?
    } else {
        let z_options = parse_z_options(
            z_restore,
            z_value,
            z_values,
            z_value_offsets,
            relax_iterations,
        )?;
        py.detach(|| {
            zennah_geometry_core::lines::offset_contours_with_options_and_z_options(
                &contours, offset, options, z_options,
            )
        })
        .map_err(PyValueError::new_err)?
    };
    super::contours_to_dict(py, contours)
}

#[pyfunction(signature = (contour_points, contour_offsets, offset, min_angle_precision, mode = "offset", end_type = "round", corner_type = "round", max_sharp_angle = std::f64::consts::PI * 2.0 / 3.0, z_restore = "default", z_value = None, relax_iterations = 1, z_values = None, z_value_offsets = None, z_callback = None))]
fn offset_contours_with_origins(
    py: Python<'_>,
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
    offset: f64,
    min_angle_precision: f64,
    mode: &str,
    end_type: &str,
    corner_type: &str,
    max_sharp_angle: f64,
    z_restore: &str,
    z_value: Option<f64>,
    relax_iterations: i64,
    z_values: Option<PyReadonlyArray1<'_, f64>>,
    z_value_offsets: Option<PyReadonlyArray1<'_, i64>>,
    z_callback: Option<Py<PyAny>>,
) -> PyResult<Py<PyDict>> {
    let contours = super::read_contours3(contour_points, contour_offsets)?;
    let mode = zennah_geometry_core::lines::OffsetContoursMode::from_str(mode)
        .map_err(PyValueError::new_err)?;
    let end_type = zennah_geometry_core::lines::OffsetContoursEndType::from_str(end_type)
        .map_err(PyValueError::new_err)?;
    let corner_type = zennah_geometry_core::lines::OffsetContoursCornerType::from_str(corner_type)
        .map_err(PyValueError::new_err)?;
    let options = zennah_geometry_core::lines::OffsetContoursOptions {
        mode,
        end_type,
        corner_type,
        min_angle_precision,
        max_sharp_angle,
    };
    let result = if let Some(callback) = z_callback {
        if z_values.is_some() || z_value_offsets.is_some() {
            return Err(PyValueError::new_err(
                "z_values and z_value_offsets are not accepted when z_callback is provided",
            ));
        }
        zennah_geometry_core::lines::offset_contours_with_options_and_origins_and_z_callback(
            &contours,
            offset,
            options,
            parse_relax_iterations(relax_iterations),
            |offset_contours, offset_index, origin| {
                call_z_callback(py, &callback, offset_contours, offset_index, origin)
            },
        )
        .map_err(PyValueError::new_err)?
    } else {
        let z_options = parse_z_options(
            z_restore,
            z_value,
            z_values,
            z_value_offsets,
            relax_iterations,
        )?;
        py.detach(|| {
            zennah_geometry_core::lines::offset_contours_with_options_and_origins_and_z_options(
                &contours, offset, options, z_options,
            )
        })
        .map_err(PyValueError::new_err)?
    };
    contours_result_to_dict(py, result)
}

#[pyfunction(signature = (contour_points, contour_offsets, point_offsets, offset_offsets, min_angle_precision, mode = "offset", end_type = "round", corner_type = "round", max_sharp_angle = std::f64::consts::PI * 2.0 / 3.0, z_restore = "default", z_value = None, relax_iterations = 1, z_values = None, z_value_offsets = None, z_callback = None))]
fn offset_contours_variable(
    py: Python<'_>,
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
    point_offsets: PyReadonlyArray1<'_, f64>,
    offset_offsets: PyReadonlyArray1<'_, i64>,
    min_angle_precision: f64,
    mode: &str,
    end_type: &str,
    corner_type: &str,
    max_sharp_angle: f64,
    z_restore: &str,
    z_value: Option<f64>,
    relax_iterations: i64,
    z_values: Option<PyReadonlyArray1<'_, f64>>,
    z_value_offsets: Option<PyReadonlyArray1<'_, i64>>,
    z_callback: Option<Py<PyAny>>,
) -> PyResult<Py<pyo3::types::PyDict>> {
    let contours = super::read_contours3(contour_points, contour_offsets)?;
    let offsets = read_contour_offsets(point_offsets, offset_offsets)?;
    let mode = zennah_geometry_core::lines::OffsetContoursMode::from_str(mode)
        .map_err(PyValueError::new_err)?;
    let end_type = zennah_geometry_core::lines::OffsetContoursEndType::from_str(end_type)
        .map_err(PyValueError::new_err)?;
    let corner_type = zennah_geometry_core::lines::OffsetContoursCornerType::from_str(corner_type)
        .map_err(PyValueError::new_err)?;
    let options = zennah_geometry_core::lines::OffsetContoursOptions {
        mode,
        end_type,
        corner_type,
        min_angle_precision,
        max_sharp_angle,
    };
    let contours = if let Some(callback) = z_callback {
        if z_values.is_some() || z_value_offsets.is_some() {
            return Err(PyValueError::new_err(
                "z_values and z_value_offsets are not accepted when z_callback is provided",
            ));
        }
        zennah_geometry_core::lines::offset_contours_with_variable_offsets_and_z_callback(
            &contours,
            &offsets,
            options,
            parse_relax_iterations(relax_iterations),
            |offset_contours, offset_index, origin| {
                call_z_callback(py, &callback, offset_contours, offset_index, origin)
            },
        )
        .map_err(PyValueError::new_err)?
    } else {
        let z_options = parse_z_options(
            z_restore,
            z_value,
            z_values,
            z_value_offsets,
            relax_iterations,
        )?;
        py.detach(|| {
            zennah_geometry_core::lines::offset_contours_with_variable_offsets_and_z_options(
                &contours, &offsets, options, z_options,
            )
        })
        .map_err(PyValueError::new_err)?
    };
    super::contours_to_dict(py, contours)
}

#[pyfunction(signature = (contour_points, contour_offsets, point_offsets, offset_offsets, min_angle_precision, mode = "offset", end_type = "round", corner_type = "round", max_sharp_angle = std::f64::consts::PI * 2.0 / 3.0, z_restore = "default", z_value = None, relax_iterations = 1, z_values = None, z_value_offsets = None, z_callback = None))]
fn offset_contours_variable_with_origins(
    py: Python<'_>,
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
    point_offsets: PyReadonlyArray1<'_, f64>,
    offset_offsets: PyReadonlyArray1<'_, i64>,
    min_angle_precision: f64,
    mode: &str,
    end_type: &str,
    corner_type: &str,
    max_sharp_angle: f64,
    z_restore: &str,
    z_value: Option<f64>,
    relax_iterations: i64,
    z_values: Option<PyReadonlyArray1<'_, f64>>,
    z_value_offsets: Option<PyReadonlyArray1<'_, i64>>,
    z_callback: Option<Py<PyAny>>,
) -> PyResult<Py<PyDict>> {
    let contours = super::read_contours3(contour_points, contour_offsets)?;
    let offsets = read_contour_offsets(point_offsets, offset_offsets)?;
    let mode = zennah_geometry_core::lines::OffsetContoursMode::from_str(mode)
        .map_err(PyValueError::new_err)?;
    let end_type = zennah_geometry_core::lines::OffsetContoursEndType::from_str(end_type)
        .map_err(PyValueError::new_err)?;
    let corner_type = zennah_geometry_core::lines::OffsetContoursCornerType::from_str(corner_type)
        .map_err(PyValueError::new_err)?;
    let options = zennah_geometry_core::lines::OffsetContoursOptions {
        mode,
        end_type,
        corner_type,
        min_angle_precision,
        max_sharp_angle,
    };
    let result = if let Some(callback) = z_callback {
        if z_values.is_some() || z_value_offsets.is_some() {
            return Err(PyValueError::new_err(
                "z_values and z_value_offsets are not accepted when z_callback is provided",
            ));
        }
        zennah_geometry_core::lines::offset_contours_with_variable_offsets_and_origins_and_z_callback(
            &contours,
            &offsets,
            options,
            parse_relax_iterations(relax_iterations),
            |offset_contours, offset_index, origin| {
                call_z_callback(py, &callback, offset_contours, offset_index, origin)
            },
        )
        .map_err(PyValueError::new_err)?
    } else {
        let z_options = parse_z_options(
            z_restore,
            z_value,
            z_values,
            z_value_offsets,
            relax_iterations,
        )?;
        py.detach(|| {
            zennah_geometry_core::lines::offset_contours_with_variable_offsets_and_origins_and_z_options(
                &contours, &offsets, options, z_options,
            )
        })
        .map_err(PyValueError::new_err)?
    };
    contours_result_to_dict(py, result)
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(offset_contours, module)?)?;
    module.add_function(wrap_pyfunction!(offset_contours_with_origins, module)?)?;
    module.add_function(wrap_pyfunction!(offset_contours_variable, module)?)?;
    module.add_function(wrap_pyfunction!(
        offset_contours_variable_with_origins,
        module
    )?)?;
    Ok(())
}
