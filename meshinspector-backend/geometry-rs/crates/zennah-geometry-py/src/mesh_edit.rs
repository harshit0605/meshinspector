use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_f32_values, read_faces, read_i64_values, read_vertices};

mod decimate;

pub(super) fn read_edge_pairs(
    name: &str,
    values: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<[usize; 2]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (n, 2)"
        )));
    }
    let mut pairs = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        if row[0] < 0 || row[1] < 0 {
            return Err(PyValueError::new_err(format!(
                "{name} must contain non-negative vertex indices"
            )));
        }
        pairs.push([row[0] as usize, row[1] as usize]);
    }
    Ok(pairs)
}

fn read_nonnegative_indices(name: &str, values: PyReadonlyArray1<'_, i64>) -> PyResult<Vec<usize>> {
    read_i64_values(values)
        .into_iter()
        .map(|index| {
            if index < 0 {
                Err(PyValueError::new_err(format!(
                    "{name} must contain non-negative indices"
                )))
            } else {
                Ok(index as usize)
            }
        })
        .collect::<PyResult<Vec<_>>>()
}

#[pyfunction(signature = (vertices, faces, max_edge_len, max_edge_splits = 1000, region_faces = None, not_flippable_edges = None, subdivide_border = true, max_tri_aspect_ratio = 0.0, max_splittable_tri_aspect_ratio = 1.7976931348623157e308, curvature_priority = 0.0, project_on_original_mesh = false, smooth_mode = false, min_sharp_dihedral_angle = 0.5235987755982989, max_deviation_after_flip = None, max_angle_change_after_flip = None, critical_tri_aspect_ratio_flip = None))]
fn subdivide_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    max_edge_len: f64,
    max_edge_splits: usize,
    region_faces: Option<PyReadonlyArray1<'_, i64>>,
    not_flippable_edges: Option<PyReadonlyArray2<'_, i64>>,
    subdivide_border: bool,
    max_tri_aspect_ratio: f64,
    max_splittable_tri_aspect_ratio: f64,
    curvature_priority: f64,
    project_on_original_mesh: bool,
    smooth_mode: bool,
    min_sharp_dihedral_angle: f64,
    max_deviation_after_flip: Option<f64>,
    max_angle_change_after_flip: Option<f64>,
    critical_tri_aspect_ratio_flip: Option<f64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
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
    let rust_not_flippable_edges = not_flippable_edges
        .map(|values| read_edge_pairs("not_flippable_edges", values))
        .transpose()?
        .unwrap_or_default();
    let result = py
        .detach(|| {
            zennah_geometry_core::subdivide_mesh(
                &rust_vertices,
                &rust_faces,
                zennah_geometry_core::SubdivideMeshOptions {
                    max_edge_len,
                    curvature_priority,
                    max_edge_splits,
                    subdivide_border,
                    project_on_original_mesh,
                    project_new_vertices_to_unit_sphere: false,
                    smooth_mode,
                    min_sharp_dihedral_angle,
                    max_tri_aspect_ratio,
                    max_splittable_tri_aspect_ratio,
                    max_deviation_after_flip,
                    max_angle_change_after_flip,
                    critical_tri_aspect_ratio_flip,
                    region_faces: rust_region_faces,
                    not_flippable_edges: rust_not_flippable_edges,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    let vertex_values: Vec<f64> = result.mesh.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.mesh.faces.into_iter().flatten().collect();
    let region_values: Vec<i64> = result
        .region_faces
        .into_iter()
        .map(|index| index as i64)
        .collect();
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    output.set_item("splits_done", result.splits_done)?;
    output.set_item("region_faces", region_values.into_pyarray(py))?;
    output.set_item("region_face_count", result.region_face_count)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, num_iters = 1, region_faces = None, max_deviation_after_flip = None, max_angle_change = None, critical_tri_aspect_ratio = None, not_flippable_edges = None, vert_region = None))]
fn make_delone_edge_flips(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    num_iters: usize,
    region_faces: Option<PyReadonlyArray1<'_, i64>>,
    max_deviation_after_flip: Option<f64>,
    max_angle_change: Option<f64>,
    critical_tri_aspect_ratio: Option<f64>,
    not_flippable_edges: Option<PyReadonlyArray2<'_, i64>>,
    vert_region: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_not_flippable_edges = not_flippable_edges
        .map(|values| read_edge_pairs("not_flippable_edges", values))
        .transpose()?
        .unwrap_or_default();
    let rust_vert_region = vert_region
        .map(|values| read_nonnegative_indices("vert_region", values))
        .transpose()?;
    let rust_region_faces = region_faces
        .map(|values| read_nonnegative_indices("region_faces", values))
        .transpose()?;
    let result = py
        .detach(|| {
            zennah_geometry_core::make_delone_edge_flips(
                &rust_vertices,
                &rust_faces,
                zennah_geometry_core::MakeDeloneEdgeFlipsOptions {
                    num_iters,
                    region_faces: rust_region_faces,
                    max_deviation_after_flip,
                    max_angle_change,
                    critical_tri_aspect_ratio,
                    not_flippable_edges: rust_not_flippable_edges,
                    vert_region: rust_vert_region,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    let vertex_values: Vec<f64> = result.mesh.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.mesh.faces.into_iter().flatten().collect();
    let region_values: Vec<i64> = result
        .region_faces
        .into_iter()
        .map(|index| index as i64)
        .collect();
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    output.set_item("flips_done", result.flips_done)?;
    output.set_item("region_faces", region_values.into_pyarray(py))?;
    output.set_item("region_face_count", result.region_face_count)?;
    Ok(output.unbind())
}

#[pyfunction]
fn offset_verts_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    offsets: PyReadonlyArray1<'_, f32>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_offsets = read_f32_values(offsets);
    let result = py
        .detach(|| {
            zennah_geometry_core::offset_verts_mesh(&rust_vertices, &rust_faces, &rust_offsets)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    decimate::register(module)?;
    module.add_function(wrap_pyfunction!(subdivide_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(make_delone_edge_flips, module)?)?;
    module.add_function(wrap_pyfunction!(offset_verts_mesh, module)?)?;
    Ok(())
}
