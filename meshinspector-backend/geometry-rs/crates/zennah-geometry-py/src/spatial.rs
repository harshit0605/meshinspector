use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{
    read_faces, read_i64_values, read_points, read_shape3, read_vec3, read_vertices,
};

#[pyfunction(signature = (vertices, faces, epsilon = 1e-8, touch_is_intersection = true))]
fn self_intersecting_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    epsilon: f64,
    touch_is_intersection: bool,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let face_ids = py
        .detach(|| {
            zennah_geometry_core::self_intersecting_faces_with_touch(
                &rust_vertices,
                &rust_faces,
                epsilon,
                touch_is_intersection,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<i64> = face_ids.into_iter().map(|face_id| face_id as i64).collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (triangle_a, triangle_b, epsilon = 1e-8))]
fn triangles_intersect(
    triangle_a: PyReadonlyArray2<'_, f64>,
    triangle_b: PyReadonlyArray2<'_, f64>,
    epsilon: f64,
) -> PyResult<bool> {
    let rust_triangle_a = read_triangle("triangle_a", triangle_a)?;
    let rust_triangle_b = read_triangle("triangle_b", triangle_b)?;
    Ok(zennah_geometry_core::triangles_intersect(
        rust_triangle_a,
        rust_triangle_b,
        epsilon,
    ))
}

#[pyfunction]
fn point_mesh_distances(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_points = read_points(points)?;
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let distances = py
        .detach(|| {
            zennah_geometry_core::point_mesh_distances(&rust_points, &rust_vertices, &rust_faces)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f32> = distances
        .into_iter()
        .map(|distance| distance as f32)
        .collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn closest_points_on_mesh(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_points = read_points(points)?;
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::closest_points_on_mesh(&rust_points, &rust_vertices, &rust_faces)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let closest_points: Vec<f64> = result.closest_points.into_iter().flatten().collect();
    let face_indices: Vec<i64> = result.face_indices;
    let output = PyDict::new(py);
    output.set_item("closest_points", closest_points.into_pyarray(py))?;
    output.set_item("distances", result.distances.into_pyarray(py))?;
    output.set_item("face_indices", face_indices.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction]
fn closest_point_on_triangle(
    py: Python<'_>,
    point: PyReadonlyArray1<'_, f64>,
    triangle: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_point = read_vec3("point", point)?;
    let rust_triangle = read_triangle("triangle", triangle)?;
    let closest =
        py.detach(|| zennah_geometry_core::closest_point_on_triangle(rust_point, rust_triangle));
    Ok(closest.to_vec().into_pyarray(py).unbind())
}

#[pyfunction]
fn winding_numbers(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_points = read_points(points)?;
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let output = py
        .detach(|| zennah_geometry_core::winding_numbers(&rust_points, &rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (points, vertices, faces, winding_threshold = 0.5))]
fn signed_point_mesh_distances(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    winding_threshold: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_points = read_points(points)?;
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let distances = py
        .detach(|| {
            zennah_geometry_core::signed_point_mesh_distances(
                &rust_points,
                &rust_vertices,
                &rust_faces,
                winding_threshold,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f32> = distances
        .into_iter()
        .map(|distance| distance as f32)
        .collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, faces, epsilon = 1e-5))]
fn ray_thickness_at_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    epsilon: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let thickness = py
        .detach(|| {
            zennah_geometry_core::ray_thickness_at_vertices(&rust_vertices, &rust_faces, epsilon)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f32> = thickness.into_iter().map(|value| value as f32).collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, faces, origin, shape, voxel_size_mm, winding_threshold = 0.5))]
fn sdf_grid_values(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    winding_threshold: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let values = py
        .detach(|| {
            zennah_geometry_core::sdf_grid_values(
                &rust_vertices,
                &rust_faces,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                winding_threshold,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(values.into_pyarray(py).unbind())
}

#[pyfunction]
fn first_ray_hit(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    origin: PyReadonlyArray1<'_, f64>,
    direction: PyReadonlyArray1<'_, f64>,
    epsilon: f64,
    ignored_faces: PyReadonlyArray1<'_, i64>,
) -> PyResult<Option<Py<PyDict>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let ray_origin = read_vec3("origin", origin)?;
    let ray_direction = read_vec3("direction", direction)?;
    let ignored = read_i64_values(ignored_faces);
    let hit = py
        .detach(|| {
            zennah_geometry_core::first_ray_hit(
                &rust_vertices,
                &rust_faces,
                ray_origin,
                ray_direction,
                epsilon,
                &ignored,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let Some(hit) = hit else {
        return Ok(None);
    };
    let output = PyDict::new(py);
    output.set_item("face_index", hit.face_index)?;
    output.set_item("distance", hit.distance)?;
    output.set_item("point", hit.point.to_vec())?;
    Ok(Some(output.unbind()))
}

#[pyfunction]
fn ray_triangle_hits(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    origin: PyReadonlyArray1<'_, f64>,
    direction: PyReadonlyArray1<'_, f64>,
    epsilon: f64,
    ignored_faces: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let ray_origin = read_vec3("origin", origin)?;
    let ray_direction = read_vec3("direction", direction)?;
    let ignored = read_i64_values(ignored_faces);
    let hits = py
        .detach(|| {
            zennah_geometry_core::ray_triangle_hits(
                &rust_vertices,
                &rust_faces,
                ray_origin,
                ray_direction,
                epsilon,
                &ignored,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    ray_hits_to_py_dict(py, hits)
}

#[pyfunction]
fn first_ray_hits(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    origins: PyReadonlyArray2<'_, f64>,
    directions: PyReadonlyArray2<'_, f64>,
    epsilon: f64,
    ignored_faces: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_origins = read_points(origins)?;
    let rust_directions = read_points(directions)?;
    let ignored = read_i64_values(ignored_faces);
    let result = py
        .detach(|| {
            zennah_geometry_core::first_ray_hits(
                &rust_vertices,
                &rust_faces,
                &rust_origins,
                &rust_directions,
                epsilon,
                &ignored,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let point_values: Vec<f64> = result.points.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("face_indices", result.face_indices.into_pyarray(py))?;
    output.set_item("distances", result.distances.into_pyarray(py))?;
    output.set_item("points", point_values.into_pyarray(py))?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(self_intersecting_faces, module)?)?;
    module.add_function(wrap_pyfunction!(triangles_intersect, module)?)?;
    module.add_function(wrap_pyfunction!(closest_point_on_triangle, module)?)?;
    module.add_function(wrap_pyfunction!(point_mesh_distances, module)?)?;
    module.add_function(wrap_pyfunction!(closest_points_on_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(winding_numbers, module)?)?;
    module.add_function(wrap_pyfunction!(signed_point_mesh_distances, module)?)?;
    module.add_function(wrap_pyfunction!(ray_thickness_at_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_grid_values, module)?)?;
    module.add_function(wrap_pyfunction!(first_ray_hit, module)?)?;
    module.add_function(wrap_pyfunction!(ray_triangle_hits, module)?)?;
    module.add_function(wrap_pyfunction!(first_ray_hits, module)?)?;
    Ok(())
}

fn read_triangle(
    name: &'static str,
    triangle: PyReadonlyArray2<'_, f64>,
) -> PyResult<[[f64; 3]; 3]> {
    let vertices = read_vertices(triangle)?;
    if vertices.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (3, 3)"
        )));
    }
    Ok([vertices[0], vertices[1], vertices[2]])
}

fn ray_hits_to_py_dict(
    py: Python<'_>,
    hits: Vec<zennah_geometry_core::RayHit>,
) -> PyResult<Py<PyDict>> {
    let face_indices: Vec<i64> = hits.iter().map(|hit| hit.face_index as i64).collect();
    let distances: Vec<f64> = hits.iter().map(|hit| hit.distance).collect();
    let points: Vec<f64> = hits.into_iter().flat_map(|hit| hit.point).collect();
    let output = PyDict::new(py);
    output.set_item("face_indices", face_indices.into_pyarray(py))?;
    output.set_item("distances", distances.into_pyarray(py))?;
    output.set_item("points", points.into_pyarray(py))?;
    Ok(output.unbind())
}
