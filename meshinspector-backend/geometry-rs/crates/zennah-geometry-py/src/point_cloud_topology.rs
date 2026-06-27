use crate::point_cloud::{candidate_mesh_dict, read_point_cloud_inputs};
use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

#[pyfunction(signature = (points, radius = 0.0, num_neighbors = 16, boundary_angle = std::f64::consts::PI * 0.9, max_removes = 2_147_483_647, crit_angle = std::f64::consts::TAU, normals = None, untrusted_indices = None))]
fn point_cloud_triangulate_topology_candidate_mesh(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    untrusted_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let (rust_points, rust_normals, rust_untrusted_indices) =
        read_point_cloud_inputs(points, normals, untrusted_indices)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_triangulate_topology_candidate_mesh(
                &rust_points,
                radius,
                num_neighbors,
                boundary_angle,
                max_removes,
                crit_angle,
                rust_normals.as_deref(),
                &rust_untrusted_indices,
            )
        })
        .map_err(PyValueError::new_err)?;

    let output = candidate_mesh_dict(
        py,
        result.vertices,
        result.faces,
        result.repetition_counts,
        result.repeated_3_count,
        result.repeated_2_count,
    )?;
    for (key, value) in [
        ("candidate_face_count", result.candidate_face_count),
        (
            "topology_skipped_face_count",
            result.topology_skipped_face_count,
        ),
        (
            "topology_degenerate_face_count",
            result.topology_degenerate_face_count,
        ),
        (
            "topology_nonmanifold_edge_face_count",
            result.topology_nonmanifold_edge_face_count,
        ),
        (
            "topology_nonmanifold_vertex_face_count",
            result.topology_nonmanifold_vertex_face_count,
        ),
        (
            "topology_unsafe_retry_face_count",
            result.topology_unsafe_retry_face_count,
        ),
        (
            "removed_hole_complicating_face_count",
            result.removed_hole_complicating_face_count,
        ),
        (
            "output_repeated_boundary_vertex_count",
            result.output_repeated_boundary_vertex_count,
        ),
    ] {
        output.set_item(key, value)?;
    }
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(
        point_cloud_triangulate_topology_candidate_mesh,
        module
    )?)?;
    Ok(())
}
