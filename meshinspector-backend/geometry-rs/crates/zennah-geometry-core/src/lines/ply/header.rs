use super::{
    parse_format_kind, parse_identifier, parse_identifier_prefix, parse_scalar_type, parse_usize,
    validate_meshlib_format_version, PlyElement, PlyHeader, PlyProperty,
};

pub(super) fn parse_ply_header(bytes: &[u8]) -> Result<PlyHeader, String> {
    let (header_text, data_offset) = split_header(bytes)?;
    let mut lines = header_text.lines().map(|line| line.trim_end_matches('\r'));
    if !is_ply_magic_line(lines.next()) {
        return Err("unsupported .PLY file with polylines".to_string());
    }

    let mut format = None;
    let mut elements = Vec::new();
    let mut current_element = None;
    let mut comments = Vec::new();
    for line in lines {
        if has_leading_meshlib_header_whitespace(line) {
            return Err("unsupported .PLY file with polylines".to_string());
        }
        if let Some(comment) = parse_comment_line(line) {
            comments.push(comment.to_string());
            continue;
        }
        if is_skipped_header_line(line) {
            continue;
        }
        if is_end_header_line(line.as_bytes()) {
            break;
        }
        if let Some((kind, version)) = parse_format_line(line)? {
            if format.is_some() {
                return Err("unsupported .PLY file with polylines".to_string());
            }
            validate_meshlib_format_version(&version)?;
            format = Some(parse_format_kind(kind)?);
            continue;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["element", name, count, ..] => {
                if format.is_none() {
                    return Err("unsupported .PLY file with polylines".to_string());
                }
                elements.push(PlyElement {
                    name: parse_identifier(name)?,
                    count: parse_usize(count)?,
                    properties: Vec::new(),
                });
                current_element = Some(elements.len() - 1);
            }
            ["property", "list", count_ty, item_ty, name, ..] => {
                let index = current_element
                    .ok_or_else(|| "unsupported .PLY file with polylines".to_string())?;
                elements[index].properties.push(PlyProperty::List {
                    count_ty: parse_scalar_type(count_ty)?,
                    item_ty: parse_scalar_type(item_ty)?,
                    name: parse_identifier_prefix(name)?,
                });
            }
            ["property", ty, name, ..] => {
                let index = current_element
                    .ok_or_else(|| "unsupported .PLY file with polylines".to_string())?;
                elements[index].properties.push(PlyProperty::Scalar {
                    ty: parse_scalar_type(ty)?,
                    name: parse_identifier_prefix(name)?,
                });
            }
            _ => return Err("unsupported .PLY file with polylines".to_string()),
        }
    }

    Ok(PlyHeader {
        format: format.ok_or_else(|| "unsupported .PLY file with polylines".to_string())?,
        elements,
        comments,
        data_offset,
    })
}

fn split_header(bytes: &[u8]) -> Result<(&str, usize), String> {
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
                        .map_err(|_| "unsupported .PLY file with polylines".to_string())?,
                    offset,
                ));
            }
            line_start = index + 1;
        }
    }
    Err("unsupported .PLY file with polylines".to_string())
}

fn is_end_header_line(line: &[u8]) -> bool {
    let Some(rest) = line.strip_prefix(b"end_header") else {
        return false;
    };
    rest.iter().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn is_ply_magic_line(line: Option<&str>) -> bool {
    let Some(line) = line else {
        return false;
    };
    let Some(rest) = line.strip_prefix("ply") else {
        return false;
    };
    rest.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn is_skipped_header_line(line: &str) -> bool {
    line.starts_with("obj_info")
}

fn parse_comment_line(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("comment")?;
    Some(rest.trim())
}

fn has_leading_meshlib_header_whitespace(line: &str) -> bool {
    matches!(line.as_bytes().first(), Some(b' ' | b'\t' | b'\r'))
}

fn parse_format_line(line: &str) -> Result<Option<(&str, String)>, String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    let Some("format") = parts.first().copied() else {
        return Ok(None);
    };
    let Some(kind) = parts.get(1).copied() else {
        return Err("unsupported .PLY file with polylines".to_string());
    };
    let version = match parts[2..] {
        [major, ".", minor, ..] => format!("{major}.{minor}"),
        [major, dot_minor, ..] if dot_minor.starts_with('.') => format!("{major}{dot_minor}"),
        [major_dot, minor, ..] if major_dot.ends_with('.') => format!("{major_dot}{minor}"),
        [version, ..] => (*version).to_string(),
        _ => return Err("unsupported .PLY file with polylines".to_string()),
    };
    Ok(Some((kind, version)))
}
