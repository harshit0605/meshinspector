use super::{close_contour, ellipse_points};

pub(super) fn parse_svg_transform_numbers(source: &str) -> Result<Vec<f64>, String> {
    let mut parser = SvgPathParser { source, index: 0 };
    let mut result = Vec::new();
    while !parser.is_done() {
        parser.skip_separators();
        if parser.is_done() {
            break;
        }
        let Some(value) = parser.parse_number()? else {
            return Err("Failed to parse points".to_string());
        };
        result.push(value);
    }
    Ok(result)
}

#[derive(Clone, Copy)]
enum SvgPathCommand {
    Move,
    Line,
    Horizontal,
    Vertical,
    Cubic,
    SmoothCubic,
    Quadratic,
    SmoothQuadratic,
    Arc,
}

struct SvgPathParser<'a> {
    source: &'a str,
    index: usize,
}

pub(super) fn parse_svg_linear_path(source: &str) -> Result<Vec<Vec<[f64; 3]>>, String> {
    let mut parser = SvgPathParser { source, index: 0 };
    let mut contours: Vec<Vec<[f64; 3]>> = vec![Vec::new()];
    let mut pos = [0.0, 0.0, 0.0];
    let mut smooth_control_point = None;
    let mut current_command = None;

    while !parser.is_done() {
        parser.skip_separators();
        if parser.is_done() {
            break;
        }

        let relative = match parser.peek_char() {
            Some(command) if command.is_ascii_alphabetic() => {
                parser.index += command.len_utf8();
                let command = match command {
                    'M' => Some((SvgPathCommand::Move, false)),
                    'm' => Some((SvgPathCommand::Move, true)),
                    'L' => Some((SvgPathCommand::Line, false)),
                    'l' => Some((SvgPathCommand::Line, true)),
                    'H' => Some((SvgPathCommand::Horizontal, false)),
                    'h' => Some((SvgPathCommand::Horizontal, true)),
                    'V' => Some((SvgPathCommand::Vertical, false)),
                    'v' => Some((SvgPathCommand::Vertical, true)),
                    'C' => Some((SvgPathCommand::Cubic, false)),
                    'c' => Some((SvgPathCommand::Cubic, true)),
                    'S' => Some((SvgPathCommand::SmoothCubic, false)),
                    's' => Some((SvgPathCommand::SmoothCubic, true)),
                    'Q' => Some((SvgPathCommand::Quadratic, false)),
                    'q' => Some((SvgPathCommand::Quadratic, true)),
                    'T' => Some((SvgPathCommand::SmoothQuadratic, false)),
                    't' => Some((SvgPathCommand::SmoothQuadratic, true)),
                    'A' => Some((SvgPathCommand::Arc, false)),
                    'a' => Some((SvgPathCommand::Arc, true)),
                    'Z' | 'z' => {
                        close_current_path_contour(&mut contours, &mut pos);
                        smooth_control_point = None;
                        current_command = None;
                        continue;
                    }
                    _ => return Ok(Vec::new()),
                };
                current_command = command.map(|(kind, _)| kind);
                command.map(|(_, relative)| relative).unwrap_or(false)
            }
            Some(_) => {
                if current_command.is_none() {
                    return Err("Failed to parse path".to_string());
                }
                false
            }
            None => break,
        };

        let Some(command) = current_command else {
            continue;
        };

        match command {
            SvgPathCommand::Move => {
                let Some(point) = parser.parse_point()? else {
                    return Err("Failed to parse path".to_string());
                };
                if !contours.last().is_some_and(Vec::is_empty) {
                    contours.push(Vec::new());
                }
                pos = if relative {
                    [pos[0] + point[0], pos[1] + point[1], 0.0]
                } else {
                    point
                };
                contours.last_mut().unwrap().push(pos);
                smooth_control_point = None;
                current_command = Some(SvgPathCommand::Line);

                while let Some(point) = parser.parse_point()? {
                    pos = if relative {
                        [pos[0] + point[0], pos[1] + point[1], 0.0]
                    } else {
                        point
                    };
                    contours.last_mut().unwrap().push(pos);
                }
            }
            SvgPathCommand::Line => {
                while let Some(point) = parser.parse_point()? {
                    ensure_current_path_contour(&mut contours, pos);
                    pos = if relative {
                        [pos[0] + point[0], pos[1] + point[1], 0.0]
                    } else {
                        point
                    };
                    contours.last_mut().unwrap().push(pos);
                    smooth_control_point = None;
                }
            }
            SvgPathCommand::Horizontal => {
                while let Some(x) = parser.parse_number()? {
                    ensure_current_path_contour(&mut contours, pos);
                    pos[0] = if relative { pos[0] + x } else { x };
                    contours.last_mut().unwrap().push(pos);
                    smooth_control_point = None;
                }
            }
            SvgPathCommand::Vertical => {
                while let Some(y) = parser.parse_number()? {
                    ensure_current_path_contour(&mut contours, pos);
                    pos[1] = if relative { pos[1] + y } else { y };
                    contours.last_mut().unwrap().push(pos);
                    smooth_control_point = None;
                }
            }
            SvgPathCommand::Cubic => {
                while let Some(control0) = parser.parse_point()? {
                    let Some(control1) = parser.parse_point()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    let Some(end) = parser.parse_point()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    ensure_current_path_contour(&mut contours, pos);
                    let points = cubic_bezier_path_points(
                        pos,
                        if relative {
                            add_path_point(pos, control0)
                        } else {
                            control0
                        },
                        if relative {
                            add_path_point(pos, control1)
                        } else {
                            control1
                        },
                        if relative {
                            add_path_point(pos, end)
                        } else {
                            end
                        },
                        &mut smooth_control_point,
                    );
                    contours.last_mut().unwrap().extend(points);
                    pos = *contours.last().unwrap().last().unwrap();
                }
            }
            SvgPathCommand::SmoothCubic => {
                while let Some(control1) = parser.parse_point()? {
                    let Some(end) = parser.parse_point()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    ensure_current_path_contour(&mut contours, pos);
                    let control0 = smooth_control_point.unwrap_or(pos);
                    let points = cubic_bezier_path_points(
                        pos,
                        control0,
                        if relative {
                            add_path_point(pos, control1)
                        } else {
                            control1
                        },
                        if relative {
                            add_path_point(pos, end)
                        } else {
                            end
                        },
                        &mut smooth_control_point,
                    );
                    contours.last_mut().unwrap().extend(points);
                    pos = *contours.last().unwrap().last().unwrap();
                }
            }
            SvgPathCommand::Quadratic => {
                while let Some(control) = parser.parse_point()? {
                    let Some(end) = parser.parse_point()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    ensure_current_path_contour(&mut contours, pos);
                    let points = quadratic_bezier_path_points(
                        pos,
                        if relative {
                            add_path_point(pos, control)
                        } else {
                            control
                        },
                        if relative {
                            add_path_point(pos, end)
                        } else {
                            end
                        },
                        &mut smooth_control_point,
                    );
                    contours.last_mut().unwrap().extend(points);
                    pos = *contours.last().unwrap().last().unwrap();
                }
            }
            SvgPathCommand::SmoothQuadratic => {
                while let Some(end) = parser.parse_point()? {
                    ensure_current_path_contour(&mut contours, pos);
                    let control = smooth_control_point.unwrap_or(pos);
                    let points = quadratic_bezier_path_points(
                        pos,
                        control,
                        if relative {
                            add_path_point(pos, end)
                        } else {
                            end
                        },
                        &mut smooth_control_point,
                    );
                    contours.last_mut().unwrap().extend(points);
                    pos = *contours.last().unwrap().last().unwrap();
                }
            }
            SvgPathCommand::Arc => {
                while let Some(radii) = parser.parse_point()? {
                    let Some(x_axis_rotation) = parser.parse_number()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    let Some(large_arc) = parser.parse_number()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    let Some(sweep) = parser.parse_number()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    let Some(end) = parser.parse_point()? else {
                        return Err("Failed to parse path".to_string());
                    };
                    ensure_current_path_contour(&mut contours, pos);
                    let points = arc_path_points(
                        pos,
                        radii,
                        x_axis_rotation,
                        large_arc != 0.0,
                        sweep != 0.0,
                        if relative {
                            add_path_point(pos, end)
                        } else {
                            end
                        },
                    );
                    contours.last_mut().unwrap().extend(points);
                    pos = *contours.last().unwrap().last().unwrap();
                    smooth_control_point = None;
                }
            }
        }
    }

    contours.retain(|contour| contour.len() > 1);
    Ok(contours)
}

