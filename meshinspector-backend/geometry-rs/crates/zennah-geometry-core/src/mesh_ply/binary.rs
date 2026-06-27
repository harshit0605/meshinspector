use super::*;

pub(super) fn parse_binary_vertex_element(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
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
        let values = parse_binary_property_row(bytes, cursor, format, &element.properties)?;
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

pub(super) fn parse_binary_face_element(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
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
        let values = parse_binary_property_row(bytes, cursor, format, &element.properties)?;
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

pub(super) fn parse_binary_edge_element(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
    element: &PlyElement,
) -> Result<PlyEdgeMeshData, String> {
    let edge_indices = first_property_indices2(&element.properties, &[&["vertex1", "vertex2"]]);
    let mut edges = edge_indices
        .is_some()
        .then(|| Vec::with_capacity(element.count))
        .unwrap_or_default();

    for _ in 0..element.count {
        let values = parse_binary_property_row(bytes, cursor, format, &element.properties)?;
        append_edge_cells(&values, edge_indices, &mut edges)?;
    }
    Ok(PlyEdgeMeshData { edges })
}

pub(super) fn parse_binary_property_row(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
    properties: &[PlyProperty],
) -> Result<Vec<PlyCell>, String> {
    let mut values = Vec::with_capacity(properties.len());
    for property in properties {
        match property {
            PlyProperty::Scalar { ty, .. } => {
                values.push(PlyCell::Scalar(read_binary_scalar_text(
                    bytes, cursor, format, *ty,
                )?));
            }
            PlyProperty::List {
                count_ty, item_ty, ..
            } => {
                let count = read_binary_scalar_i64(bytes, cursor, format, *count_ty)?;
                if count < 0 {
                    return Err("PLY file read or parse error".to_string());
                }
                let mut list = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    list.push(read_binary_scalar_text(bytes, cursor, format, *item_ty)?);
                }
                values.push(PlyCell::List(list));
            }
        }
    }
    Ok(values)
}

pub(super) fn skip_binary_element(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
    element: &PlyElement,
) -> Result<(), String> {
    for _ in 0..element.count {
        parse_binary_property_row(bytes, cursor, format, &element.properties)?;
    }
    Ok(())
}

pub(super) fn read_binary_scalar_text(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
    ty: PlyScalarType,
) -> Result<String, String> {
    Ok(match ty {
        PlyScalarType::Char => read_binary_i8(bytes, cursor)?.to_string(),
        PlyScalarType::UChar => read_binary_u8(bytes, cursor)?.to_string(),
        PlyScalarType::Short => read_binary_i16(bytes, cursor, format)?.to_string(),
        PlyScalarType::UShort => read_binary_u16(bytes, cursor, format)?.to_string(),
        PlyScalarType::Int => read_binary_i32(bytes, cursor, format)?.to_string(),
        PlyScalarType::UInt => read_binary_u32(bytes, cursor, format)?.to_string(),
        PlyScalarType::Float => read_binary_f32(bytes, cursor, format)?.to_string(),
        PlyScalarType::Double => read_binary_f64(bytes, cursor, format)?.to_string(),
    })
}

pub(super) fn read_binary_scalar_i64(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
    ty: PlyScalarType,
) -> Result<i64, String> {
    Ok(match ty {
        PlyScalarType::Char => read_binary_i8(bytes, cursor)? as i64,
        PlyScalarType::UChar => read_binary_u8(bytes, cursor)? as i64,
        PlyScalarType::Short => read_binary_i16(bytes, cursor, format)? as i64,
        PlyScalarType::UShort => read_binary_u16(bytes, cursor, format)? as i64,
        PlyScalarType::Int => read_binary_i32(bytes, cursor, format)? as i64,
        PlyScalarType::UInt => read_binary_u32(bytes, cursor, format)? as i64,
        PlyScalarType::Float => read_binary_f32(bytes, cursor, format)?.trunc() as i64,
        PlyScalarType::Double => read_binary_f64(bytes, cursor, format)?.trunc() as i64,
    })
}

pub(super) fn read_binary_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, String> {
    let data = take_binary(bytes, cursor, 1)?;
    Ok(data[0])
}

pub(super) fn read_binary_i8(bytes: &[u8], cursor: &mut usize) -> Result<i8, String> {
    Ok(read_binary_u8(bytes, cursor)? as i8)
}

pub(super) fn read_binary_i16(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
) -> Result<i16, String> {
    let data = take_binary_array::<2>(bytes, cursor)?;
    Ok(match format {
        PlyFormat::BinaryBigEndian => i16::from_be_bytes(data),
        _ => i16::from_le_bytes(data),
    })
}

pub(super) fn read_binary_u16(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
) -> Result<u16, String> {
    let data = take_binary_array::<2>(bytes, cursor)?;
    Ok(match format {
        PlyFormat::BinaryBigEndian => u16::from_be_bytes(data),
        _ => u16::from_le_bytes(data),
    })
}

pub(super) fn read_binary_i32(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
) -> Result<i32, String> {
    let data = take_binary_array::<4>(bytes, cursor)?;
    Ok(match format {
        PlyFormat::BinaryBigEndian => i32::from_be_bytes(data),
        _ => i32::from_le_bytes(data),
    })
}

pub(super) fn read_binary_u32(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
) -> Result<u32, String> {
    let data = take_binary_array::<4>(bytes, cursor)?;
    Ok(match format {
        PlyFormat::BinaryBigEndian => u32::from_be_bytes(data),
        _ => u32::from_le_bytes(data),
    })
}

pub(super) fn read_binary_f32(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
) -> Result<f32, String> {
    let data = take_binary_array::<4>(bytes, cursor)?;
    let value = match format {
        PlyFormat::BinaryBigEndian => f32::from_be_bytes(data),
        _ => f32::from_le_bytes(data),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("PLY file read or parse error".to_string())
    }
}

pub(super) fn read_binary_f64(
    bytes: &[u8],
    cursor: &mut usize,
    format: PlyFormat,
) -> Result<f64, String> {
    let data = take_binary_array::<8>(bytes, cursor)?;
    let value = match format {
        PlyFormat::BinaryBigEndian => f64::from_be_bytes(data),
        _ => f64::from_le_bytes(data),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("PLY file read or parse error".to_string())
    }
}

pub(super) fn take_binary_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], String> {
    let data = take_binary(bytes, cursor, N)?;
    data.try_into()
        .map_err(|_| "PLY file read or parse error".to_string())
}

pub(super) fn take_binary<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'a [u8], String> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| "PLY file read or parse error".to_string())?;
    let data = bytes
        .get(*cursor..end)
        .ok_or_else(|| "PLY file read or parse error".to_string())?;
    *cursor = end;
    Ok(data)
}
