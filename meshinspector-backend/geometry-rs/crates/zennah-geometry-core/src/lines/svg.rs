use super::{object_lines_from_contours, ObjectLinesDocument, ObjectLinesOptions};

mod path;

use path::{parse_svg_linear_path, parse_svg_transform_numbers};

pub fn object_lines_from_svg(source: &str) -> Result<ObjectLinesDocument, String> {
    let document = roxmltree::Document::parse(source).map_err(|err| err.to_string())?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" {
        return Err("Not an SVG document".to_string());
    }
    let mut contours = Vec::new();
    parse_children(root, &mut contours)?;
    for contour in &mut contours {
        for point in contour {
            point[1] = -point[1];
        }
    }
    object_lines_from_contours(&contours, ObjectLinesOptions::default())
}

fn parse_children(
    parent: roxmltree::Node<'_, '_>,
    contours: &mut Vec<Vec<[f64; 3]>>,
) -> Result<(), String> {
    for child in parent.children().filter(roxmltree::Node::is_element) {
        let mut child_contours = Vec::new();
        parse_element(child, &mut child_contours)?;
        if let Some(transform) = child.attribute("transform") {
            let transform = parse_svg_transform(transform)?;
            for contour in &mut child_contours {
                for point in contour {
                    *point = transform.apply(*point);
                }
            }
        }
        contours.extend(child_contours);
    }
    Ok(())
}

