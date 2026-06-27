fn property_indices(properties: &[PlyProperty], names: &[&str]) -> Result<Vec<usize>, String> {
    names
        .iter()
        .map(|name| {
            properties
                .iter()
                .position(|property| match property {
                    PlyProperty::Scalar {
                        name: property_name,
                        ..
                    } => property_name == *name,
                    PlyProperty::List {
                        name: property_name,
                        ..
                    } => property_name == *name,
                })
                .ok_or_else(|| "unsupported .PLY file with polylines".to_string())
        })
        .collect()
}

fn color_indices(properties: &[PlyProperty]) -> Option<[usize; 3]> {
    let rgb = property_indices(properties, &["r", "g", "b"]).ok();
    let red_green_blue = property_indices(properties, &["red", "green", "blue"]).ok();
    rgb.or(red_green_blue)
        .map(|indices| [indices[0], indices[1], indices[2]])
}

fn uv_indices(properties: &[PlyProperty]) -> Option<[usize; 2]> {
    [
        ["u", "v"],
        ["s", "t"],
        ["texture_u", "texture_v"],
        ["texture_s", "texture_t"],
    ]
    .into_iter()
    .find_map(|names| {
        property_indices(properties, &names)
            .ok()
            .map(|indices| [indices[0], indices[1]])
    })
}

fn parse_ascii_property_row<'a>(
    row: Option<&'a str>,
    properties: &[PlyProperty],
    error: &str,
) -> Result<Vec<Option<&'a str>>, String> {
    let row = row.ok_or_else(|| error.to_string())?;
    let mut values = Vec::with_capacity(properties.len());
    let mut cursor = 0;
    for property in properties {
        skip_ascii_value_whitespace(row, &mut cursor);
        match property {
            PlyProperty::Scalar { ty, .. } => {
                let value = parse_ascii_scalar_literal(row, &mut cursor, *ty, error)?;
                values.push(Some(value));
                skip_ascii_value_whitespace(row, &mut cursor);
            }
            PlyProperty::List {
                count_ty, item_ty, ..
            } => {
                if matches!(count_ty, PlyScalarType::Float | PlyScalarType::Double) {
                    return Err(error.to_string());
                }
                let count = parse_ascii_int_literal(row, &mut cursor, error)?
                    .parse::<i64>()
                    .map_err(|_| error.to_string())?;
                if count < 0 {
                    return Err(error.to_string());
                }
                skip_ascii_value_whitespace(row, &mut cursor);
                for _ in 0..count {
                    parse_ascii_scalar_literal(row, &mut cursor, *item_ty, error)?;
                    skip_ascii_value_whitespace(row, &mut cursor);
                }
                values.push(None);
            }
        }
    }
    Ok(values)
}

fn skip_ascii_value_whitespace(row: &str, cursor: &mut usize) {
    let bytes = row.as_bytes();
    while bytes
        .get(*cursor)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r'))
    {
        *cursor += 1;
    }
}

fn parse_ascii_scalar_literal<'a>(
    row: &'a str,
    cursor: &mut usize,
    ty: PlyScalarType,
    error: &str,
) -> Result<&'a str, String> {
    match ty {
        PlyScalarType::Char
        | PlyScalarType::UChar
        | PlyScalarType::Short
        | PlyScalarType::UShort
        | PlyScalarType::Int
        | PlyScalarType::UInt => parse_ascii_int_literal(row, cursor, error),
        PlyScalarType::Float | PlyScalarType::Double => {
            parse_ascii_float_literal(row, cursor, error)
        }
    }
}

