use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{parse_voxel_mesh_extractor, read_faces, read_i64_values, read_vertices};

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
    vertices,
    faces,
    region_ids,
    vertex_offsets,
    vertex_indices,
    selected_region_ids,
    offset_mm,
    voxel_size_mm,
    padding_mm = None,
    extractor = "marching",
    refine = false
))]
#[allow(clippy::too_many_arguments)]
fn voxel_partial_offset_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    selected_region_ids: Vec<String>,
    offset_mm: f64,
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
            zennah_geometry_core::voxel_partial_offset_mesh(
                &rust_vertices,
                &rust_faces,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                &selected_region_ids,
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

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(voxel_partial_offset_mesh, module)?)?;
    Ok(())
}
