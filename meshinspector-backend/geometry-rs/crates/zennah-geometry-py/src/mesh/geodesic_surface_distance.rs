fn read_edge_pairs(field: &str, values: PyReadonlyArray2<'_, i64>) -> PyResult<Vec<[i64; 2]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err(format!(
            "{field} must have shape (n, 2)"
        )));
    }
    Ok(rows.outer_iter().map(|row| [row[0], row[1]]).collect())
}

#[pyfunction(signature=(vertices, faces, start_vertices, end_vertices, max_distance_mm = 1.7976931348623157e308))]
fn mesh_closest_surface_path_targets(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_vertices: PyReadonlyArray1<'_, i64>,
    end_vertices: PyReadonlyArray1<'_, i64>,
    max_distance_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let starts = read_nonnegative_face_ids("start_vertices", start_vertices)?;
    let ends = read_nonnegative_face_ids("end_vertices", end_vertices)?;
    let targets = py
        .detach(|| {
            zennah_geometry_core::mesh_closest_surface_path_targets(
                &rust_vertices,
                &rust_faces,
                &starts,
                &ends,
                max_distance_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "start_vertices",
        targets
            .start_vertices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "end_vertices",
        targets
            .end_vertices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "target_vertices",
        targets
            .target_vertices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item("target_distances_mm", targets.target_distances_mm)?;
    output.set_item("distances_mm", targets.distances_mm)?;
    output.set_item(
        "predecessor_vertices",
        targets
            .predecessor_vertices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item("meshlib_reference", "MR::computeClosestSurfacePathTargets")?;
    Ok(output.unbind())
}

#[pyfunction]
fn mesh_surface_distance_seed_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    seed_vertices: PyReadonlyArray1<'_, i64>,
    seed_edges: PyReadonlyArray2<'_, i64>,
    seed_face_ids: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let seeds = read_nonnegative_face_ids("seed_vertices", seed_vertices)?;
    let edges = read_edge_pairs("seed_edges", seed_edges)?;
    let face_ids = read_nonnegative_face_ids("seed_face_ids", seed_face_ids)?;
    let sources = py
        .detach(|| {
            zennah_geometry_core::mesh_surface_distance_seed_vertices(
                &rust_vertices,
                &rust_faces,
                &seeds,
                &edges,
                &face_ids,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "seed_vertices",
        sources
            .seed_vertices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "selected_edges",
        sources
            .selected_edges
            .into_iter()
            .map(|edge| vec![edge[0] as i64, edge[1] as i64])
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "selected_face_indices",
        sources
            .selected_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "selected_face_boundary_edges",
        sources
            .selected_face_boundary_edges
            .into_iter()
            .map(|edge| vec![edge[0] as i64, edge[1] as i64])
            .collect::<Vec<_>>(),
    )?;
    output.set_item("meshlib_reference", sources.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction(signature=(vertices, faces, seed_vertices, iso_value_mm, max_distance_mm = 1.7976931348623157e308))]
fn mesh_geodesic_iso_region(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    seed_vertices: PyReadonlyArray1<'_, i64>,
    iso_value_mm: f64,
    max_distance_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let seeds = read_nonnegative_face_ids("seed_vertices", seed_vertices)?;
    let region = py
        .detach(|| {
            zennah_geometry_core::mesh_geodesic_iso_region(
                &rust_vertices,
                &rust_faces,
                &seeds,
                iso_value_mm,
                max_distance_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "seed_vertices",
        region
            .field
            .seed_vertices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("distances_mm", region.field.distances_mm)?;
    output.set_item(
        "predecessor_vertices",
        region
            .field
            .predecessor_vertices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item("reachable_vertex_count", region.field.reachable_vertex_count)?;
    output.set_item("max_distance_mm", region.field.max_distance_mm)?;
    output.set_item("iso_value_mm", region.iso_value_mm)?;
    output.set_item(
        "selected_vertex_indices",
        region
            .selected_vertex_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "selected_face_indices",
        region
            .selected_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "crossing_face_indices",
        region
            .crossing_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "boundary_edges",
        region
            .boundary_edges
            .into_iter()
            .map(|edge| vec![edge[0] as i64, edge[1] as i64])
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "iso_segments",
        region
            .iso_segments
            .into_iter()
            .map(|segment| vec![segment[0].to_vec(), segment[1].to_vec()])
            .collect::<Vec<_>>(),
    )?;
    output.set_item("clipped_vertices", vec3_lists(region.clipped_vertices))?;
    output.set_item(
        "clipped_faces",
        region
            .clipped_faces
            .into_iter()
            .map(|face| face.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "clipped_source_face_indices",
        region
            .clipped_source_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "clipped_source_vertex_indices",
        region
            .clipped_source_vertex_indices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "meshlib_reference",
        "MR::computeClosestSurfacePathTargets surface-distance iso",
    )?;
    Ok(output.unbind())
}
