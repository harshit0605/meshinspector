use super::super::loop_normal;
use super::FillHoleMetricMode;
use crate::math::{cross, dot, norm, scale, sub};

pub(super) const BAD_TRIANGULATION_METRIC: f64 = 1.0e10;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FillMetricContext {
    pub normal: Option<[f64; 3]>,
    pub reverse_characteristic_tri_area: f64,
    pub fill_metric_up_dir: Option<[f64; 3]>,
}

pub(super) fn fill_metric_context_with_up_dir(
    mode: FillHoleMetricMode,
    points: &[[f64; 3]],
    fill_metric_up_dir: Option<[f64; 3]>,
) -> FillMetricContext {
    let normal = matches!(
        mode,
        FillHoleMetricMode::ParallelPlane
            | FillHoleMetricMode::Plane
            | FillHoleMetricMode::PlaneNormalized
    )
    .then(|| loop_normal(points));
    let reverse_characteristic_tri_area = if mode == FillHoleMetricMode::ComplexFill {
        reverse_characteristic_tri_area(points)
    } else {
        0.0
    };
    FillMetricContext {
        normal,
        reverse_characteristic_tri_area,
        fill_metric_up_dir,
    }
}

pub(super) fn triangle_fill_metric_with_mode(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    fill_metric_mode: FillHoleMetricMode,
    metric_context: FillMetricContext,
) -> f64 {
    match fill_metric_mode {
        FillHoleMetricMode::Circumscribed => circumscribed_triangle_metric(a, b, c),
        FillHoleMetricMode::MinArea => double_area_triangle_metric(a, b, c),
        FillHoleMetricMode::EdgeLength => 0.0,
        FillHoleMetricMode::Universal => circumscribed_triangle_metric(a, b, c),
        FillHoleMetricMode::MaxDihedralAngle => 0.0,
        FillHoleMetricMode::ParallelPlane => 0.0,
        FillHoleMetricMode::ComplexFill => complex_fill_triangle_metric(a, b, c, metric_context),
        FillHoleMetricMode::MinTriAngle => min_tri_angle_metric(a, b, c),
        FillHoleMetricMode::Plane => plane_triangle_metric(a, b, c, metric_context.normal),
        FillHoleMetricMode::PlaneNormalized => {
            plane_normalized_triangle_metric(a, b, c, metric_context.normal)
        }
        FillHoleMetricMode::ComplexStitch => complex_stitch_triangle_metric(a, b, c),
        FillHoleMetricMode::EdgeLengthStitch => norm(sub(c, a)),
        FillHoleMetricMode::VerticalStitch => {
            vertical_stitch_triangle_metric(a, b, c, metric_context.fill_metric_up_dir)
        }
        FillHoleMetricMode::VerticalStitchEdgeBased => 0.0,
    }
}

pub(super) fn edge_fill_metric_with_mode(
    vertices: &[[f64; 3]],
    a: usize,
    b: usize,
    left: usize,
    right: usize,
    fill_metric_mode: FillHoleMetricMode,
    metric_context: FillMetricContext,
) -> Option<f64> {
    match fill_metric_mode {
        FillHoleMetricMode::EdgeLength => Some(norm(sub(vertices[b], vertices[a]))),
        FillHoleMetricMode::Universal => Some(universal_edge_metric(vertices, a, b, left, right)),
        FillHoleMetricMode::MaxDihedralAngle => {
            Some(max_dihedral_angle_edge_metric(vertices, a, b, left, right))
        }
        FillHoleMetricMode::ParallelPlane => Some(parallel_plane_edge_metric(
            vertices,
            a,
            b,
            metric_context.normal,
        )),
        FillHoleMetricMode::ComplexFill => {
            Some(complex_fill_edge_metric(vertices, a, b, left, right))
        }
        FillHoleMetricMode::ComplexStitch => {
            Some(complex_stitch_edge_metric(vertices, a, b, left, right))
        }
        FillHoleMetricMode::VerticalStitchEdgeBased => Some(vertical_stitch_edge_based_metric(
            vertices[a],
            vertices[b],
            metric_context.fill_metric_up_dir,
        )),
        _ => None,
    }
}

pub(super) fn combine_fill_metric_with_mode(
    left: f64,
    right: f64,
    fill_metric_mode: FillHoleMetricMode,
) -> f64 {
    if fill_metric_mode == FillHoleMetricMode::MaxDihedralAngle {
        left.max(right)
    } else {
        left + right
    }
}

