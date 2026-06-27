use super::*;

pub(super) fn parse_ply_header(bytes: &[u8]) -> Result<PlyHeader, String> {
    let (header_text, data_offset) = split_header(bytes)?;
    let mut lines = header_text.lines().map(|line| line.trim_end_matches('\r'));
    if !is_ply_magic_line(lines.next()) {
        return Err("unsupported .PLY mesh file".to_string());
    }

    let mut format = None;
    let mut elements = Vec::new();
    let mut comments = Vec::new();
    let mut current_element = None;
    for line in lines {
        if line.starts_with("comment") {
            comments.push(line["comment".len()..].trim().to_string());
            continue;
        }
        if line.starts_with("obj_info") {
            continue;
        }
        if is_end_header_line(line.as_bytes()) {
            break;
        }
        if let Some((kind, version)) = parse_format_line(line)? {
            if format.is_some() {
                return Err("unsupported .PLY mesh file".to_string());
            }
            validate_meshlib_format_version(&version)?;
            format = Some(parse_format_kind(kind)?);
            continue;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["element", name, count, ..] => {
                if format.is_none() {
                    return Err("unsupported .PLY mesh file".to_string());
                }
                elements.push(PlyElement {
                    name: parse_identifier(name)?,
                    count: parse_usize(count)?,
                    properties: Vec::new(),
                });
                current_element = Some(elements.len() - 1);
            }
            ["property", "list", count_ty, item_ty, name, ..] => {
                let index =
                    current_element.ok_or_else(|| "unsupported .PLY mesh file".to_string())?;
                elements[index].properties.push(PlyProperty::List {
                    count_ty: parse_scalar_type(count_ty)?,
                    item_ty: parse_scalar_type(item_ty)?,
                    name: parse_identifier_prefix(name)?,
                });
            }
            ["property", ty, name, ..] => {
                let index =
                    current_element.ok_or_else(|| "unsupported .PLY mesh file".to_string())?;
                elements[index].properties.push(PlyProperty::Scalar {
                    ty: parse_scalar_type(ty)?,
                    name: parse_identifier_prefix(name)?,
                });
            }
            _ => return Err("unsupported .PLY mesh file".to_string()),
        }
    }

    Ok(PlyHeader {
        format: format.ok_or_else(|| "unsupported .PLY mesh file".to_string())?,
        elements,
        comments,
        data_offset,
    })
}

pub(super) fn split_header(bytes: &[u8]) -> Result<(&str, usize), String> {
    let mut line_start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let line_end = if index > line_start && bytes[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            if is_end_header_line(&bytes[line_start..line_end]) {
                let offset = index + 1;
                return Ok((
                    std::str::from_utf8(&bytes[..offset])
                        .map_err(|_| "unsupported .PLY mesh file".to_string())?,
                    offset,
                ));
            }
            line_start = index + 1;
        }
    }
    Err("unsupported .PLY mesh file".to_string())
}

pub(super) fn parse_scalar_type(value: &str) -> Result<PlyScalarType, String> {
    match value {
        "char" | "int8" => Ok(PlyScalarType::Char),
        "uchar" | "uint8" => Ok(PlyScalarType::UChar),
        "short" | "int16" => Ok(PlyScalarType::Short),
        "ushort" | "uint16" => Ok(PlyScalarType::UShort),
        "int" | "int32" => Ok(PlyScalarType::Int),
        "uint" | "uint32" => Ok(PlyScalarType::UInt),
        "float" | "float32" => Ok(PlyScalarType::Float),
        "double" => Ok(PlyScalarType::Double),
        _ => Err("unsupported .PLY mesh file".to_string()),
    }
}

pub(super) fn parse_format_kind(value: &str) -> Result<PlyFormat, String> {
    match value {
        "ascii" => Ok(PlyFormat::Ascii),
        "binary_little_endian" => Ok(PlyFormat::BinaryLittleEndian),
        "binary_big_endian" => Ok(PlyFormat::BinaryBigEndian),
        _ => Err("unsupported .PLY mesh file".to_string()),
    }
}

