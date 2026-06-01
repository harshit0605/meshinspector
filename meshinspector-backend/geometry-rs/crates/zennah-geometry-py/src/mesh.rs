use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vec3, read_vertices};

fn vec3_lists(values: Vec<[f64; 3]>) -> Vec<Vec<f64>> {
    values.into_iter().map(|value| value.to_vec()).collect()
}

#[pyfunction]
fn safe_normalize_vector(py: Python<'_>, vector: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let rust_vector = read_vec3("vector", vector)?;
    let normalized = py.detach(|| zennah_geometry_core::safe_normalize_vector(rust_vector));
    Ok(normalized.to_vec())
}

#[pyfunction]
fn safe_normalize_vectors(
    py: Python<'_>,
    vectors: PyReadonlyArray2<'_, f64>,
) -> PyResult<Vec<Vec<f64>>> {
    let rust_vectors = read_vertices(vectors)?;
    Ok(vec3_lists(py.detach(|| {
        zennah_geometry_core::safe_normalize_vectors(&rust_vectors)
    })))
}

#[pyfunction]
fn normalize_axis(py: Python<'_>, axis: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let rust_axis = read_vec3("axis", axis)?;
    let normalized = py
        .detach(|| zennah_geometry_core::normalize_axis_vector(rust_axis))
        .map_err(|_| PyValueError::new_err("Axis vector magnitude is too small"))?;
    Ok(normalized.to_vec())
}

#[pyfunction]
fn mesh_bounds(py: Python<'_>, vertices: PyReadonlyArray2<'_, f64>) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let (bbox_min, bbox_max) = py.detach(|| zennah_geometry_core::mesh_bounds(&rust_vertices));
    let output = PyDict::new(py);
    output.set_item("min", bbox_min.to_vec())?;
    output.set_item("max", bbox_max.to_vec())?;
    Ok(output.unbind())
}

#[pyfunction]
fn face_normals(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let normals = py
        .detach(|| zennah_geometry_core::face_normals_for_mesh(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(vec3_lists(normals))
}

#[pyfunction]
fn vertex_normals(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let normals = py
        .detach(|| zennah_geometry_core::vertex_normals_for_mesh(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(vec3_lists(normals))
}

#[pyfunction]
fn surface_area(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<f64> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::mesh_surface_area(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn signed_volume(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<f64> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::mesh_signed_volume(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn volume(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<f64> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::mesh_volume(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn edge_face_map(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let entries = py
        .detach(|| {
            zennah_geometry_core::ordered_edge_face_entries(&rust_faces, rust_vertices.len())
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    for entry in entries {
        output.set_item(
            (entry.edge[0] as i64, entry.edge[1] as i64),
            entry
                .face_indices
                .into_iter()
                .map(|face| face as i64)
                .collect::<Vec<_>>(),
        )?;
    }
    Ok(output.unbind())
}

#[pyfunction]
fn boundary_edges(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let edges = py
        .detach(|| zennah_geometry_core::boundary_edges_for_mesh(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(edges.into_iter().map(|edge| edge.to_vec()).collect())
}

#[pyfunction]
fn face_adjacency(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::face_adjacency_for_mesh(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn connected_face_components(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::connected_face_components_for_mesh(&rust_vertices, &rust_faces)
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn vertex_neighbors(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::vertex_neighbors_for_mesh(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn mesh_stats(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;

    let stats = py
        .detach(|| zennah_geometry_core::mesh_stats(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("bbox_min", stats.bbox_min.to_vec())?;
    output.set_item("bbox_max", stats.bbox_max.to_vec())?;
    output.set_item("bbox_size", stats.bbox_size.to_vec())?;
    output.set_item("surface_area_mm2", stats.surface_area_mm2)?;
    output.set_item("volume_mm3", stats.volume_mm3)?;
    output.set_item("vertex_count", stats.vertex_count)?;
    output.set_item("face_count", stats.face_count)?;
    output.set_item("connected_components", stats.connected_components)?;
    output.set_item("boundary_edge_count", stats.boundary_edge_count)?;
    Ok(output.unbind())
}

#[pyfunction]
fn boundary_loops(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let loops = py
        .detach(|| zennah_geometry_core::boundary_loops(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(loops
        .into_iter()
        .map(|component| component.into_iter().map(|value| value as i64).collect())
        .collect())
}

#[pyfunction]
fn mesh_health(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    detect_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let health = py
        .detach(|| {
            zennah_geometry_core::mesh_health(
                &rust_vertices,
                &rust_faces,
                detect_self_intersections,
                max_self_intersection_faces,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    output.set_item("is_closed", health.is_closed)?;
    output.set_item("holes_count", health.holes_count)?;
    output.set_item("boundary_edge_count", health.boundary_edge_count)?;
    output.set_item("nonmanifold_edge_count", health.nonmanifold_edge_count)?;
    output.set_item("self_intersections", health.self_intersections)?;
    output.set_item(
        "self_intersections_available",
        health.self_intersections_available,
    )?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(safe_normalize_vector, module)?)?;
    module.add_function(wrap_pyfunction!(safe_normalize_vectors, module)?)?;
    module.add_function(wrap_pyfunction!(normalize_axis, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_bounds, module)?)?;
    module.add_function(wrap_pyfunction!(face_normals, module)?)?;
    module.add_function(wrap_pyfunction!(vertex_normals, module)?)?;
    module.add_function(wrap_pyfunction!(surface_area, module)?)?;
    module.add_function(wrap_pyfunction!(signed_volume, module)?)?;
    module.add_function(wrap_pyfunction!(volume, module)?)?;
    module.add_function(wrap_pyfunction!(edge_face_map, module)?)?;
    module.add_function(wrap_pyfunction!(boundary_edges, module)?)?;
    module.add_function(wrap_pyfunction!(face_adjacency, module)?)?;
    module.add_function(wrap_pyfunction!(connected_face_components, module)?)?;
    module.add_function(wrap_pyfunction!(vertex_neighbors, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_stats, module)?)?;
    module.add_function(wrap_pyfunction!(boundary_loops, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_health, module)?)?;
    Ok(())
}
