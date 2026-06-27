use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use zennah_geometry_core::{FillHoleMetricMode, FillHoleMultipleEdgesResolveMode};

use crate::convert::{read_faces, read_vertices};

fn hole_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::HoleFillReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_holes", report.input_holes)?;
    output.set_item("filled_holes", report.filled_holes)?;
    output.set_item("added_vertices", report.added_vertices)?;
    output.set_item("added_faces", report.added_faces)?;
    output.set_item("new_face_indices", report.new_face_indices)?;
    output.set_item("skipped_holes", report.skipped_holes)?;
    Ok(output.unbind())
}

fn hole_fill_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::HoleFillResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = hole_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

fn parse_multiple_edges_resolve_mode(
    value: Option<&str>,
) -> PyResult<FillHoleMultipleEdgesResolveMode> {
    use FillHoleMultipleEdgesResolveMode as M;

    let normalized = value.unwrap_or("simple").trim().to_ascii_lowercase();
    match normalized.as_str() {
        "none" => Ok(M::None),
        "simple" => Ok(M::Simple),
        "strong" => Ok(M::Strong),
        other => Err(PyValueError::new_err(format!(
            "multiple_edges_resolve_mode must be 'none', 'simple', or 'strong', got {other}"
        ))),
    }
}

fn parse_fill_metric_mode(value: Option<&str>) -> PyResult<FillHoleMetricMode> {
    use FillHoleMetricMode as M;
    let normalized = value
        .unwrap_or("circumscribed")
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' '], "_");
    match normalized.as_str() {
        "circumscribed" => Ok(M::Circumscribed),
        "min_area" | "minarea" => Ok(M::MinArea),
        "edge_length" | "edgelength" => Ok(M::EdgeLength),
        "universal" => Ok(M::Universal),
        "max_dihedral_angle" | "maxdihedralangle" => Ok(M::MaxDihedralAngle),
        "parallel_plane" | "parallelplane" => Ok(M::ParallelPlane),
        "complex_fill" | "complexfill" => Ok(M::ComplexFill),
        "min_tri_angle" | "mintriangle" => Ok(M::MinTriAngle),
        "plane" => Ok(M::Plane),
        "plane_normalized" | "planenormalized" => Ok(M::PlaneNormalized),
        "complex_stitch" | "complexstitch" => Ok(M::ComplexStitch),
        "edge_length_stitch" | "edgelengthstitch" => Ok(M::EdgeLengthStitch),
        "vertical_stitch" | "verticalstitch" => Ok(M::VerticalStitch),
        "vertical_stitch_edge_based" | "verticalstitchedgebased" => Ok(M::VerticalStitchEdgeBased),
        other => Err(PyValueError::new_err(format!(
            "fill_metric must be 'circumscribed', 'min_area', 'edge_length', 'universal', 'max_dihedral_angle', 'parallel_plane', 'complex_fill', 'min_tri_angle', 'plane', 'plane_normalized', 'complex_stitch', 'edge_length_stitch', 'vertical_stitch', or 'vertical_stitch_edge_based', got {other}"
        ))),
    }
}

fn parse_fill_metric_up_dir(value: Option<Vec<f64>>) -> PyResult<Option<[f64; 3]>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "fill_metric_up_dir must contain exactly 3 floats, got {}",
            value.len()
        )));
    }
    let up_dir = [value[0], value[1], value[2]];
    let length_sq = up_dir[0] * up_dir[0] + up_dir[1] * up_dir[1] + up_dir[2] * up_dir[2];
    if length_sq <= 1.0e-16 {
        return Err(PyValueError::new_err(
            "fill_metric_up_dir must be a non-zero 3D vector",
        ));
    }
    Ok(Some(up_dir))
}

fn plan_entry_dict(
    py: Python<'_>,
    plan: zennah_geometry_core::HoleFillPlanEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("hole_index", plan.hole_index)?;
    output.set_item("representative_edge", plan.representative_edge)?;
    output.set_item("boundary_vertex_indices", plan.boundary_vertex_indices)?;
    output.set_item("boundary_edge_count", plan.boundary_edge_count)?;
    output.set_item("planned_triangles", plan.planned_triangles)?;
    output.set_item("skipped", plan.skipped)?;
    output.set_item("skip_reason", plan.skip_reason)?;
    Ok(output.unbind())
}

fn diagnostics_dict(
    py: Python<'_>,
    report: zennah_geometry_core::HoleFillPlanDiagnostics,
) -> PyResult<Py<PyDict>> {
    let plans = PyList::empty(py);
    for plan in report.plans {
        plans.append(plan_entry_dict(py, plan)?)?;
    }
    let output = PyDict::new(py);
    output.set_item("input_holes", report.input_holes)?;
    output.set_item("planned_holes", report.planned_holes)?;
    output.set_item("skipped_holes", report.skipped_holes)?;
    output.set_item("total_boundary_edges", report.total_boundary_edges)?;
    output.set_item("total_planned_triangles", report.total_planned_triangles)?;
    output.set_item("max_edges", report.max_edges)?;
    output.set_item("plans", plans)?;
    Ok(output.unbind())
}

