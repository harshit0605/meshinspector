use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_i64_values, read_vec3, read_vertices};

#[path = "distance_payload.rs"]
mod distance_payload;
use distance_payload::read_distance_map;

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

#[pyfunction]
fn distance_map_from_contours(
    py: Python<'_>,
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
    width: usize,
    height: usize,
    origin: (f64, f64),
    pixel_size: (f64, f64),
    signed: bool,
) -> PyResult<Py<PyDict>> {
    let contours = read_contours(contour_points, contour_offsets)?;
    let map = py
        .detach(|| {
            zennah_geometry_core::distance_map_from_contours(
                &contours,
                width,
                height,
                [origin.0, origin.1],
                [pixel_size.0, pixel_size.1],
                signed,
            )
        })
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item("width", map.width)?;
    output.set_item("height", map.height)?;
    output.set_item("origin", map.origin)?;
    output.set_item("pixel_size", map.pixel_size)?;
    output.set_item(
        "model_transform",
        map.model_transform.map(|value| value.to_vec()),
    )?;
    output.set_item("values", map.values.into_pyarray(py))?;
    output.set_item("valid_count", map.valid_count)?;
    output.set_item("min_value", map.min_value)?;
    output.set_item("max_value", map.max_value)?;
    Ok(output.unbind())
}

#[pyfunction]
fn distance_map_from_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    width: usize,
    height: usize,
    origin: PyReadonlyArray1<'_, f64>,
    x_range: PyReadonlyArray1<'_, f64>,
    y_range: PyReadonlyArray1<'_, f64>,
    direction: PyReadonlyArray1<'_, f64>,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_origin = read_vec3("origin", origin)?;
    let rust_x_range = read_vec3("x_range", x_range)?;
    let rust_y_range = read_vec3("y_range", y_range)?;
    let rust_direction = read_vec3("direction", direction)?;
    let map = py
        .detach(|| {
            zennah_geometry_core::distance_map_from_mesh(
                &rust_vertices,
                &rust_faces,
                width,
                height,
                rust_origin,
                rust_x_range,
                rust_y_range,
                rust_direction,
                epsilon,
            )
        })
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item("width", map.width)?;
    output.set_item("height", map.height)?;
    output.set_item("origin", map.origin)?;
    output.set_item("pixel_size", map.pixel_size)?;
    output.set_item(
        "model_transform",
        map.model_transform.map(|value| value.to_vec()),
    )?;
    output.set_item("values", map.values.into_pyarray(py))?;
    output.set_item("valid_count", map.valid_count)?;
    output.set_item("min_value", map.min_value)?;
    output.set_item("max_value", map.max_value)?;
    Ok(output.unbind())
}

#[pyfunction]
fn distance_map_from_tiff(py: Python<'_>, path: &str) -> PyResult<Py<PyDict>> {
    let map = py
        .detach(|| zennah_geometry_core::distance_map_from_tiff(path))
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item("width", map.width)?;
    output.set_item("height", map.height)?;
    output.set_item("origin", map.origin)?;
    output.set_item("pixel_size", map.pixel_size)?;
    output.set_item(
        "model_transform",
        map.model_transform.map(|value| value.to_vec()),
    )?;
    output.set_item("values", map.values.into_pyarray(py))?;
    output.set_item("valid_count", map.valid_count)?;
    output.set_item("min_value", map.min_value)?;
    output.set_item("max_value", map.max_value)?;
    Ok(output.unbind())
}

#[pyfunction]
fn distance_map_to_tiff(
    py: Python<'_>,
    values: PyReadonlyArray2<'_, f32>,
    origin: (f64, f64),
    pixel_size: (f64, f64),
    model_transform: Option<Vec<f64>>,
    path: &str,
) -> PyResult<()> {
    let map = read_distance_map(values, origin, pixel_size, model_transform)?;
    py.detach(|| zennah_geometry_core::distance_map_to_tiff(&map, path))
        .map_err(PyValueError::new_err)
}