fn parse_ascii_int_literal<'a>(
    row: &'a str,
    cursor: &mut usize,
    error: &str,
) -> Result<&'a str, String> {
    let bytes = row.as_bytes();
    let start = *cursor;
    if matches!(bytes.get(*cursor), Some(b'-' | b'+')) {
        *cursor += 1;
    }

    let has_leading_zeroes = matches!(bytes.get(*cursor), Some(b'0'));
    if has_leading_zeroes {
        while matches!(bytes.get(*cursor), Some(b'0')) {
            *cursor += 1;
        }
    }

    let digit_start = *cursor;
    while bytes.get(*cursor).is_some_and(u8::is_ascii_digit) {
        *cursor += 1;
    }
    let mut digit_count = *cursor - digit_start;
    if digit_count == 0 && has_leading_zeroes {
        digit_count = 1;
    }
    let trailing = bytes.get(*cursor).copied();
    if digit_count == 0
        || trailing.is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        || digit_count > 10
    {
        return Err(error.to_string());
    }

    Ok(&row[start..*cursor])
}

fn parse_ascii_float_literal<'a>(
    row: &'a str,
    cursor: &mut usize,
    error: &str,
) -> Result<&'a str, String> {
    let bytes = row.as_bytes();
    let start = *cursor;
    if matches!(bytes.get(*cursor), Some(b'-' | b'+')) {
        *cursor += 1;
    }

    let has_int_digits = bytes.get(*cursor).is_some_and(u8::is_ascii_digit);
    while bytes.get(*cursor).is_some_and(u8::is_ascii_digit) {
        *cursor += 1;
    }
    if !has_int_digits && !matches!(bytes.get(*cursor), Some(b'.')) {
        return Err(error.to_string());
    }

    if matches!(bytes.get(*cursor), Some(b'.')) {
        *cursor += 1;
        let has_frac_digits = bytes.get(*cursor).is_some_and(u8::is_ascii_digit);
        while bytes.get(*cursor).is_some_and(u8::is_ascii_digit) {
            *cursor += 1;
        }
        if !has_frac_digits && !has_int_digits {
            return Err(error.to_string());
        }
    }

    if matches!(bytes.get(*cursor), Some(b'e' | b'E')) {
        *cursor += 1;
        if matches!(bytes.get(*cursor), Some(b'-' | b'+')) {
            *cursor += 1;
        }
        if !bytes.get(*cursor).is_some_and(u8::is_ascii_digit) {
            return Err(error.to_string());
        }
        while bytes.get(*cursor).is_some_and(u8::is_ascii_digit) {
            *cursor += 1;
        }
    }

    let trailing = bytes.get(*cursor).copied();
    if trailing.is_some_and(|byte| byte == b'.' || byte == b'_' || byte.is_ascii_alphanumeric()) {
        return Err(error.to_string());
    }

    Ok(&row[start..*cursor])
}

