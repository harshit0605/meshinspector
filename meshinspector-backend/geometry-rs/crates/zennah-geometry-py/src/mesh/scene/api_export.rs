#[pyfunction(signature = (
    object_name,
    child_index = 0,
    model_extension = ".ply",
    texture_images = None,
    texture_per_face = None,
    tri_corner_uvs = None,
    vertex_uvs = None
))]
fn meshlib_object_mesh_scene_payload(
    py: Python<'_>,
    object_name: &str,
    child_index: usize,
    model_extension: &str,
    texture_images: Option<&Bound<'_, PyAny>>,
    texture_per_face: Option<PyReadonlyArray1<'_, i64>>,
    tri_corner_uvs: Option<PyReadonlyArray3<'_, f64>>,
    vertex_uvs: Option<PyReadonlyArray2<'_, f64>>,
) -> PyResult<String> {
    let input = meshlib_scene_input_from_py(
        object_name,
        child_index,
        model_extension,
        texture_images,
        texture_per_face,
        tri_corner_uvs,
        vertex_uvs,
    )?;

    py.detach(|| zennah_geometry_core::meshlib_object_mesh_scene_json(&input))
        .map_err(PyValueError::new_err)
}

#[pyfunction(signature = (
    object_name,
    model_bytes,
    child_index = 0,
    model_extension = ".ply",
    texture_images = None,
    texture_per_face = None,
    tri_corner_uvs = None,
    vertex_uvs = None
))]
fn meshlib_object_mesh_mru_scene(
    py: Python<'_>,
    object_name: &str,
    model_bytes: &[u8],
    child_index: usize,
    model_extension: &str,
    texture_images: Option<&Bound<'_, PyAny>>,
    texture_per_face: Option<PyReadonlyArray1<'_, i64>>,
    tri_corner_uvs: Option<PyReadonlyArray3<'_, f64>>,
    vertex_uvs: Option<PyReadonlyArray2<'_, f64>>,
) -> PyResult<Py<PyBytes>> {
    let input = meshlib_scene_input_from_py(
        object_name,
        child_index,
        model_extension,
        texture_images,
        texture_per_face,
        tri_corner_uvs,
        vertex_uvs,
    )?;
    let archive = py
        .detach(|| zennah_geometry_core::meshlib_object_mesh_mru_scene_bytes(&input, model_bytes))
        .map_err(PyValueError::new_err)?;
    Ok(PyBytes::new(py, &archive).unbind())
}

#[pyfunction(signature = (
    root_name,
    root_key,
    vertices,
    faces,
    scene_objects,
    scene_line_objects = None,
    scene_point_objects = None,
    scene_distance_map_objects = None,
    scene_feature_objects = None,
    scene_voxel_objects = None,
    scene_child_order = None,
    scene_group_objects = None
))]
fn meshlib_multi_object_mru_scene(
    py: Python<'_>,
    root_name: &str,
    root_key: &str,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    scene_objects: &Bound<'_, PyAny>,
    scene_line_objects: Option<&Bound<'_, PyAny>>,
    scene_point_objects: Option<&Bound<'_, PyAny>>,
    scene_distance_map_objects: Option<&Bound<'_, PyAny>>,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
    scene_voxel_objects: Option<&Bound<'_, PyAny>>,
    scene_child_order: Option<&Bound<'_, PyAny>>,
    scene_group_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyBytes>> {
    let input = MeshlibSceneExportInput {
        root_name: root_name.to_owned(),
        root_key: root_key.to_owned(),
        vertices: read_vertices(vertices)?,
        faces: read_faces(faces)?,
        objects: read_scene_export_objects(scene_objects)?,
        group_objects: read_scene_group_objects(scene_group_objects)?,
        line_objects: read_scene_line_objects(scene_line_objects)?,
        point_objects: read_scene_point_objects(scene_point_objects)?,
        distance_map_objects: read_scene_distance_map_objects(scene_distance_map_objects)?,
        voxel_objects: read_scene_voxel_objects(scene_voxel_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
    };
    let scene_child_order = read_scene_child_order(scene_child_order)?;
    let archive = py
        .detach(|| {
            zennah_geometry_core::meshlib_multi_object_mru_scene_bytes_with_child_order(
                &input,
                &scene_child_order,
            )
        })
        .map_err(PyValueError::new_err)?;
    Ok(PyBytes::new(py, &archive).unbind())
}

