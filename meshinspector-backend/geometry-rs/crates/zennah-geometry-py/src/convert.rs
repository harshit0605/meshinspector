use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use zennah_geometry_core::{SdfBooleanOperation, VoxelMeshExtractor};

pub(crate) fn read_vertices(vertices: PyReadonlyArray2<'_, f64>) -> PyResult<Vec<[f64; 3]>> {
    let vertex_rows = vertices.as_array();
    if vertex_rows.ndim() != 2 || vertex_rows.shape()[1] != 3 {
        return Err(PyValueError::new_err("vertices must have shape (n, 3)"));
    }

    let mut rust_vertices = Vec::with_capacity(vertex_rows.shape()[0]);
    for row in vertex_rows.outer_iter() {
        rust_vertices.push([row[0], row[1], row[2]]);
    }
    Ok(rust_vertices)
}

pub(crate) fn read_points(points: PyReadonlyArray2<'_, f64>) -> PyResult<Vec<[f64; 3]>> {
    let point_rows = points.as_array();
    if point_rows.ndim() != 2 || point_rows.shape()[1] != 3 {
        return Err(PyValueError::new_err("points must have shape (n, 3)"));
    }

    let mut rust_points = Vec::with_capacity(point_rows.shape()[0]);
    for row in point_rows.outer_iter() {
        rust_points.push([row[0], row[1], row[2]]);
    }
    Ok(rust_points)
}

pub(crate) fn read_vec3(name: &str, values: PyReadonlyArray1<'_, f64>) -> PyResult<[f64; 3]> {
    let rows = values.as_array();
    if rows.ndim() != 1 || rows.shape()[0] != 3 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (3,)"
        )));
    }
    Ok([rows[0], rows[1], rows[2]])
}

pub(crate) fn read_i64_values(values: PyReadonlyArray1<'_, i64>) -> Vec<i64> {
    values.as_array().iter().copied().collect()
}

pub(crate) fn read_f32_values(values: PyReadonlyArray1<'_, f32>) -> Vec<f32> {
    values.as_array().iter().copied().collect()
}

pub(crate) fn read_f64_values(values: PyReadonlyArray1<'_, f64>) -> Vec<f64> {
    values.as_array().iter().copied().collect()
}

pub(crate) fn read_shape3(values: PyReadonlyArray1<'_, i64>) -> PyResult<[usize; 3]> {
    let rows = values.as_array();
    if rows.ndim() != 1 || rows.shape()[0] != 3 {
        return Err(PyValueError::new_err("shape must have shape (3,)"));
    }
    if rows.iter().any(|value| *value <= 0) {
        return Err(PyValueError::new_err("shape values must be positive"));
    }
    Ok([rows[0] as usize, rows[1] as usize, rows[2] as usize])
}

pub(crate) fn read_faces(faces: PyReadonlyArray2<'_, i64>) -> PyResult<Vec<[i64; 3]>> {
    let face_rows = faces.as_array();
    if face_rows.ndim() != 2 || face_rows.shape()[1] != 3 {
        return Err(PyValueError::new_err("faces must have shape (m, 3)"));
    }

    let mut rust_faces = Vec::with_capacity(face_rows.shape()[0]);
    for row in face_rows.outer_iter() {
        rust_faces.push([row[0], row[1], row[2]]);
    }
    Ok(rust_faces)
}

pub(crate) fn parse_sdf_boolean_operation(operation: &str) -> PyResult<SdfBooleanOperation> {
    match operation {
        "union" => Ok(SdfBooleanOperation::Union),
        "intersection" => Ok(SdfBooleanOperation::Intersection),
        "difference" => Ok(SdfBooleanOperation::Difference),
        _ => Err(PyValueError::new_err(
            "operation must be 'union', 'intersection', or 'difference'",
        )),
    }
}

pub(crate) fn parse_voxel_mesh_extractor(extractor: &str) -> PyResult<VoxelMeshExtractor> {
    match extractor {
        "marching" => Ok(VoxelMeshExtractor::Marching),
        "cells" => Ok(VoxelMeshExtractor::Cells),
        _ => Err(PyValueError::new_err(
            "extractor must be 'marching' or 'cells'",
        )),
    }
}