fn ensure_current_path_contour(contours: &mut Vec<Vec<[f64; 3]>>, pos: [f64; 3]) {
    if contours.is_empty() {
        contours.push(Vec::new());
    }
    if contours.last().is_some_and(Vec::is_empty) {
        contours.last_mut().unwrap().push(pos);
    }
}

fn close_current_path_contour(contours: &mut Vec<Vec<[f64; 3]>>, pos: &mut [f64; 3]) {
    if contours.last().is_some_and(Vec::is_empty) {
        return;
    }
    if let Some(contour) = contours.last_mut() {
        close_contour(contour);
        if let Some(last) = contour.last().copied() {
            *pos = last;
        }
    }
    contours.push(Vec::new());
}

fn add_path_point(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [lhs[0] + rhs[0], lhs[1] + rhs[1], 0.0]
}

fn sub_path_point(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [lhs[0] - rhs[0], lhs[1] - rhs[1], 0.0]
}

fn scale_path_point(point: [f64; 3], scale: f64) -> [f64; 3] {
    [point[0] * scale, point[1] * scale, 0.0]
}

fn div_path_point(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [lhs[0] / rhs[0], lhs[1] / rhs[1], 0.0]
}

fn path_length_sq(point: [f64; 3]) -> f64 {
    point[0] * point[0] + point[1] * point[1]
}

