#[pyfunction(signature = (source, material_dir=None))]
fn mesh_from_obj(
    py: Python<'_>,
    source: &[u8],
    material_dir: Option<&str>,
) -> PyResult<Py<PyDict>> {
    let material_dir = material_dir.map(PathBuf::from);
    let document = py
        .detach(|| match material_dir.as_ref() {
            Some(material_dir) => {
                zennah_geometry_core::mesh_from_obj_with_material_dir(source, material_dir)
            }
            None => zennah_geometry_core::mesh_from_obj(source),
        })
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vec3_lists(document.vertices))?;
    output.set_item(
        "faces",
        document
            .faces
            .into_iter()
            .map(|face| face.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("object_names", document.object_names)?;
    output.set_item("material_names", document.material_names)?;
    output.set_item("diffuse_color", document.diffuse_color)?;
    output.set_item("texture_files", document.texture_files)?;
    output.set_item(
        "tri_corner_uvs",
        document
            .tri_corner_uvs
            .into_iter()
            .map(|tri| tri.into_iter().map(|uv| uv.to_vec()).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "texture_images",
        document
            .texture_images
            .into_iter()
            .map(|texture| {
                let output = PyDict::new(py);
                output.set_item("file", texture.file)?;
                output.set_item("resolved_path", texture.resolved_path)?;
                output.set_item("width", texture.width)?;
                output.set_item("height", texture.height)?;
                output.set_item("filter", texture.filter)?;
                output.set_item("wrap", texture.wrap)?;
                output.set_item(
                    "pixels_rgba",
                    texture
                        .pixels_rgba
                        .into_iter()
                        .map(|pixel| pixel.into_iter().map(i64::from).collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                )?;
                Ok(output.unbind())
            })
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item("texture_per_face", document.texture_per_face)?;
    Ok(output.unbind())
}

fn read_ply_texture_files(value: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    value
        .extract::<Vec<String>>()
        .map_err(|_| PyValueError::new_err("texture_files must be a list of strings"))
}

fn read_ply_colors(values: PyReadonlyArray2<'_, i64>) -> PyResult<Vec<[u8; 4]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] < 3 {
        return Err(PyValueError::new_err("PLY colors must have shape (n, >=3)"));
    }

    let mut output = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        output.push([
            clamp_ply_color(row[0]),
            clamp_ply_color(row[1]),
            clamp_ply_color(row[2]),
            255,
        ]);
    }
    Ok(output)
}

fn clamp_ply_color(value: i64) -> u8 {
    value.clamp(0, u8::MAX as i64) as u8
}
