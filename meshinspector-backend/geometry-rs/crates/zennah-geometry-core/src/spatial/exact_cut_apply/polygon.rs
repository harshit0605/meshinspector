use crate::math::{cross, dot, sub};

pub(super) fn boundary_path(
    face: [usize; 3],
    start_vertex: usize,
    start_pos: f64,
    end_vertex: usize,
    end_pos: f64,
) -> Vec<usize> {
    let mut path = vec![start_vertex];
    let end_unwrapped = if start_pos < end_pos {
        end_pos
    } else {
        end_pos + 3.0
    };
    let mut position = start_pos.floor() as i32 + 1;
    while (position as f64) < end_unwrapped {
        let vertex = face[position.rem_euclid(3) as usize];
        path.push(vertex);
        position += 1;
    }
    path.push(end_vertex);
    dedupe_adjacent(path)
}

pub(super) fn dedupe_closed_polygon(vertices: &[usize]) -> Vec<usize> {
    let mut output = dedupe_adjacent(vertices.to_vec());
    if output.len() > 1 && output.first() == output.last() {
        output.pop();
    }
    output
}

fn dedupe_adjacent(vertices: Vec<usize>) -> Vec<usize> {
    let mut output = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        if output.last().copied() != Some(vertex) {
            output.push(vertex);
        }
    }
    output
}

/// Ear-clip a simple (possibly non-convex) planar polygon given as vertex
/// indices into `vertices`, returning a triangulation as vertex-index triples.
/// The polygon lies on the plane of `face_normal`; every output triangle's
/// winding is normalised to that normal so the cut surface stays consistently
/// oriented. Returns `None` if the polygon is degenerate or not ear-clippable
/// (the caller then falls back, and the watertightness gate keeps production
/// on the safe cap path).
pub(super) fn ear_clip_planar_polygon(
    polygon: &[usize],
    vertices: &[[f64; 3]],
    face_normal: [f64; 3],
    epsilon: f64,
) -> Option<Vec<[usize; 3]>> {
    let polygon = dedupe_closed_polygon(polygon);
    let n = polygon.len();
    if n < 3 {
        return None;
    }
    // Project onto the plane's dominant axes (drop the largest normal component).
    let drop_axis = dominant_axis(face_normal);
    let proj: Vec<[f64; 2]> = polygon
        .iter()
        .map(|vertex| project_2d(vertices[*vertex], drop_axis))
        .collect();
    // Iterate vertices in CCW order so a positive turn marks a convex (ear) corner.
    let mut order: Vec<usize> = (0..n).collect();
    if signed_area_2d(&proj, &order) < 0.0 {
        order.reverse();
    }
    let area_epsilon = epsilon * epsilon;

    let mut triangles = Vec::with_capacity(n - 2);
    let mut guard = 0usize;
    while order.len() > 3 {
        let m = order.len();
        let mut clipped = false;
        for i in 0..m {
            let prev = order[(i + m - 1) % m];
            let curr = order[i];
            let next = order[(i + 1) % m];
            if is_ear(prev, curr, next, &order, &proj, area_epsilon) {
                triangles.push([polygon[prev], polygon[curr], polygon[next]]);
                order.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            return None;
        }
        guard += 1;
        if guard > n * n + n {
            return None;
        }
    }
    triangles.push([polygon[order[0]], polygon[order[1]], polygon[order[2]]]);

    for triangle in &mut triangles {
        let triangle_normal = cross(
            sub(vertices[triangle[1]], vertices[triangle[0]]),
            sub(vertices[triangle[2]], vertices[triangle[0]]),
        );
        if dot(triangle_normal, face_normal) < 0.0 {
            triangle.swap(1, 2);
        }
    }
    Some(triangles)
}

fn dominant_axis(normal: [f64; 3]) -> usize {
    let abs = [normal[0].abs(), normal[1].abs(), normal[2].abs()];
    if abs[0] >= abs[1] && abs[0] >= abs[2] {
        0
    } else if abs[1] >= abs[2] {
        1
    } else {
        2
    }
}

fn project_2d(point: [f64; 3], drop_axis: usize) -> [f64; 2] {
    match drop_axis {
        0 => [point[1], point[2]],
        1 => [point[2], point[0]],
        _ => [point[0], point[1]],
    }
}

fn signed_area_2d(proj: &[[f64; 2]], order: &[usize]) -> f64 {
    let mut area = 0.0;
    let m = order.len();
    for i in 0..m {
        let a = proj[order[i]];
        let b = proj[order[(i + 1) % m]];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area * 0.5
}

fn cross_2d(origin: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - origin[0]) * (b[1] - origin[1]) - (a[1] - origin[1]) * (b[0] - origin[0])
}

fn is_ear(
    prev: usize,
    curr: usize,
    next: usize,
    order: &[usize],
    proj: &[[f64; 2]],
    area_epsilon: f64,
) -> bool {
    let a = proj[prev];
    let b = proj[curr];
    let c = proj[next];
    // Reflex or collinear corners are not ears.
    if cross_2d(a, b, c) <= area_epsilon {
        return false;
    }
    for &idx in order {
        if idx == prev || idx == curr || idx == next {
            continue;
        }
        if point_in_triangle_2d(proj[idx], a, b, c) {
            return false;
        }
    }
    true
}

fn point_in_triangle_2d(point: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let d1 = cross_2d(a, b, point);
    let d2 = cross_2d(b, c, point);
    let d3 = cross_2d(c, a, point);
    let has_negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_negative && has_positive)
}