fn lerp_path_point(lhs: [f64; 3], rhs: [f64; 3], t: f64) -> [f64; 3] {
    [
        lhs[0] + (rhs[0] - lhs[0]) * t,
        lhs[1] + (rhs[1] - lhs[1]) * t,
        0.0,
    ]
}

fn rotate_path_point(point: [f64; 3], angle: f64) -> [f64; 3] {
    let cos = angle.cos();
    let sin = angle.sin();
    [
        cos * point[0] - sin * point[1],
        sin * point[0] + cos * point[1],
        0.0,
    ]
}

fn rotate_path_point_transposed(point: [f64; 3], angle: f64) -> [f64; 3] {
    let cos = angle.cos();
    let sin = angle.sin();
    [
        cos * point[0] + sin * point[1],
        -sin * point[0] + cos * point[1],
        0.0,
    ]
}

fn cubic_bezier_path_points(
    start: [f64; 3],
    control0: [f64; 3],
    control1: [f64; 3],
    end: [f64; 3],
    smooth_control_point: &mut Option<[f64; 3]>,
) -> Vec<[f64; 3]> {
    *smooth_control_point = Some(add_path_point(end, sub_path_point(end, control1)));

    (1..=32)
        .map(|index| {
            let t = index as f64 / 32.0;
            let q0 = lerp_path_point(start, control0, t);
            let q1 = lerp_path_point(control0, control1, t);
            let q2 = lerp_path_point(control1, end, t);
            let r0 = lerp_path_point(q0, q1, t);
            let r1 = lerp_path_point(q1, q2, t);
            lerp_path_point(r0, r1, t)
        })
        .collect()
}

fn quadratic_bezier_path_points(
    start: [f64; 3],
    control: [f64; 3],
    end: [f64; 3],
    smooth_control_point: &mut Option<[f64; 3]>,
) -> Vec<[f64; 3]> {
    *smooth_control_point = Some(add_path_point(end, sub_path_point(end, control)));

    (1..=32)
        .map(|index| {
            let t = index as f64 / 32.0;
            let q0 = lerp_path_point(start, control, t);
            let q1 = lerp_path_point(control, end, t);
            lerp_path_point(q0, q1, t)
        })
        .collect()
}