fn repeated_vertex_entry_dict(
    py: Python<'_>,
    entry: zennah_geometry_core::RepeatedHoleBoundaryVertexEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("vertex_index", entry.vertex_index)?;
    output.set_item("hole_indices", entry.hole_indices)?;
    output.set_item("occurrences", entry.occurrences)?;
    Ok(output.unbind())
}

fn repeated_vertices_diagnostics_dict(
    py: Python<'_>,
    report: zennah_geometry_core::RepeatedHoleBoundaryVerticesDiagnostics,
) -> PyResult<Py<PyDict>> {
    let vertices = PyList::empty(py);
    for entry in report.vertices {
        vertices.append(repeated_vertex_entry_dict(py, entry)?)?;
    }
    let output = PyDict::new(py);
    output.set_item("input_holes", report.input_holes)?;
    output.set_item("repeated_vertex_count", report.repeated_vertex_count)?;
    output.set_item("vertices", vertices)?;
    Ok(output.unbind())
}

fn complicating_face_entry_dict(
    py: Python<'_>,
    entry: zennah_geometry_core::HoleComplicatingFaceEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("repeated_vertex_index", entry.repeated_vertex_index)?;
    output.set_item("face_index", entry.face_index)?;
    Ok(output.unbind())
}

fn complicating_faces_diagnostics_dict(
    py: Python<'_>,
    report: zennah_geometry_core::HoleComplicatingFacesDiagnostics,
) -> PyResult<Py<PyDict>> {
    let faces = PyList::empty(py);
    for entry in report.faces {
        faces.append(complicating_face_entry_dict(py, entry)?)?;
    }
    let output = PyDict::new(py);
    output.set_item(
        "input_repeated_vertex_count",
        report.input_repeated_vertex_count,
    )?;
    output.set_item("complicating_face_count", report.complicating_face_count)?;
    output.set_item("faces", faces)?;
    Ok(output.unbind())
}

fn remove_complicating_faces_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::RemoveHoleComplicatingFacesResult,
) -> PyResult<Py<PyDict>> {
    let report = PyDict::new(py);
    report.set_item("input_face_count", result.report.input_face_count)?;
    report.set_item("output_face_count", result.report.output_face_count)?;
    report.set_item("removed_face_count", result.report.removed_face_count)?;
    report.set_item(
        "input_repeated_vertex_count",
        result.report.input_repeated_vertex_count,
    )?;
    report.set_item(
        "output_repeated_vertex_count",
        result.report.output_repeated_vertex_count,
    )?;

    let output = PyDict::new(py);
    output.set_item("vertices", result.vertices)?;
    output.set_item("faces", result.faces)?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, max_edges = None))]
fn hole_fill_plan_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    max_edges: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::hole_fill_plan_diagnostics(&rust_vertices, &rust_faces, max_edges)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    diagnostics_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, max_edges = None))]
fn fill_planar_holes(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    max_edges: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::fill_planar_holes(&rust_vertices, &rust_faces, max_edges))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    hole_fill_result_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, max_edges = None, max_polygon_subdivisions = None, multiple_edges_resolve_mode = None, make_degenerate_band = false, stop_before_bad_triangulation = false, smooth_bd = true, fill_metric = None, fill_metric_up_dir = None))]
fn service_fill_holes(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    max_edges: Option<usize>,
    max_polygon_subdivisions: Option<usize>,
    multiple_edges_resolve_mode: Option<&str>,
    make_degenerate_band: bool,
    stop_before_bad_triangulation: bool,
    smooth_bd: bool,
    fill_metric: Option<&str>,
    fill_metric_up_dir: Option<Vec<f64>>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let mode = parse_multiple_edges_resolve_mode(multiple_edges_resolve_mode)?;
    let metric_mode = parse_fill_metric_mode(fill_metric)?;
    let metric_up_dir = parse_fill_metric_up_dir(fill_metric_up_dir)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::service_fill_holes_with_fill_params_and_metric_up_dir(
                &rust_vertices,
                &rust_faces,
                max_edges,
                max_polygon_subdivisions.unwrap_or(20),
                mode,
                make_degenerate_band,
                stop_before_bad_triangulation,
                smooth_bd,
                metric_mode,
                metric_up_dir,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    hole_fill_result_dict(py, result)
}

#[pyfunction]
fn repeated_hole_boundary_vertices_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::repeated_hole_boundary_vertices_diagnostics(
                &rust_vertices,
                &rust_faces,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    repeated_vertices_diagnostics_dict(py, result)
}

#[pyfunction]
fn hole_complicating_faces_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::hole_complicating_faces_diagnostics(&rust_vertices, &rust_faces)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    complicating_faces_diagnostics_dict(py, result)
}

#[pyfunction]
fn remove_hole_complicating_faces(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::remove_hole_complicating_faces(&rust_vertices, &rust_faces)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    remove_complicating_faces_result_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(hole_fill_plan_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(fill_planar_holes, module)?)?;
    module.add_function(wrap_pyfunction!(service_fill_holes, module)?)?;
    module.add_function(wrap_pyfunction!(
        repeated_hole_boundary_vertices_diagnostics,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        hole_complicating_faces_diagnostics,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(remove_hole_complicating_faces, module)?)?;
    Ok(())
}