fn circumscribed_triangle_metric(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = norm(sub(b, a));
    let bc = norm(sub(c, b));
    let ca = norm(sub(a, c));
    let double_area = double_area_triangle_metric(a, b, c);
    if double_area <= 1e-12 {
        return f64::MAX / 4.0;
    }
    ab * bc * ca / double_area
}

fn double_area_triangle_metric(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    norm(cross(sub(b, a), sub(c, a)))
}

fn reverse_characteristic_tri_area(points: &[[f64; 3]]) -> f64 {
    let mut max_edge_length_sq: f64 = 0.0;
    for (index, point) in points.iter().enumerate() {
        let next = points[(index + 1) % points.len()];
        let edge = sub(next, *point);
        max_edge_length_sq = max_edge_length_sq.max(dot(edge, edge));
    }
    if max_edge_length_sq <= 0.0 {
        1.0
    } else {
        1.0 / max_edge_length_sq
    }
}

fn complex_fill_triangle_metric(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    context: FillMetricContext,
) -> f64 {
    const TRIANGLE_AREA_MODIFIER: f64 = 1.0e2;
    let aspect_ratio = triangle_aspect_ratio(a, b, c);
    if aspect_ratio > BAD_TRIANGULATION_METRIC {
        return BAD_TRIANGULATION_METRIC;
    }
    aspect_ratio
        + TRIANGLE_AREA_MODIFIER
            * double_area_triangle_metric(a, b, c)
            * context.reverse_characteristic_tri_area
}

fn complex_stitch_triangle_metric(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    (triangle_aspect_ratio(a, b, c) - 1.0) * 1.0e-2
}

fn universal_edge_metric(
    vertices: &[[f64; 3]],
    a: usize,
    b: usize,
    left: usize,
    right: usize,
) -> f64 {
    let a_point = vertices[a];
    let b_point = vertices[b];
    let ab = sub(b_point, a_point);
    let left_normal = cross(sub(vertices[left], a_point), ab);
    let right_normal = cross(ab, sub(vertices[right], a_point));
    let double_area_sum = norm(left_normal) + norm(right_normal);
    double_area_sum.sqrt() * (5.0 * dihedral_angle(left_normal, right_normal, ab).abs()).exp()
}

fn max_dihedral_angle_edge_metric(
    vertices: &[[f64; 3]],
    a: usize,
    b: usize,
    left: usize,
    right: usize,
) -> f64 {
    let a_point = vertices[a];
    let b_point = vertices[b];
    let ab = sub(b_point, a_point);
    let left_normal = cross(sub(vertices[left], a_point), ab);
    let right_normal = cross(ab, sub(vertices[right], a_point));
    dihedral_angle(left_normal, right_normal, ab).abs()
}

fn parallel_plane_edge_metric(
    vertices: &[[f64; 3]],
    a: usize,
    b: usize,
    normal: Option<[f64; 3]>,
) -> f64 {
    let Some(normal) = normal else {
        return 0.0;
    };
    dot(normal, sub(vertices[b], vertices[a])).abs()
}

fn complex_fill_edge_metric(
    vertices: &[[f64; 3]],
    a: usize,
    b: usize,
    left: usize,
    right: usize,
) -> f64 {
    let ab = sub(vertices[b], vertices[a]);
    let bc = [-ab[0], -ab[1], -ab[2]];
    let norm_a = cross(sub(vertices[right], vertices[b]), bc);
    let norm_c = cross(sub(vertices[left], vertices[a]), ab);
    let double_area_a = norm(norm_a);
    let double_area_c = norm(norm_c);
    let denom = double_area_a * double_area_c;
    if denom == 0.0 {
        return BAD_TRIANGULATION_METRIC;
    }
    let cos_ac = dot(norm_a, norm_c) / denom;
    if cos_ac <= -1.0 {
        return BAD_TRIANGULATION_METRIC;
    }
    let ratio = (1.0 - cos_ac) / (1.0 + cos_ac);
    ratio * ratio * ratio * ratio
}

