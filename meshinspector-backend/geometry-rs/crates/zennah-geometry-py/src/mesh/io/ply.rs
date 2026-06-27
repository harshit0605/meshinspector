#[pyfunction(signature = (source, texture_dir=None))]
fn mesh_from_ply(py: Python<'_>, source: &[u8], texture_dir: Option<&str>) -> PyResult<Py<PyDict>> {
    let texture_dir = texture_dir.map(PathBuf::from);
    let document = py
        .detach(|| match texture_dir.as_ref() {
            Some(texture_dir) => {
                zennah_geometry_core::mesh_from_ply_with_textures(source, texture_dir)
            }
            None => zennah_geometry_core::mesh_from_ply(source),
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
    output.set_item(
        "vertex_colors",
        document
            .vertex_colors
            .into_iter()
            .map(|color| color.into_iter().map(i64::from).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "face_colors",
        document
            .face_colors
            .into_iter()
            .map(|color| color.into_iter().map(i64::from).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "vertex_uvs",
        document
            .vertex_uvs
            .into_iter()
            .map(|uv| uv.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("vertex_normals", vec3_lists(document.vertex_normals))?;
    output.set_item(
        "tri_corner_uvs",
        document
            .tri_corner_uvs
            .into_iter()
            .map(|tri| tri.into_iter().map(|uv| uv.to_vec()).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "edges",
        document
            .edges
            .into_iter()
            .map(|edge| edge.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("texture_files", document.texture_files)?;
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
    Ok(output.unbind())
}

#[pyfunction(signature = (
    vertices,
    faces,
    texture_files = None,
    vertex_uvs = None,
    tri_corner_uvs = None,
    vertex_colors = None,
    face_colors = None
))]
fn mesh_to_ply(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    texture_files: Option<&Bound<'_, PyAny>>,
    vertex_uvs: Option<PyReadonlyArray2<'_, f64>>,
    tri_corner_uvs: Option<PyReadonlyArray3<'_, f64>>,
    vertex_colors: Option<PyReadonlyArray2<'_, i64>>,
    face_colors: Option<PyReadonlyArray2<'_, i64>>,
) -> PyResult<Py<PyBytes>> {
    let document = zennah_geometry_core::MeshPlyDocument {
        vertices: read_vertices(vertices)?,
        faces: read_faces(faces)?,
        vertex_colors: vertex_colors
            .map(read_ply_colors)
            .transpose()?
            .unwrap_or_default(),
        face_colors: face_colors
            .map(read_ply_colors)
            .transpose()?
            .unwrap_or_default(),
        vertex_uvs: vertex_uvs.map(read_vertex_uvs).transpose()?.unwrap_or_default(),
        vertex_normals: Vec::new(),
        tri_corner_uvs: tri_corner_uvs
            .map(read_tri_corner_uvs)
            .transpose()?
            .unwrap_or_default(),
        edges: Vec::new(),
        texture_files: read_ply_texture_files(texture_files)?,
        texture_images: Vec::new(),
    };
    let output = py
        .detach(|| zennah_geometry_core::mesh_to_ply(&document))
        .map_err(PyValueError::new_err)?;
    Ok(PyBytes::new(py, &output).unbind())
}

