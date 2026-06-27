use crate::convert::{read_f64_values, read_i64_values, read_points, read_vec3};
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub(crate) type PointCloudInputs = (Vec<[f64; 3]>, Option<Vec<[f64; 3]>>, Vec<usize>);

pub(crate) fn read_point_cloud_inputs(
    points: PyReadonlyArray2<'_, f64>,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    untrusted_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<PointCloudInputs> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let rust_untrusted_indices = match untrusted_indices {
        Some(indices) => read_i64_values(indices)
            .into_iter()
            .map(|index| {
                usize::try_from(index)
                    .map_err(|_| PyValueError::new_err("untrusted_indices must be non-negative"))
            })
            .collect::<PyResult<Vec<_>>>()?,
        None => Vec::new(),
    };
    Ok((rust_points, rust_normals, rust_untrusted_indices))
}

pub(crate) fn candidate_mesh_dict<'py>(
    py: Python<'py>,
    vertices: Vec<[f64; 3]>,
    faces: Vec<[i64; 3]>,
    repetition_counts: [usize; 4],
    repeated_3_count: usize,
    repeated_2_count: usize,
) -> PyResult<Bound<'py, PyDict>> {
    let vertices = vertices.into_iter().flatten().collect::<Vec<_>>();
    let faces = faces.into_iter().flatten().collect::<Vec<_>>();
    let repetition_counts = repetition_counts
        .into_iter()
        .map(|value| value as i64)
        .collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("repetition_counts", repetition_counts.into_pyarray(py))?;
    output.set_item("repeated_3_count", repeated_3_count)?;
    output.set_item("repeated_2_count", repeated_2_count)?;
    Ok(output)
}

fn read_view_projection(values: PyReadonlyArray1<'_, f64>) -> PyResult<[f64; 16]> {
    let flat = read_f64_values(values);
    flat.try_into()
        .map_err(|_| PyValueError::new_err("view_projection_4x4 must have 16 values"))
}

fn read_screen_points(name: &str, points: PyReadonlyArray2<'_, f64>) -> PyResult<Vec<[f64; 2]>> {
    let rows = points.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (n, 2)"
        )));
    }
    let mut output = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        output.push([row[0], row[1]]);
    }
    Ok(output)
}

#[pyfunction(signature = (points, view_projection_4x4, polygon_xy, normals = None, include_backfaces = true, visible_only = false))]
fn point_cloud_select_by_screen_polygon(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    view_projection_4x4: PyReadonlyArray1<'_, f64>,
    polygon_xy: PyReadonlyArray2<'_, f64>,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    include_backfaces: bool,
    visible_only: bool,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let view_projection = read_view_projection(view_projection_4x4)?;
    let polygon = read_screen_points("polygon_xy", polygon_xy)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::select_point_cloud_points_by_screen_polygon(
                &rust_points,
                rust_normals.as_deref(),
                &view_projection,
                &polygon,
                include_backfaces,
                visible_only,
            )
        })
        .map_err(PyValueError::new_err)?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (points, view_projection_4x4, rect_min_xy, rect_max_xy, normals = None, include_backfaces = true, visible_only = false))]
fn point_cloud_select_by_screen_rect(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    view_projection_4x4: PyReadonlyArray1<'_, f64>,
    rect_min_xy: PyReadonlyArray1<'_, f64>,
    rect_max_xy: PyReadonlyArray1<'_, f64>,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    include_backfaces: bool,
    visible_only: bool,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let view_projection = read_view_projection(view_projection_4x4)?;
    let rect_min = read_f64_values(rect_min_xy)
        .try_into()
        .map_err(|_| PyValueError::new_err("rect_min_xy must have 2 values"))?;
    let rect_max = read_f64_values(rect_max_xy)
        .try_into()
        .map_err(|_| PyValueError::new_err("rect_max_xy must have 2 values"))?;
    let output = py
        .detach(|| {
            zennah_geometry_core::select_point_cloud_points_by_screen_rect(
                &rust_points,
                rust_normals.as_deref(),
                &view_projection,
                rect_min,
                rect_max,
                include_backfaces,
                visible_only,
            )
        })
        .map_err(PyValueError::new_err)?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (points, view_projection_4x4, brush_path_xy, radius_px, normals = None, include_backfaces = true, visible_only = false))]
fn point_cloud_select_by_screen_brush(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    view_projection_4x4: PyReadonlyArray1<'_, f64>,
    brush_path_xy: PyReadonlyArray2<'_, f64>,
    radius_px: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    include_backfaces: bool,
    visible_only: bool,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let view_projection = read_view_projection(view_projection_4x4)?;
    let brush_path = read_screen_points("brush_path_xy", brush_path_xy)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::select_point_cloud_points_by_screen_brush(
                &rust_points,
                rust_normals.as_deref(),
                &view_projection,
                &brush_path,
                radius_px,
                include_backfaces,
                visible_only,
            )
        })
        .map_err(PyValueError::new_err)?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (points, ray_origin, ray_direction, max_distance_to_ray, max_depth = f64::INFINITY, normals = None, include_backfaces = true))]
fn point_cloud_pick_by_ray(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    ray_origin: PyReadonlyArray1<'_, f64>,
    ray_direction: PyReadonlyArray1<'_, f64>,
    max_distance_to_ray: f64,
    max_depth: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    include_backfaces: bool,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let origin = read_vec3("ray_origin", ray_origin)?;
    let direction = read_vec3("ray_direction", ray_direction)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::point_cloud_pick_by_ray(
                &rust_points,
                origin,
                direction,
                max_distance_to_ray,
                max_depth,
                rust_normals.as_deref(),
                include_backfaces,
            )
        })
        .map_err(PyValueError::new_err)?;
    Ok(output.into_pyarray(py).unbind())
}