fn complex_stitch_edge_metric(
    vertices: &[[f64; 3]],
    a: usize,
    b: usize,
    left: usize,
    right: usize,
) -> f64 {
    let ab = sub(vertices[b], vertices[a]);
    let Some(norm_left) = normalized(cross(sub(vertices[left], vertices[a]), ab)) else {
        return BAD_TRIANGULATION_METRIC;
    };
    let Some(norm_right) = normalized(cross(ab, sub(vertices[right], vertices[a]))) else {
        return BAD_TRIANGULATION_METRIC;
    };
    (1.0 - dot(norm_left, norm_right)) * 1.0e4
}

fn vertical_stitch_triangle_metric(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    fill_metric_up_dir: Option<[f64; 3]>,
) -> f64 {
    let up = fill_metric_up_dir.unwrap_or([0.0, 0.0, 1.0]);
    let ab = sub(b, a);
    let ac = sub(c, a);
    let bc = sub(c, b);
    let normal = cross(ab, ac);
    let parallel_penalty = dot(up, normal).abs();
    dot(normal, normal)
        + 100.0 * parallel_penalty * parallel_penalty
        + 0.5 * (dot(ab, ab) + dot(ac, ac) + dot(bc, bc)).powi(2)
}

fn vertical_stitch_edge_based_metric(
    a: [f64; 3],
    b: [f64; 3],
    fill_metric_up_dir: Option<[f64; 3]>,
) -> f64 {
    let up = fill_metric_up_dir.unwrap_or([0.0, 0.0, 1.0]);
    let ab = sub(b, a);
    let orthogonal = sub(ab, scale(up, dot(ab, up)));
    dot(orthogonal, orthogonal)
}

fn normalized(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = norm(vector);
    if length <= 1e-12 {
        None
    } else {
        Some(scale(vector, 1.0 / length))
    }
}

fn dihedral_angle(left_normal: [f64; 3], right_normal: [f64; 3], edge: [f64; 3]) -> f64 {
    let edge_norm = norm(edge);
    if edge_norm <= 1e-12 {
        return 0.0;
    }
    let edge_dir = [
        edge[0] / edge_norm,
        edge[1] / edge_norm,
        edge[2] / edge_norm,
    ];
    let sin = dot(edge_dir, cross(left_normal, right_normal));
    let cos = dot(left_normal, right_normal);
    sin.atan2(cos)
}

fn plane_triangle_metric(a: [f64; 3], b: [f64; 3], c: [f64; 3], normal: Option<[f64; 3]>) -> f64 {
    if normal.is_some_and(|n| dot(n, cross(sub(b, a), sub(c, a))) < 0.0) {
        return BAD_TRIANGULATION_METRIC;
    }
    circumscribed_triangle_metric(a, b, c)
}

fn plane_normalized_triangle_metric(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    normal: Option<[f64; 3]>,
) -> f64 {
    let Some(normal) = normal else {
        return BAD_TRIANGULATION_METRIC;
    };
    let face_normal = cross(sub(b, a), sub(c, a));
    let face_double_area_sq = dot(face_normal, face_normal);
    if face_double_area_sq == 0.0 {
        return BAD_TRIANGULATION_METRIC;
    }
    let dot_res = dot(normal, face_normal);
    if dot_res < 0.0 || dot_res * dot_res * 4.0 < face_double_area_sq {
        return BAD_TRIANGULATION_METRIC;
    }
    let aspect_ratio = triangle_aspect_ratio(a, b, c);
    if aspect_ratio > BAD_TRIANGULATION_METRIC {
        return BAD_TRIANGULATION_METRIC;
    }
    circumscribed_triangle_metric(a, b, c) * aspect_ratio
}

fn triangle_aspect_ratio(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let bc = norm(sub(c, b));
    let ca = norm(sub(a, c));
    let ab = norm(sub(b, a));
    let half_perimeter = (bc + ca + ab) / 2.0;
    let den = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if den <= 0.0 {
        return f64::MAX;
    }
    bc * ca * ab / den
}

fn min_tri_angle_metric(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    const MAX_SIN: f64 = 0.8660254037844386;
    (25.0 * (MAX_SIN - min_triangle_angle_sin(a, b, c))).exp()
}

fn min_triangle_angle_sin(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = norm(sub(b, a));
    let ca = norm(sub(a, c));
    let bc = norm(sub(c, b));
    if ab <= 0.0 || ca <= 0.0 || bc <= 0.0 {
        return 0.0;
    }
    double_area_triangle_metric(a, b, c) * ab.min(ca).min(bc) / (ab * ca * bc)
}
