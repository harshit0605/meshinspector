fn meshlib_scene_input_from_py(
    object_name: &str,
    child_index: usize,
    model_extension: &str,
    texture_images: Option<&Bound<'_, PyAny>>,
    texture_per_face: Option<PyReadonlyArray1<'_, i64>>,
    tri_corner_uvs: Option<PyReadonlyArray3<'_, f64>>,
    vertex_uvs: Option<PyReadonlyArray2<'_, f64>>,
) -> PyResult<MeshlibObjectMeshSceneInput> {
    Ok(MeshlibObjectMeshSceneInput {
        object_name: object_name.to_owned(),
        child_index,
        model_extension: model_extension.to_owned(),
        textures: read_texture_images(texture_images)?,
        texture_per_face: texture_per_face
            .map(read_texture_per_face)
            .transpose()?
            .unwrap_or_default(),
        tri_corner_uvs: tri_corner_uvs
            .map(read_tri_corner_uvs)
            .transpose()?
            .unwrap_or_default(),
        vertex_uvs: vertex_uvs.map(read_vertex_uvs).transpose()?.unwrap_or_default(),
    })
}

fn read_texture_per_face(values: PyReadonlyArray1<'_, i64>) -> PyResult<Vec<i64>> {
    let rows = values.as_array();
    if rows.ndim() != 1 {
        return Err(PyValueError::new_err("texture_per_face must have shape (n,)"));
    }
    Ok(rows.iter().copied().collect())
}

fn read_tri_corner_uvs(values: PyReadonlyArray3<'_, f64>) -> PyResult<Vec<[[f64; 2]; 3]>> {
    let rows = values.as_array();
    if rows.ndim() != 3 || rows.shape()[1] != 3 || rows.shape()[2] != 2 {
        return Err(PyValueError::new_err("tri_corner_uvs must have shape (n, 3, 2)"));
    }

    let mut output = Vec::with_capacity(rows.shape()[0]);
    for face_index in 0..rows.shape()[0] {
        output.push([
            [rows[[face_index, 0, 0]], rows[[face_index, 0, 1]]],
            [rows[[face_index, 1, 0]], rows[[face_index, 1, 1]]],
            [rows[[face_index, 2, 0]], rows[[face_index, 2, 1]]],
        ]);
    }
    Ok(output)
}

fn read_vertex_uvs(values: PyReadonlyArray2<'_, f64>) -> PyResult<Vec<[f64; 2]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err("vertex_uvs must have shape (n, 2)"));
    }

    let mut output = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        output.push([row[0], row[1]]);
    }
    Ok(output)
}
