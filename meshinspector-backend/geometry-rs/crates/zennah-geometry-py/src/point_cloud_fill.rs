use crate::convert::{read_i64_values, read_points};
use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pyfunction(signature = (points, radius = 0.0, num_neighbors = 16, boundary_angle = std::f64::consts::PI * 0.9, max_removes = 2_147_483_647, crit_angle = std::f64::consts::TAU, crit_hole_length = -1.0, normals = None, untrusted_indices = None))]
fn point_cloud_triangulate_filled_candidate_mesh(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    crit_hole_length: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    untrusted_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let rust_untrusted_indices = match untrusted_indices {
        Some(indices) => read_i64_values(indices)
            .into_iter()
            .map(|index| {
                usize::try_from(index)
                    .map_err(|_| PyValueError::new_err("untrusted_indices must be non-negative"))
            })
            .collect::<PyResult<Vec<_>>>()?,
        None => Vec::new(),
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_triangulate_filled_candidate_mesh(
                &rust_points,
                radius,
                num_neighbors,
                boundary_angle,
                max_removes,
                crit_angle,
                crit_hole_length,
                rust_normals.as_deref(),
                &rust_untrusted_indices,
            )
        })
        .map_err(PyValueError::new_err)?;

    let output = PyDict::new(py);
    output.set_item(
        "vertices",
        result
            .vertices
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into_pyarray(py),
    )?;
    output.set_item(
        "faces",
        result
            .faces
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into_pyarray(py),
    )?;
    output.set_item(
        "repetition_counts",
        result
            .repetition_counts
            .into_iter()
            .map(|value| value as i64)
            .collect::<Vec<_>>()
            .into_pyarray(py),
    )?;
    output.set_item("repeated_3_count", result.repeated_3_count)?;
    output.set_item("repeated_2_count", result.repeated_2_count)?;
    output.set_item("candidate_face_count", result.candidate_face_count)?;
    output.set_item(
        "topology_skipped_face_count",
        result.topology_skipped_face_count,
    )?;
    output.set_item(
        "topology_degenerate_face_count",
        result.topology_degenerate_face_count,
    )?;
    output.set_item(
        "topology_nonmanifold_edge_face_count",
        result.topology_nonmanifold_edge_face_count,
    )?;
    output.set_item(
        "topology_nonmanifold_vertex_face_count",
        result.topology_nonmanifold_vertex_face_count,
    )?;
    output.set_item(
        "topology_unsafe_retry_face_count",
        result.topology_unsafe_retry_face_count,
    )?;
    output.set_item(
        "removed_hole_complicating_face_count",
        result.removed_hole_complicating_face_count,
    )?;
    output.set_item("input_hole_count", result.input_hole_count)?;
    output.set_item("filled_hole_count", result.filled_hole_count)?;
    output.set_item("skipped_hole_count", result.skipped_hole_count)?;
    output.set_item("added_fill_face_count", result.added_fill_face_count)?;
    output.set_item("max_hole_perimeter", result.max_hole_perimeter)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(
        point_cloud_triangulate_filled_candidate_mesh,
        module
    )?)?;
    Ok(())
}
