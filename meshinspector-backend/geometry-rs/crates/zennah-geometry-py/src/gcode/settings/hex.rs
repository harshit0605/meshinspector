fn parse_meshlib_decimal_prefix(token: &str) -> Option<f64> {
    for end in token
        .char_indices()
        .skip(1)
        .map(|(index, _)| index)
        .chain(std::iter::once(token.len()))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let prefix = &token[..end];
        if let Ok(value) = prefix.parse::<f64>() {
            return Some(value);
        }
    }
    None
}

fn parse_meshlib_hex_float_prefix(token: &str) -> Option<f64> {
    for end in token
        .char_indices()
        .skip(1)
        .map(|(index, _)| index)
        .chain(std::iter::once(token.len()))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if let Some(value) = parse_meshlib_hex_float(&token[..end]) {
            return Some(value);
        }
    }
    None
}

fn parse_meshlib_hex_float(token: &str) -> Option<f64> {
    let mut rest = token;
    let sign = if let Some(stripped) = rest.strip_prefix('-') {
        rest = stripped;
        -1.0
    } else if let Some(stripped) = rest.strip_prefix('+') {
        rest = stripped;
        1.0
    } else {
        1.0
    };
    let rest = rest
        .strip_prefix("0x")
        .or_else(|| rest.strip_prefix("0X"))?;
    let (significand, exponent) = rest.split_once('p').or_else(|| rest.split_once('P'))?;
    let exponent = exponent.parse::<i32>().ok()?;
    let (integer, fractional) = significand.split_once('.').unwrap_or((significand, ""));

    let mut saw_digit = false;
    let mut value = 0.0;
    for ch in integer.chars() {
        let digit = ch.to_digit(16)? as f64;
        value = value * 16.0 + digit;
        saw_digit = true;
    }
    let mut place = 1.0 / 16.0;
    for ch in fractional.chars() {
        let digit = ch.to_digit(16)? as f64;
        value += digit * place;
        place /= 16.0;
        saw_digit = true;
    }
    if !saw_digit {
        return None;
    }
    Some(sign * value * 2.0f64.powi(exponent))
}