fn parse_element(
    element: roxmltree::Node<'_, '_>,
    contours: &mut Vec<Vec<[f64; 3]>>,
) -> Result<(), String> {
    match element.tag_name().name() {
        "g" => parse_children(element, contours),
        "path" => {
            let parsed = parse_svg_linear_path(element.attribute("d").unwrap_or_default())?;
            contours.extend(parsed);
            Ok(())
        }
        "circle" => {
            let cx = svg_float_attr(element, "cx", 0.0)?;
            let cy = svg_float_attr(element, "cy", 0.0)?;
            let r = svg_float_attr(element, "r", 0.0)?;
            if r == 0.0 {
                return Ok(());
            }
            contours.push(ellipse_points(cx, cy, r, r, 0.0, std::f64::consts::TAU, 32));
            Ok(())
        }
        "ellipse" => {
            let cx = svg_float_attr(element, "cx", 0.0)?;
            let cy = svg_float_attr(element, "cy", 0.0)?;
            let rx = svg_float_attr(element, "rx", 0.0)?;
            let ry = svg_float_attr(element, "ry", 0.0)?;
            if rx == 0.0 || ry == 0.0 {
                return Ok(());
            }
            contours.push(ellipse_points(
                cx,
                cy,
                rx,
                ry,
                0.0,
                std::f64::consts::TAU,
                32,
            ));
            Ok(())
        }
        "line" => {
            contours.push(vec![
                [
                    svg_float_attr(element, "x1", 0.0)?,
                    svg_float_attr(element, "y1", 0.0)?,
                    0.0,
                ],
                [
                    svg_float_attr(element, "x2", 0.0)?,
                    svg_float_attr(element, "y2", 0.0)?,
                    0.0,
                ],
            ]);
            Ok(())
        }
        "polyline" => {
            let points = parse_svg_points(element.attribute("points").unwrap_or_default())?;
            if points.len() > 1 {
                contours.push(points);
            }
            Ok(())
        }
        "polygon" => {
            let mut points = parse_svg_points(element.attribute("points").unwrap_or_default())?;
            close_contour(&mut points);
            if points.len() > 1 {
                contours.push(points);
            }
            Ok(())
        }
        "rect" => {
            let x = svg_float_attr(element, "x", 0.0)?;
            let y = svg_float_attr(element, "y", 0.0)?;
            let width = svg_float_attr(element, "width", 0.0)?;
            let height = svg_float_attr(element, "height", 0.0)?;
            let mut rx = svg_float_attr(element, "rx", 0.0)?;
            let mut ry = svg_float_attr(element, "ry", 0.0)?;
            if width == 0.0 || height == 0.0 {
                return Ok(());
            }
            if rx == 0.0 && ry == 0.0 {
                contours.push(vec![
                    [x, y, 0.0],
                    [x, y + height, 0.0],
                    [x + width, y + height, 0.0],
                    [x + width, y, 0.0],
                    [x, y, 0.0],
                ]);
            } else {
                if rx == 0.0 {
                    rx = ry;
                } else if ry == 0.0 {
                    ry = rx;
                }
                if width / 2.0 < rx {
                    rx = width / 2.0;
                }
                if height / 2.0 < ry {
                    ry = height / 2.0;
                }

                let mut points = Vec::new();
                points.extend(ellipse_points(
                    x + width - rx,
                    y + ry,
                    rx,
                    ry,
                    -std::f64::consts::FRAC_PI_2,
                    0.0,
                    32,
                ));
                points.extend(ellipse_points(
                    x + width - rx,
                    y + height - ry,
                    rx,
                    ry,
                    0.0,
                    std::f64::consts::FRAC_PI_2,
                    32,
                ));
                points.extend(ellipse_points(
                    x + rx,
                    y + height - ry,
                    rx,
                    ry,
                    std::f64::consts::FRAC_PI_2,
                    std::f64::consts::PI,
                    32,
                ));
                points.extend(ellipse_points(
                    x + rx,
                    y + ry,
                    rx,
                    ry,
                    -std::f64::consts::PI,
                    -std::f64::consts::FRAC_PI_2,
                    32,
                ));
                close_contour(&mut points);
                contours.push(points);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn svg_float_attr(
    element: roxmltree::Node<'_, '_>,
    name: &str,
    default: f64,
) -> Result<f64, String> {
    let Some(value) = element.attribute(name) else {
        return Ok(default);
    };
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("Failed to parse SVG attribute {name}"))?;
    if !parsed.is_finite() {
        return Err(format!("Failed to parse SVG attribute {name}"));
    }
    Ok(parsed)
}

fn parse_svg_points(points: &str) -> Result<Vec<[f64; 3]>, String> {
    let values = parse_svg_number_list(points)?;
    if values.len() % 2 != 0 {
        return Err("Failed to parse points".to_string());
    }
    Ok(values
        .chunks_exact(2)
        .map(|pair| [pair[0], pair[1], 0.0])
        .collect())
}

fn parse_svg_number_list(source: &str) -> Result<Vec<f64>, String> {
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < source.len() {
        let skipped = skip_svg_number_separators(source, index);
        index = skipped;
        if index >= source.len() {
            break;
        }
        let Some((value, next_index)) = parse_svg_number_at(source, index)? else {
            return Err("Failed to parse points".to_string());
        };
        values.push(value);
        index = next_index;
    }
    Ok(values)
}

fn skip_svg_number_separators(source: &str, mut index: usize) -> usize {
    while let Some(byte) = source.as_bytes().get(index) {
        if byte.is_ascii_whitespace() || *byte == b',' {
            index += 1;
        } else {
            break;
        }
    }
    index
}

fn parse_svg_number_at(source: &str, start: usize) -> Result<Option<(f64, usize)>, String> {
    let bytes = source.as_bytes();
    let mut index = start;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;

    if matches!(bytes.get(index), Some(b'+' | b'-')) {
        index += 1;
    }

    while let Some(byte) = bytes.get(index) {
        match *byte {
            b'0'..=b'9' => {
                seen_digit = true;
                index += 1;
            }
            b'.' if !seen_dot && !seen_exp => {
                seen_dot = true;
                index += 1;
            }
            b'e' | b'E' if !seen_exp && seen_digit => {
                seen_exp = true;
                index += 1;
                if matches!(bytes.get(index), Some(b'+' | b'-')) {
                    index += 1;
                }
            }
            _ => break,
        }
    }

    if !seen_digit {
        return Ok(None);
    }

    let parsed = source[start..index]
        .parse::<f64>()
        .map_err(|_| "Failed to parse points".to_string())?;
    if !parsed.is_finite() {
        return Err("Failed to parse points".to_string());
    }
    Ok(Some((parsed, index)))
}

fn close_contour(contour: &mut Vec<[f64; 3]>) {
    if let (Some(first), Some(last)) = (contour.first().copied(), contour.last()) {
        if *last != first {
            contour.push(first);
        }
    }
}

fn ellipse_points(
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    a0: f64,
    a1: f64,
    resolution: usize,
) -> Vec<[f64; 3]> {
    (0..=resolution)
        .map(|index| {
            let a = a0 + (a1 - a0) * index as f64 / resolution as f64;
            [a.cos() * rx + cx, a.sin() * ry + cy, 0.0]
        })
        .collect()
}

#[derive(Clone, Copy)]
struct SvgTransform {
    rows: [[f64; 2]; 2],
    shift: [f64; 2],
}

impl SvgTransform {
    fn identity() -> Self {
        Self {
            rows: [[1.0, 0.0], [0.0, 1.0]],
            shift: [0.0, 0.0],
        }
    }

    fn matrix(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self {
            rows: [[a, c], [b, d]],
            shift: [e, f],
        }
    }

    fn translation(x: f64, y: f64) -> Self {
        Self {
            rows: [[1.0, 0.0], [0.0, 1.0]],
            shift: [x, y],
        }
    }

    fn scale(x: f64, y: f64) -> Self {
        Self {
            rows: [[x, 0.0], [0.0, y]],
            shift: [0.0, 0.0],
        }
    }

    fn rotation(angle_degrees: f64) -> Self {
        let angle = angle_degrees.to_radians();
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            rows: [[cos, -sin], [sin, cos]],
            shift: [0.0, 0.0],
        }
    }

    fn skew_x(angle_degrees: f64) -> Self {
        Self {
            rows: [[1.0, angle_degrees.to_radians().tan()], [0.0, 1.0]],
            shift: [0.0, 0.0],
        }
    }

    fn skew_y(angle_degrees: f64) -> Self {
        Self {
            rows: [[1.0, 0.0], [angle_degrees.to_radians().tan(), 1.0]],
            shift: [0.0, 0.0],
        }
    }

    fn multiply(self, other: Self) -> Self {
        let rows = [
            [
                self.rows[0][0] * other.rows[0][0] + self.rows[0][1] * other.rows[1][0],
                self.rows[0][0] * other.rows[0][1] + self.rows[0][1] * other.rows[1][1],
            ],
            [
                self.rows[1][0] * other.rows[0][0] + self.rows[1][1] * other.rows[1][0],
                self.rows[1][0] * other.rows[0][1] + self.rows[1][1] * other.rows[1][1],
            ],
        ];
        let transformed_shift = self.apply([other.shift[0], other.shift[1], 0.0]);
        Self {
            rows,
            shift: [transformed_shift[0], transformed_shift[1]],
        }
    }

    fn around(self, point: [f64; 3]) -> Self {
        SvgTransform::translation(point[0], point[1])
            .multiply(self)
            .multiply(SvgTransform::translation(-point[0], -point[1]))
    }

    fn apply(&self, point: [f64; 3]) -> [f64; 3] {
        [
            self.rows[0][0] * point[0] + self.rows[0][1] * point[1] + self.shift[0],
            self.rows[1][0] * point[0] + self.rows[1][1] * point[1] + self.shift[1],
            point[2],
        ]
    }
}

struct SvgTransformParser<'a> {
    source: &'a str,
    index: usize,
}

impl<'a> SvgTransformParser<'a> {
    fn is_done(&self) -> bool {
        self.index >= self.source.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_whitespace() {
                self.index += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn skip_separators(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_whitespace() || ch == ',' {
                self.index += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn parse_name(&mut self) -> Result<&'a str, String> {
        let start = self.index;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphabetic() {
                self.index += ch.len_utf8();
            } else {
                break;
            }
        }
        if self.index == start {
            return Err("Failed to parse points".to_string());
        }
        Ok(&self.source[start..self.index])
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        if self.peek_char() != Some(expected) {
            return Err("Failed to parse points".to_string());
        }
        self.index += expected.len_utf8();
        Ok(())
    }
}

fn parse_svg_transform(source: &str) -> Result<SvgTransform, String> {
    let mut parser = SvgTransformParser { source, index: 0 };
    let mut result = SvgTransform::identity();

    while !parser.is_done() {
        parser.skip_separators();
        if parser.is_done() {
            break;
        }
        let name = parser.parse_name()?;
        parser.skip_whitespace();
        parser.expect_char('(')?;
        let params_start = parser.index;
        while !parser.is_done() && parser.peek_char() != Some(')') {
            parser.index += parser.peek_char().unwrap().len_utf8();
        }
        if parser.is_done() {
            return Err("Failed to parse points".to_string());
        }
        let params = parse_svg_transform_numbers(&source[params_start..parser.index])?;
        parser.expect_char(')')?;

        let transform = match name {
            "matrix" => {
                if params.len() != 6 {
                    return Err("Failed to parse points".to_string());
                }
                SvgTransform::matrix(
                    params[0], params[1], params[2], params[3], params[4], params[5],
                )
            }
            "translate" => {
                if params.is_empty() || params.len() > 2 {
                    return Err("Failed to parse points".to_string());
                }
                SvgTransform::translation(params[0], *params.get(1).unwrap_or(&0.0))
            }
            "scale" => {
                if params.is_empty() || params.len() > 2 {
                    return Err("Failed to parse points".to_string());
                }
                SvgTransform::scale(params[0], *params.get(1).unwrap_or(&params[0]))
            }
            "rotate" => match params.len() {
                1 => SvgTransform::rotation(params[0]),
                3 => SvgTransform::rotation(params[0]).around([params[1], params[2], 0.0]),
                _ => return Err("Failed to parse points".to_string()),
            },
            "skewX" => {
                if params.len() != 1 {
                    return Err("Failed to parse points".to_string());
                }
                SvgTransform::skew_x(params[0])
            }
            "skewY" => {
                if params.len() != 1 {
                    return Err("Failed to parse points".to_string());
                }
                SvgTransform::skew_y(params[0])
            }
            _ => return Err("Failed to parse points".to_string()),
        };
        result = result.multiply(transform);
    }

    Ok(result)
}
