fn segment_intersects_triangle_strict(p0: [f64; 3], p1: [f64; 3], triangle: [[f64; 3]; 3]) -> bool {
    let direction = sub(p1, p0);
    let [a, b, c] = triangle;
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let h = cross(direction, edge2);
    let det = dot(edge1, h);
    if det == 0.0 {
        return false;
    }
    let inv_det = 1.0 / det;
    let s = sub(p0, a);
    let u = inv_det * dot(s, h);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross(s, edge1);
    let v = inv_det * dot(direction, q);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = inv_det * dot(edge2, q);
    (0.0..=1.0).contains(&t)
}

fn segment_intersects_triangle_touching(
    p0: [f64; 3],
    p1: [f64; 3],
    triangle: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    let direction = sub(p1, p0);
    let [a, b, c] = triangle;
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let h = cross(direction, edge2);
    let det = dot(edge1, h);
    if det.abs() < epsilon {
        return false;
    }
    let inv_det = 1.0 / det;
    let s = sub(p0, a);
    let u = inv_det * dot(s, h);
    if u < -epsilon || u > 1.0 + epsilon {
        return false;
    }
    let q = cross(s, edge1);
    let v = inv_det * dot(direction, q);
    if v < -epsilon || u + v > 1.0 + epsilon {
        return false;
    }
    let t = inv_det * dot(edge2, q);
    -epsilon <= t && t <= 1.0 + epsilon
}

fn point_in_triangle(point: [f64; 3], triangle: [[f64; 3]; 3], epsilon: f64) -> bool {
    let [a, b, c] = triangle;
    let normal = cross(sub(b, a), sub(c, a));
    if norm(normal) < epsilon {
        return false;
    }
    if dot(sub(point, a), normal).abs() > epsilon * f64::max(norm(normal), 1.0) {
        return false;
    }
    let v0 = sub(c, a);
    let v1 = sub(b, a);
    let v2 = sub(point, a);
    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);
    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < epsilon {
        return false;
    }
    let inv = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv;
    let v = (dot00 * dot12 - dot01 * dot02) * inv;
    u >= -epsilon && v >= -epsilon && u + v <= 1.0 + epsilon
}

fn coplanar_triangles_overlap_with_area(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    let normal_a = dir_dbl_area(triangle_a[0], triangle_a[1], triangle_a[2]);
    let normal_b = dir_dbl_area(triangle_b[0], triangle_b[1], triangle_b[2]);
    let normal_a_len_sq = dot(normal_a, normal_a);
    let normal_b_len_sq = dot(normal_b, normal_b);
    if normal_a_len_sq == 0.0 || normal_b_len_sq == 0.0 {
        return false;
    }
    let tolerance = epsilon.max(1e-12);
    let normal_cross = cross(normal_a, normal_b);
    if dot(normal_cross, normal_cross) > tolerance * tolerance * normal_a_len_sq * normal_b_len_sq {
        return false;
    }
    let normal_a_len = normal_a_len_sq.sqrt();
    if triangle_b
        .iter()
        .any(|point| dot(sub(*point, triangle_a[0]), normal_a).abs() > tolerance * normal_a_len)
    {
        return false;
    }

    let axis = dominant_axis(normal_a);
    let mut subject = triangle_a
        .iter()
        .map(|point| project_to_2d(*point, axis))
        .collect::<Vec<_>>();
    let mut clipper = triangle_b
        .iter()
        .map(|point| project_to_2d(*point, axis))
        .collect::<Vec<_>>();
    if polygon_signed_area(&subject) < 0.0 {
        subject.reverse();
    }
    if polygon_signed_area(&clipper) < 0.0 {
        clipper.reverse();
    }
    let clipped = clip_polygon(subject, &clipper, tolerance);
    polygon_signed_area(&clipped).abs() > tolerance * tolerance
}

fn dominant_axis(vector: [f64; 3]) -> usize {
    let abs = [vector[0].abs(), vector[1].abs(), vector[2].abs()];
    if abs[0] >= abs[1] && abs[0] >= abs[2] {
        0
    } else if abs[1] >= abs[2] {
        1
    } else {
        2
    }
}

fn project_to_2d(point: [f64; 3], dropped_axis: usize) -> [f64; 2] {
    match dropped_axis {
        0 => [point[1], point[2]],
        1 => [point[0], point[2]],
        _ => [point[0], point[1]],
    }
}

fn polygon_signed_area(points: &[[f64; 2]]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += cross2(points[index], points[next]);
    }
    0.5 * area
}

fn clip_polygon(mut subject: Vec<[f64; 2]>, clipper: &[[f64; 2]], epsilon: f64) -> Vec<[f64; 2]> {
    for index in 0..clipper.len() {
        if subject.is_empty() {
            return subject;
        }
        let clip_start = clipper[index];
        let clip_end = clipper[(index + 1) % clipper.len()];
        let input = subject;
        subject = Vec::new();
        let mut previous = *input.last().expect("non-empty polygon");
        let mut previous_inside = point_inside_clip_edge(previous, clip_start, clip_end, epsilon);
        for current in input {
            let current_inside = point_inside_clip_edge(current, clip_start, clip_end, epsilon);
            if current_inside {
                if !previous_inside {
                    subject.push(line_intersection_2d(
                        previous, current, clip_start, clip_end,
                    ));
                }
                subject.push(current);
            } else if previous_inside {
                subject.push(line_intersection_2d(
                    previous, current, clip_start, clip_end,
                ));
            }
            previous = current;
            previous_inside = current_inside;
        }
    }
    subject
}

fn point_inside_clip_edge(point: [f64; 2], start: [f64; 2], end: [f64; 2], epsilon: f64) -> bool {
    cross2(sub2(end, start), sub2(point, start)) >= -epsilon
}

fn line_intersection_2d(a0: [f64; 2], a1: [f64; 2], b0: [f64; 2], b1: [f64; 2]) -> [f64; 2] {
    let da = sub2(a1, a0);
    let db = sub2(b1, b0);
    let denom = cross2(da, db);
    if denom == 0.0 {
        return a1;
    }
    let t = cross2(sub2(b0, a0), db) / denom;
    [a0[0] + da[0] * t, a0[1] + da[1] * t]
}

fn sub2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

pub(super) fn faces_share_vertex(face_a: [usize; 3], face_b: [usize; 3]) -> bool {
    face_a.iter().any(|vertex_a| face_b.contains(vertex_a))
}
