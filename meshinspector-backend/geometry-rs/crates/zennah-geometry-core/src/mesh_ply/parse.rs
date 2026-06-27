use std::path::Path;

use super::*;

pub fn mesh_from_ply(bytes: &[u8]) -> Result<MeshPlyDocument, String> {
    let header = parse_ply_header(bytes)?;
    match header.format {
        PlyFormat::Ascii => parse_ascii_mesh_ply(bytes, &header),
        PlyFormat::BinaryLittleEndian | PlyFormat::BinaryBigEndian => {
            parse_binary_mesh_ply(bytes, &header)
        }
    }
}

pub fn mesh_from_ply_with_textures(
    bytes: &[u8],
    texture_dir: impl AsRef<Path>,
) -> Result<MeshPlyDocument, String> {
    let mut document = mesh_from_ply(bytes)?;
    document.texture_images =
        texture_images_from_files(&document.texture_files, texture_dir.as_ref());
    Ok(document)
}

pub(super) fn parse_ascii_mesh_ply(
    bytes: &[u8],
    header: &PlyHeader,
) -> Result<MeshPlyDocument, String> {
    let payload = std::str::from_utf8(&bytes[header.data_offset..])
        .map_err(|_| "unsupported .PLY mesh file".to_string())?;
    let mut rows = payload.lines();
    let mut vertex_data = None;
    let mut face_data = None;
    let mut edge_data = None;
    for element in &header.elements {
        match element.name.as_str() {
            "vertex" => vertex_data = Some(parse_ascii_vertex_element(&mut rows, element)?),
            "face" => face_data = Some(parse_ascii_face_element(&mut rows, element)?),
            "edge" => edge_data = Some(parse_ascii_edge_element(&mut rows, element)?),
            _ => skip_ascii_element(&mut rows, element.count)?,
        }
    }
    let vertex_data =
        vertex_data.ok_or_else(|| "PLY file does not contain vertices".to_string())?;
    let face_data = face_data.unwrap_or(PlyFaceMeshData {
        faces: Vec::new(),
        colors: Vec::new(),
        tri_corner_uvs: Vec::new(),
    });
    let edge_data = edge_data.unwrap_or(PlyEdgeMeshData { edges: Vec::new() });
    validate_mesh_faces(vertex_data.vertices.len(), &face_data.faces)?;
    validate_mesh_edges(vertex_data.vertices.len(), &edge_data.edges)?;

    Ok(MeshPlyDocument {
        vertices: vertex_data.vertices,
        faces: face_data.faces,
        vertex_colors: vertex_data.colors,
        face_colors: face_data.colors,
        vertex_uvs: vertex_data.uvs,
        vertex_normals: vertex_data.normals,
        tri_corner_uvs: face_data.tri_corner_uvs,
        edges: edge_data.edges,
        texture_files: texture_files_from_comments(&header.comments),
        texture_images: Vec::new(),
    })
}

pub(super) fn parse_binary_mesh_ply(
    bytes: &[u8],
    header: &PlyHeader,
) -> Result<MeshPlyDocument, String> {
    let mut cursor = header.data_offset;
    let mut vertex_data = None;
    let mut face_data = None;
    let mut edge_data = None;
    for element in &header.elements {
        match element.name.as_str() {
            "vertex" => {
                vertex_data = Some(parse_binary_vertex_element(
                    bytes,
                    &mut cursor,
                    header.format,
                    element,
                )?)
            }
            "face" => {
                face_data = Some(parse_binary_face_element(
                    bytes,
                    &mut cursor,
                    header.format,
                    element,
                )?)
            }
            "edge" => {
                edge_data = Some(parse_binary_edge_element(
                    bytes,
                    &mut cursor,
                    header.format,
                    element,
                )?)
            }
            _ => skip_binary_element(bytes, &mut cursor, header.format, element)?,
        }
    }
    let vertex_data =
        vertex_data.ok_or_else(|| "PLY file does not contain vertices".to_string())?;
    let face_data = face_data.unwrap_or(PlyFaceMeshData {
        faces: Vec::new(),
        colors: Vec::new(),
        tri_corner_uvs: Vec::new(),
    });
    let edge_data = edge_data.unwrap_or(PlyEdgeMeshData { edges: Vec::new() });
    validate_mesh_faces(vertex_data.vertices.len(), &face_data.faces)?;
    validate_mesh_edges(vertex_data.vertices.len(), &edge_data.edges)?;

    Ok(MeshPlyDocument {
        vertices: vertex_data.vertices,
        faces: face_data.faces,
        vertex_colors: vertex_data.colors,
        face_colors: face_data.colors,
        vertex_uvs: vertex_data.uvs,
        vertex_normals: vertex_data.normals,
        tri_corner_uvs: face_data.tri_corner_uvs,
        edges: edge_data.edges,
        texture_files: texture_files_from_comments(&header.comments),
        texture_images: Vec::new(),
    })
}
