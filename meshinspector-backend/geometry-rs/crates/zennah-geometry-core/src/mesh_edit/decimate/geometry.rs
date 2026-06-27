pub(super) fn points_almost_equal(left: [f64; 3], right: [f64; 3]) -> bool {
    distance_sq(left, right) <= 1e-24
}

pub(super) fn triangle_area_sq(vertices: &[[f64; 3]], face: [usize; 3]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    distance_sq(cross, [0.0, 0.0, 0.0])
}

pub(super) fn triangle_aspect_ratio(vertices: &[[f64; 3]], face: [usize; 3]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let bc = distance_sq(b, c).sqrt();
    let ca = distance_sq(c, a).sqrt();
    let ab = distance_sq(a, b).sqrt();
    let half_perimeter = 0.5 * (bc + ca + ab);
    let denominator = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if denominator <= 0.0 {
        f64::MAX
    } else {
        bc * ca * ab / denominator
    }
}

pub(super) fn midpoint(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        0.5 * (left[0] + right[0]),
        0.5 * (left[1] + right[1]),
        0.5 * (left[2] + right[2]),
    ]
}

pub(super) fn distance_sq_to_segment(point: [f64; 3], start: [f64; 3], end: [f64; 3]) -> f64 {
    let segment = sub(end, start);
    let len_sq = dot(segment, segment);
    if len_sq <= 1e-24 {
        return distance_sq(point, start);
    }
    let t = (dot(sub(point, start), segment) / len_sq).clamp(0.0, 1.0);
    let closest = add(start, scale(segment, t));
    distance_sq(point, closest)
}

pub(super) fn face_normal(vertices: &[[f64; 3]], face: [usize; 3]) -> [f64; 3] {
    normalized(cross(
        sub(vertices[face[1]], vertices[face[0]]),
        sub(vertices[face[2]], vertices[face[0]]),
    ))
}

fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let len_sq = distance_sq(vector, [0.0, 0.0, 0.0]);
    if len_sq <= 1e-24 {
        [0.0, 0.0, 0.0]
    } else {
        scale(vector, 1.0 / len_sq.sqrt())
    }
}

pub(super) fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

pub(super) fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

pub(super) fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

pub(super) fn mat_vec(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        dot(matrix[0], vector),
        dot(matrix[1], vector),
        dot(matrix[2], vector),
    ]
}

pub(super) fn invert_3x3(matrix: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
    if !det.is_finite() || det.abs() <= 1e-18 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some([
        [
            (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) * inv_det,
            (matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2]) * inv_det,
            (matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1]) * inv_det,
        ],
        [
            (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) * inv_det,
            (matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0]) * inv_det,
            (matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2]) * inv_det,
        ],
        [
            (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]) * inv_det,
            (matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1]) * inv_det,
            (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]) * inv_det,
        ],
    ])
}

pub(super) fn distance_sq(left: [f64; 3], right: [f64; 3]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    dx * dx + dy * dy + dz * dz
}
