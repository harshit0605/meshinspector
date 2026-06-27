use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use crate::convert::{read_f32_values, read_faces, read_shape3, read_vec3, read_vertices};

#[pyfunction(signature = (values, shape, voxel_size, iso_value, level_set=false))]
fn voxel_to_mesh_simple_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    iso_value: f32,
    level_set: bool,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_to_mesh_simple_values(
                &rust_values,
                rust_shape,
                rust_voxel_size,
                iso_value,
                level_set,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (values, shape, voxel_size, iso_value, level_set=false))]
fn voxel_to_mesh_dual_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    iso_value: f32,
    level_set: bool,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_to_mesh_dual_values(
                &rust_values,
                rust_shape,
                rust_voxel_size,
                iso_value,
                level_set,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (values, shape, voxel_size, iso_value, level_set=false, max_faces=-1, max_vertices=-1, adaptivity=0.0, relax_disoriented_triangles=true))]
fn voxel_to_mesh_dual_values_with_settings(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    iso_value: f32,
    level_set: bool,
    max_faces: i64,
    max_vertices: i64,
    adaptivity: f32,
    relax_disoriented_triangles: bool,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let settings = zennah_geometry_core::VoxelDualMeshSettings {
        iso_value,
        level_set,
        adaptivity,
        max_faces: read_mesh_limit("max_faces", max_faces)?,
        max_vertices: read_mesh_limit("max_vertices", max_vertices)?,
        relax_disoriented_triangles,
        ..zennah_geometry_core::VoxelDualMeshSettings::default()
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_to_mesh_dual_values_with_settings(
                &rust_values,
                rust_shape,
                rust_voxel_size,
                settings,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

fn read_mesh_limit(name: &str, value: i64) -> PyResult<usize> {
    if value < 0 {
        return Ok(usize::MAX);
    }
    usize::try_from(value).map_err(|_| PyValueError::new_err(format!("{name} is too large")))
}

#[pyfunction(signature = (model_bytes, dimensions, voxel_size, iso_value, max_faces=-1, max_vertices=-1, adaptivity=0.0, relax_disoriented_triangles=true))]
fn meshlib_vdb_payload_to_dual_mesh(
    py: Python<'_>,
    model_bytes: &Bound<'_, PyBytes>,
    dimensions: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    iso_value: f32,
    max_faces: i64,
    max_vertices: i64,
    adaptivity: f32,
    relax_disoriented_triangles: bool,
) -> PyResult<Py<PyDict>> {
    let rust_dimensions = read_shape3(dimensions)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let rust_voxel_size = [
        rust_voxel_size[0] as f32,
        rust_voxel_size[1] as f32,
        rust_voxel_size[2] as f32,
    ];
    let rust_model_bytes = model_bytes.as_bytes().to_vec();
    let settings = zennah_geometry_core::VoxelDualMeshSettings {
        iso_value,
        level_set: true,
        adaptivity,
        max_faces: read_mesh_limit("max_faces", max_faces)?,
        max_vertices: read_mesh_limit("max_vertices", max_vertices)?,
        relax_disoriented_triangles,
        ..zennah_geometry_core::VoxelDualMeshSettings::default()
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::meshlib_vdb_payload_to_dual_mesh_with_settings(
                &rust_model_bytes,
                rust_dimensions,
                rust_voxel_size,
                settings,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    vertices,
    faces,
    values,
    shape,
    voxel_size,
    iters = 30,
    sample_points = 6,
    degree = 3,
    outlier_threshold = 1.0,
    intermediate_smooth_force = 0.3,
    preparation_smooth_force = 0.1,
    smooth_shift_iterations = 15,
    final_relax_iterations = 15,
    final_relax_force = 0.01
))]
#[allow(clippy::too_many_arguments)]
fn voxel_move_mesh_to_max_deriv_values(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    iters: usize,
    sample_points: usize,
    degree: usize,
    outlier_threshold: f64,
    intermediate_smooth_force: f64,
    preparation_smooth_force: f64,
    smooth_shift_iterations: usize,
    final_relax_iterations: usize,
    final_relax_force: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let settings = zennah_geometry_core::VoxelMaxDerivSettings {
        iters,
        sample_points,
        degree,
        outlier_threshold,
        intermediate_smooth_force,
        preparation_smooth_force,
        smooth_shift_iterations,
        final_relax_iterations,
        final_relax_force,
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_move_mesh_to_max_deriv_values(
                &rust_vertices,
                &rust_faces,
                &rust_values,
                rust_shape,
                rust_voxel_size,
                settings,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("corrected_indices", result.corrected_indices)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    values,
    shape,
    voxel_size,
    iso_value,
    level_set = false,
    iters = 30,
    sample_points = 6,
    degree = 3,
    outlier_threshold = 1.0,
    intermediate_smooth_force = 0.3,
    preparation_smooth_force = 0.1,
    smooth_shift_iterations = 15,
    final_relax_iterations = 15,
    final_relax_force = 0.01
))]
#[allow(clippy::too_many_arguments)]
fn voxel_to_mesh_smart_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    iso_value: f32,
    level_set: bool,
    iters: usize,
    sample_points: usize,
    degree: usize,
    outlier_threshold: f64,
    intermediate_smooth_force: f64,
    preparation_smooth_force: f64,
    smooth_shift_iterations: usize,
    final_relax_iterations: usize,
    final_relax_force: f64,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let settings = zennah_geometry_core::VoxelMaxDerivSettings {
        iters,
        sample_points,
        degree,
        outlier_threshold,
        intermediate_smooth_force,
        preparation_smooth_force,
        smooth_shift_iterations,
        final_relax_iterations,
        final_relax_force,
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_to_mesh_smart_values(
                &rust_values,
                rust_shape,
                rust_voxel_size,
                iso_value,
                level_set,
                settings,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    output.set_item("corrected_indices", result.corrected_indices)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(voxel_to_mesh_simple_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_to_mesh_dual_values, module)?)?;
    module.add_function(wrap_pyfunction!(
        voxel_to_mesh_dual_values_with_settings,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(meshlib_vdb_payload_to_dual_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(
        voxel_move_mesh_to_max_deriv_values,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(voxel_to_mesh_smart_values, module)?)?;
    Ok(())
}
