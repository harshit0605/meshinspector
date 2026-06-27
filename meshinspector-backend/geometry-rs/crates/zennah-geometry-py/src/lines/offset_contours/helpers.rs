use numpy::{IntoPyArray, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

pub(super) fn parse_z_options(
    z_restore: &str,
    z_value: Option<f64>,
    z_values: Option<PyReadonlyArray1<'_, f64>>,
    z_value_offsets: Option<PyReadonlyArray1<'_, i64>>,
    relax_iterations: i64,
) -> PyResult<zennah_geometry_core::lines::OffsetContoursZOptions> {
    let relax_iterations = parse_relax_iterations(relax_iterations);
    let restore_mode = match z_restore {
        "default" | "Default" | "meshlib_default" | "MeshLibDefault" => {
            zennah_geometry_core::lines::OffsetContoursZRestoreMode::Default
        }
        "constant" | "Constant" => {
            let Some(z_value) = z_value else {
                return Err(PyValueError::new_err(
                    "z_value is required when z_restore is 'constant'",
                ));
            };
            if !z_value.is_finite() {
                return Err(PyValueError::new_err("z_value must be finite"));
            }
            zennah_geometry_core::lines::OffsetContoursZRestoreMode::Constant(z_value)
        }
        "custom" | "Custom" | "callable" | "Callable" | "zCallback" | "z_callback"
        | "ZCallback" => {
            let Some(z_values) =
                read_optional_f64_rows(z_values, z_value_offsets, "z_values", "z_value_offsets")?
            else {
                return Err(PyValueError::new_err(
                    "z_values is required when z_restore is 'custom' or 'callable'",
                ));
            };
            zennah_geometry_core::lines::OffsetContoursZRestoreMode::Custom(z_values)
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "z_restore must be 'default', 'constant', 'custom', or 'callable', got {other}"
            )))
        }
    };
    Ok(zennah_geometry_core::lines::OffsetContoursZOptions {
        restore_mode,
        relax_iterations,
    })
}

pub(super) fn parse_relax_iterations(relax_iterations: i64) -> usize {
    if relax_iterations <= 0 {
        0
    } else {
        relax_iterations as usize
    }
}

pub(super) fn call_z_callback(
    py: Python<'_>,
    callback: &Py<PyAny>,
    offset_contours: &[Vec<[f64; 3]>],
    offset_index: zennah_geometry_core::lines::OffsetContourIndex,
    origin: &zennah_geometry_core::lines::OffsetContoursOrigin,
) -> Result<f64, String> {
    let contour_id = usize::try_from(offset_index.contour_id)
        .map_err(|_| "OffsetContours zCallback received invalid contour id".to_string())?;
    let vert_id = usize::try_from(offset_index.vert_id)
        .map_err(|_| "OffsetContours zCallback received invalid vertex id".to_string())?;
    let point = offset_contours
        .get(contour_id)
        .and_then(|contour| contour.get(vert_id))
        .ok_or_else(|| "OffsetContours zCallback received out-of-range offset index".to_string())?;
    let point_arg = (point[0], point[1], point[2]);
    let index_arg = contour_index_to_dict(py, offset_index).map_err(|error| error.to_string())?;
    let origin_arg = origin_to_dict(py, *origin).map_err(|error| error.to_string())?;
    let value = callback
        .bind(py)
        .call1((point_arg, index_arg, origin_arg))
        .map_err(|error| error.to_string())?;
    value.extract::<f64>().map_err(|error| error.to_string())
}

pub(super) fn read_contour_offsets(
    point_offsets: PyReadonlyArray1<'_, f64>,
    offset_offsets: PyReadonlyArray1<'_, i64>,
) -> PyResult<Vec<Vec<f64>>> {
    read_f64_rows(
        point_offsets,
        offset_offsets,
        "point_offsets",
        "offset_offsets",
    )
}

