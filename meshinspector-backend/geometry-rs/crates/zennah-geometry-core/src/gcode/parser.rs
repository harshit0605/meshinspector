#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Command {
    pub(super) key: u8,
    pub(super) value: f64,
}

pub(super) fn parse_frame(frame: &str) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut index = 0;
    let mut in_parenthesized_comment = false;
    let bytes = frame.as_bytes();

    while index < bytes.len() {
        let byte = bytes[index];
        if in_parenthesized_comment {
            if byte == b')' {
                in_parenthesized_comment = false;
            }
            index += 1;
            continue;
        }

        if byte == b';' {
            break;
        }
        if byte == b'(' {
            in_parenthesized_comment = true;
            index += 1;
            continue;
        }

        if byte.is_ascii_alphabetic() {
            let key = byte.to_ascii_uppercase();
            index += 1;
            if let Some((value, consumed)) = parse_float_prefix(&frame[index..]) {
                commands.push(Command { key, value });
                index += consumed;
            }
            continue;
        }

        index += 1;
    }

    commands
}

fn parse_float_prefix(input: &str) -> Option<(f64, usize)> {
    let trimmed = input.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
    let leading_whitespace = input.len() - trimmed.len();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((value, consumed)) = parse_hex_float_prefix(trimmed) {
        return Some((narrow_float(value), leading_whitespace + consumed));
    }

    for end in trimmed
        .char_indices()
        .skip(1)
        .map(|(index, _)| index)
        .chain(std::iter::once(trimmed.len()))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let prefix = &trimmed[..end];
        if let Ok(value) = prefix.parse::<f32>() {
            return Some((f64::from(value), leading_whitespace + end));
        }
        if starts_with_ignore_ascii_case(prefix, "nan") {
            return Some((f64::NAN, leading_whitespace + 3));
        }
        if starts_with_ignore_ascii_case(prefix, "+nan") {
            return Some((f64::NAN, leading_whitespace + 4));
        }
        if starts_with_ignore_ascii_case(prefix, "-nan") {
            return Some((f64::NAN, leading_whitespace + 4));
        }
    }

    None
}

fn narrow_float(value: f64) -> f64 {
    f64::from(value as f32)
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value.len() >= prefix.len() && value[..prefix.len()].eq_ignore_ascii_case(prefix)
}

fn parse_hex_float_prefix(input: &str) -> Option<(f64, usize)> {
    for end in input
        .char_indices()
        .skip(1)
        .map(|(index, _)| index)
        .chain(std::iter::once(input.len()))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if let Some(value) = parse_hex_float(&input[..end]) {
            return Some((value, end));
        }
    }
    None
}

fn parse_hex_float(input: &str) -> Option<f64> {
    let mut rest = input;
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
        let digit = hex_digit_value(ch)?;
        value = value * 16.0 + digit;
        saw_digit = true;
    }

    let mut place = 1.0 / 16.0;
    for ch in fractional.chars() {
        let digit = hex_digit_value(ch)?;
        value += digit * place;
        place /= 16.0;
        saw_digit = true;
    }

    if saw_digit {
        Some(sign * value * 2.0f64.powi(exponent))
    } else {
        None
    }
}

fn hex_digit_value(ch: char) -> Option<f64> {
    ch.to_digit(16).map(f64::from)
}
