use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{
    read_f32_values, read_faces, read_points, read_shape3, read_vec3, read_vertices,
};

#[allow(clippy::too_many_arguments)]
#[pyfunction]
fn sample_sdf_grid_in_bounds(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    bbox_min: PyReadonlyArray1<'_, f64>,
    bbox_max: PyReadonlyArray1<'_, f64>,
    voxel_size_mm: f64,
    padding_mm: f64,
    origin_phase: PyReadonlyArray1<'_, f64>,
    winding_threshold: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_bbox_min = read_vec3("bbox_min", bbox_min)?;
    let rust_bbox_max = read_vec3("bbox_max", bbox_max)?;
    let rust_origin_phase = read_vec3("origin_phase", origin_phase)?;
    let sample = py
        .detach(|| {
            zennah_geometry_core::sample_sdf_grid_in_bounds(
                &rust_vertices,
                &rust_faces,
                rust_bbox_min,
                rust_bbox_max,
                voxel_size_mm,
                padding_mm,
                rust_origin_phase,
                winding_threshold,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("origin", sample.origin.to_vec())?;
    output.set_item(
        "shape",
        sample
            .shape
            .into_iter()
            .map(|dimension| dimension as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("values", sample.values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction]
fn combine_bounding_boxes(
    py: Python<'_>,
    bbox_mins: PyReadonlyArray2<'_, f64>,
    bbox_maxs: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyDict>> {
    let rust_bbox_mins = read_points(bbox_mins)?;
    let rust_bbox_maxs = read_points(bbox_maxs)?;
    let (bbox_min, bbox_max) = py
        .detach(|| zennah_geometry_core::combine_bounding_boxes(&rust_bbox_mins, &rust_bbox_maxs))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("min", bbox_min.to_vec())?;
    output.set_item("max", bbox_max.to_vec())?;
    Ok(output.unbind())
}

#[pyfunction]
fn sdf_cell_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let output = py
        .detach(|| zennah_geometry_core::sdf_cell_values(&rust_values, rust_shape))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (values, shape, iso_value = 0.0))]
fn sdf_occupancy(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    iso_value: f32,
) -> PyResult<Py<PyArray1<u8>>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let output = py
        .detach(|| zennah_geometry_core::sdf_occupancy(&rust_values, rust_shape, iso_value))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (values, shape, voxel_size_mm, iso_value = 0.0))]
fn estimate_sdf_volume(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    iso_value: f32,
) -> PyResult<f64> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    py.detach(|| {
        zennah_geometry_core::estimate_sdf_volume(
            &rust_values,
            rust_shape,
            voxel_size_mm,
            iso_value,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn sdf_grid_points(
    py: Python<'_>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let output = py
        .detach(|| zennah_geometry_core::sdf_grid_points(rust_origin, rust_shape, voxel_size_mm))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let flat: Vec<f64> = output.into_iter().flatten().collect();
    Ok(flat.into_pyarray(py).unbind())
}

#[pyfunction]
fn sdf_points_to_grid(
    py: Python<'_>,
    origin: PyReadonlyArray1<'_, f64>,
    voxel_size_mm: f64,
    points: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_origin = read_vec3("origin", origin)?;
    let rust_points = read_points(points)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::sdf_points_to_grid(rust_origin, voxel_size_mm, &rust_points)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let flat: Vec<f64> = output.into_iter().flatten().collect();
    Ok(flat.into_pyarray(py).unbind())
}

#[pyfunction]
fn sample_sdf_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    points: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let rust_points = read_vertices(points)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::sample_sdf_values_batch(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                &rust_points,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn sample_sdf_gradients(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    points: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let rust_points = read_vertices(points)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::sample_sdf_gradients_batch(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                &rust_points,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let flat: Vec<f32> = output.into_iter().flatten().collect();
    Ok(flat.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(sample_sdf_grid_in_bounds, module)?)?;
    module.add_function(wrap_pyfunction!(combine_bounding_boxes, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_cell_values, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_occupancy, module)?)?;
    module.add_function(wrap_pyfunction!(estimate_sdf_volume, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_grid_points, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_points_to_grid, module)?)?;
    module.add_function(wrap_pyfunction!(sample_sdf_values, module)?)?;
    module.add_function(wrap_pyfunction!(sample_sdf_gradients, module)?)?;
    Ok(())
}
