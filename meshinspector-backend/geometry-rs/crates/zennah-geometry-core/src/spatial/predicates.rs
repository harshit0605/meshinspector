use crate::math::{cross, dot, norm, sub};

pub(super) fn triangles_intersect(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    if !triangle_aabb_overlap(triangle_a, triangle_b, epsilon) {
        return false;
    }
    for index in 0..3 {
        if segment_intersects_triangle(
            triangle_a[index],
            triangle_a[(index + 1) % 3],
            triangle_b,
            epsilon,
        ) {
            return true;
        }
        if segment_intersects_triangle(
            triangle_b[index],
            triangle_b[(index + 1) % 3],
            triangle_a,
            epsilon,
        ) {
            return true;
        }
    }
    triangle_a
        .iter()
        .any(|point| point_in_triangle(*point, triangle_b, epsilon))
        || triangle_b
            .iter()
            .any(|point| point_in_triangle(*point, triangle_a, epsilon))
}

fn triangle_aabb_overlap(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    let (a_min, a_max) = triangle_bounds(triangle_a);
    let (b_min, b_max) = triangle_bounds(triangle_b);
    aabb_bounds_overlap(a_min, a_max, b_min, b_max, epsilon)
}

pub(super) fn aabb_bounds_overlap(
    a_min: [f64; 3],
    a_max: [f64; 3],
    b_min: [f64; 3],
    b_max: [f64; 3],
    epsilon: f64,
) -> bool {
    (0..3).all(|axis| a_min[axis] <= b_max[axis] + epsilon && b_min[axis] <= a_max[axis] + epsilon)
}

fn triangle_bounds(triangle: [[f64; 3]; 3]) -> ([f64; 3], [f64; 3]) {
    let mut bbox_min = triangle[0];
    let mut bbox_max = triangle[0];
    for vertex in triangle.iter().skip(1) {
        for axis in 0..3 {
            bbox_min[axis] = bbox_min[axis].min(vertex[axis]);
            bbox_max[axis] = bbox_max[axis].max(vertex[axis]);
        }
    }
    (bbox_min, bbox_max)
}

fn segment_intersects_triangle(
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

pub(super) fn faces_share_vertex(face_a: [usize; 3], face_b: [usize; 3]) -> bool {
    face_a.iter().any(|vertex_a| face_b.contains(vertex_a))
}
