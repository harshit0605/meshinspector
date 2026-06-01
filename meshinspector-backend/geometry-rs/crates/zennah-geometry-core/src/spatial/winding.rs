use crate::math::{cross, dot, norm, sub};

pub(super) fn triangle_solid_angle(point: [f64; 3], triangle: [[f64; 3]; 3]) -> f64 {
    let a = sub(triangle[0], point);
    let b = sub(triangle[1], point);
    let c = sub(triangle[2], point);
    let la = norm(a);
    let lb = norm(b);
    let lc = norm(c);
    let numerator = dot(a, cross(b, c));
    let denominator = la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb;
    2.0 * numerator.atan2(denominator)
}
