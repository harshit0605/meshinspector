use super::topology::ordered_edge;

pub(super) fn is_inner_edge(faces: &[[usize; 3]], edge: [usize; 2]) -> bool {
    faces
        .iter()
        .filter(|face| {
            face_edges(**face)
                .into_iter()
                .any(|face_edge| face_edge == edge)
        })
        .take(2)
        .count()
        == 2
}

pub(super) fn project_vertices_to_original_mesh(
    vertices: &mut [[f64; 3]],
    vertex_indices: &[usize],
    original_vertices: &[[f64; 3]],
    original_faces: &[[usize; 3]],
) {
    for vertex_index in vertex_indices {
        let point = vertices[*vertex_index];
        if let Some(projected) = closest_point_on_mesh(point, original_vertices, original_faces) {
            vertices[*vertex_index] = projected;
        }
    }
}

fn closest_point_on_mesh(
    point: [f64; 3],
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
) -> Option<[f64; 3]> {
    let mut best_point = None;
    let mut best_distance_sq = f64::INFINITY;
    for face in faces {
        let candidate = closest_point_on_triangle(
            point,
            vertices[face[0]],
            vertices[face[1]],
            vertices[face[2]],
        );
        let distance_sq = distance_sq(point, candidate);
        if distance_sq < best_distance_sq {
            best_distance_sq = distance_sq;
            best_point = Some(candidate);
        }
    }
    best_point
}

fn closest_point_on_triangle(point: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [f64; 3] {
    let ab = subtract(b, a);
    let ac = subtract(c, a);
    let ap = subtract(point, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = subtract(point, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return add(a, scale(ab, d1 / (d1 - d3)));
    }

    let cp = subtract(point, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return add(a, scale(ac, d2 / (d2 - d6)));
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return add(
            b,
            scale(subtract(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))),
        );
    }

    let denominator = va + vb + vc;
    if denominator == 0.0 {
        return a;
    }
    let v = vb / denominator;
    let w = vc / denominator;
    add(a, add(scale(ab, v), scale(ac, w)))
}

fn face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [
        ordered_edge(face[0], face[1]),
        ordered_edge(face[1], face[2]),
        ordered_edge(face[2], face[0]),
    ]
}

fn distance_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(subtract(a, b), subtract(a, b))
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
