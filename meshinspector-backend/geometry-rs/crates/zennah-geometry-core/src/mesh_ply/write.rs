use super::*;

pub fn mesh_to_ply(document: &MeshPlyDocument) -> Result<Vec<u8>, String> {
    validate_mesh_faces(document.vertices.len(), &document.faces)?;

    let vertex_uvs = valid_vertex_uvs(document);
    let tri_corner_uvs = valid_tri_corner_uvs(document);
    let vertex_colors = valid_vertex_colors(document);
    let face_colors = valid_face_colors(document);

    let mut output = String::new();
    output.push_str("ply\nformat ascii 1.0\n");
    for texture_file in document
        .texture_files
        .iter()
        .filter(|texture_file| !texture_file.is_empty())
    {
        output.push_str("comment TextureFile ");
        output.push_str(texture_file);
        output.push('\n');
    }

    output.push_str(&format!("element vertex {}\n", document.vertices.len()));
    output.push_str("property double x\nproperty double y\nproperty double z\n");
    if vertex_uvs.is_some() {
        output.push_str("property double s\nproperty double t\n");
    }
    if vertex_colors.is_some() {
        output.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\n");
    }

    output.push_str(&format!("element face {}\n", document.faces.len()));
    output.push_str("property list uchar int vertex_indices\n");
    if tri_corner_uvs.is_some() {
        output.push_str("property list uchar float texcoord\n");
    }
    if face_colors.is_some() {
        output.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\n");
    }
    output.push_str("end_header\n");

    for (index, vertex) in document.vertices.iter().enumerate() {
        output.push_str(&format!(
            "{} {} {}",
            format_ply_number(vertex[0]),
            format_ply_number(vertex[1]),
            format_ply_number(vertex[2])
        ));
        if let Some(uvs) = vertex_uvs {
            output.push_str(&format!(
                " {} {}",
                format_ply_number(uvs[index][0]),
                format_ply_number(uvs[index][1])
            ));
        }
        if let Some(colors) = vertex_colors {
            output.push_str(&format!(
                " {} {} {}",
                colors[index][0], colors[index][1], colors[index][2]
            ));
        }
        output.push('\n');
    }

    for (index, face) in document.faces.iter().enumerate() {
        output.push_str(&format!("3 {} {} {}", face[0], face[1], face[2]));
        if let Some(uvs) = tri_corner_uvs {
            output.push_str(" 6");
            for uv in &uvs[index] {
                output.push_str(&format!(
                    " {} {}",
                    format_ply_number(uv[0]),
                    format_ply_number(uv[1])
                ));
            }
        }
        if let Some(colors) = face_colors {
            output.push_str(&format!(
                " {} {} {}",
                colors[index][0], colors[index][1], colors[index][2]
            ));
        }
        output.push('\n');
    }

    Ok(output.into_bytes())
}

fn valid_vertex_uvs(document: &MeshPlyDocument) -> Option<&[[f64; 2]]> {
    (document.vertex_uvs.len() == document.vertices.len()).then_some(&document.vertex_uvs)
}

fn valid_tri_corner_uvs(document: &MeshPlyDocument) -> Option<&[[[f64; 2]; 3]]> {
    (document.tri_corner_uvs.len() == document.faces.len()).then_some(&document.tri_corner_uvs)
}

fn valid_vertex_colors(document: &MeshPlyDocument) -> Option<&[[u8; 4]]> {
    (document.vertex_colors.len() == document.vertices.len()).then_some(&document.vertex_colors)
}

fn valid_face_colors(document: &MeshPlyDocument) -> Option<&[[u8; 4]]> {
    (document.face_colors.len() == document.faces.len()).then_some(&document.face_colors)
}

fn format_ply_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let fixed = format!("{value:.17}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}
