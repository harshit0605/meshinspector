fn scene_feature_render_object_to_py(
    py: Python<'_>,
    object: MeshlibSceneFeatureRenderObject,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("object_key", object.object_key)?;
    output.set_item("object_name", object.object_name)?;
    output.set_item("feature_type", object.feature_type)?;
    output.set_item("selected", object.selected)?;
    output.set_item("label", object.label)?;
    output.set_item("primary_points", vec3_lists(object.primary_points))?;
    output.set_item(
        "primary_polylines",
        object
            .primary_polylines
            .into_iter()
            .map(|polyline| scene_feature_render_polyline_to_py(py, polyline))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "primary_mesh_vertices",
        vec3_lists(object.primary_mesh_vertices),
    )?;
    output.set_item(
        "primary_mesh_faces",
        object
            .primary_mesh_faces
            .into_iter()
            .map(|face| face.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("subfeature_points", vec3_lists(object.subfeature_points))?;
    output.set_item(
        "subfeature_polylines",
        object
            .subfeature_polylines
            .into_iter()
            .map(|polyline| scene_feature_render_polyline_to_py(py, polyline))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "dimensions",
        object
            .dimensions
            .into_iter()
            .map(|dimension| scene_feature_render_dimension_to_py(py, dimension))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(output.unbind())
}

fn scene_feature_render_polyline_to_py(
    py: Python<'_>,
    polyline: MeshlibSceneFeatureRenderPolyline,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("points", vec3_lists(polyline.points))?;
    output.set_item("closed", polyline.closed)?;
    Ok(output.unbind())
}

fn scene_feature_render_dimension_to_py(
    py: Python<'_>,
    dimension: MeshlibSceneFeatureRenderDimension,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("kind", dimension.kind)?;
    output.set_item("points", vec3_lists(dimension.points))?;
    Ok(output.unbind())
}
