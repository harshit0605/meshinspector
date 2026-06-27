use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

mod offset_contours;

#[pyfunction]
fn object_lines_from_contours(
    py: Python<'_>,
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
    line_width: f32,
    show_points: u32,
    smooth_connections: u32,
) -> PyResult<Py<PyDict>> {
    let contours = read_contours3(contour_points, contour_offsets)?;
    let options = zennah_geometry_core::lines::ObjectLinesOptions {
        line_width,
        show_points,
        smooth_connections,
        ..Default::default()
    };
    let document = py
        .detach(|| zennah_geometry_core::lines::object_lines_from_contours(&contours, options))
        .map_err(PyValueError::new_err)?;
    object_lines_to_dict(py, document)
}

#[pyfunction]
fn object_lines_to_contours(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    lines: PyReadonlyArray2<'_, i64>,
    line_width: f32,
    show_points: u32,
    smooth_connections: u32,
) -> PyResult<Py<PyDict>> {
    let document = zennah_geometry_core::lines::ObjectLinesDocument {
        points: read_points(points)?,
        lines: read_lines(lines)?,
        line_width,
        show_points,
        smooth_connections,
        ..Default::default()
    };
    let contours = py
        .detach(|| zennah_geometry_core::lines::object_lines_to_contours(&document))
        .map_err(PyValueError::new_err)?;
    let mut contour_points = Vec::new();
    let mut contour_offsets = Vec::with_capacity(contours.len() + 1);
    contour_offsets.push(0_i64);
    for contour in contours {
        contour_points.extend(contour);
        contour_offsets.push(contour_points.len() as i64);
    }
    let flat_points = contour_points.into_iter().flatten().collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("contour_points", flat_points.into_pyarray(py))?;
    output.set_item("contour_offsets", contour_offsets.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction]
fn object_lines_to_pts(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    lines: PyReadonlyArray2<'_, i64>,
    line_width: f32,
    show_points: u32,
    smooth_connections: u32,
) -> PyResult<String> {
    let document = read_document(points, lines, line_width, show_points, smooth_connections)?;
    py.detach(|| zennah_geometry_core::lines::object_lines_to_pts(&document))
        .map_err(PyValueError::new_err)
}

#[pyfunction]
fn object_lines_from_pts(py: Python<'_>, source: &str) -> PyResult<Py<PyDict>> {
    let document = py
        .detach(|| zennah_geometry_core::lines::object_lines_from_pts(source))
        .map_err(PyValueError::new_err)?;
    object_lines_to_dict(py, document)
}

#[pyfunction]
fn object_lines_to_dxf(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    lines: PyReadonlyArray2<'_, i64>,
    line_width: f32,
    show_points: u32,
    smooth_connections: u32,
) -> PyResult<String> {
    let document = read_document(points, lines, line_width, show_points, smooth_connections)?;
    py.detach(|| zennah_geometry_core::lines::object_lines_to_dxf(&document))
        .map_err(PyValueError::new_err)
}

#[pyfunction]
fn object_lines_to_mrlines(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    lines: PyReadonlyArray2<'_, i64>,
    line_width: f32,
    show_points: u32,
    smooth_connections: u32,
) -> PyResult<Py<PyBytes>> {
    let document = read_document(points, lines, line_width, show_points, smooth_connections)?;
    let bytes = py
        .detach(|| zennah_geometry_core::lines::object_lines_to_mrlines(&document))
        .map_err(PyValueError::new_err)?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

#[pyfunction]
fn object_lines_from_mrlines(py: Python<'_>, source: &[u8]) -> PyResult<Py<PyDict>> {
    let document = py
        .detach(|| zennah_geometry_core::lines::object_lines_from_mrlines(source))
        .map_err(PyValueError::new_err)?;
    object_lines_to_dict(py, document)
}

#[pyfunction]
fn object_lines_to_ply(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    lines: PyReadonlyArray2<'_, i64>,
    line_width: f32,
    show_points: u32,
    smooth_connections: u32,
    vert_colors: PyReadonlyArray2<'_, u8>,
) -> PyResult<Py<PyBytes>> {
    let mut document = read_document(points, lines, line_width, show_points, smooth_connections)?;
    document.vert_colors = read_colors(vert_colors)?;
    if !document.vert_colors.is_empty() {
        document.coloring_type = zennah_geometry_core::lines::ObjectLinesColoringType::PerVertex;
    }
    let bytes = py
        .detach(|| zennah_geometry_core::lines::object_lines_to_ply(&document))
        .map_err(PyValueError::new_err)?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

#[pyfunction]
fn object_lines_from_ply(py: Python<'_>, source: &[u8]) -> PyResult<Py<PyDict>> {
    let document = py
        .detach(|| zennah_geometry_core::lines::object_lines_from_ply(source))
        .map_err(PyValueError::new_err)?;
    object_lines_to_dict(py, document)
}

#[pyfunction]
fn object_lines_from_svg(py: Python<'_>, source: &str) -> PyResult<Py<PyDict>> {
    let document = py
        .detach(|| zennah_geometry_core::lines::object_lines_from_svg(source))
        .map_err(PyValueError::new_err)?;
    object_lines_to_dict(py, document)
}

fn object_lines_to_dict(
    py: Python<'_>,
    document: zennah_geometry_core::lines::ObjectLinesDocument,
) -> PyResult<Py<PyDict>> {
    let points = document.points.into_iter().flatten().collect::<Vec<_>>();
    let lines = document
        .lines
        .into_iter()
        .flat_map(|line| [line[0] as i64, line[1] as i64])
        .collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("points", points.into_pyarray(py))?;
    output.set_item("lines", lines.into_pyarray(py))?;
    output.set_item("show_points", document.show_points)?;
    output.set_item("smooth_connections", document.smooth_connections)?;
    output.set_item("line_width", document.line_width)?;
    output.set_item("coloring_type", coloring_type_name(document.coloring_type))?;
    output.set_item("line_colors", colors_to_rows(document.line_colors))?;
    output.set_item("vert_colors", colors_to_rows(document.vert_colors))?;
    output.set_item("uv_coords", uv_to_rows(document.uv_coords))?;
    output.set_item("texture_files", document.texture_files)?;
    Ok(output.unbind())
}

fn contours_to_dict(py: Python<'_>, contours: Vec<Vec<[f64; 3]>>) -> PyResult<Py<PyDict>> {
    let mut contour_points = Vec::new();
    let mut contour_offsets = Vec::with_capacity(contours.len() + 1);
    contour_offsets.push(0_i64);
    for contour in contours {
        contour_points.extend(contour);
        contour_offsets.push(contour_points.len() as i64);
    }
    let flat_points = contour_points.into_iter().flatten().collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("contour_points", flat_points.into_pyarray(py))?;
    output.set_item("contour_offsets", contour_offsets.into_pyarray(py))?;
    Ok(output.unbind())
}

fn read_contours3(
    contour_points: PyReadonlyArray2<'_, f64>,
    contour_offsets: PyReadonlyArray1<'_, i64>,
) -> PyResult<Vec<Vec<[f64; 3]>>> {
    let points = contour_points.as_array();
    if points.shape().len() != 2 || points.shape()[1] != 3 {
        return Err(PyValueError::new_err(
            "contour_points must have shape (n, 3)",
        ));
    }
    let offsets = contour_offsets.as_array();
    if offsets.len() < 2 || offsets[0] != 0 {
        return Err(PyValueError::new_err(
            "contour_offsets must start at 0 and contain at least two entries",
        ));
    }
    let point_count = points.shape()[0];
    let mut contours = Vec::with_capacity(offsets.len() - 1);
    for pair in offsets.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if start < 0 || end < start || end as usize > point_count {
            return Err(PyValueError::new_err(
                "contour_offsets must be sorted and within contour_points length",
            ));
        }
        contours.push(
            (start as usize..end as usize)
                .map(|index| [points[[index, 0]], points[[index, 1]], points[[index, 2]]])
                .collect(),
        );
    }
    Ok(contours)
}

fn read_points(points: PyReadonlyArray2<'_, f64>) -> PyResult<Vec<[f64; 3]>> {
    let points = points.as_array();
    if points.shape().len() != 2 || points.shape()[1] != 3 {
        return Err(PyValueError::new_err("points must have shape (n, 3)"));
    }
    Ok((0..points.shape()[0])
        .map(|index| [points[[index, 0]], points[[index, 1]], points[[index, 2]]])
        .collect())
}

fn read_lines(lines: PyReadonlyArray2<'_, i64>) -> PyResult<Vec<[usize; 2]>> {
    let lines = lines.as_array();
    if lines.shape().len() != 2 || lines.shape()[1] != 2 {
        return Err(PyValueError::new_err("lines must have shape (n, 2)"));
    }
    let mut output = Vec::with_capacity(lines.shape()[0]);
    for index in 0..lines.shape()[0] {
        let a = lines[[index, 0]];
        let b = lines[[index, 1]];
        if a < 0 || b < 0 {
            return Err(PyValueError::new_err(
                "ObjectLines line indices must be non-negative",
            ));
        }
        output.push([a as usize, b as usize]);
    }
    Ok(output)
}

fn read_colors(colors: PyReadonlyArray2<'_, u8>) -> PyResult<Vec<[u8; 4]>> {
    let colors = colors.as_array();
    if colors.shape().len() != 2 || colors.shape()[1] != 4 {
        return Err(PyValueError::new_err(
            "ObjectLines vert_colors must have shape (n, 4)",
        ));
    }
    Ok((0..colors.shape()[0])
        .map(|index| {
            [
                colors[[index, 0]],
                colors[[index, 1]],
                colors[[index, 2]],
                colors[[index, 3]],
            ]
        })
        .collect())
}

fn coloring_type_name(value: zennah_geometry_core::lines::ObjectLinesColoringType) -> &'static str {
    match value {
        zennah_geometry_core::lines::ObjectLinesColoringType::Solid => "Solid",
        zennah_geometry_core::lines::ObjectLinesColoringType::PerVertex => "PerVertex",
        zennah_geometry_core::lines::ObjectLinesColoringType::PerLine => "PerLine",
    }
}

fn colors_to_rows(colors: Vec<[u8; 4]>) -> Vec<Vec<i64>> {
    colors
        .into_iter()
        .map(|color| color.into_iter().map(i64::from).collect())
        .collect()
}

fn uv_to_rows(uv_coords: Vec<[f64; 2]>) -> Vec<Vec<f64>> {
    uv_coords.into_iter().map(Vec::from).collect()
}

fn read_document(
    points: PyReadonlyArray2<'_, f64>,
    lines: PyReadonlyArray2<'_, i64>,
    line_width: f32,
    show_points: u32,
    smooth_connections: u32,
) -> PyResult<zennah_geometry_core::lines::ObjectLinesDocument> {
    Ok(zennah_geometry_core::lines::ObjectLinesDocument {
        points: read_points(points)?,
        lines: read_lines(lines)?,
        line_width,
        show_points,
        smooth_connections,
        ..Default::default()
    })
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(object_lines_from_contours, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_to_contours, module)?)?;
    offset_contours::register(module)?;
    module.add_function(wrap_pyfunction!(object_lines_from_pts, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_to_pts, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_to_dxf, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_from_mrlines, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_to_mrlines, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_from_ply, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_from_svg, module)?)?;
    module.add_function(wrap_pyfunction!(object_lines_to_ply, module)?)?;
    Ok(())
}
