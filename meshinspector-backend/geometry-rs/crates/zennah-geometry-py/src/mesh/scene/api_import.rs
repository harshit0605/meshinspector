#[pyfunction]
fn mesh_from_mru_scene(py: Python<'_>, source: &[u8]) -> PyResult<Py<PyDict>> {
    let document = py
        .detach(|| zennah_geometry_core::meshlib_object_mesh_document_from_mru_scene_bytes(source))
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
    output.set_item("root_file", document.root_file)?;
    output.set_item("root_key", document.root_key)?;
    output.set_item("object_name", document.object_name)?;
    output.set_item("object_key", document.object_key)?;
    output.set_item("model_file", document.model_file)?;
    output.set_item("model_extension", document.model_extension)?;
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
    output.set_item("vertex_normals_ply", vec3_lists(document.vertex_normals))?;
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
    output.set_item("texture_per_face", document.texture_per_face)?;
    output.set_item("object_names", document.object_names)?;
    output.set_item("material_names", document.material_names)?;
    output.set_item("diffuse_color", document.diffuse_color)?;
    output.set_item(
        "meshlib_uv_coordinates",
        document
            .meshlib_uv_coordinates
            .into_iter()
            .map(|uv| uv.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("texture_images", scene_texture_images_to_py(py, document.texture_images)?)?;
    output.set_item(
        "scene_objects",
        document
            .scene_objects
            .into_iter()
            .map(|scene_object| {
                let output = PyDict::new(py);
                output.set_item("object_name", scene_object.object_name)?;
                output.set_item("object_key", scene_object.object_key)?;
                output.set_item("parent_key", scene_object.parent_key)?;
                output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
                output.set_item("model_file", scene_object.model_file)?;
                output.set_item("model_extension", scene_object.model_extension)?;
                output.set_item("link", scene_object.link)?;
                output.set_item(
                    "shared_model_source_index",
                    scene_object.shared_model_source_index,
                )?;
                output.set_item("vertex_range", scene_object.vertex_range.to_vec())?;
                output.set_item("face_range", scene_object.face_range.to_vec())?;
                output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
                output.set_item("visibility_mask", scene_object.visibility_mask)?;
                output.set_item("selected", scene_object.selected)?;
                output.set_item("locked", scene_object.locked)?;
                output.set_item("parent_locked", scene_object.parent_locked)?;
                Ok(output.unbind())
            })
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_line_objects",
        document
            .scene_line_objects
            .into_iter()
            .map(|scene_object| scene_line_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_group_objects",
        document
            .scene_group_objects
            .into_iter()
            .map(|scene_object| scene_group_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_point_objects",
        document
            .scene_point_objects
            .into_iter()
            .map(|scene_object| scene_point_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_distance_map_objects",
        document
            .scene_distance_map_objects
            .into_iter()
            .map(|scene_object| scene_distance_map_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_voxel_objects",
        document
            .scene_voxel_objects
            .into_iter()
            .map(|scene_object| scene_voxel_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_feature_objects",
        document
            .scene_feature_objects
            .into_iter()
            .map(|scene_object| scene_feature_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_child_order",
        document
            .scene_child_order
            .into_iter()
            .map(|child_order| scene_child_order_to_py(py, child_order))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(output.unbind())
}

fn scene_xf_to_py(py: Python<'_>, xf: zennah_geometry_core::MeshlibSceneXf) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("row_x", xf.row_x.to_vec())?;
    output.set_item("row_y", xf.row_y.to_vec())?;
    output.set_item("row_z", xf.row_z.to_vec())?;
    output.set_item("b", xf.b.to_vec())?;
    Ok(output.unbind())
}

fn scene_texture_images_to_py(
    py: Python<'_>,
    textures: Vec<MeshlibSceneTextureImage>,
) -> PyResult<Vec<Py<PyDict>>> {
    textures
        .into_iter()
        .map(|texture| {
            let output = PyDict::new(py);
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
        .collect()
}
