use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use zennah_geometry_core::{
    RawVoxelScalarType, SdfBooleanOperation, VoxelBinaryOperation, VoxelMeshExtractor,
    VoxelPathMetric, VoxelPathPlane,
};

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

pub(crate) fn read_index3(name: &str, values: PyReadonlyArray1<'_, i64>) -> PyResult<[usize; 3]> {
    let rows = values.as_array();
    if rows.ndim() != 1 || rows.shape()[0] != 3 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (3,)"
        )));
    }
    if rows.iter().any(|value| *value < 0) {
        return Err(PyValueError::new_err(format!(
            "{name} values must be non-negative"
        )));
    }
    Ok([rows[0] as usize, rows[1] as usize, rows[2] as usize])
}

pub(crate) fn read_index3_rows(
    name: &str,
    values: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<[usize; 3]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 3 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (n, 3)"
        )));
    }
    let mut output = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        if row.iter().any(|value| *value < 0) {
            return Err(PyValueError::new_err(format!(
                "{name} values must be non-negative"
            )));
        }
        output.push([row[0] as usize, row[1] as usize, row[2] as usize]);
    }
    Ok(output)
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

pub(crate) fn parse_voxel_binary_operation(operation: &str) -> PyResult<VoxelBinaryOperation> {
    match operation {
        "union" => Ok(VoxelBinaryOperation::Union),
        "intersection" => Ok(VoxelBinaryOperation::Intersection),
        "difference" => Ok(VoxelBinaryOperation::Difference),
        "max" => Ok(VoxelBinaryOperation::Max),
        "min" => Ok(VoxelBinaryOperation::Min),
        "sum" => Ok(VoxelBinaryOperation::Sum),
        "multiply" => Ok(VoxelBinaryOperation::Multiply),
        "divide" => Ok(VoxelBinaryOperation::Divide),
        _ => Err(PyValueError::new_err(
            "operation must be one of: union, intersection, difference, max, min, sum, multiply, divide",
        )),
    }
}

pub(crate) fn parse_raw_voxel_scalar_type(scalar_type: &str) -> PyResult<RawVoxelScalarType> {
    match scalar_type {
        "uint8" => Ok(RawVoxelScalarType::UInt8),
        "int8" => Ok(RawVoxelScalarType::Int8),
        "uint16" => Ok(RawVoxelScalarType::UInt16),
        "int16" => Ok(RawVoxelScalarType::Int16),
        "uint32" => Ok(RawVoxelScalarType::UInt32),
        "int32" => Ok(RawVoxelScalarType::Int32),
        "uint64" => Ok(RawVoxelScalarType::UInt64),
        "int64" => Ok(RawVoxelScalarType::Int64),
        "float32" => Ok(RawVoxelScalarType::Float32),
        "float64" => Ok(RawVoxelScalarType::Float64),
        "float32_4" => Ok(RawVoxelScalarType::Float32_4),
        _ => Err(PyValueError::new_err(
            "scalar_type must be one of: uint8, int8, uint16, int16, uint32, int32, uint64, int64, float32, float64, float32_4",
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

pub(crate) fn parse_voxel_path_metric(metric: &str) -> PyResult<VoxelPathMetric> {
    match metric {
        "difference" | "sum_diffs" => Ok(VoxelPathMetric::Difference),
        "exponent" => Ok(VoxelPathMetric::Exponent),
        _ => Err(PyValueError::new_err(
            "metric must be 'difference' or 'exponent'",
        )),
    }
}

pub(crate) fn parse_voxel_path_plane(plane: &str) -> PyResult<VoxelPathPlane> {
    match plane {
        "none" => Ok(VoxelPathPlane::None),
        "yz" => Ok(VoxelPathPlane::YZ),
        "zx" => Ok(VoxelPathPlane::ZX),
        "xy" => Ok(VoxelPathPlane::XY),
        _ => Err(PyValueError::new_err(
            "plane must be 'none', 'yz', 'zx', or 'xy'",
        )),
    }
}

pub(crate) fn parse_voxel_axis(axis: &str) -> PyResult<usize> {
    match axis {
        "x" => Ok(0),
        "y" => Ok(1),
        "z" => Ok(2),
        _ => Err(PyValueError::new_err("axis must be 'x', 'y', or 'z'")),
    }
}
