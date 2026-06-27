mod binary;
mod header;

use super::{validate_document, ObjectLinesColoringType, ObjectLinesDocument};
use binary::{
    binary_scalar_value, read_binary_property_row, skip_binary_property, PlyBinaryReader,
};
use header::parse_ply_header;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyBinaryEndian {
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
enum PlyProperty {
    Scalar {
        ty: PlyScalarType,
        name: String,
    },
    List {
        count_ty: PlyScalarType,
        item_ty: PlyScalarType,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlyElement {
    name: String,
    count: usize,
    properties: Vec<PlyProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlyHeader {
    format: PlyFormat,
    elements: Vec<PlyElement>,
    comments: Vec<String>,
    data_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct PlyVertexData {
    points: Vec<[f64; 3]>,
    colors: Option<Vec<[u8; 4]>>,
    uv_coords: Option<Vec<[f64; 2]>>,
}

pub fn object_lines_to_ply(document: &ObjectLinesDocument) -> Result<Vec<u8>, String> {
    validate_document(document)?;
    let save_colors = !document.vert_colors.is_empty();
    let mut output = format!(
        "ply\nformat binary_little_endian 1.0\ncomment MeshInspector.com\n\
element vertex {}\nproperty float x\nproperty float y\nproperty float z\n\
{}element edge {}\nproperty int vertex1\nproperty int vertex2\nend_header\n",
        document.points.len(),
        if save_colors {
            "property uchar red\nproperty uchar green\nproperty uchar blue\n"
        } else {
            ""
        },
        document.lines.len(),
    )
    .into_bytes();
    for (index, point) in document.points.iter().enumerate() {
        push_f32(&mut output, f64_to_f32(point[0])?);
        push_f32(&mut output, f64_to_f32(point[1])?);
        push_f32(&mut output, f64_to_f32(point[2])?);
        if save_colors {
            let color = document.vert_colors[index];
            output.extend_from_slice(&color[..3]);
        }
    }
    for line in &document.lines {
        push_i32(&mut output, line[0] as i32);
        push_i32(&mut output, line[1] as i32);
    }
    Ok(output)
}

pub fn object_lines_from_ply(bytes: &[u8]) -> Result<ObjectLinesDocument, String> {
    let header = parse_ply_header(bytes)?;
    let document = match header.format {
        PlyFormat::Ascii => parse_ascii_ply(bytes, &header)?,
        PlyFormat::BinaryLittleEndian => parse_binary_ply(bytes, &header, PlyBinaryEndian::Little)?,
        PlyFormat::BinaryBigEndian => parse_binary_ply(bytes, &header, PlyBinaryEndian::Big)?,
    };
    validate_document(&document)?;
    Ok(document)
}

fn parse_ascii_ply(bytes: &[u8], header: &PlyHeader) -> Result<ObjectLinesDocument, String> {
    let payload = std::str::from_utf8(&bytes[header.data_offset..])
        .map_err(|_| "unsupported .PLY file with polylines".to_string())?;
    let mut rows = payload.lines();
    let mut vertices = None;
    let mut lines = None;
    for element in &header.elements {
        match element.name.as_str() {
            "vertex" => vertices = Some(parse_ascii_vertex_element(&mut rows, element)?),
            "edge" => lines = Some(parse_ascii_edge_element(&mut rows, element)?),
            _ => skip_ascii_element(&mut rows, element.count)?,
        }
    }
    let vertices = vertices.ok_or_else(|| "unsupported .PLY file with polylines".to_string())?;
    let lines = meshlib_valid_lines(vertices.points.len(), &lines.unwrap_or_default());
    Ok(ObjectLinesDocument {
        points: vertices.points,
        lines,
        coloring_type: if vertices.colors.is_some() {
            ObjectLinesColoringType::PerVertex
        } else {
            ObjectLinesColoringType::Solid
        },
        vert_colors: vertices.colors.unwrap_or_default(),
        uv_coords: vertices.uv_coords.unwrap_or_default(),
        texture_files: meshlib_texture_files(&header.comments),
        ..ObjectLinesDocument::default()
    })
}

fn parse_ascii_vertex_element<'a>(
    rows: &mut impl Iterator<Item = &'a str>,
    element: &PlyElement,
) -> Result<PlyVertexData, String> {
    let position_indices = property_indices(&element.properties, &["x", "y", "z"])?;
    let color_indices = color_indices(&element.properties);
    let uv_indices = uv_indices(&element.properties);
    let mut points = Vec::with_capacity(element.count);
    let mut colors = color_indices.map(|_| Vec::with_capacity(element.count));
    let mut uv_coords = uv_indices.map(|_| Vec::with_capacity(element.count));
    for _ in 0..element.count {
        let values = parse_ascii_property_row(
            rows.next(),
            &element.properties,
            "Error reading points from PLY-format",
        )?;
        points.push(meshlib_vector3f_point([
            parse_ascii_position_f64(
                ascii_scalar_value(&values, position_indices[0]),
                &element.properties[position_indices[0]],
                "Error reading points from PLY-format",
            )?,
            parse_ascii_position_f64(
                ascii_scalar_value(&values, position_indices[1]),
                &element.properties[position_indices[1]],
                "Error reading points from PLY-format",
            )?,
            parse_ascii_position_f64(
                ascii_scalar_value(&values, position_indices[2]),
                &element.properties[position_indices[2]],
                "Error reading points from PLY-format",
            )?,
        ])?);
        if let (Some(indices), Some(colors)) = (color_indices, colors.as_mut()) {
            colors.push([
                parse_ascii_color_u8(
                    ascii_scalar_value(&values, indices[0]),
                    &element.properties[indices[0]],
                    "Error reading vertex colors from PLY-format",
                )?,
                parse_ascii_color_u8(
                    ascii_scalar_value(&values, indices[1]),
                    &element.properties[indices[1]],
                    "Error reading vertex colors from PLY-format",
                )?,
                parse_ascii_color_u8(
                    ascii_scalar_value(&values, indices[2]),
                    &element.properties[indices[2]],
                    "Error reading vertex colors from PLY-format",
                )?,
                255,
            ]);
        }
        if let (Some(indices), Some(uv_coords)) = (uv_indices, uv_coords.as_mut()) {
            uv_coords.push([
                parse_ascii_uv_f64(
                    ascii_scalar_value(&values, indices[0]),
                    &element.properties[indices[0]],
                    "Error reading texture coordinates from PLY-format",
                )?,
                parse_ascii_uv_f64(
                    ascii_scalar_value(&values, indices[1]),
                    &element.properties[indices[1]],
                    "Error reading texture coordinates from PLY-format",
                )?,
            ]);
        }
    }
    Ok(PlyVertexData {
        points,
        colors,
        uv_coords,
    })
}

fn parse_ascii_edge_element<'a>(
    rows: &mut impl Iterator<Item = &'a str>,
    element: &PlyElement,
) -> Result<Vec<[i64; 2]>, String> {
    let Ok(edge_indices) = property_indices(&element.properties, &["vertex1", "vertex2"]) else {
        skip_ascii_element(rows, element.count)?;
        return Ok(Vec::new());
    };
    let mut lines = Vec::with_capacity(element.count);
    for _ in 0..element.count {
        let values = parse_ascii_property_row(
            rows.next(),
            &element.properties,
            "Error reading edges from PLY-format",
        )?;
        lines.push([
            parse_ascii_edge_i64(
                ascii_scalar_value(&values, edge_indices[0]),
                &element.properties[edge_indices[0]],
                "Error reading edges from PLY-format",
            )?,
            parse_ascii_edge_i64(
                ascii_scalar_value(&values, edge_indices[1]),
                &element.properties[edge_indices[1]],
                "Error reading edges from PLY-format",
            )?,
        ]);
    }
    Ok(lines)
}

fn skip_ascii_element<'a>(
    rows: &mut impl Iterator<Item = &'a str>,
    count: usize,
) -> Result<(), String> {
    for _ in 0..count {
        rows.next()
            .ok_or_else(|| "unsupported .PLY file with polylines".to_string())?;
    }
    Ok(())
}

fn parse_binary_ply(
    bytes: &[u8],
    header: &PlyHeader,
    endian: PlyBinaryEndian,
) -> Result<ObjectLinesDocument, String> {
    let mut reader = PlyBinaryReader::new(&bytes[header.data_offset..], endian);
    let mut vertices = None;
    let mut lines = None;
    for element in &header.elements {
        match element.name.as_str() {
            "vertex" => vertices = Some(parse_binary_vertex_element(&mut reader, element)?),
            "edge" => lines = Some(parse_binary_edge_element(&mut reader, element)?),
            _ => skip_binary_element(&mut reader, element)?,
        }
    }
    let vertices = vertices.ok_or_else(|| "unsupported .PLY file with polylines".to_string())?;
    let lines = meshlib_valid_lines(vertices.points.len(), &lines.unwrap_or_default());
    Ok(ObjectLinesDocument {
        points: vertices.points,
        lines,
        coloring_type: if vertices.colors.is_some() {
            ObjectLinesColoringType::PerVertex
        } else {
            ObjectLinesColoringType::Solid
        },
        vert_colors: vertices.colors.unwrap_or_default(),
        uv_coords: vertices.uv_coords.unwrap_or_default(),
        texture_files: meshlib_texture_files(&header.comments),
        ..ObjectLinesDocument::default()
    })
}

fn parse_binary_vertex_element(
    reader: &mut PlyBinaryReader<'_>,
    element: &PlyElement,
) -> Result<PlyVertexData, String> {
    let position_indices = property_indices(&element.properties, &["x", "y", "z"])?;
    let color_indices = color_indices(&element.properties);
    let uv_indices = uv_indices(&element.properties);
    let mut points = Vec::with_capacity(element.count);
    let mut colors = color_indices.map(|_| Vec::with_capacity(element.count));
    let mut uv_coords = uv_indices.map(|_| Vec::with_capacity(element.count));
    for _ in 0..element.count {
        let values = read_binary_property_row(
            reader,
            &element.properties,
            "Error reading points from PLY-format",
        )?;
        points.push(meshlib_vector3f_point([
            binary_scalar_value(
                &values,
                position_indices[0],
                "Error reading points from PLY-format",
            )?
            .as_f64(),
            binary_scalar_value(
                &values,
                position_indices[1],
                "Error reading points from PLY-format",
            )?
            .as_f64(),
            binary_scalar_value(
                &values,
                position_indices[2],
                "Error reading points from PLY-format",
            )?
            .as_f64(),
        ])?);
        if let (Some(indices), Some(colors)) = (color_indices, colors.as_mut()) {
            colors.push([
                binary_scalar_value(
                    &values,
                    indices[0],
                    "Error reading vertex colors from PLY-format",
                )?
                .as_u8("Error reading vertex colors from PLY-format")?,
                binary_scalar_value(
                    &values,
                    indices[1],
                    "Error reading vertex colors from PLY-format",
                )?
                .as_u8("Error reading vertex colors from PLY-format")?,
                binary_scalar_value(
                    &values,
                    indices[2],
                    "Error reading vertex colors from PLY-format",
                )?
                .as_u8("Error reading vertex colors from PLY-format")?,
                255,
            ]);
        }
        if let (Some(indices), Some(uv_coords)) = (uv_indices, uv_coords.as_mut()) {
            uv_coords.push([
                parse_binary_uv_f64(
                    binary_scalar_value(
                        &values,
                        indices[0],
                        "Error reading texture coordinates from PLY-format",
                    )?
                    .as_f64(),
                )?,
                parse_binary_uv_f64(
                    binary_scalar_value(
                        &values,
                        indices[1],
                        "Error reading texture coordinates from PLY-format",
                    )?
                    .as_f64(),
                )?,
            ]);
        }
    }
    Ok(PlyVertexData {
        points,
        colors,
        uv_coords,
    })
}

fn parse_binary_edge_element(
    reader: &mut PlyBinaryReader<'_>,
    element: &PlyElement,
) -> Result<Vec<[i64; 2]>, String> {
    let Ok(edge_indices) = property_indices(&element.properties, &["vertex1", "vertex2"]) else {
        skip_binary_element(reader, element)?;
        return Ok(Vec::new());
    };
    let mut lines = Vec::with_capacity(element.count);
    for _ in 0..element.count {
        let values = read_binary_property_row(
            reader,
            &element.properties,
            "Error reading edges from PLY-format",
        )?;
        lines.push([
            binary_scalar_value(
                &values,
                edge_indices[0],
                "Error reading edges from PLY-format",
            )?
            .as_i64("Error reading edges from PLY-format")?,
            binary_scalar_value(
                &values,
                edge_indices[1],
                "Error reading edges from PLY-format",
            )?
            .as_i64("Error reading edges from PLY-format")?,
        ]);
    }
    Ok(lines)
}

fn skip_binary_element(
    reader: &mut PlyBinaryReader<'_>,
    element: &PlyElement,
) -> Result<(), String> {
    for _ in 0..element.count {
        for property in &element.properties {
            skip_binary_property(reader, property, "unsupported .PLY file with polylines")?;
        }
    }
    Ok(())
}
