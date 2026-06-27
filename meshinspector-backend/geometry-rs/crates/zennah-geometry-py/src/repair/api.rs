use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{parse_voxel_mesh_extractor, read_faces, read_vertices};

fn mesh_edit_dict(
    py: Python<'_>,
    result: zennah_geometry_core::MeshEditResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("changed_count", result.changed_count)?;
    Ok(output.unbind())
}

fn report_dict(py: Python<'_>, report: zennah_geometry_core::RepairReport) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_vertex_count", report.output_vertex_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    output.set_item("merged_vertices", report.merged_vertices)?;
    output.set_item("removed_degenerate_faces", report.removed_degenerate_faces)?;
    output.set_item(
        "removed_unreferenced_vertices",
        report.removed_unreferenced_vertices,
    )?;
    Ok(output.unbind())
}

fn voxel_rebuild_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::VoxelRebuildReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_vertex_count", report.output_vertex_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    output.set_item(
        "input_boundary_edge_count",
        report.input_boundary_edge_count,
    )?;
    output.set_item(
        "output_boundary_edge_count",
        report.output_boundary_edge_count,
    )?;
    output.set_item(
        "input_nonmanifold_edge_count",
        report.input_nonmanifold_edge_count,
    )?;
    output.set_item(
        "output_nonmanifold_edge_count",
        report.output_nonmanifold_edge_count,
    )?;
    output.set_item("input_self_intersections", report.input_self_intersections)?;
    output.set_item(
        "output_self_intersections",
        report.output_self_intersections,
    )?;
    output.set_item("voxel_size_mm", report.voxel_size_mm)?;
    output.set_item("offset_mm", report.offset_mm)?;
    output.set_item("extractor", report.extractor)?;
    output.set_item("refine", report.refine)?;
    Ok(output.unbind())
}

fn fix_self_intersections_relax_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::FixSelfIntersectionsRelaxReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_vertex_count", report.output_vertex_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    output.set_item("input_self_intersections", report.input_self_intersections)?;
    output.set_item(
        "output_self_intersections",
        report.output_self_intersections,
    )?;
    output.set_item("relaxed_face_count", report.relaxed_face_count)?;
    output.set_item("moved_vertex_count", report.moved_vertex_count)?;
    output.set_item("relax_iterations", report.relax_iterations)?;
    output.set_item("max_expand", report.max_expand)?;
    output.set_item("force", report.force)?;
    output.set_item("method", report.method)?;
    output.set_item(
        "subdivide_edge_len_disabled",
        report.subdivide_edge_len_disabled,
    )?;
    output.set_item("topology_changed", report.topology_changed)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, area_epsilon = 1e-12))]
fn remove_degenerate_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    area_epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::remove_degenerate_faces(&rust_vertices, &rust_faces, area_epsilon)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_edit_dict(py, result)
}

#[pyfunction]
fn remove_unreferenced_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::remove_unreferenced_vertices(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_edit_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, tolerance = 1e-6))]
fn merge_close_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    tolerance: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::merge_close_vertices(&rust_vertices, &rust_faces, tolerance)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_edit_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, close_dist = 0.0, unite_only_boundary = true))]
fn unite_close_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    close_dist: f64,
    unite_only_boundary: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::unite_close_vertices(
                &rust_vertices,
                &rust_faces,
                close_dist,
                unite_only_boundary,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_edit_dict(py, result)
}

#[pyfunction]
fn orient_faces_outward(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let oriented_faces = py
        .detach(|| zennah_geometry_core::orient_faces_outward(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output_faces: Vec<i64> = oriented_faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("faces", output_faces.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, merge_tolerance = 1e-6, area_epsilon = 1e-12))]
fn basic_repair(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    merge_tolerance: f64,
    area_epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::basic_repair(
                &rust_vertices,
                &rust_faces,
                merge_tolerance,
                area_epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    vertices,
    faces,
    relax_iterations = 5,
    max_expand = 3,
    touch_is_intersection = true,
    force = 0.5,
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn fix_self_intersections_relax(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    relax_iterations: usize,
    max_expand: usize,
    touch_is_intersection: bool,
    force: f64,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::fix_self_intersections_relax(
                &rust_vertices,
                &rust_faces,
                zennah_geometry_core::FixSelfIntersectionsRelaxOptions {
                    touch_is_intersection,
                    relax_iterations,
                    max_expand,
                    force,
                    epsilon,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = fix_self_intersections_relax_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}
#[pyfunction]
fn repaired_surface_area(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<f64> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::repaired_surface_area(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}
#[pyfunction]
fn ordered_boundary_loops(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let loops = py
        .detach(|| zennah_geometry_core::ordered_boundary_loops(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(loops
        .into_iter()
        .map(|boundary_loop| {
            boundary_loop
                .into_iter()
                .map(|value| value as i64)
                .collect()
        })
        .collect())
}
#[pyfunction(signature = (
    vertices,
    faces,
    voxel_size_mm,
    offset_mm = 0.0,
    padding_mm = None,
    extractor = "marching",
    refine = true
))]
#[allow(clippy::too_many_arguments)]
fn rebuild_via_sdf(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    voxel_size_mm: f64,
    offset_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_rebuild_via_sdf(
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
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = voxel_rebuild_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}
