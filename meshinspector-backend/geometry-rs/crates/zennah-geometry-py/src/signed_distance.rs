use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_faces, read_points, read_vec3, read_vertices};

#[pyfunction(signature = (
    vertices,
    faces,
    reject_self_intersections = true,
    max_self_intersection_faces = Some(50000),
    epsilon = 1e-8
))]
fn supports_winding_sign(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<bool> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::supports_winding_sign_for_mesh(
            &rust_vertices,
            &rust_faces,
            reject_self_intersections,
            max_self_intersection_faces,
            epsilon,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn point_inside_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    point: PyReadonlyArray1<'_, f64>,
    direction: PyReadonlyArray1<'_, f64>,
    epsilon: f64,
) -> PyResult<bool> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_point = read_vec3("point", point)?;
    let rust_direction = read_vec3("direction", direction)?;
    py.detach(|| {
        zennah_geometry_core::point_inside_mesh(
            &rust_vertices,
            &rust_faces,
            rust_point,
            rust_direction,
            epsilon,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, point, threshold = 0.5, require_closed = true, epsilon = 1e-8))]
fn point_inside_mesh_winding(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    point: PyReadonlyArray1<'_, f64>,
    threshold: f64,
    require_closed: bool,
    epsilon: f64,
) -> PyResult<bool> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_point = read_vec3("point", point)?;
    py.detach(|| {
        zennah_geometry_core::point_inside_mesh_winding(
            &rust_vertices,
            &rust_faces,
            rust_point,
            threshold,
            require_closed,
            epsilon,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (
    points,
    vertices,
    faces,
    sign_method = "auto",
    winding_threshold = 0.5,
    topology_epsilon = 1e-8,
    ray_epsilon = 1e-7
))]
#[allow(clippy::too_many_arguments)]
fn signed_point_mesh_distances_with_method(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    sign_method: &str,
    winding_threshold: f64,
    topology_epsilon: f64,
    ray_epsilon: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_points = read_points(points)?;
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let distances = py
        .detach(|| {
            zennah_geometry_core::signed_point_mesh_distances_with_method(
                &rust_points,
                &rust_vertices,
                &rust_faces,
                sign_method,
                winding_threshold,
                topology_epsilon,
                ray_epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f32> = distances
        .into_iter()
        .map(|distance| distance as f32)
        .collect();
    Ok(output.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(supports_winding_sign, module)?)?;
    module.add_function(wrap_pyfunction!(point_inside_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(point_inside_mesh_winding, module)?)?;
    module.add_function(wrap_pyfunction!(
        signed_point_mesh_distances_with_method,
        module
    )?)?;
    Ok(())
}