fn read_optional_f64_rows(
    values: Option<PyReadonlyArray1<'_, f64>>,
    offsets: Option<PyReadonlyArray1<'_, i64>>,
    values_name: &str,
    offsets_name: &str,
) -> PyResult<Option<Vec<Vec<f64>>>> {
    match (values, offsets) {
        (None, None) => Ok(None),
        (Some(values), Some(offsets)) => {
            read_f64_rows(values, offsets, values_name, offsets_name).map(Some)
        }
        (Some(_), None) => Err(PyValueError::new_err(format!(
            "{offsets_name} is required when {values_name} is provided"
        ))),
        (None, Some(_)) => Err(PyValueError::new_err(format!(
            "{values_name} is required when {offsets_name} is provided"
        ))),
    }
}

fn read_f64_rows(
    point_values: PyReadonlyArray1<'_, f64>,
    row_offsets: PyReadonlyArray1<'_, i64>,
    values_name: &str,
    offsets_name: &str,
) -> PyResult<Vec<Vec<f64>>> {
    let values = point_values.as_array();
    let offsets = row_offsets.as_array();
    if offsets.len() < 2 || offsets[0] != 0 {
        return Err(PyValueError::new_err(format!(
            "{offsets_name} must start at 0 and contain at least two entries"
        )));
    }
    let mut contours = Vec::with_capacity(offsets.len() - 1);
    for pair in offsets.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if start < 0 || end < start || end as usize > values.len() {
            return Err(PyValueError::new_err(format!(
                "{offsets_name} must be sorted and within {values_name} length"
            )));
        }
        contours.push(
            (start as usize..end as usize)
                .map(|index| values[index])
                .collect(),
        );
    }
    Ok(contours)
}

pub(super) fn contours_result_to_dict(
    py: Python<'_>,
    result: zennah_geometry_core::lines::OffsetContoursResult,
) -> PyResult<Py<PyDict>> {
    let mut contour_points = Vec::new();
    let mut contour_offsets = Vec::with_capacity(result.contours.len() + 1);
    contour_offsets.push(0_i64);
    for contour in result.contours {
        contour_points.extend(contour);
        contour_offsets.push(contour_points.len() as i64);
    }
    let flat_points = contour_points.into_iter().flatten().collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("contour_points", flat_points.into_pyarray(py))?;
    output.set_item("contour_offsets", contour_offsets.into_pyarray(py))?;
    let origins = PyList::empty(py);
    for contour_origins in result.origins {
        let row = PyList::empty(py);
        for origin in contour_origins {
            let item = PyDict::new(py);
            item.set_item("l_org", contour_index_to_dict(py, origin.l_org)?)?;
            item.set_item("l_dest", contour_index_to_dict(py, origin.l_dest)?)?;
            item.set_item("u_org", contour_index_to_dict(py, origin.u_org)?)?;
            item.set_item("u_dest", contour_index_to_dict(py, origin.u_dest)?)?;
            item.set_item("l_ratio", origin.l_ratio)?;
            item.set_item("u_ratio", origin.u_ratio)?;
            item.set_item("is_intersection", origin.is_intersection())?;
            row.append(item)?;
        }
        origins.append(row)?;
    }
    output.set_item("origins", origins)?;
    Ok(output.unbind())
}

fn contour_index_to_dict(
    py: Python<'_>,
    index: zennah_geometry_core::lines::OffsetContourIndex,
) -> PyResult<Bound<'_, PyDict>> {
    let output = PyDict::new(py);
    output.set_item("contour_id", index.contour_id)?;
    output.set_item("vert_id", index.vert_id)?;
    Ok(output)
}

fn origin_to_dict(
    py: Python<'_>,
    origin: zennah_geometry_core::lines::OffsetContoursOrigin,
) -> PyResult<Bound<'_, PyDict>> {
    let output = PyDict::new(py);
    output.set_item("l_org", contour_index_to_dict(py, origin.l_org)?)?;
    output.set_item("l_dest", contour_index_to_dict(py, origin.l_dest)?)?;
    output.set_item("u_org", contour_index_to_dict(py, origin.u_org)?)?;
    output.set_item("u_dest", contour_index_to_dict(py, origin.u_dest)?)?;
    output.set_item("l_ratio", origin.l_ratio)?;
    output.set_item("u_ratio", origin.u_ratio)?;
    output.set_item("is_intersection", origin.is_intersection())?;
    Ok(output)
}