pub(super) fn validate_meshlib_format_version(value: &str) -> Result<(), String> {
    let Some((major, minor)) = value.split_once('.') else {
        return Err("unsupported .PLY mesh file".to_string());
    };
    parse_meshlib_i32_literal(major)?;
    parse_meshlib_i32_literal_prefix(minor)?;
    Ok(())
}

pub(super) fn parse_meshlib_i32_literal(value: &str) -> Result<i32, String> {
    let rest = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    if rest.is_empty() || !rest.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("unsupported .PLY mesh file".to_string());
    }
    value
        .parse::<i32>()
        .map_err(|_| "unsupported .PLY mesh file".to_string())
}

pub(super) fn parse_meshlib_i32_literal_prefix(value: &str) -> Result<i32, String> {
    let sign_len = usize::from(value.starts_with(['-', '+']));
    let digit_len = value[sign_len..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if digit_len == 0 {
        return Err("unsupported .PLY mesh file".to_string());
    }
    parse_meshlib_i32_literal(&value[..sign_len + digit_len])
}

pub(super) fn parse_usize(value: &str) -> Result<usize, String> {
    let parsed = parse_meshlib_i32_literal_prefix(value)?;
    if parsed < 0 {
        return Err("unsupported .PLY mesh file".to_string());
    }
    Ok(parsed as usize)
}

pub(super) fn parse_identifier(value: &str) -> Result<String, String> {
    if is_meshlib_identifier(value) {
        Ok(value.to_string())
    } else {
        Err("unsupported .PLY mesh file".to_string())
    }
}

pub(super) fn parse_identifier_prefix(value: &str) -> Result<String, String> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err("unsupported .PLY mesh file".to_string());
    };
    if !is_meshlib_identifier_start(first) {
        return Err("unsupported .PLY mesh file".to_string());
    }
    let len = 1 + bytes
        .take_while(|byte| is_meshlib_identifier_part(*byte))
        .count();
    Ok(value[..len].to_string())
}

pub(super) fn is_meshlib_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    is_meshlib_identifier_start(first) && bytes.all(is_meshlib_identifier_part)
}

pub(super) fn is_meshlib_identifier_start(value: u8) -> bool {
    value == b'_' || value.is_ascii_alphabetic()
}

pub(super) fn is_meshlib_identifier_part(value: u8) -> bool {
    value == b'_' || value.is_ascii_alphanumeric()
}

pub(super) fn is_end_header_line(line: &[u8]) -> bool {
    let Some(rest) = line.strip_prefix(b"end_header") else {
        return false;
    };
    rest.iter().all(|byte| matches!(byte, b' ' | b'\t'))
}

pub(super) fn is_ply_magic_line(line: Option<&str>) -> bool {
    let Some(line) = line else {
        return false;
    };
    let Some(rest) = line.strip_prefix("ply") else {
        return false;
    };
    rest.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

pub(super) fn parse_format_line(line: &str) -> Result<Option<(&str, String)>, String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    let Some("format") = parts.first().copied() else {
        return Ok(None);
    };
    let Some(kind) = parts.get(1).copied() else {
        return Err("unsupported .PLY mesh file".to_string());
    };
    let version = match parts[2..] {
        [major, ".", minor, ..] => format!("{major}.{minor}"),
        [major, dot_minor, ..] if dot_minor.starts_with('.') => format!("{major}{dot_minor}"),
        [major_dot, minor, ..] if major_dot.ends_with('.') => format!("{major_dot}{minor}"),
        [version, ..] => (*version).to_string(),
        _ => return Err("unsupported .PLY mesh file".to_string()),
    };
    Ok(Some((kind, version)))
}

impl PlyProperty {
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Scalar { name, .. } | Self::List { name, .. } => name,
        }
    }
}
