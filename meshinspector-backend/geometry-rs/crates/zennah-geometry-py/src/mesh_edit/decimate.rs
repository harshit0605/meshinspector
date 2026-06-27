use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_i64_values, read_vertices};

use super::read_edge_pairs;

#[pyfunction(signature = (vertices, faces, strategy = "minimize_error", max_error = 1.7976931348623157e308, max_edge_len = 1.7976931348623157e308, max_bd_shift = 1.7976931348623157e308, stabilizer = 0.001, target_face_count = None, target_face_ratio = None, subdivide_parts = 1, decimate_between_parts = true, angle_weighted_dist_to_plane = false, not_flippable_edges = None, collapse_near_not_flippable = false, max_deleted_vertices = 2147483647, max_deleted_faces = 2147483647, max_triangle_aspect_ratio = 20.0, touch_near_bd_edges = true, touch_bd_verts = true, optimize_vertex_pos = true, pack_mesh = false, region_faces = None, vertex_uvs = None, vertex_colors = None, twin_map = None, edges_to_collapse = None, critical_tri_aspect_ratio = 1.7976931348623157e308, tiny_edge_length = -1.0, max_angle_change = -1.0))]
fn decimate_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    strategy: &str,
    max_error: f64,
    max_edge_len: f64,
    max_bd_shift: f64,
    stabilizer: f64,
    target_face_count: Option<usize>,
    target_face_ratio: Option<f64>,
    subdivide_parts: usize,
    decimate_between_parts: bool,
    angle_weighted_dist_to_plane: bool,
    not_flippable_edges: Option<PyReadonlyArray2<'_, i64>>,
    collapse_near_not_flippable: bool,
    max_deleted_vertices: usize,
    max_deleted_faces: usize,
    max_triangle_aspect_ratio: f64,
    touch_near_bd_edges: bool,
    touch_bd_verts: bool,
    optimize_vertex_pos: bool,
    pack_mesh: bool,
    region_faces: Option<PyReadonlyArray1<'_, i64>>,
    vertex_uvs: Option<PyReadonlyArray2<'_, f64>>,
    vertex_colors: Option<PyReadonlyArray2<'_, i64>>,
    twin_map: Option<PyReadonlyArray2<'_, i64>>,
    edges_to_collapse: Option<PyReadonlyArray2<'_, i64>>,
    critical_tri_aspect_ratio: f64,
    tiny_edge_length: f64,
    max_angle_change: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_not_flippable_edges = not_flippable_edges
        .map(|values| read_edge_pairs("not_flippable_edges", values))
        .transpose()?
        .unwrap_or_default();
    let rust_twin_map = twin_map
        .map(|values| read_twin_map_entries("twin_map", values))
        .transpose()?
        .unwrap_or_default();
    let rust_edges_to_collapse = edges_to_collapse
        .map(|values| read_edge_pairs("edges_to_collapse", values))
        .transpose()?;
    let rust_vertex_uvs = vertex_uvs
        .map(|values| read_uv_pairs("vertex_uvs", values))
        .transpose()?;
    let rust_vertex_colors = vertex_colors
        .map(|values| read_color_quads("vertex_colors", values))
        .transpose()?;
    let rust_region_faces = region_faces
        .map(|values| {
            read_i64_values(values)
                .into_iter()
                .map(|index| {
                    if index < 0 {
                        Err(PyValueError::new_err(
                            "region_faces must contain non-negative face indices",
                        ))
                    } else {
                        Ok(index as usize)
                    }
                })
                .collect::<PyResult<Vec<_>>>()
        })
        .transpose()?;
    let rust_strategy = match strategy {
        "shortest_edge_first" => zennah_geometry_core::DecimateMeshStrategy::ShortestEdgeFirst,
        "minimize_error" => zennah_geometry_core::DecimateMeshStrategy::MinimizeError,
        other => {
            return Err(PyValueError::new_err(format!(
                "unsupported decimate strategy {other:?}"
            )))
        }
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::decimate_mesh(
                &rust_vertices,
                &rust_faces,
                zennah_geometry_core::DecimateMeshOptions {
                    strategy: rust_strategy,
                    max_error,
                    max_edge_len,
                    max_bd_shift,
                    stabilizer,
                    target_face_count,
                    target_face_ratio,
                    subdivide_parts,
                    decimate_between_parts,
                    not_flippable_edges: rust_not_flippable_edges,
                    twin_map: rust_twin_map,
                    collapse_near_not_flippable,
                    angle_weighted_dist_to_plane,
                    max_deleted_vertices,
                    max_deleted_faces,
                    max_triangle_aspect_ratio,
                    critical_tri_aspect_ratio,
                    tiny_edge_length,
                    max_angle_change,
                    touch_near_bd_edges,
                    touch_bd_verts,
                    optimize_vertex_pos,
                    pack_mesh,
                    edges_to_collapse: rust_edges_to_collapse,
                    vertex_uvs: rust_vertex_uvs,
                    vertex_colors: rust_vertex_colors,
                    region_faces: rust_region_faces,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    let vertex_values: Vec<f64> = result.mesh.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.mesh.faces.into_iter().flatten().collect();
    let not_flippable_values: Vec<i64> = result
        .not_flippable_edges
        .into_iter()
        .flatten()
        .map(|index| index as i64)
        .collect();
    let edges_to_collapse_values: Vec<i64> = result
        .edges_to_collapse
        .into_iter()
        .flatten()
        .map(|index| index as i64)
        .collect();
    let twin_map_values: Vec<i64> = result
        .twin_map
        .into_iter()
        .flat_map(|entry| entry.into_iter().flatten())
        .map(|index| index as i64)
        .collect();
    let vertex_uv_values = result
        .vertex_uvs
        .map(|uvs| uvs.into_iter().flatten().collect::<Vec<f64>>());
    let vertex_color_values = result
        .vertex_colors
        .map(|colors| colors.into_iter().flatten().collect::<Vec<u8>>());
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    output.set_item("verts_deleted", result.verts_deleted)?;
    output.set_item("faces_deleted", result.faces_deleted)?;
    output.set_item("error_introduced", result.error_introduced)?;
    output.set_item("cancelled", result.cancelled)?;
    output.set_item(
        "remapped_not_flippable_edges",
        not_flippable_values.into_pyarray(py),
    )?;
    output.set_item(
        "remapped_edges_to_collapse",
        edges_to_collapse_values.into_pyarray(py),
    )?;
    output.set_item("remapped_twin_map", twin_map_values.into_pyarray(py))?;
    if let Some(values) = vertex_uv_values {
        output.set_item("vertex_uvs", values.into_pyarray(py))?;
    }
    if let Some(values) = vertex_color_values {
        output.set_item("vertex_colors", values.into_pyarray(py))?;
    }
    Ok(output.unbind())
}

fn read_twin_map_entries(
    name: &str,
    values: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<[[usize; 2]; 2]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 4 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (n, 4)"
        )));
    }
    let mut entries = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        if row.iter().any(|index| *index < 0) {
            return Err(PyValueError::new_err(format!(
                "{name} must contain non-negative vertex indices"
            )));
        }
        entries.push([
            [row[0] as usize, row[1] as usize],
            [row[2] as usize, row[3] as usize],
        ]);
    }
    Ok(entries)
}

fn read_uv_pairs(name: &str, values: PyReadonlyArray2<'_, f64>) -> PyResult<Vec<[f64; 2]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (n, 2)"
        )));
    }
    let mut pairs = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        if !row[0].is_finite() || !row[1].is_finite() {
            return Err(PyValueError::new_err(format!(
                "{name} must contain finite coordinates"
            )));
        }
        pairs.push([row[0], row[1]]);
    }
    Ok(pairs)
}

fn read_color_quads(name: &str, values: PyReadonlyArray2<'_, i64>) -> PyResult<Vec<[u8; 4]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 4 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (n, 4)"
        )));
    }
    let mut colors = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        if row.iter().any(|value| *value < 0 || *value > 255) {
            return Err(PyValueError::new_err(format!(
                "{name} values must be in the 0..=255 range"
            )));
        }
        colors.push([row[0] as u8, row[1] as u8, row[2] as u8, row[3] as u8]);
    }
    Ok(colors)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(decimate_mesh, module)?)?;
    Ok(())
}
