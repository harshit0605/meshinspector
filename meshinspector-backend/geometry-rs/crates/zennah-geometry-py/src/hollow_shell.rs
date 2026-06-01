use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{parse_voxel_mesh_extractor, read_faces, read_i64_values, read_vertices};

#[pyfunction]
fn service_hollow_voxel_size(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    wall_thickness_mm: f64,
) -> PyResult<f64> {
    let rust_vertices = read_vertices(vertices)?;
    py.detach(|| zennah_geometry_core::service_hollow_voxel_size(&rust_vertices, wall_thickness_mm))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn service_hollow_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    wall_thickness_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::service_hollow_mesh(
                &rust_vertices,
                &rust_faces,
                wall_thickness_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    crate::hollow::mesh_arrays_dict(py, result)
}

#[pyfunction(signature = (
    vertices,
    faces,
    region_ids,
    vertex_offsets,
    vertex_indices,
    protect_region_ids,
    wall_thickness_mm,
    voxel_size_mm = 0.5,
    padding_mm = None,
    extractor = "marching",
    refine = false
))]
#[allow(clippy::too_many_arguments)]
fn protected_hollow_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    protect_region_ids: Vec<String>,
    wall_thickness_mm: f64,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_vertex_offsets = read_i64_values(vertex_offsets);
    let rust_vertex_indices = read_i64_values(vertex_indices);
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::protected_hollow_mesh(
                &rust_vertices,
                &rust_faces,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                &protect_region_ids,
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
    crate::hollow::mesh_arrays_dict(py, result)
}

#[pyfunction(signature = (
    vertices,
    faces,
    region_ids,
    vertex_offsets,
    vertex_indices,
    protect_region_ids,
    target_weight_g,
    material = "gold_18k",
    tolerance_g = 0.1,
    min_thickness_mm = 0.5,
    max_thickness_mm = 3.0,
    max_iterations = 20,
    voxel_size_mm = 0.5,
    padding_mm = None,
    extractor = "marching",
    refine = false
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_protected_hollow_to_weight(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    protect_region_ids: Vec<String>,
    target_weight_g: f64,
    material: &str,
    tolerance_g: f64,
    min_thickness_mm: f64,
    max_thickness_mm: f64,
    max_iterations: usize,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_vertex_offsets = read_i64_values(vertex_offsets);
    let rust_vertex_indices = read_i64_values(vertex_indices);
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::adaptive_protected_hollow_to_weight(
                &rust_vertices,
                &rust_faces,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                &protect_region_ids,
                target_weight_g,
                material,
                tolerance_g,
                min_thickness_mm,
                max_thickness_mm,
                max_iterations,
                zennah_geometry_core::VoxelMeshOptions {
                    voxel_size: voxel_size_mm,
                    padding_mm,
                    extractor: rust_extractor,
                    refine,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    crate::hollow::adaptive_hollow_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(service_hollow_voxel_size, module)?)?;
    module.add_function(wrap_pyfunction!(service_hollow_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(protected_hollow_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(
        adaptive_protected_hollow_to_weight,
        module
    )?)?;
    Ok(())
}
