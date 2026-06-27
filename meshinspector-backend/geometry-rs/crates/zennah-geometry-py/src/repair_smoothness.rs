use numpy::IntoPyArray;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::convert::{read_faces, read_vertices};

fn crease_edge_entry_dict(
    py: Python<'_>,
    edge: zennah_geometry_core::CreaseEdgeEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("edge", edge.edge)?;
    output.set_item("face_indices", edge.face_indices)?;
    output.set_item("dihedral_cosine", edge.dihedral_cosine)?;
    Ok(output.unbind())
}

fn crease_edge_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::CreaseEdgeDiagnostics,
) -> PyResult<Py<PyDict>> {
    let edges = PyList::empty(py);
    for edge in report.edges {
        edges.append(crease_edge_entry_dict(py, edge)?)?;
    }
    let output = PyDict::new(py);
    output.set_item(
        "angle_from_planar_radians",
        report.angle_from_planar_radians,
    )?;
    output.set_item("min_component_length_mm", report.min_component_length_mm)?;
    output.set_item("min_branch_length_mm", report.min_branch_length_mm)?;
    output.set_item("edge_count", report.edge_count)?;
    output.set_item("raw_crease_edge_count", report.raw_crease_edge_count)?;
    output.set_item("crease_edge_count", report.crease_edge_count)?;
    output.set_item("edges", edges)?;
    Ok(output.unbind())
}

fn crease_repair_plan_region_dict(
    py: Python<'_>,
    region: zennah_geometry_core::CreaseRepairPlanRegion,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("crease_edge", region.crease_edge)?;
    output.set_item("selected_origin_vertex", region.selected_origin_vertex)?;
    output.set_item("selected_face_indices", region.selected_face_indices)?;
    Ok(output.unbind())
}

fn crease_repair_plan_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::CreaseRepairPlanDiagnostics,
) -> PyResult<Py<PyDict>> {
    let regions = PyList::empty(py);
    for region in report.regions {
        regions.append(crease_repair_plan_region_dict(py, region)?)?;
    }
    let output = PyDict::new(py);
    output.set_item(
        "angle_from_planar_radians",
        report.angle_from_planar_radians,
    )?;
    output.set_item(
        "critical_tri_aspect_ratio",
        report.critical_tri_aspect_ratio,
    )?;
    output.set_item("crease_edge_count", report.crease_edge_count)?;
    output.set_item("planned_region_count", report.planned_region_count)?;
    output.set_item("planned_face_count", report.planned_face_count)?;
    output.set_item("regions", regions)?;
    Ok(output.unbind())
}

fn fix_mesh_creases_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::FixMeshCreasesReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    output.set_item("input_crease_edge_count", report.input_crease_edge_count)?;
    output.set_item("output_crease_edge_count", report.output_crease_edge_count)?;
    output.set_item("repaired_region_count", report.repaired_region_count)?;
    output.set_item("removed_face_count", report.removed_face_count)?;
    output.set_item("added_face_count", report.added_face_count)?;
    output.set_item("filled_hole_count", report.filled_hole_count)?;
    output.set_item("skipped_hole_count", report.skipped_hole_count)?;
    output.set_item("iteration_count", report.iteration_count)?;
    Ok(output.unbind())
}

fn fix_mesh_creases_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::FixMeshCreasesResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = fix_mesh_creases_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

fn not_smooth_face_entry_dict(
    py: Python<'_>,
    face: zennah_geometry_core::NotSmoothFaceEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("face_index", face.face_index)?;
    output.set_item("face", face.face)?;
    output.set_item("angle_delta_radians", face.angle_delta_radians)?;
    Ok(output.unbind())
}

fn not_smooth_face_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::NotSmoothFaceDiagnostics,
) -> PyResult<Py<PyDict>> {
    let faces = PyList::empty(py);
    for face in report.faces {
        faces.append(not_smooth_face_entry_dict(py, face)?)?;
    }
    let output = PyDict::new(py);
    output.set_item("min_angle_radians", report.min_angle_radians)?;
    output.set_item("face_count", report.face_count)?;
    output.set_item("not_smooth_face_count", report.not_smooth_face_count)?;
    output.set_item("faces", faces)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, min_angle_radians))]
fn not_smooth_face_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    min_angle_radians: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::not_smooth_face_diagnostics(
                &rust_vertices,
                &rust_faces,
                min_angle_radians,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    not_smooth_face_report_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, min_angle_radians))]
fn select_not_smooth_faces(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    min_angle_radians: f64,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::select_not_smooth_faces(
            &rust_vertices,
            &rust_faces,
            min_angle_radians,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, angle_from_planar_radians, min_component_length_mm=None, min_branch_length_mm=None))]
fn crease_edge_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    angle_from_planar_radians: f64,
    min_component_length_mm: Option<f64>,
    min_branch_length_mm: Option<f64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::crease_edge_diagnostics_with_filter(
                &rust_vertices,
                &rust_faces,
                angle_from_planar_radians,
                zennah_geometry_core::CreaseEdgeFilterOptions {
                    min_component_length_mm,
                    min_branch_length_mm,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    crease_edge_report_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, angle_from_planar_radians, critical_tri_aspect_ratio))]
fn crease_repair_plan_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    angle_from_planar_radians: f64,
    critical_tri_aspect_ratio: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::crease_repair_plan_diagnostics(
                &rust_vertices,
                &rust_faces,
                angle_from_planar_radians,
                critical_tri_aspect_ratio,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    crease_repair_plan_report_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, angle_from_planar_radians, critical_tri_aspect_ratio, max_iters))]
fn fix_mesh_creases(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    angle_from_planar_radians: f64,
    critical_tri_aspect_ratio: f64,
    max_iters: usize,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::fix_mesh_creases(
                &rust_vertices,
                &rust_faces,
                angle_from_planar_radians,
                critical_tri_aspect_ratio,
                max_iters,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    fix_mesh_creases_result_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(not_smooth_face_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(select_not_smooth_faces, module)?)?;
    module.add_function(wrap_pyfunction!(crease_edge_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(crease_repair_plan_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(fix_mesh_creases, module)?)?;
    Ok(())
}
