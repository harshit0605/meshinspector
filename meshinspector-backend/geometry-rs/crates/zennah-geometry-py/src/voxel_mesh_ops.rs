use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{
    parse_sdf_boolean_operation, parse_voxel_mesh_extractor, read_f32_values, read_faces,
    read_i64_values, read_shape3, read_vec3, read_vertices,
};

fn mesh_arrays_to_dict(
    py: Python<'_>,
    result: zennah_geometry_core::MeshArrays,
) -> PyResult<Py<PyDict>> {
    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    values,
    origin,
    shape,
    voxel_size_mm,
    extractor = "marching",
    refine = false,
    smooth_iterations = 1,
    smooth_strength = 0.2,
    projection_iterations = 3
))]
#[allow(clippy::too_many_arguments)]
fn extract_grid_mesh(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    extractor: &str,
    refine: bool,
    smooth_iterations: i64,
    smooth_strength: f64,
    projection_iterations: i64,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::extract_grid_mesh(
                &rust_values,
                rust_origin,
                rust_shape,
                zennah_geometry_core::GridMeshExtractionOptions {
                    voxel_size: voxel_size_mm,
                    extractor: rust_extractor,
                    refine,
                    smooth_iterations,
                    smooth_strength,
                    projection_iterations,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, offset_mm, voxel_size_mm, padding_mm = None, extractor = "marching", refine = false))]
#[allow(clippy::too_many_arguments)]
fn voxel_offset_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    offset_mm: f64,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_offset_mesh(
                &rust_vertices,
                &rust_faces,
                offset_mm,
                zennah_geometry_core::VoxelMeshOptions {
                    voxel_size: voxel_size_mm,
                    padding_mm,
                    extractor: rust_extractor,
                    refine,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, wall_thickness_mm, voxel_size_mm, padding_mm = None, extractor = "marching", refine = false))]
#[allow(clippy::too_many_arguments)]
fn voxel_shell_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    wall_thickness_mm: f64,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_shell_mesh(
                &rust_vertices,
                &rust_faces,
                wall_thickness_mm,
                zennah_geometry_core::VoxelMeshOptions {
                    voxel_size: voxel_size_mm,
                    padding_mm,
                    extractor: rust_extractor,
                    refine,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, thickness_mm, voxel_size_mm, padding_mm = None, extractor = "marching", refine = false))]
#[allow(clippy::too_many_arguments)]
fn voxel_thicken_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    thickness_mm: f64,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_thicken_mesh(
                &rust_vertices,
                &rust_faces,
                thickness_mm,
                zennah_geometry_core::VoxelMeshOptions {
                    voxel_size: voxel_size_mm,
                    padding_mm,
                    extractor: rust_extractor,
                    refine,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (
    vertices,
    faces,
    region_ids,
    vertex_offsets,
    vertex_indices,
    weighted_region_ids,
    region_weights,
    offset_mm,
    interpolation_distance_mm,
    voxel_size_mm,
    padding_mm = None,
    extractor = "marching",
    refine = false
))]
#[allow(clippy::too_many_arguments)]
fn voxel_weighted_shell_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    weighted_region_ids: Vec<String>,
    region_weights: PyReadonlyArray1<'_, f32>,
    offset_mm: f64,
    interpolation_distance_mm: f64,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_vertex_offsets = read_i64_values(vertex_offsets);
    let rust_vertex_indices = read_i64_values(vertex_indices);
    let rust_region_weights = read_f32_values(region_weights);
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_weighted_shell_mesh(
                &rust_vertices,
                &rust_faces,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                &weighted_region_ids,
                &rust_region_weights,
                offset_mm,
                interpolation_distance_mm,
                zennah_geometry_core::VoxelMeshOptions {
                    voxel_size: voxel_size_mm,
                    padding_mm,
                    extractor: rust_extractor,
                    refine,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (
    left_vertices,
    left_faces,
    right_vertices,
    right_faces,
    operation,
    voxel_size_mm,
    padding_mm = None,
    origin_phase = None,
    extractor = "marching",
    refine = false
))]
#[allow(clippy::too_many_arguments)]
fn voxel_boolean_mesh(
    py: Python<'_>,
    left_vertices: PyReadonlyArray2<'_, f64>,
    left_faces: PyReadonlyArray2<'_, i64>,
    right_vertices: PyReadonlyArray2<'_, f64>,
    right_faces: PyReadonlyArray2<'_, i64>,
    operation: &str,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    origin_phase: Option<PyReadonlyArray1<'_, f64>>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_left_vertices = read_vertices(left_vertices)?;
    let rust_left_faces = read_faces(left_faces)?;
    let rust_right_vertices = read_vertices(right_vertices)?;
    let rust_right_faces = read_faces(right_faces)?;
    let rust_operation = parse_sdf_boolean_operation(operation)?;
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let rust_origin_phase = match origin_phase {
        Some(values) => read_vec3("origin_phase", values)?,
        None => zennah_geometry_core::default_boolean_origin_phase(),
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_boolean_mesh(
                &rust_left_vertices,
                &rust_left_faces,
                &rust_right_vertices,
                &rust_right_faces,
                rust_operation,
                zennah_geometry_core::VoxelBooleanMeshOptions {
                    voxel_size: voxel_size_mm,
                    padding_mm,
                    origin_phase: rust_origin_phase,
                    extractor: rust_extractor,
                    refine,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, min_target_thickness_mm))]
fn global_thicken_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    min_target_thickness_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::global_thicken_mesh(
                &rust_vertices,
                &rust_faces,
                min_target_thickness_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(extract_grid_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_offset_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_shell_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_thicken_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_weighted_shell_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_boolean_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(global_thicken_mesh, module)?)?;
    Ok(())
}
