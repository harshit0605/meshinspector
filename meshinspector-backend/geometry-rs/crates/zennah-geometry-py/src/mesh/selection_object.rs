fn read_optional_vec2_f64(
    name: &str,
    values: Option<PyReadonlyArray2<'_, f64>>,
) -> PyResult<Option<Vec<[f64; 2]>>> {
    let Some(values) = values else {
        return Ok(None);
    };
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err(format!("{name} must have shape (n, 2)")));
    }
    Ok(Some(rows.outer_iter().map(|row| [row[0], row[1]]).collect()))
}

fn read_optional_rgba_u8(
    name: &str,
    values: Option<PyReadonlyArray2<'_, u8>>,
) -> PyResult<Option<Vec<[u8; 4]>>> {
    let Some(values) = values else {
        return Ok(None);
    };
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 4 {
        return Err(PyValueError::new_err(format!("{name} must have shape (n, 4)")));
    }
    Ok(Some(
        rows.outer_iter()
            .map(|row| [row[0], row[1], row[2], row[3]])
            .collect(),
    ))
}

fn vec2_selection_rows(values: Vec<[f64; 2]>) -> Vec<Vec<f64>> {
    values.into_iter().map(|value| value.to_vec()).collect()
}

fn rgba_selection_rows(values: Vec<[u8; 4]>) -> Vec<Vec<i64>> {
    values
        .into_iter()
        .map(|value| value.into_iter().map(i64::from).collect())
        .collect()
}

#[pyfunction(signature = (
    vertices,
    faces,
    selected_face_ids,
    vertex_uvs = None,
    vertex_colors = None,
    face_colors = None,
    texture_per_face = None
))]
fn extract_selected_faces_as_mesh(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    selected_face_ids: PyReadonlyArray1<'_, i64>,
    vertex_uvs: Option<PyReadonlyArray2<'_, f64>>,
    vertex_colors: Option<PyReadonlyArray2<'_, u8>>,
    face_colors: Option<PyReadonlyArray2<'_, u8>>,
    texture_per_face: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let selected = read_nonnegative_face_ids("selected_face_ids", selected_face_ids)?;
    let attributes = zennah_geometry_core::MeshSelectionAttributes {
        vertex_uvs: read_optional_vec2_f64("vertex_uvs", vertex_uvs)?,
        vertex_colors: read_optional_rgba_u8("vertex_colors", vertex_colors)?,
        face_colors: read_optional_rgba_u8("face_colors", face_colors)?,
        texture_per_face: texture_per_face.map(read_i64_values),
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::extract_selected_faces_as_mesh_with_attributes(
                &rust_vertices,
                &rust_faces,
                &selected,
                attributes,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("vertices", vec3_lists(result.vertices))?;
    output.set_item(
        "faces",
        result
            .faces
            .into_iter()
            .map(|face| face.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "source_vertex_indices",
        result
            .source_vertex_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "source_face_indices",
        result
            .source_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    if let Some(vertex_uvs) = result.vertex_uvs {
        output.set_item("vertex_uvs", vec2_selection_rows(vertex_uvs))?;
    }
    if let Some(vertex_colors) = result.vertex_colors {
        output.set_item("vertex_colors", rgba_selection_rows(vertex_colors))?;
    }
    if let Some(face_colors) = result.face_colors {
        output.set_item("face_colors", rgba_selection_rows(face_colors))?;
    }
    if let Some(texture_per_face) = result.texture_per_face {
        output.set_item("texture_per_face", texture_per_face)?;
    }
    Ok(output.unbind())
}
