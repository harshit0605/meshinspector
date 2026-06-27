pub(super) fn compute_per_vertex_pseudo_normals(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
) -> Vec<[f64; 3]> {
    let mut sums = vec![[0.0; 3]; vertices.len()];
    for face in faces {
        let face_normal = face_normal(vertices, *face);
        if length_sq(face_normal) <= 0.0 {
            continue;
        }
        for corner in 0..3 {
            let vertex = face[corner];
            let next = face[(corner + 1) % 3];
            let previous = face[(corner + 2) % 3];
            let first = subtract(vertices[next], vertices[vertex]);
            let second = subtract(vertices[previous], vertices[vertex]);
            let angle = angle_between(first, second);
            sums[vertex] = add(sums[vertex], scale(face_normal, angle));
        }
    }
    sums.into_iter().map(normalized).collect()
}

pub(super) fn edge_len_sq(
    vertices: &[[f64; 3]],
    vertex_pseudo_normals: Option<&[[f64; 3]]>,
    edge: [usize; 2],
    curvature_priority: f64,
) -> f64 {
    let mut len_sq = distance_sq(vertices[edge[0]], vertices[edge[1]]);
    if curvature_priority > 0.0 {
        if let Some(normals) = vertex_pseudo_normals {
            len_sq *= 1.0 + curvature_priority * distance_sq(normals[edge[0]], normals[edge[1]]);
        }
    }
    len_sq
}

pub(super) fn interpolated_split_normal(normals: &[[f64; 3]], edge: [usize; 2]) -> [f64; 3] {
    normalized(add(normals[edge[0]], normals[edge[1]]))
}

pub(super) fn triangle_aspect_ratio(vertices: &[[f64; 3]], face: [usize; 3]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let bc = distance(c, b);
    let ca = distance(a, c);
    let ab = distance(b, a);
    let half_perimeter = (bc + ca + ab) / 2.0;
    let denominator = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if denominator <= 0.0 {
        return f64::MAX;
    }
    bc * ca * ab / denominator
}

fn face_normal(vertices: &[[f64; 3]], face: [usize; 3]) -> [f64; 3] {
    normalized(cross(
        subtract(vertices[face[1]], vertices[face[0]]),
        subtract(vertices[face[2]], vertices[face[0]]),
    ))
}

fn angle_between(first: [f64; 3], second: [f64; 3]) -> f64 {
    let denominator = length(first) * length(second);
    if denominator <= 0.0 {
        return 0.0;
    }
    let cosine = (dot(first, second) / denominator).clamp(-1.0, 1.0);
    cosine.acos()
}

fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let len = length(vector);
    if len <= 0.0 {
        return [0.0; 3];
    }
    [vector[0] / len, vector[1] / len, vector[2] / len]
}

fn distance_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    length_sq(subtract(a, b))
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    distance_sq(a, b).sqrt()
}

fn length(vector: [f64; 3]) -> f64 {
    length_sq(vector).sqrt()
}

fn length_sq(vector: [f64; 3]) -> f64 {
    dot(vector, vector)
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

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
