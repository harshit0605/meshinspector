use super::*;

pub(super) fn parse_ascii_vertex_element<'a>(
    rows: &mut impl Iterator<Item = &'a str>,
    element: &PlyElement,
) -> Result<PlyVertexMeshData, String> {
    let position_indices = property_indices(&element.properties, &["x", "y", "z"])?;
    let color_indices = first_property_indices(
        &element.properties,
        &[&["r", "g", "b"], &["red", "green", "blue"]],
    );
    let normal_indices = first_property_indices(&element.properties, &[&["nx", "ny", "nz"]]);
    let uv_indices = first_property_indices2(
        &element.properties,
        &[
            &["u", "v"],
            &["s", "t"],
            &["texture_u", "texture_v"],
            &["texture_s", "texture_t"],
        ],
    );
    let mut vertices = Vec::with_capacity(element.count);
    let mut colors = color_indices
        .is_some()
        .then(|| Vec::with_capacity(element.count))
        .unwrap_or_default();
    let mut uvs = uv_indices
        .is_some()
        .then(|| Vec::with_capacity(element.count))
        .unwrap_or_default();
    let mut normals = normal_indices
        .is_some()
        .then(|| Vec::with_capacity(element.count))
        .unwrap_or_default();

    for _ in 0..element.count {
        let values = parse_ascii_property_row(
            rows.next(),
            &element.properties,
            "PLY file read or parse error",
        )?;
        append_vertex_cells(
            &values,
            position_indices.as_slice(),
            color_indices,
            uv_indices,
            normal_indices,
            &mut vertices,
            &mut colors,
            &mut uvs,
            &mut normals,
        )?;
    }
    Ok(PlyVertexMeshData {
        vertices,
        colors,
        uvs,
        normals,
    })
}

pub(super) fn parse_ascii_face_element<'a>(
    rows: &mut impl Iterator<Item = &'a str>,
    element: &PlyElement,
) -> Result<PlyFaceMeshData, String> {
    let index_property =
        first_property_index(&element.properties, &["vertex_indices", "vertex_index"])?;
    let color_indices = first_property_indices(
        &element.properties,
        &[&["r", "g", "b"], &["red", "green", "blue"]],
    );
    let texcoord_index = first_property_index(&element.properties, &["texcoord"]).ok();
    let mut faces = Vec::with_capacity(element.count);
    let mut colors = color_indices
        .is_some()
        .then(|| Vec::with_capacity(element.count))
        .unwrap_or_default();
    let mut tri_corner_uvs = Vec::new();

    for _ in 0..element.count {
        let values = parse_ascii_property_row(
            rows.next(),
            &element.properties,
            "PLY file read or parse error",
        )?;
        append_face_cells(
            &values,
            index_property,
            color_indices,
            texcoord_index,
            &mut faces,
            &mut colors,
            &mut tri_corner_uvs,
        )?;
    }
    Ok(PlyFaceMeshData {
        faces,
        colors,
        tri_corner_uvs,
    })
}

pub(super) fn parse_ascii_edge_element<'a>(
    rows: &mut impl Iterator<Item = &'a str>,
    element: &PlyElement,
) -> Result<PlyEdgeMeshData, String> {
    let edge_indices = first_property_indices2(&element.properties, &[&["vertex1", "vertex2"]]);
    let mut edges = edge_indices
        .is_some()
        .then(|| Vec::with_capacity(element.count))
        .unwrap_or_default();

    for _ in 0..element.count {
        let values = parse_ascii_property_row(
            rows.next(),
            &element.properties,
            "PLY file read or parse error",
        )?;
        append_edge_cells(&values, edge_indices, &mut edges)?;
    }
    Ok(PlyEdgeMeshData { edges })
}

pub(super) fn parse_ascii_property_row(
    row: Option<&str>,
    properties: &[PlyProperty],
    error: &str,
) -> Result<Vec<PlyCell>, String> {
    let mut tokens = row.ok_or_else(|| error.to_string())?.split_whitespace();
    let mut values = Vec::with_capacity(properties.len());
    for property in properties {
        match property {
            PlyProperty::Scalar { .. } => {
                values.push(PlyCell::Scalar(
                    tokens.next().ok_or_else(|| error.to_string())?.to_string(),
                ));
            }
            PlyProperty::List { .. } => {
                let count_token = tokens.next().ok_or_else(|| error.to_string())?;
                let count = parse_i64(count_token, error)?;
                if count < 0 {
                    return Err(error.to_string());
                }
                let mut list = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    list.push(tokens.next().ok_or_else(|| error.to_string())?.to_string());
                }
                values.push(PlyCell::List(list));
            }
        }
    }
    Ok(values)
}

pub(super) fn skip_ascii_element<'a>(
    rows: &mut impl Iterator<Item = &'a str>,
    count: usize,
) -> Result<(), String> {
    for _ in 0..count {
        rows.next()
            .ok_or_else(|| "PLY file read or parse error".to_string())?;
    }
    Ok(())
}
