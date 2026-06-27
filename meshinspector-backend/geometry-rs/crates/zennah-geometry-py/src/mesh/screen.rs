#[pyfunction(signature=(vertices, faces, view_projection_4x4, polygon_xy, include_backfaces=true, visible_only=false))]
fn select_faces_by_screen_polygon(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    view_projection_4x4: PyReadonlyArray1<'_, f64>,
    polygon_xy: PyReadonlyArray2<'_, f64>,
    include_backfaces: bool,
    visible_only: bool,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let view_projection_values = read_f64_values(view_projection_4x4);
    if view_projection_values.len() != 16 {
        return Err(PyValueError::new_err(
            "view_projection_4x4 must have shape (16,)",
        ));
    }
    let mut rust_view_projection = [0.0_f64; 16];
    rust_view_projection.copy_from_slice(&view_projection_values);
    let polygon_rows = polygon_xy.as_array();
    if polygon_rows.ndim() != 2 || polygon_rows.shape()[1] != 2 {
        return Err(PyValueError::new_err("polygon_xy must have shape (n, 2)"));
    }
    let mut rust_polygon = Vec::<[f64; 2]>::with_capacity(polygon_rows.shape()[0]);
    for row in polygon_rows.outer_iter() {
        rust_polygon.push([row[0], row[1]]);
    }
    py.detach(|| {
        zennah_geometry_core::select_faces_by_screen_polygon(
            &rust_vertices,
            &rust_faces,
            &rust_view_projection,
            &rust_polygon,
            include_backfaces,
            visible_only,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature=(vertices, faces, view_projection_4x4, rect_min_xy, rect_max_xy, include_backfaces=true, visible_only=false))]
fn select_faces_by_screen_rect(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    view_projection_4x4: PyReadonlyArray1<'_, f64>,
    rect_min_xy: PyReadonlyArray1<'_, f64>,
    rect_max_xy: PyReadonlyArray1<'_, f64>,
    include_backfaces: bool,
    visible_only: bool,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let view_projection_values = read_f64_values(view_projection_4x4);
    if view_projection_values.len() != 16 {
        return Err(PyValueError::new_err(
            "view_projection_4x4 must have shape (16,)",
        ));
    }
    let mut rust_view_projection = [0.0_f64; 16];
    rust_view_projection.copy_from_slice(&view_projection_values);

    let rect_min_values = read_f64_values(rect_min_xy);
    if rect_min_values.len() != 2 {
        return Err(PyValueError::new_err("rect_min_xy must have shape (2,)"));
    }
    let rect_max_values = read_f64_values(rect_max_xy);
    if rect_max_values.len() != 2 {
        return Err(PyValueError::new_err("rect_max_xy must have shape (2,)"));
    }

    py.detach(|| {
        zennah_geometry_core::select_faces_by_screen_rect(
            &rust_vertices,
            &rust_faces,
            &rust_view_projection,
            [rect_min_values[0], rect_min_values[1]],
            [rect_max_values[0], rect_max_values[1]],
            include_backfaces,
            visible_only,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn select_inside_part_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::select_inside_part_faces(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature=(vertices, faces, view_projection_4x4, brush_path_xy, radius_px, include_backfaces=true, visible_only=false))]
fn select_faces_by_screen_brush(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    view_projection_4x4: PyReadonlyArray1<'_, f64>,
    brush_path_xy: PyReadonlyArray2<'_, f64>,
    radius_px: f64,
    include_backfaces: bool,
    visible_only: bool,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let view_projection_values = read_f64_values(view_projection_4x4);
    if view_projection_values.len() != 16 {
        return Err(PyValueError::new_err(
            "view_projection_4x4 must have shape (16,)",
        ));
    }
    let mut rust_view_projection = [0.0_f64; 16];
    rust_view_projection.copy_from_slice(&view_projection_values);

    let brush_path_rows = brush_path_xy.as_array();
    if brush_path_rows.ndim() != 2 || brush_path_rows.shape()[1] != 2 {
        return Err(PyValueError::new_err(
            "brush_path_xy must have shape (n, 2)",
        ));
    }
    let mut rust_brush_path = Vec::<[f64; 2]>::with_capacity(brush_path_rows.shape()[0]);
    for row in brush_path_rows.outer_iter() {
        rust_brush_path.push([row[0], row[1]]);
    }

    py.detach(|| {
        zennah_geometry_core::select_faces_by_screen_brush(
            &rust_vertices,
            &rust_faces,
            &rust_view_projection,
            &rust_brush_path,
            radius_px,
            include_backfaces,
            visible_only,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