fn arc_path_points(
    start: [f64; 3],
    mut radii: [f64; 3],
    x_axis_rotation: f64,
    large_arc: bool,
    sweep: bool,
    end: [f64; 3],
) -> Vec<[f64; 3]> {
    let phi = x_axis_rotation.to_radians();
    let p0 = scale_path_point(sub_path_point(start, end), 0.5);
    let p0_rotated = rotate_path_point_transposed(p0, phi);

    radii[0] = radii[0].abs();
    radii[1] = radii[1].abs();
    if radii[0] == 0.0 || radii[1] == 0.0 {
        return vec![end];
    }

    let lambda = path_length_sq(div_path_point(p0_rotated, radii));
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        radii[0] *= scale;
        radii[1] *= scale;
    }

    let rp = [radii[0] * p0_rotated[1], radii[1] * p0_rotated[0], 0.0];
    let rp_len_sq = path_length_sq(rp);
    if rp_len_sq == 0.0 {
        return vec![end];
    }
    let numerator = (radii[0] * radii[1]).powi(2);
    let k1_sq = (numerator / rp_len_sq - 1.0).max(0.0);
    let k1_sign = if large_arc != sweep { 1.0 } else { -1.0 };
    let k1 = k1_sq.sqrt() * k1_sign;
    let center_rotated = [k1 * rp[0] / radii[1], -k1 * rp[1] / radii[0], 0.0];
    let midpoint = scale_path_point(add_path_point(start, end), 0.5);
    let center = add_path_point(rotate_path_point(center_rotated, phi), midpoint);

    let angle = |point: [f64; 3]| point[1].atan2(point[0]);
    let theta0 = angle(div_path_point(
        sub_path_point(p0_rotated, center_rotated),
        radii,
    ));
    let mut theta1 = angle(div_path_point(
        sub_path_point(scale_path_point(p0_rotated, -1.0), center_rotated),
        radii,
    ));
    if sweep && theta1 < theta0 {
        theta1 += std::f64::consts::TAU;
    }
    if !sweep && theta0 < theta1 {
        theta1 -= std::f64::consts::TAU;
    }

    let mut points = ellipse_points(center[0], center[1], radii[0], radii[1], theta0, theta1, 32);
    if phi != 0.0 {
        for point in &mut points {
            *point = add_path_point(
                rotate_path_point(sub_path_point(*point, center), phi),
                center,
            );
        }
    }
    points.remove(0);
    points
}

impl<'a> SvgPathParser<'a> {
    fn is_done(&self) -> bool {
        self.index >= self.source.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
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

    fn parse_point(&mut self) -> Result<Option<[f64; 3]>, String> {
        let Some(x) = self.parse_number()? else {
            return Ok(None);
        };
        let Some(y) = self.parse_number()? else {
            return Err("Failed to parse path".to_string());
        };
        Ok(Some([x, y, 0.0]))
    }

    fn parse_number(&mut self) -> Result<Option<f64>, String> {
        self.skip_separators();
        let start = self.index;
        let mut seen_digit = false;
        let mut seen_dot = false;
        let mut seen_exp = false;

        if matches!(self.peek_char(), Some('+') | Some('-')) {
            self.index += 1;
        }

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                seen_digit = true;
                self.index += 1;
            } else if ch == '.' && !seen_dot && !seen_exp {
                seen_dot = true;
                self.index += 1;
            } else if matches!(ch, 'e' | 'E') && !seen_exp && seen_digit {
                seen_exp = true;
                self.index += 1;
                if matches!(self.peek_char(), Some('+') | Some('-')) {
                    self.index += 1;
                }
            } else {
                break;
            }
        }

        if !seen_digit {
            self.index = start;
            return Ok(None);
        }

        let parsed = self.source[start..self.index]
            .parse::<f64>()
            .map_err(|_| "Failed to parse path".to_string())?;
        if !parsed.is_finite() {
            return Err("Failed to parse path".to_string());
        }
        Ok(Some(parsed))
    }
}
