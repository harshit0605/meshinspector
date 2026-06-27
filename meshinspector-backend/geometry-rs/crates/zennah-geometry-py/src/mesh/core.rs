use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::PathBuf;

use crate::convert::{
    read_f64_values, read_faces, read_i64_values, read_points, read_vec3, read_vertices,
};

fn vec3_lists(values: Vec<[f64; 3]>) -> Vec<Vec<f64>> {
    values.into_iter().map(|value| value.to_vec()).collect()
}

fn read_nonnegative_face_ids(
    field: &str,
    values: PyReadonlyArray1<'_, i64>,
) -> PyResult<Vec<usize>> {
    let mut output = Vec::new();
    for value in read_i64_values(values) {
        if value < 0 {
            return Err(PyValueError::new_err(format!(
                "{field} must be non-negative"
            )));
        }
        output.push(value as usize);
    }
    Ok(output)
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

#[pyfunction(signature=(vertices, faces, start_vertex, end_vertex, max_path_len_mm = 1.7976931348623157e308))]
fn mesh_geodesic_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_vertex: usize,
    end_vertex: usize,
    max_path_len_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_geodesic_path(
                &rust_vertices,
                &rust_faces,
                start_vertex,
                end_vertex,
                max_path_len_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let line_segments = path.edge_lengths.len();
    let output = PyDict::new(py);
    output.set_item(
        "vertex_indices",
        path.vertex_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("point_normals", vec3_lists(path.point_normals))?;
    output.set_item("edge_lengths", path.edge_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item("line_segments", line_segments)?;
    output.set_item("meshlib_reference", "MR::buildShortestPath")?;
    Ok(output.unbind())
}

#[pyfunction(signature=(vertices, faces, seed_vertices, max_distance_mm = 1.7976931348623157e308))]
fn mesh_geodesic_distance_field(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    seed_vertices: PyReadonlyArray1<'_, i64>,
    max_distance_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let seeds = read_nonnegative_face_ids("seed_vertices", seed_vertices)?;
    let field = py
        .detach(|| {
            zennah_geometry_core::mesh_geodesic_distance_field(
                &rust_vertices,
                &rust_faces,
                &seeds,
                max_distance_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "seed_vertices",
        field
            .seed_vertices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("distances_mm", field.distances_mm)?;
    output.set_item(
        "predecessor_vertices",
        field
            .predecessor_vertices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item("reachable_vertex_count", field.reachable_vertex_count)?;
    output.set_item("max_distance_mm", field.max_distance_mm)?;
    output.set_item(
        "meshlib_reference",
        "MR::computeSurfaceDistances / SurfaceDistanceBuilder",
    )?;
    Ok(output.unbind())
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
fn select_boundary_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::select_boundary_faces(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn select_boundary_edges(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let edges = py
        .detach(|| zennah_geometry_core::select_boundary_edges(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(edges.into_iter().map(|edge| edge.to_vec()).collect())
}

#[pyfunction]
fn bounded_seed_indices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    indices: PyReadonlyArray1<'_, i64>,
    max_count: i64,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_indices = read_i64_values(indices);
    let rust_max_count = if max_count <= 0 {
        0_usize
    } else {
        max_count as usize
    };
    Ok(py.detach(|| {
        zennah_geometry_core::bounded_seed_indices(
            &rust_vertices,
            &rust_indices,
            rust_max_count,
        )
    }))
}

#[pyfunction]
fn selection_seed_indices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    vertex_ids: PyReadonlyArray1<'_, i64>,
    face_ids: PyReadonlyArray1<'_, i64>,
    region_vertex_indices: PyReadonlyArray1<'_, i64>,
    brush_points_world: PyReadonlyArray2<'_, f64>,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_vertex_ids = read_i64_values(vertex_ids);
    let rust_face_ids = read_i64_values(face_ids);
    let rust_region_vertex_indices = read_i64_values(region_vertex_indices);
    let rust_brush_points_world = read_points(brush_points_world)?;
    py.detach(|| {
        zennah_geometry_core::selection_seed_indices(
            &rust_vertices,
            &rust_faces,
            &rust_vertex_ids,
            &rust_face_ids,
            &rust_region_vertex_indices,
            &rust_brush_points_world,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}
