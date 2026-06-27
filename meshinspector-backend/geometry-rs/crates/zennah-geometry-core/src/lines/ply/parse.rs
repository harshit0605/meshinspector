fn parse_scalar_type(value: &str) -> Result<PlyScalarType, String> {
    match value {
        "char" | "int8" => Ok(PlyScalarType::Char),
        "uchar" | "uint8" => Ok(PlyScalarType::UChar),
        "short" | "int16" => Ok(PlyScalarType::Short),
        "ushort" | "uint16" => Ok(PlyScalarType::UShort),
        "int" | "int32" => Ok(PlyScalarType::Int),
        "uint" | "uint32" => Ok(PlyScalarType::UInt),
        "float" | "float32" => Ok(PlyScalarType::Float),
        "double" => Ok(PlyScalarType::Double),
        _ => Err("unsupported .PLY file with polylines".to_string()),
    }
}

fn parse_format_kind(value: &str) -> Result<PlyFormat, String> {
    match value {
        "ascii" => Ok(PlyFormat::Ascii),
        "binary_little_endian" => Ok(PlyFormat::BinaryLittleEndian),
        "binary_big_endian" => Ok(PlyFormat::BinaryBigEndian),
        _ => Err("unsupported .PLY file with polylines".to_string()),
    }
}

fn validate_meshlib_format_version(value: &str) -> Result<(), String> {
    let Some((major, minor)) = value.split_once('.') else {
        return Err("unsupported .PLY file with polylines".to_string());
    };
    parse_meshlib_i32_literal(major)?;
    parse_meshlib_i32_literal_prefix(minor)?;
    Ok(())
}

fn parse_meshlib_i32_literal(value: &str) -> Result<i32, String> {
    let rest = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    if rest.is_empty() {
        return Err("unsupported .PLY file with polylines".to_string());
    }
    if !rest.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("unsupported .PLY file with polylines".to_string());
    }
    let significant_digit_count = rest.trim_start_matches('0').len();
    if significant_digit_count > 10 {
        return Err("unsupported .PLY file with polylines".to_string());
    }
    value
        .parse::<i32>()
        .map_err(|_| "unsupported .PLY file with polylines".to_string())
}

fn parse_meshlib_i32_literal_prefix(value: &str) -> Result<i32, String> {
    let sign_len = usize::from(value.starts_with(['-', '+']));
    let digit_len = value[sign_len..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if digit_len == 0 {
        return Err("unsupported .PLY file with polylines".to_string());
    }
    let end = sign_len + digit_len;
    if value
        .as_bytes()
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        return Err("unsupported .PLY file with polylines".to_string());
    }
    parse_meshlib_i32_literal(&value[..end])
}

fn parse_usize(value: &str) -> Result<usize, String> {
    let parsed = parse_meshlib_i32_literal_prefix(value)?;
    if parsed < 0 {
        return Err("unsupported .PLY file with polylines".to_string());
    }
    Ok(parsed as usize)
}

fn parse_identifier(value: &str) -> Result<String, String> {
    if is_meshlib_identifier(value) {
        Ok(value.to_string())
    } else {
        Err("unsupported .PLY file with polylines".to_string())
    }
}

fn parse_identifier_prefix(value: &str) -> Result<String, String> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err("unsupported .PLY file with polylines".to_string());
    };
    if !is_meshlib_identifier_start(first) {
        return Err("unsupported .PLY file with polylines".to_string());
    }
    let len = 1 + bytes
        .take_while(|byte| is_meshlib_identifier_part(*byte))
        .count();
    Ok(value[..len].to_string())
}

fn is_meshlib_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !is_meshlib_identifier_start(first) {
        return false;
    }
    bytes.all(is_meshlib_identifier_part)
}

fn is_meshlib_identifier_start(value: u8) -> bool {
    value == b'_' || value.is_ascii_alphabetic()
}

fn is_meshlib_identifier_part(value: u8) -> bool {
    value == b'_' || value.is_ascii_alphanumeric()
}

impl PlyScalarType {
    fn byte_len(self) -> usize {
        match self {
            Self::Char | Self::UChar => 1,
            Self::Short | Self::UShort => 2,
            Self::Int | Self::UInt | Self::Float => 4,
            Self::Double => 8,
        }
    }
}

fn truncated_f64_to_u8(value: f64, error: &str) -> Result<u8, String> {
    let truncated = value.trunc();
    if !(0.0..=u8::MAX as f64).contains(&truncated) {
        return Err(error.to_string());
    }
    Ok(truncated as u8)
}

fn f64_to_f32(value: f64) -> Result<f32, String> {
    let converted = value as f32;
    if !converted.is_finite() {
        return Err("ObjectLines point coordinates must fit MeshLib Vector3f".to_string());
    }
    Ok(converted)
}

fn meshlib_vector3f_point(point: [f64; 3]) -> Result<[f64; 3], String> {
    Ok([
        f64_to_f32(point[0])? as f64,
        f64_to_f32(point[1])? as f64,
        f64_to_f32(point[2])? as f64,
    ])
}

fn parse_binary_uv_f64(value: f64) -> Result<f64, String> {
    if !value.is_finite() {
        return Err("Error reading texture coordinates from PLY-format".to_string());
    }
    Ok(f64_to_f32(value)
        .map_err(|_| "Error reading texture coordinates from PLY-format".to_string())?
        as f64)
}

fn meshlib_texture_files(comments: &[String]) -> Vec<String> {
    comments
        .iter()
        .filter_map(|comment| {
            let rest = comment.strip_prefix("TextureFile")?;
            let texture = rest.trim_start_matches([' ', '\t']);
            if texture.is_empty() {
                None
            } else {
                Some(texture.to_string())
            }
        })
        .collect()
}

fn meshlib_valid_lines(point_count: usize, candidates: &[[i64; 2]]) -> Vec<[usize; 2]> {
    let mut degree = vec![0_u8; point_count];
    let mut lines = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let [a, b] = *candidate;
        if a < 0 || b < 0 {
            continue;
        }
        let (a, b) = (a as usize, b as usize);
        if a >= point_count || b >= point_count || a == b {
            continue;
        }
        if degree[a] >= 2 || degree[b] >= 2 {
            continue;
        }
        lines.push([a, b]);
        degree[a] += 1;
        degree[b] += 1;
    }
    lines
}

fn push_i32(output: &mut Vec<u8>, value: i32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(output: &mut Vec<u8>, value: f32) {
    output.extend_from_slice(&value.to_le_bytes());
}
