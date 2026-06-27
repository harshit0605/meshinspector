#[pyfunction(signature=(vertices, faces, control_vertices, close_path = false, max_path_len_mm = 1.7976931348623157e308))]
fn mesh_geodesic_polyline_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    control_vertices: PyReadonlyArray1<'_, i64>,
    close_path: bool,
    max_path_len_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let controls = read_nonnegative_face_ids("control_vertices", control_vertices)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_geodesic_polyline_path_with_close(
                &rust_vertices,
                &rust_faces,
                &controls,
                close_path,
                max_path_len_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "control_vertex_indices",
        path.control_vertex_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "control_vertex_offsets",
        path.control_vertex_offsets
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "vertex_indices",
        path.vertex_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("point_normals", vec3_lists(path.point_normals))?;
    output.set_item("edge_lengths", path.edge_lengths)?;
    output.set_item("leg_lengths", path.leg_lengths)?;
    output.set_item(
        "leg_vertex_offsets",
        path.leg_vertex_offsets
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item("line_segments", path.line_segments)?;
    output.set_item("closed_path", path.closed_path)?;
    output.set_item("meshlib_reference", "MR::buildShortestPath control polyline")?;
    Ok(output.unbind())
}

#[pyfunction(signature=(vertices, faces, control_vertices, close_path = false, max_path_len_mm = 1.7976931348623157e308))]
fn mesh_cut_measure_contours(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    control_vertices: PyReadonlyArray1<'_, i64>,
    close_path: bool,
    max_path_len_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let controls = read_nonnegative_face_ids("control_vertices", control_vertices)?;
    let payload = py
        .detach(|| {
            zennah_geometry_core::mesh_cut_measure_contours(
                &rust_vertices,
                &rust_faces,
                &controls,
                close_path,
                max_path_len_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("closed_path", payload.closed_path)?;
    output.set_item("contour_count", payload.contours.len())?;
    output.set_item("cut_result_count", payload.result_cut_vertex_indices.len())?;
    output.set_item(
        "path_vertex_indices",
        payload
            .path
            .vertex_indices
            .iter()
            .map(|index| *index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("path_points", vec3_lists(payload.path.points.clone()))?;
    output.set_item("edge_lengths", payload.path.edge_lengths.clone())?;
    output.set_item("length_mm", payload.path.length_mm)?;
    output.set_item("line_segments", payload.path.line_segments)?;
    output.set_item(
        "pivot_indices",
        payload
            .pivot_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "result_cut_vertex_indices",
        payload
            .result_cut_vertex_indices
            .into_iter()
            .map(|path| path.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "bad_face_indices",
        payload
            .bad_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    let contour_list = PyList::empty(py);
    for contour in payload.contours {
        let contour_dict = PyDict::new(py);
        contour_dict.set_item("closed", contour.closed)?;
        let intersection_list = PyList::empty(py);
        for intersection in contour.intersections {
            let intersection_dict = PyDict::new(py);
            intersection_dict.set_item("primitive_type", intersection.primitive_type)?;
            intersection_dict.set_item("primitive_id", intersection.primitive_id as i64)?;
            intersection_dict.set_item("coordinate", intersection.coordinate.to_vec())?;
            intersection_list.append(intersection_dict)?;
        }
        contour_dict.set_item("intersections", intersection_list)?;
        contour_list.append(contour_dict)?;
    }
    output.set_item("contours", contour_list)?;
    output.set_item("meshlib_reference", payload.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction(signature=(vertices, faces, control_vertices, close_path = false, max_path_len_mm = 1.7976931348623157e308))]
fn mesh_cut_measure_edge_path_topology_cut(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    control_vertices: PyReadonlyArray1<'_, i64>,
    close_path: bool,
    max_path_len_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let controls = read_nonnegative_face_ids("control_vertices", control_vertices)?;
    let payload = py
        .detach(|| {
            zennah_geometry_core::mesh_cut_measure_edge_path_topology_cut(
                &rust_vertices,
                &rust_faces,
                &controls,
                close_path,
                max_path_len_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("vertices", vec3_lists(payload.vertices))?;
    output.set_item(
        "faces",
        payload
            .faces
            .into_iter()
            .map(|face| face.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "source_path_vertex_indices",
        payload
            .source_path_vertex_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "result_cut_vertex_indices",
        payload
            .result_cut_vertex_indices
            .into_iter()
            .map(|path| path.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "duplicate_vertex_map",
        payload
            .duplicate_vertex_map
            .into_iter()
            .map(|entry| entry.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "cut_edge_pairs",
        payload
            .cut_edge_pairs
            .into_iter()
            .map(|entry| entry.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "result_cut_edge_pairs",
        payload
            .result_cut_edge_pairs
            .into_iter()
            .map(|entry| entry.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "bad_face_indices",
        payload
            .bad_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("closed_path", payload.closed_path)?;
    output.set_item("length_mm", payload.length_mm)?;
    output.set_item("meshlib_reference", payload.meshlib_reference)?;
    Ok(output.unbind())
}
