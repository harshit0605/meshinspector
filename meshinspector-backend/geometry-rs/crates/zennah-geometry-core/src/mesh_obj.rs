use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshObjDocument {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub object_names: Vec<String>,
    pub material_names: Vec<String>,
    pub diffuse_color: Option<[u8; 4]>,
    pub texture_files: Vec<String>,
    pub texture_images: Vec<MeshObjTextureImage>,
    pub texture_per_face: Vec<i64>,
    pub tri_corner_uvs: Vec<[[f64; 2]; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshObjTextureImage {
    pub file: String,
    pub resolved_path: String,
    pub width: u32,
    pub height: u32,
    pub pixels_rgba: Vec<[u8; 4]>,
    pub filter: String,
    pub wrap: String,
}

pub fn mesh_from_obj(source: &[u8]) -> Result<MeshObjDocument, String> {
    mesh_from_obj_impl(source, None)
}

pub fn mesh_from_obj_with_material_dir(
    source: &[u8],
    material_dir: &Path,
) -> Result<MeshObjDocument, String> {
    mesh_from_obj_impl(source, Some(material_dir))
}

fn mesh_from_obj_impl(
    source: &[u8],
    material_dir: Option<&Path>,
) -> Result<MeshObjDocument, String> {
    let mut text =
        std::str::from_utf8(source).map_err(|_| "OBJ source must be valid UTF-8".to_string())?;
    if let Some(stripped) = text.strip_prefix('\u{feff}') {
        text = stripped;
    }

    let mut vertices = Vec::new();
    let mut texture_vertices = Vec::new();
    let mut faces = Vec::new();
    let mut tri_corner_uvs = Vec::new();
    let mut saw_face_without_uvs = false;
    let mut object_names = Vec::new();
    let mut material_library_name: Option<String> = None;
    let mut current_material: Option<String> = None;
    let mut output_face_materials: Vec<Option<String>> = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };

        match kind {
            "v" => {
                let x = parse_obj_float(parts.next(), "vertex x")?;
                let y = parse_obj_float(parts.next(), "vertex y")?;
                let z = parse_obj_float(parts.next(), "vertex z")?;
                vertices.push([x, y, z]);
            }
            "vt" => {
                let u = meshlib_f32(parse_obj_float(parts.next(), "texture vertex u")?)?;
                let v = parts
                    .next()
                    .map(|value| parse_obj_float(Some(value), "texture vertex v"))
                    .transpose()?
                    .unwrap_or(0.0);
                texture_vertices.push([u, meshlib_f32(v)?]);
            }
            "f" => {
                let face_vertices = parts
                    .map(|token| {
                        parse_obj_face_vertex(token, vertices.len(), texture_vertices.len())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if face_vertices.len() < 3 {
                    return Err("Face with less than 3 vertices in OBJ-file".to_string());
                }
                let texture_count = face_vertices
                    .iter()
                    .filter(|vertex| vertex.texture.is_some())
                    .count();
                if texture_count != 0 && texture_count != face_vertices.len() {
                    return Err("Invalid face texture count in OBJ-file".to_string());
                }
                let face_has_uvs = texture_count == face_vertices.len();
                if !face_has_uvs {
                    saw_face_without_uvs = true;
                }
                for index in 1..face_vertices.len() - 1 {
                    faces.push([
                        face_vertices[0].vertex as i64,
                        face_vertices[index].vertex as i64,
                        face_vertices[index + 1].vertex as i64,
                    ]);
                    if face_has_uvs {
                        tri_corner_uvs.push([
                            texture_vertices[face_vertices[0].texture.expect("face_has_uvs")],
                            texture_vertices[face_vertices[index].texture.expect("face_has_uvs")],
                            texture_vertices
                                [face_vertices[index + 1].texture.expect("face_has_uvs")],
                        ]);
                    }
                    output_face_materials.push(current_material.clone());
                }
            }
            "o" => {
                let name = parts.collect::<Vec<_>>().join(" ");
                if !name.is_empty() {
                    object_names.push(name);
                }
            }
            "mtllib" => {
                let name = parts.collect::<Vec<_>>().join(" ");
                if !name.is_empty() && material_library_name.is_none() {
                    material_library_name = Some(name);
                }
            }
            "usemtl" => {
                let name = parts.collect::<Vec<_>>().join(" ");
                current_material = (!name.is_empty()).then_some(name);
            }
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err("No vertex found in OBJ-file".to_string());
    }
    if faces.is_empty() {
        return Err("No face found in OBJ-file".to_string());
    }
    let materials = material_dir
        .zip(material_library_name.as_ref())
        .and_then(|(dir, file_name)| load_mtl_library(&dir.join(file_name)).ok());
    let material_metadata = collect_material_metadata(&output_face_materials, materials.as_ref());
    if saw_face_without_uvs {
        tri_corner_uvs.clear();
    }
    let texture_images = material_dir
        .map(|dir| texture_images_from_files(&material_metadata.texture_files, dir))
        .unwrap_or_default();

    Ok(MeshObjDocument {
        vertices,
        faces,
        object_names,
        material_names: material_metadata.material_names,
        diffuse_color: material_metadata.diffuse_color,
        texture_files: material_metadata.texture_files,
        texture_images,
        texture_per_face: material_metadata.texture_per_face,
        tri_corner_uvs,
    })
}

fn parse_obj_float(value: Option<&str>, name: &str) -> Result<f64, String> {
    value
        .ok_or_else(|| format!("Failed to parse {name} in OBJ-file"))?
        .parse::<f64>()
        .map_err(|_| format!("Failed to parse {name} in OBJ-file"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObjFaceVertex {
    vertex: usize,
    texture: Option<usize>,
}

fn parse_obj_face_vertex(
    token: &str,
    vertex_count: usize,
    texture_vertex_count: usize,
) -> Result<ObjFaceVertex, String> {
    let mut parts = token.split('/');
    let vertex_token = parts
        .next()
        .ok_or_else(|| "Failed to parse face in OBJ-file".to_string())?;
    let vertex_index = vertex_token
        .parse::<isize>()
        .map_err(|_| "Failed to parse face in OBJ-file".to_string())?;
    let vertex = resolve_obj_index(vertex_index, vertex_count, "Vertex")?;
    let texture = match parts.next() {
        Some(token) if !token.is_empty() => {
            let texture_index = token
                .parse::<isize>()
                .map_err(|_| "Failed to parse face in OBJ-file".to_string())?;
            Some(resolve_obj_index(
                texture_index,
                texture_vertex_count,
                "Texture Vertex",
            )?)
        }
        _ => None,
    };
    Ok(ObjFaceVertex { vertex, texture })
}

fn resolve_obj_index(index: isize, count: usize, name: &str) -> Result<usize, String> {
    if index == 0 {
        return Err(format!("Out of bounds {name} ID in OBJ-file"));
    }
    let resolved = if index < 0 {
        count as isize + index
    } else {
        index - 1
    };
    if resolved < 0 || resolved >= count as isize {
        return Err(format!("Out of bounds {name} ID in OBJ-file"));
    }
    Ok(resolved as usize)
}

fn meshlib_f32(value: f64) -> Result<f64, String> {
    let converted = value as f32;
    if !converted.is_finite() {
        return Err("OBJ coordinate must fit MeshLib float storage".to_string());
    }
    Ok(converted as f64)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ObjMaterial {
    diffuse_color: Option<[u8; 4]>,
    diffuse_texture_file: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ObjMaterialMetadata {
    material_names: Vec<String>,
    diffuse_color: Option<[u8; 4]>,
    texture_files: Vec<String>,
    texture_per_face: Vec<i64>,
}

fn load_mtl_library(path: &Path) -> Result<HashMap<String, ObjMaterial>, String> {
    let source =
        std::fs::read_to_string(path).map_err(|_| "unable to open MTL file".to_string())?;
    let mut materials = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current = ObjMaterial::default();

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };
        match kind {
            "newmtl" => {
                if let Some(name) = current_name.replace(parts.collect::<Vec<_>>().join(" ")) {
                    materials.insert(name, std::mem::take(&mut current));
                }
            }
            "Kd" => {
                let color = [
                    mtl_color_channel(parts.next())?,
                    mtl_color_channel(parts.next())?,
                    mtl_color_channel(parts.next())?,
                    255,
                ];
                current.diffuse_color = Some(color);
            }
            "map_Kd" => {
                let tokens = parts.collect::<Vec<_>>();
                let texture = parse_mtl_texture_tokens(&tokens)?;
                if !texture.is_empty() {
                    current.diffuse_texture_file = Some(texture);
                }
            }
            _ => {}
        }
    }

    if let Some(name) = current_name {
        materials.insert(name, current);
    }
    Ok(materials)
}

fn mtl_color_channel(value: Option<&str>) -> Result<u8, String> {
    let parsed = value
        .ok_or_else(|| "Failed to parse color in MTL-file".to_string())?
        .parse::<f64>()
        .map_err(|_| "Failed to parse color in MTL-file".to_string())?;
    Ok((parsed.clamp(0.0, 1.0) * 255.0) as u8)
}

fn parse_mtl_texture_tokens(tokens: &[&str]) -> Result<String, String> {
    let mut index = 0;
    while index < tokens.len() {
        let arg_count = match tokens[index] {
            "-blendu" | "-blendv" | "-cc" | "-clamp" | "-imfchan" | "-bm" | "-texres" => 1,
            "-mm" => 2,
            "-o" | "-s" | "-t" => 3,
            _ => break,
        };
        index += 1 + arg_count;
        if index > tokens.len() {
            return Err("Failed to parse texture in MTL-file".to_string());
        }
    }
    Ok(tokens[index..].join(" "))
}

fn texture_images_from_files(
    texture_files: &[String],
    material_dir: &Path,
) -> Vec<MeshObjTextureImage> {
    texture_files
        .iter()
        .filter_map(|texture_file| {
            let texture_path = material_dir.join(texture_file);
            if texture_path.is_file() {
                load_texture_image(texture_file, &texture_path)
            } else {
                None
            }
        })
        .collect()
}

fn load_texture_image(texture_file: &str, texture_path: &Path) -> Option<MeshObjTextureImage> {
    let image = image::ImageReader::open(texture_path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let pixels_rgba = image
        .pixels()
        .map(|pixel| {
            let [red, green, blue, alpha] = pixel.0;
            [red, green, blue, alpha]
        })
        .collect();
    Some(MeshObjTextureImage {
        file: texture_file.to_owned(),
        resolved_path: texture_path.to_string_lossy().into_owned(),
        width,
        height,
        pixels_rgba,
        filter: "Linear".to_string(),
        wrap: "Clamp".to_string(),
    })
}

fn collect_material_metadata(
    face_materials: &[Option<String>],
    materials: Option<&HashMap<String, ObjMaterial>>,
) -> ObjMaterialMetadata {
    let Some(materials) = materials else {
        return ObjMaterialMetadata::default();
    };
    let mut material_names = Vec::new();
    let mut diffuse_color: Option<[u8; 4]> = None;
    let mut diffuse_color_contradicts = false;
    let mut texture_files = Vec::<String>::new();
    let mut texture_index_by_file = HashMap::<String, i64>::new();
    let mut texture_per_face = Vec::new();
    let mut missing_texture = false;

    for material_name in face_materials.iter().flatten() {
        if !material_names.contains(material_name) {
            material_names.push(material_name.clone());
        }
        let Some(material) = materials.get(material_name) else {
            diffuse_color_contradicts = true;
            missing_texture = true;
            continue;
        };
        match (diffuse_color, material.diffuse_color) {
            (None, Some(color)) if !diffuse_color_contradicts => diffuse_color = Some(color),
            (Some(existing), Some(color)) if existing == color => {}
            _ => {
                diffuse_color = None;
                diffuse_color_contradicts = true;
            }
        }
    }

    for material_name in face_materials {
        let Some(material_name) = material_name else {
            missing_texture = true;
            texture_per_face.push(-1);
            continue;
        };
        let Some(texture_file) = materials
            .get(material_name)
            .and_then(|material| material.diffuse_texture_file.as_ref())
        else {
            missing_texture = true;
            texture_per_face.push(-1);
            continue;
        };
        let texture_id = if let Some(id) = texture_index_by_file.get(texture_file) {
            *id
        } else {
            let id = texture_files.len() as i64;
            texture_files.push(texture_file.clone());
            texture_index_by_file.insert(texture_file.clone(), id);
            id
        };
        texture_per_face.push(texture_id);
    }

    if missing_texture {
        texture_files.clear();
        texture_per_face.clear();
    }

    ObjMaterialMetadata {
        material_names,
        diffuse_color,
        texture_files,
        texture_per_face,
    }
}
