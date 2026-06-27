use numpy::IntoPyArray;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::convert::{read_faces, read_vertices};

fn short_edge_entry_dict(
    py: Python<'_>,
    edge: zennah_geometry_core::ShortEdgeEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("edge", edge.edge)?;
    output.set_item("length_mm", edge.length_mm)?;
    Ok(output.unbind())
}

fn short_edge_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::ShortEdgeDiagnostics,
) -> PyResult<Py<PyDict>> {
    let edges = PyList::empty(py);
    for edge in report.edges {
        edges.append(short_edge_entry_dict(py, edge)?)?;
    }
    let output = PyDict::new(py);
    output.set_item("critical_length_mm", report.critical_length_mm)?;
    output.set_item("edge_count", report.edge_count)?;
    output.set_item("short_edge_count", report.short_edge_count)?;
    output.set_item("min_short_edge_length_mm", report.min_short_edge_length_mm)?;
    output.set_item("max_short_edge_length_mm", report.max_short_edge_length_mm)?;
    output.set_item("edges", edges)?;
    Ok(output.unbind())
}

fn degenerate_face_entry_dict(
    py: Python<'_>,
    face: zennah_geometry_core::DegenerateFaceEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("face_index", face.face_index)?;
    output.set_item("face", face.face)?;
    output.set_item("aspect_ratio", face.aspect_ratio)?;
    Ok(output.unbind())
}

fn degenerate_face_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::DegenerateFaceDiagnostics,
) -> PyResult<Py<PyDict>> {
    let faces = PyList::empty(py);
    for face in report.faces {
        faces.append(degenerate_face_entry_dict(py, face)?)?;
    }
    let output = PyDict::new(py);
    output.set_item("critical_aspect_ratio", report.critical_aspect_ratio)?;
    output.set_item("face_count", report.face_count)?;
    output.set_item("degenerate_face_count", report.degenerate_face_count)?;
    output.set_item(
        "min_degenerate_aspect_ratio",
        report.min_degenerate_aspect_ratio,
    )?;
    output.set_item(
        "max_degenerate_aspect_ratio",
        report.max_degenerate_aspect_ratio,
    )?;
    output.set_item("faces", faces)?;
    Ok(output.unbind())
}

fn multiple_edge_entry_dict(
    py: Python<'_>,
    edge: zennah_geometry_core::MultipleEdgeEntry,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("vertex_pair", edge.vertex_pair)?;
    output.set_item("topology_edge_count", edge.topology_edge_count)?;
    output.set_item("face_edge_occurrences", edge.face_edge_occurrences)?;
    output.set_item("forward_occurrences", edge.forward_occurrences)?;
    output.set_item("reverse_occurrences", edge.reverse_occurrences)?;
    Ok(output.unbind())
}

fn multiple_edge_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::MultipleEdgeDiagnostics,
) -> PyResult<Py<PyDict>> {
    let edges = PyList::empty(py);
    for edge in report.edges {
        edges.append(multiple_edge_entry_dict(py, edge)?)?;
    }
    let output = PyDict::new(py);
    output.set_item("edge_count", report.edge_count)?;
    output.set_item("multiple_edge_count", report.multiple_edge_count)?;
    output.set_item("edges", edges)?;
    Ok(output.unbind())
}

fn multiple_edge_repair_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::MultipleEdgeRepairReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_edge_count", report.input_edge_count)?;
    output.set_item("output_edge_count", report.output_edge_count)?;
    output.set_item(
        "input_multiple_edge_count",
        report.input_multiple_edge_count,
    )?;
    output.set_item(
        "output_multiple_edge_count",
        report.output_multiple_edge_count,
    )?;
    output.set_item("split_edge_count", report.split_edge_count)?;
    output.set_item("split_face_count", report.split_face_count)?;
    output.set_item("added_vertex_count", report.added_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    Ok(output.unbind())
}

fn multiple_edge_repair_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::MultipleEdgeRepairResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = multiple_edge_repair_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

fn duplicate_multi_hole_vertices_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::DuplicateMultiHoleVerticesReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item(
        "input_multi_hole_vertex_count",
        report.input_multi_hole_vertex_count,
    )?;
    output.set_item(
        "output_multi_hole_vertex_count",
        report.output_multi_hole_vertex_count,
    )?;
    output.set_item("duplicated_vertex_count", report.duplicated_vertex_count)?;
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("output_vertex_count", report.output_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    Ok(output.unbind())
}

fn duplicate_multi_hole_vertices_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::DuplicateMultiHoleVerticesResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = duplicate_multi_hole_vertices_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

fn nonmanifold_edge_repair_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::NonManifoldEdgeRepairReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item(
        "input_nonmanifold_edge_count",
        report.input_nonmanifold_edge_count,
    )?;
    output.set_item(
        "output_nonmanifold_edge_count",
        report.output_nonmanifold_edge_count,
    )?;
    output.set_item("removed_face_count", report.removed_face_count)?;
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("output_vertex_count", report.output_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    Ok(output.unbind())
}

fn nonmanifold_edge_repair_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::NonManifoldEdgeRepairResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = nonmanifold_edge_repair_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

