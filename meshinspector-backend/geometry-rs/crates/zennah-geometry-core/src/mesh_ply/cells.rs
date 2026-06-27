use super::*;

pub(super) fn append_vertex_cells(
    values: &[PlyCell],
    position_indices: &[usize],
    color_indices: Option<[usize; 3]>,
    uv_indices: Option<[usize; 2]>,
    normal_indices: Option<[usize; 3]>,
    vertices: &mut Vec<[f64; 3]>,
    colors: &mut Vec<[u8; 4]>,
    uvs: &mut Vec<[f64; 2]>,
    normals: &mut Vec<[f64; 3]>,
) -> Result<(), String> {
    vertices.push(meshlib_vector3f_point([
        parse_f64(
            scalar_cell(values, position_indices[0], "PLY file read or parse error")?,
            "PLY file read or parse error",
        )?,
        parse_f64(
            scalar_cell(values, position_indices[1], "PLY file read or parse error")?,
            "PLY file read or parse error",
        )?,
        parse_f64(
            scalar_cell(values, position_indices[2], "PLY file read or parse error")?,
            "PLY file read or parse error",
        )?,
    ])?);
    if let Some(indices) = color_indices {
        colors.push([
            parse_color(
                scalar_cell(values, indices[0], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            parse_color(
                scalar_cell(values, indices[1], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            parse_color(
                scalar_cell(values, indices[2], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            255,
        ]);
    }
    if let Some(indices) = uv_indices {
        uvs.push([
            meshlib_f32(parse_f64(
                scalar_cell(values, indices[0], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?)?,
            meshlib_f32(parse_f64(
                scalar_cell(values, indices[1], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?)?,
        ]);
    }
    if let Some(indices) = normal_indices {
        normals.push(meshlib_vector3f_point([
            parse_f64(
                scalar_cell(values, indices[0], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            parse_f64(
                scalar_cell(values, indices[1], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            parse_f64(
                scalar_cell(values, indices[2], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
        ])?);
    }
    Ok(())
}

pub(super) fn append_face_cells(
    values: &[PlyCell],
    index_property: usize,
    color_indices: Option<[usize; 3]>,
    texcoord_index: Option<usize>,
    faces: &mut Vec<[i64; 3]>,
    colors: &mut Vec<[u8; 4]>,
    tri_corner_uvs: &mut Vec<[[f64; 2]; 3]>,
) -> Result<(), String> {
    let indices = list_cell(values, index_property, "PLY file read or parse error")?;
    let indices = indices
        .iter()
        .map(|value| parse_i64(value, "PLY file read or parse error"))
        .collect::<Result<Vec<_>, _>>()?;
    if indices.len() >= 3 {
        for corner in 1..indices.len() - 1 {
            faces.push([indices[0], indices[corner], indices[corner + 1]]);
        }
    }
    if let Some(indices) = color_indices {
        let color = [
            parse_color(
                scalar_cell(values, indices[0], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            parse_color(
                scalar_cell(values, indices[1], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            parse_color(
                scalar_cell(values, indices[2], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            255,
        ];
        colors.push(color);
    }
    if let Some(index) = texcoord_index {
        let values = list_cell(values, index, "PLY file read or parse error")?;
        for chunk in values.chunks(6) {
            let mut packed = [[0.0_f64, 0.0_f64]; 3];
            for (offset, value) in chunk.iter().enumerate() {
                packed[offset / 2][offset % 2] =
                    meshlib_f32(parse_f64(value, "PLY file read or parse error")?)?;
            }
            tri_corner_uvs.push(packed);
        }
    }
    Ok(())
}

pub(super) fn append_edge_cells(
    values: &[PlyCell],
    edge_indices: Option<[usize; 2]>,
    edges: &mut Vec<[i64; 2]>,
) -> Result<(), String> {
    if let Some(indices) = edge_indices {
        edges.push([
            parse_i64(
                scalar_cell(values, indices[0], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
            parse_i64(
                scalar_cell(values, indices[1], "PLY file read or parse error")?,
                "PLY file read or parse error",
            )?,
        ]);
    }
    Ok(())
}

pub(super) fn property_indices(
    properties: &[PlyProperty],
    names: &[&str],
) -> Result<Vec<usize>, String> {
    names
        .iter()
        .map(|name| first_property_index(properties, &[*name]))
        .collect()
}

pub(super) fn first_property_indices(
    properties: &[PlyProperty],
    groups: &[&[&str]],
) -> Option<[usize; 3]> {
    groups.iter().find_map(|group| {
        let indices = property_indices(properties, group).ok()?;
        (indices.len() == 3).then_some([indices[0], indices[1], indices[2]])
    })
}

pub(super) fn first_property_indices2(
    properties: &[PlyProperty],
    groups: &[&[&str]],
) -> Option<[usize; 2]> {
    groups.iter().find_map(|group| {
        let indices = property_indices(properties, group).ok()?;
        (indices.len() == 2).then_some([indices[0], indices[1]])
    })
}

pub(super) fn first_property_index(
    properties: &[PlyProperty],
    names: &[&str],
) -> Result<usize, String> {
    for name in names {
        if let Some(index) = properties
            .iter()
            .position(|property| property.name() == *name)
        {
            return Ok(index);
        }
    }
    Err("unsupported .PLY mesh file".to_string())
}

pub(super) fn scalar_cell<'a>(
    values: &'a [PlyCell],
    index: usize,
    error: &str,
) -> Result<&'a str, String> {
    match values.get(index) {
        Some(PlyCell::Scalar(value)) => Ok(value.as_str()),
        _ => Err(error.to_string()),
    }
}

pub(super) fn list_cell<'a>(
    values: &'a [PlyCell],
    index: usize,
    error: &str,
) -> Result<&'a [String], String> {
    match values.get(index) {
        Some(PlyCell::List(value)) => Ok(value),
        _ => Err(error.to_string()),
    }
}

pub(super) fn parse_color(value: &str, error: &str) -> Result<u8, String> {
    let parsed = parse_f64(value, error)?;
    let truncated = parsed.trunc();
    if !(0.0..=u8::MAX as f64).contains(&truncated) {
        return Err(error.to_string());
    }
    Ok(truncated as u8)
}

pub(super) fn parse_i64(value: &str, error: &str) -> Result<i64, String> {
    if let Ok(parsed) = value.parse::<i64>() {
        return Ok(parsed);
    }
    let parsed = parse_f64(value, error)?;
    if !parsed.is_finite() {
        return Err(error.to_string());
    }
    Ok(parsed.trunc() as i64)
}

pub(super) fn parse_f64(value: &str, error: &str) -> Result<f64, String> {
    let parsed = value.parse::<f64>().map_err(|_| error.to_string())?;
    if !parsed.is_finite() {
        return Err(error.to_string());
    }
    Ok(parsed)
}

pub(super) fn meshlib_f32(value: f64) -> Result<f64, String> {
    let converted = value as f32;
    if !converted.is_finite() {
        return Err("PLY coordinate must fit MeshLib float storage".to_string());
    }
    Ok(converted as f64)
}

pub(super) fn meshlib_vector3f_point(point: [f64; 3]) -> Result<[f64; 3], String> {
    Ok([
        meshlib_f32(point[0])?,
        meshlib_f32(point[1])?,
        meshlib_f32(point[2])?,
    ])
}

pub(super) fn validate_mesh_faces(vertex_count: usize, faces: &[[i64; 3]]) -> Result<(), String> {
    for face in faces {
        for vertex in face {
            if *vertex < 0 || *vertex as usize >= vertex_count {
                return Err("vertex id is larger than total point coordinates".to_string());
            }
        }
    }
    Ok(())
}

pub(super) fn validate_mesh_edges(vertex_count: usize, edges: &[[i64; 2]]) -> Result<(), String> {
    for edge in edges {
        for vertex in edge {
            if *vertex < 0 || *vertex as usize >= vertex_count {
                return Err("vertex id is larger than total point coordinates".to_string());
            }
        }
    }
    Ok(())
}