#[pyfunction]
fn distance_map_to_iso_segments(
    py: Python<'_>,
    values: PyReadonlyArray2<'_, f32>,
    origin: (f64, f64),
    pixel_size: (f64, f64),
    iso_value: f32,
) -> PyResult<Py<PyDict>> {
    let map = read_distance_map(values, origin, pixel_size, None)?;
    let iso = py
        .detach(|| zennah_geometry_core::distance_map_to_iso_segments(&map, iso_value))
        .map_err(PyValueError::new_err)?;
    let segment_values: Vec<f64> = iso.segments.into_iter().flatten().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("iso_value", iso.iso_value)?;
    output.set_item("segments", segment_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction]
fn distance_map_merge(
    py: Python<'_>,
    left_values: PyReadonlyArray2<'_, f32>,
    left_origin: (f64, f64),
    left_pixel_size: (f64, f64),
    right_values: PyReadonlyArray2<'_, f32>,
    right_origin: (f64, f64),
    right_pixel_size: (f64, f64),
    mode: &str,
) -> PyResult<Py<PyDict>> {
    let left = read_distance_map(left_values, left_origin, left_pixel_size, None)?;
    let right = read_distance_map(right_values, right_origin, right_pixel_size, None)?;
    let merge_mode = read_merge_mode(mode)?;
    let map = py
        .detach(|| zennah_geometry_core::distance_map_merge(&left, &right, merge_mode))
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item("width", map.width)?;
    output.set_item("height", map.height)?;
    output.set_item("origin", map.origin)?;
    output.set_item("pixel_size", map.pixel_size)?;
    output.set_item(
        "model_transform",
        map.model_transform.map(|value| value.to_vec()),
    )?;
    output.set_item("values", map.values.into_pyarray(py))?;
    output.set_item("valid_count", map.valid_count)?;
    output.set_item("min_value", map.min_value)?;
    output.set_item("max_value", map.max_value)?;
    Ok(output.unbind())
}

#[pyfunction]
fn distance_map_contour_boolean(
    py: Python<'_>,
    contour_points_a: PyReadonlyArray2<'_, f64>,
    contour_offsets_a: PyReadonlyArray1<'_, i64>,
    contour_points_b: PyReadonlyArray2<'_, f64>,
    contour_offsets_b: PyReadonlyArray1<'_, i64>,
    mode: &str,
    width: usize,
    height: usize,
    origin: (f64, f64),
    pixel_size: (f64, f64),
    iso_value: f32,
) -> PyResult<Py<PyDict>> {
    let contours_a = read_contours(contour_points_a, contour_offsets_a)?;
    let contours_b = read_contours(contour_points_b, contour_offsets_b)?;
    let boolean_mode = read_contour_boolean_mode(mode)?;
    let iso = py
        .detach(|| {
            zennah_geometry_core::distance_map_contour_boolean(
                &contours_a,
                &contours_b,
                boolean_mode,
                width,
                height,
                [origin.0, origin.1],
                [pixel_size.0, pixel_size.1],
                iso_value,
            )
        })
        .map_err(PyValueError::new_err)?;
    let segment_values: Vec<f64> = iso.segments.into_iter().flatten().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("iso_value", iso.iso_value)?;
    output.set_item("segments", segment_values.into_pyarray(py))?;
    output.set_item("mode", mode)?;
    Ok(output.unbind())
}

fn read_merge_mode(mode: &str) -> PyResult<zennah_geometry_core::DistanceMapMergeMode> {
    match mode {
        "min" => Ok(zennah_geometry_core::DistanceMapMergeMode::Min),
        "max" => Ok(zennah_geometry_core::DistanceMapMergeMode::Max),
        "subtract" => Ok(zennah_geometry_core::DistanceMapMergeMode::Subtract),
        _ => Err(PyValueError::new_err(
            "distance map merge mode must be 'min', 'max', or 'subtract'",
        )),
    }
}

fn read_contour_boolean_mode(mode: &str) -> PyResult<zennah_geometry_core::ContourBooleanMode> {
    match mode {
        "union" => Ok(zennah_geometry_core::ContourBooleanMode::Union),
        "intersection" => Ok(zennah_geometry_core::ContourBooleanMode::Intersection),
        "subtract" => Ok(zennah_geometry_core::ContourBooleanMode::Subtract),
        _ => Err(PyValueError::new_err(
            "contour boolean mode must be 'union', 'intersection', or 'subtract'",
        )),
    }
}

fn read_contours(
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
) -> PyResult<Vec<Vec<[f64; 2]>>> {
    let points = contour_points.as_array();
    if points.shape().len() != 2 || points.shape()[1] != 2 {
        return Err(PyValueError::new_err(
            "contour_points must have shape (n, 2)",
        ));
    }
    let offsets = contour_offsets.as_array();
    if offsets.len() < 2 {
        return Err(PyValueError::new_err(
            "contour_offsets must contain at least start and end offsets",
        ));
    }
    if offsets[0] != 0 {
        return Err(PyValueError::new_err("contour_offsets must start at 0"));
    }
    let point_count = points.shape()[0];
    let mut contours = Vec::with_capacity(offsets.len() - 1);
    for pair in offsets.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if start < 0 || end < start || end as usize > point_count {
            return Err(PyValueError::new_err(
                "contour_offsets must be sorted and within contour_points length",
            ));
        }
        let contour = (start as usize..end as usize)
            .map(|index| [points[[index, 0]], points[[index, 1]]])
            .collect();
        contours.push(contour);
    }
    Ok(contours)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(nearest_distances_to_indices, module)?)?;
    module.add_function(wrap_pyfunction!(distance_map_from_contours, module)?)?;
    module.add_function(wrap_pyfunction!(distance_map_from_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(distance_map_from_tiff, module)?)?;
    module.add_function(wrap_pyfunction!(distance_map_to_tiff, module)?)?;
    module.add_function(wrap_pyfunction!(distance_map_to_iso_segments, module)?)?;
    module.add_function(wrap_pyfunction!(distance_map_merge, module)?)?;
    module.add_function(wrap_pyfunction!(distance_map_contour_boolean, module)?)?;
    Ok(())
}