fn ascii_scalar_value<'a>(values: &'a [Option<&'a str>], index: usize) -> Option<&'a str> {
    values.get(index).copied().flatten()
}

fn parse_ascii_position_f64(
    value: Option<&str>,
    property: &PlyProperty,
    error: &str,
) -> Result<f64, String> {
    let value = value.ok_or_else(|| error.to_string())?;
    let ty = match property {
        PlyProperty::Scalar { ty, .. } => *ty,
        PlyProperty::List { .. } => return Err(error.to_string()),
    };
    match ty {
        PlyScalarType::Char => value
            .parse::<i64>()
            .map(|parsed| (parsed as i8) as f64)
            .map_err(|_| error.to_string()),
        PlyScalarType::UChar => value
            .parse::<i64>()
            .map(|parsed| (parsed as u8) as f64)
            .map_err(|_| error.to_string()),
        PlyScalarType::Short => value
            .parse::<i64>()
            .map(|parsed| (parsed as i16) as f64)
            .map_err(|_| error.to_string()),
        PlyScalarType::UShort => value
            .parse::<i64>()
            .map(|parsed| (parsed as u16) as f64)
            .map_err(|_| error.to_string()),
        PlyScalarType::Int => value
            .parse::<i64>()
            .map(|parsed| (parsed as i32) as f64)
            .map_err(|_| error.to_string()),
        PlyScalarType::UInt => value
            .parse::<i64>()
            .map(|parsed| (parsed as u32) as f64)
            .map_err(|_| error.to_string()),
        PlyScalarType::Float | PlyScalarType::Double => {
            value.parse::<f64>().map_err(|_| error.to_string())
        }
    }
}

fn parse_ascii_edge_i64(
    value: Option<&str>,
    property: &PlyProperty,
    error: &str,
) -> Result<i64, String> {
    let value = value.ok_or_else(|| error.to_string())?;
    let ty = match property {
        PlyProperty::Scalar { ty, .. } => *ty,
        PlyProperty::List { .. } => return Err(error.to_string()),
    };
    match ty {
        PlyScalarType::Char => value
            .parse::<i64>()
            .map(|parsed| (parsed as i8) as i64)
            .map_err(|_| error.to_string()),
        PlyScalarType::Short => value
            .parse::<i64>()
            .map(|parsed| (parsed as i16) as i64)
            .map_err(|_| error.to_string()),
        PlyScalarType::Int => value.parse::<i64>().map_err(|_| error.to_string()),
        PlyScalarType::UChar => value
            .parse::<i64>()
            .map(|parsed| (parsed as u8) as i64)
            .map_err(|_| error.to_string()),
        PlyScalarType::UShort => value
            .parse::<i64>()
            .map(|parsed| (parsed as u16) as i64)
            .map_err(|_| error.to_string()),
        PlyScalarType::UInt => value
            .parse::<i64>()
            .map(|parsed| (parsed as i32) as i64)
            .map_err(|_| error.to_string()),
        PlyScalarType::Float | PlyScalarType::Double => {
            let parsed = value.parse::<f64>().map_err(|_| error.to_string())?;
            if !parsed.is_finite() {
                return Err(error.to_string());
            }
            Ok(parsed.trunc() as i64)
        }
    }
}

fn parse_ascii_color_u8(
    value: Option<&str>,
    property: &PlyProperty,
    error: &str,
) -> Result<u8, String> {
    let value = value.ok_or_else(|| error.to_string())?;
    let ty = match property {
        PlyProperty::Scalar { ty, .. } => *ty,
        PlyProperty::List { .. } => return Err(error.to_string()),
    };
    match ty {
        PlyScalarType::Char | PlyScalarType::Short | PlyScalarType::Int => value
            .parse::<i64>()
            .map(|parsed| parsed as u8)
            .map_err(|_| error.to_string()),
        PlyScalarType::UChar | PlyScalarType::UShort | PlyScalarType::UInt => value
            .parse::<i64>()
            .map(|parsed| parsed as u8)
            .map_err(|_| error.to_string()),
        PlyScalarType::Float | PlyScalarType::Double => {
            let parsed = value.parse::<f64>().map_err(|_| error.to_string())?;
            if !parsed.is_finite() {
                return Err(error.to_string());
            }
            truncated_f64_to_u8(parsed, error)
        }
    }
}

fn parse_ascii_uv_f64(
    value: Option<&str>,
    property: &PlyProperty,
    error: &str,
) -> Result<f64, String> {
    let value = value.ok_or_else(|| error.to_string())?;
    match property {
        PlyProperty::Scalar { ty, .. } if matches!(ty, PlyScalarType::Float | PlyScalarType::Double) => {
            let parsed = value.parse::<f64>().map_err(|_| error.to_string())?;
            if !parsed.is_finite() {
                return Err(error.to_string());
            }
            Ok(f64_to_f32(parsed).map_err(|_| error.to_string())? as f64)
        }
        PlyProperty::Scalar { .. } => value
            .parse::<i64>()
            .map(|parsed| {
                f64_to_f32(parsed as f64)
                    .map(|value| value as f64)
                    .map_err(|_| error.to_string())
            })
            .map_err(|_| error.to_string())?,
        PlyProperty::List { .. } => Err(error.to_string()),
    }
}
