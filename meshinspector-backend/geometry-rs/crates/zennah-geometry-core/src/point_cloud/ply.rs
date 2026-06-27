use super::PointCloudPlyDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyEndian {
    Little,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyScalarType {
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Float,
    Double,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlyVertexProperty {
    ty: PlyScalarType,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlyHeader {
    format: PlyFormat,
    vertex_count: usize,
    vertex_properties: Vec<PlyVertexProperty>,
    data_offset: usize,
}

pub fn point_cloud_from_ply(bytes: &[u8]) -> Result<PointCloudPlyDocument, String> {
    let header = parse_header(bytes)?;
    match header.format {
        PlyFormat::Ascii => parse_ascii(bytes, &header),
        PlyFormat::BinaryLittleEndian => parse_binary(bytes, &header, PlyEndian::Little),
        PlyFormat::BinaryBigEndian => parse_binary(bytes, &header, PlyEndian::Big),
    }
}

pub fn point_cloud_to_ply(
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    colors: Option<&[[u8; 3]]>,
) -> Result<Vec<u8>, String> {
    validate_points(points)?;
    let normals = match normals {
        Some(normals) if !normals.is_empty() => {
            if normals.len() != points.len() {
                return Err("point cloud normals must match point count".to_string());
            }
            validate_points(normals)?;
            Some(normals)
        }
        _ => None,
    };
    let colors = match colors {
        Some(colors) if !colors.is_empty() => {
            if colors.len() != points.len() {
                return Err("point cloud colors must match point count".to_string());
            }
            Some(colors)
        }
        _ => None,
    };

    let mut header = format!(
        "ply\nformat binary_little_endian 1.0\ncomment MeshInspector.com\n\
element vertex {}\nproperty float x\nproperty float y\nproperty float z\n",
        points.len()
    );
    if normals.is_some() {
        header.push_str("property float nx\nproperty float ny\nproperty float nz\n");
    }
    if colors.is_some() {
        header.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\n");
    }
    header.push_str("end_header\n");

    let mut output = header.into_bytes();
    for (index, point) in points.iter().enumerate() {
        push_f32(&mut output, f64_to_f32(point[0])?);
        push_f32(&mut output, f64_to_f32(point[1])?);
        push_f32(&mut output, f64_to_f32(point[2])?);
        if let Some(normals) = normals {
            let normal = normals[index];
            push_f32(&mut output, f64_to_f32(normal[0])?);
            push_f32(&mut output, f64_to_f32(normal[1])?);
            push_f32(&mut output, f64_to_f32(normal[2])?);
        }
        if let Some(colors) = colors {
            output.extend_from_slice(&colors[index]);
        }
    }
    Ok(output)
}

fn parse_header(bytes: &[u8]) -> Result<PlyHeader, String> {
    let marker = b"end_header";
    let marker_start = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or_else(|| "PLY header is missing end_header".to_string())?;
    let line_end = bytes[marker_start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|offset| marker_start + offset + 1)
        .ok_or_else(|| "PLY header is not terminated".to_string())?;
    let header_text = std::str::from_utf8(&bytes[..line_end])
        .map_err(|_| "PLY header must be ASCII".to_string())?;
    let mut lines = header_text.lines();
    if lines.next().map(str::trim) != Some("ply") {
        return Err("Point cloud artifact is not a PLY file".to_string());
    }

    let mut format = None;
    let mut vertex_count = 0usize;
    let mut vertex_properties = Vec::new();
    let mut active_element = "";
    for raw_line in lines {
        let parts = raw_line.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() || parts[0] == "comment" {
            continue;
        }
        match parts[0] {
            "format" if parts.len() >= 2 => {
                format = Some(match parts[1] {
                    "ascii" => PlyFormat::Ascii,
                    "binary_little_endian" => PlyFormat::BinaryLittleEndian,
                    "binary_big_endian" => PlyFormat::BinaryBigEndian,
                    other => return Err(format!("Unsupported point cloud PLY format: {other}")),
                });
            }
            "element" if parts.len() >= 3 => {
                active_element = parts[1];
                if active_element == "vertex" {
                    vertex_count = parts[2].parse::<usize>().map_err(|_| {
                        "Point cloud PLY vertex count must be non-negative".to_string()
                    })?;
                }
            }
            "property" if active_element == "vertex" => {
                if parts.get(1) == Some(&"list") {
                    return Err(
                        "Point cloud PLY vertex list properties are unsupported".to_string()
                    );
                }
                if parts.len() < 3 {
                    return Err("Point cloud PLY vertex properties are malformed".to_string());
                }
                vertex_properties.push(PlyVertexProperty {
                    ty: parse_scalar_type(parts[1])?,
                    name: parts[2].to_string(),
                });
            }
            _ => {}
        }
    }
    if vertex_properties.is_empty() {
        return Err("Point cloud PLY vertex properties are missing".to_string());
    }
    Ok(PlyHeader {
        format: format.ok_or_else(|| "Point cloud PLY format is missing".to_string())?,
        vertex_count,
        vertex_properties,
        data_offset: line_end,
    })
}

fn parse_ascii(bytes: &[u8], header: &PlyHeader) -> Result<PointCloudPlyDocument, String> {
    let payload = std::str::from_utf8(&bytes[header.data_offset..])
        .map_err(|_| "Point cloud PLY ASCII payload must be UTF-8".to_string())?;
    let indices = property_indices(&header.vertex_properties)?;
    let mut points = Vec::with_capacity(header.vertex_count);
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    for row in payload.lines().take(header.vertex_count) {
        let values = row.split_whitespace().collect::<Vec<_>>();
        if values.len() < header.vertex_properties.len() {
            return Err("Point cloud PLY vertex row has too few values".to_string());
        }
        points.push([
            parse_ascii_f64(values[indices.xyz[0]])?,
            parse_ascii_f64(values[indices.xyz[1]])?,
            parse_ascii_f64(values[indices.xyz[2]])?,
        ]);
        if let Some(normal_indices) = indices.normals {
            normals.push([
                parse_ascii_f64(values[normal_indices[0]])?,
                parse_ascii_f64(values[normal_indices[1]])?,
                parse_ascii_f64(values[normal_indices[2]])?,
            ]);
        }
        if let Some(color_indices) = indices.colors {
            colors.push([
                parse_ascii_u8(values[color_indices[0]])?,
                parse_ascii_u8(values[color_indices[1]])?,
                parse_ascii_u8(values[color_indices[2]])?,
            ]);
        }
    }
    validate_loaded_document(points, normals, colors)
}

fn parse_binary(
    bytes: &[u8],
    header: &PlyHeader,
    endian: PlyEndian,
) -> Result<PointCloudPlyDocument, String> {
    let indices = property_indices(&header.vertex_properties)?;
    let row_size = header
        .vertex_properties
        .iter()
        .map(|property| property.ty.size())
        .sum::<usize>();
    let expected = header
        .data_offset
        .checked_add(header.vertex_count.saturating_mul(row_size))
        .ok_or_else(|| "Point cloud PLY payload is too large".to_string())?;
    if bytes.len() < expected {
        return Err("Point cloud PLY binary payload is truncated".to_string());
    }

    let mut cursor = header.data_offset;
    let mut points = Vec::with_capacity(header.vertex_count);
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    for _ in 0..header.vertex_count {
        let row_start = cursor;
        let values = header
            .vertex_properties
            .iter()
            .map(|property| {
                let value = read_binary_scalar(bytes, &mut cursor, property.ty, endian)?;
                Ok(value)
            })
            .collect::<Result<Vec<_>, String>>()?;
        if cursor != row_start + row_size {
            return Err("Point cloud PLY binary row size mismatch".to_string());
        }
        points.push([
            values[indices.xyz[0]],
            values[indices.xyz[1]],
            values[indices.xyz[2]],
        ]);
        if let Some(normal_indices) = indices.normals {
            normals.push([
                values[normal_indices[0]],
                values[normal_indices[1]],
                values[normal_indices[2]],
            ]);
        }
        if let Some(color_indices) = indices.colors {
            colors.push([
                f64_to_u8(values[color_indices[0]])?,
                f64_to_u8(values[color_indices[1]])?,
                f64_to_u8(values[color_indices[2]])?,
            ]);
        }
    }
    validate_loaded_document(points, normals, colors)
}

#[derive(Debug, Clone, Copy)]
struct PropertyIndices {
    xyz: [usize; 3],
    normals: Option<[usize; 3]>,
    colors: Option<[usize; 3]>,
}

fn property_indices(properties: &[PlyVertexProperty]) -> Result<PropertyIndices, String> {
    let xyz = find_properties(properties, ["x", "y", "z"])
        .ok_or_else(|| "Point cloud PLY must include x/y/z vertex properties".to_string())?;
    let normals = find_properties(properties, ["nx", "ny", "nz"]);
    let colors = find_properties(properties, ["red", "green", "blue"])
        .or_else(|| find_properties(properties, ["r", "g", "b"]));
    Ok(PropertyIndices {
        xyz,
        normals,
        colors,
    })
}

fn find_properties<const N: usize>(
    properties: &[PlyVertexProperty],
    names: [&str; N],
) -> Option<[usize; N]> {
    let mut indices = [0usize; N];
    for (slot, name) in names.iter().enumerate() {
        indices[slot] = properties
            .iter()
            .position(|property| property.name == *name)?;
    }
    Some(indices)
}

fn parse_scalar_type(value: &str) -> Result<PlyScalarType, String> {
    match value {
        "char" | "int8" => Ok(PlyScalarType::Char),
        "uchar" | "uint8" | "unsigned_char" => Ok(PlyScalarType::UChar),
        "short" | "int16" => Ok(PlyScalarType::Short),
        "ushort" | "uint16" | "unsigned_short" => Ok(PlyScalarType::UShort),
        "int" | "int32" => Ok(PlyScalarType::Int),
        "uint" | "uint32" | "unsigned_int" => Ok(PlyScalarType::UInt),
        "float" | "float32" => Ok(PlyScalarType::Float),
        "double" | "float64" => Ok(PlyScalarType::Double),
        _ => Err("Unsupported point cloud PLY vertex scalar type".to_string()),
    }
}

fn parse_ascii_f64(value: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| "Point cloud PLY vertex value is invalid".to_string())?;
    if !parsed.is_finite() {
        return Err("Point cloud PLY vertex value must be finite".to_string());
    }
    Ok(parsed)
}

fn parse_ascii_u8(value: &str) -> Result<u8, String> {
    let parsed = parse_ascii_f64(value)?;
    f64_to_u8(parsed)
}

fn read_binary_scalar(
    bytes: &[u8],
    cursor: &mut usize,
    ty: PlyScalarType,
    endian: PlyEndian,
) -> Result<f64, String> {
    let size = ty.size();
    let end = cursor
        .checked_add(size)
        .ok_or_else(|| "Point cloud PLY binary cursor overflow".to_string())?;
    if end > bytes.len() {
        return Err("Point cloud PLY binary payload is truncated".to_string());
    }
    let slice = &bytes[*cursor..end];
    *cursor = end;
    let value = match (ty, endian) {
        (PlyScalarType::Char, _) => i8::from_ne_bytes([slice[0]]) as f64,
        (PlyScalarType::UChar, _) => u8::from_ne_bytes([slice[0]]) as f64,
        (PlyScalarType::Short, PlyEndian::Little) => {
            i16::from_le_bytes([slice[0], slice[1]]) as f64
        }
        (PlyScalarType::Short, PlyEndian::Big) => i16::from_be_bytes([slice[0], slice[1]]) as f64,
        (PlyScalarType::UShort, PlyEndian::Little) => {
            u16::from_le_bytes([slice[0], slice[1]]) as f64
        }
        (PlyScalarType::UShort, PlyEndian::Big) => u16::from_be_bytes([slice[0], slice[1]]) as f64,
        (PlyScalarType::Int, PlyEndian::Little) => {
            i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyScalarType::Int, PlyEndian::Big) => {
            i32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyScalarType::UInt, PlyEndian::Little) => {
            u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyScalarType::UInt, PlyEndian::Big) => {
            u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyScalarType::Float, PlyEndian::Little) => {
            f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyScalarType::Float, PlyEndian::Big) => {
            f32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyScalarType::Double, PlyEndian::Little) => f64::from_le_bytes([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ]),
        (PlyScalarType::Double, PlyEndian::Big) => f64::from_be_bytes([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ]),
    };
    if !value.is_finite() {
        return Err("Point cloud PLY vertex value must be finite".to_string());
    }
    Ok(value)
}

impl PlyScalarType {
    fn size(self) -> usize {
        match self {
            PlyScalarType::Char | PlyScalarType::UChar => 1,
            PlyScalarType::Short | PlyScalarType::UShort => 2,
            PlyScalarType::Int | PlyScalarType::UInt | PlyScalarType::Float => 4,
            PlyScalarType::Double => 8,
        }
    }
}

fn validate_loaded_document(
    points: Vec<[f64; 3]>,
    normals: Vec<[f64; 3]>,
    colors: Vec<[u8; 3]>,
) -> Result<PointCloudPlyDocument, String> {
    validate_points(&points)?;
    if !normals.is_empty() {
        if normals.len() != points.len() {
            return Err("point cloud normals must match point count".to_string());
        }
        validate_points(&normals)?;
    }
    if !colors.is_empty() && colors.len() != points.len() {
        return Err("point cloud colors must match point count".to_string());
    }
    Ok(PointCloudPlyDocument {
        points,
        normals,
        colors,
    })
}

fn validate_points(points: &[[f64; 3]]) -> Result<(), String> {
    for point in points {
        if !point.iter().all(|value| value.is_finite()) {
            return Err("point cloud coordinates must be finite".to_string());
        }
    }
    Ok(())
}

fn f64_to_f32(value: f64) -> Result<f32, String> {
    if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
        return Err("point cloud coordinate cannot be represented as float32".to_string());
    }
    Ok(value as f32)
}

fn f64_to_u8(value: f64) -> Result<u8, String> {
    if !value.is_finite() {
        return Err("point cloud color must be finite".to_string());
    }
    Ok(value.trunc().clamp(0.0, 255.0) as u8)
}

fn push_f32(output: &mut Vec<u8>, value: f32) {
    output.extend_from_slice(&value.to_le_bytes());
}
