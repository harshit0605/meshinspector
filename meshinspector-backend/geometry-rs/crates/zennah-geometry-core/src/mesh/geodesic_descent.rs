use crate::math::{add, distance_sq, dot, scale, sub};
use crate::types::GeometryError;

use super::base::validate_faces;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSteepestDescentTriangleStep {
    pub face_index: usize,
    pub start_barycentric: [f64; 3],
    pub start_point: [f64; 3],
    pub start_value: f64,
    pub gradient: [f64; 3],
    pub gradient_norm: f64,
    pub crossed_edge: Option<[usize; 2]>,
    pub edge_position: Option<f64>,
    pub crossing_point: Option<[f64; 3]>,
    pub kind: &'static str,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSteepestDescentEdgeStep {
    pub start_edge: [usize; 2],
    pub edge_position: f64,
    pub start_point: [f64; 3],
    pub start_value: f64,
    pub crossed_edge: Option<[usize; 2]>,
    pub crossing_edge_position: Option<f64>,
    pub crossing_point: Option<[f64; 3]>,
    pub kind: &'static str,
    pub side: &'static str,
    pub meshlib_reference: &'static str,
}

pub fn mesh_steepest_descent_triangle_step(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    vertex_scalars: &[f64],
    face_index: usize,
    start_barycentric: [f64; 3],
) -> Result<MeshSteepestDescentTriangleStep, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if face_index >= faces.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "face_index",
            value: format!("{face_index} for {} faces", faces.len()),
        });
    }
    if vertex_scalars.len() != vertices.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_scalars",
            value: format!(
                "scalar count {} does not match vertex count {}",
                vertex_scalars.len(),
                vertices.len()
            ),
        });
    }
    validate_barycentric(start_barycentric)?;

    let face = faces[face_index];
    let pv = [vertices[face[0]], vertices[face[1]], vertices[face[2]]];
    let vv = [
        vertex_scalars[face[0]],
        vertex_scalars[face[1]],
        vertex_scalars[face[2]],
    ];
    if vv.iter().any(|value| !value.is_finite()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_scalars",
            value: "scalars_must_be_finite".to_string(),
        });
    }
    let start_point = interpolate_triangle(pv, start_barycentric);
    let start_value =
        start_barycentric[0] * vv[0] + start_barycentric[1] * vv[1] + start_barycentric[2] * vv[2];
    let gradient = triangle_gradient(pv[0], pv[1], pv[2], vv[1] - vv[0], vv[2] - vv[0]);
    let gradient_norm = distance_sq(gradient, [0.0, 0.0, 0.0]).sqrt();
    if vv[0] == vv[1] && vv[1] == vv[2] {
        return Ok(MeshSteepestDescentTriangleStep {
            face_index,
            start_barycentric,
            start_point,
            start_value,
            gradient,
            gradient_norm,
            crossed_edge: None,
            edge_position: None,
            crossing_point: None,
            kind: "flat",
            meshlib_reference: "MR::findSteepestDescentPoint(MeshTriPoint)",
        });
    }

    if gradient_norm > 0.0 {
        if let Some((edge, position, point, kind)) =
            gradient_exit_edge(face, pv, start_point, scale(gradient, 1.0 / gradient_norm))
        {
            return Ok(MeshSteepestDescentTriangleStep {
                face_index,
                start_barycentric,
                start_point,
                start_value,
                gradient,
                gradient_norm,
                crossed_edge: Some(edge),
                edge_position: Some(position),
                crossing_point: Some(point),
                kind,
                meshlib_reference: "MR::findSteepestDescentPoint(MeshTriPoint)",
            });
        }
    }

    let (edge, point) = fallback_lowest_vertex(face, pv, vv, start_point, start_value)?;
    Ok(MeshSteepestDescentTriangleStep {
        face_index,
        start_barycentric,
        start_point,
        start_value,
        gradient,
        gradient_norm,
        crossed_edge: Some(edge),
        edge_position: Some(0.0),
        crossing_point: Some(point),
        kind: "vertex",
        meshlib_reference: "MR::findSteepestDescentPoint(MeshTriPoint)",
    })
}

pub fn mesh_steepest_descent_edge_step(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    vertex_scalars: &[f64],
    edge_i64: [i64; 2],
    edge_position: f64,
) -> Result<MeshSteepestDescentEdgeStep, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if vertex_scalars.len() != vertices.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_scalars",
            value: format!(
                "scalar count {} does not match vertex count {}",
                vertex_scalars.len(),
                vertices.len()
            ),
        });
    }
    if vertex_scalars.iter().any(|value| !value.is_finite()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_scalars",
            value: "scalars_must_be_finite".to_string(),
        });
    }
    if !edge_position.is_finite() || !(0.0..=1.0).contains(&edge_position) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "edge_position",
            value: format!("expected_0_to_1_got_{edge_position}"),
        });
    }
    let start_edge = validate_edge(edge_i64, vertices.len())?;
    let [o, d] = start_edge;
    let po = vertices[o];
    let pd = vertices[d];
    let fo = vertex_scalars[o];
    let fd = vertex_scalars[d];
    let start_point = edge_point(po, pd, edge_position);
    let start_value = (1.0 - edge_position) * fo + edge_position * fd;

    let mut best: Option<EdgeCandidate> = None;
    let mut max_grad_sq = f64::NEG_INFINITY;
    if fo != fd {
        let od_sq = distance_sq(po, pd);
        max_grad_sq = if od_sq > 0.0 {
            (fo - fd).powi(2) / od_sq
        } else {
            f64::MAX
        };
        let edge = if fo < fd { [o, d] } else { [d, o] };
        best = Some(EdgeCandidate::vertex(edge, 0.0, "edge"));
    }

    if let Some(face) = directed_face(&faces, o, d) {
        consider_edge_side(
            vertices,
            vertex_scalars,
            [o, d, face.third],
            start_point,
            start_value,
            "left",
            &mut max_grad_sq,
            &mut best,
        );
    }
    if let Some(face) = directed_face(&faces, d, o) {
        consider_edge_side(
            vertices,
            vertex_scalars,
            [d, o, face.third],
            start_point,
            start_value,
            "right",
            &mut max_grad_sq,
            &mut best,
        );
    }

    let candidate = best.unwrap_or_else(|| {
        if edge_position <= 0.5 {
            EdgeCandidate::vertex([o, d], 0.0, "edge")
        } else {
            EdgeCandidate::vertex([d, o], 0.0, "edge")
        }
    });
    let crossing_point = edge_point(
        vertices[candidate.edge[0]],
        vertices[candidate.edge[1]],
        candidate.position,
    );
    Ok(MeshSteepestDescentEdgeStep {
        start_edge,
        edge_position,
        start_point,
        start_value,
        crossed_edge: Some(candidate.edge),
        crossing_edge_position: Some(candidate.position),
        crossing_point: Some(crossing_point),
        kind: candidate.kind,
        side: candidate.side,
        meshlib_reference: "MR::findSteepestDescentPoint(MeshEdgePoint)",
    })
}

#[derive(Debug, Clone, Copy)]
struct DirectedFace {
    third: usize,
}

#[derive(Debug, Clone, Copy)]
struct EdgeCandidate {
    edge: [usize; 2],
    position: f64,
    kind: &'static str,
    side: &'static str,
}

impl EdgeCandidate {
    fn edge(edge: [usize; 2], position: f64, side: &'static str) -> Self {
        let kind = if position <= 1e-12 || position >= 1.0 - 1e-12 {
            "vertex"
        } else {
            "edge"
        };
        Self {
            edge,
            position,
            kind,
            side,
        }
    }

    fn vertex(edge: [usize; 2], position: f64, side: &'static str) -> Self {
        Self {
            edge,
            position,
            kind: "vertex",
            side,
        }
    }
}

fn consider_edge_side(
    vertices: &[[f64; 3]],
    vertex_scalars: &[f64],
    tri_vertices: [usize; 3],
    start_point: [f64; 3],
    start_value: f64,
    side: &'static str,
    max_grad_sq: &mut f64,
    best: &mut Option<EdgeCandidate>,
) {
    let [a, b, c] = tri_vertices;
    let pa = vertices[a];
    let pb = vertices[b];
    let pc = vertices[c];
    let fa = vertex_scalars[a];
    let fb = vertex_scalars[b];
    let fc = vertex_scalars[c];
    let gradient = triangle_gradient(pa, pb, pc, fb - fa, fc - fa);
    let grad_sq = distance_sq(gradient, [0.0, 0.0, 0.0]);
    let mut move_to_third = true;
    if grad_sq > *max_grad_sq {
        let unit_dir = scale(gradient, 1.0 / grad_sq.sqrt());
        move_to_third = false;
        if !dir_enters_01([pa, pb, pc], unit_dir) {
            if let Some(position) = compute_enter_01_cross([pb, pc, pa], unit_dir, start_point) {
                if position >= 0.0 {
                    if position <= 1.0 {
                        *best = Some(EdgeCandidate::edge([b, c], position, side));
                        *max_grad_sq = grad_sq;
                        move_to_third = false;
                    } else {
                        move_to_third = true;
                    }
                }
            }
            if let Some(position) = compute_enter_01_cross([pc, pa, pb], unit_dir, start_point) {
                if position <= 1.0 {
                    if position >= 0.0 {
                        *best = Some(EdgeCandidate::edge([c, a], position, side));
                        *max_grad_sq = grad_sq;
                        move_to_third = false;
                    } else {
                        move_to_third = true;
                    }
                }
            }
        }
    }
    if move_to_third && fc <= start_value {
        let dist_sq = distance_sq(pc, start_point);
        let vert_grad_sq = if dist_sq > 0.0 {
            (fc - start_value).powi(2) / dist_sq
        } else {
            f64::MAX
        };
        if vert_grad_sq >= *max_grad_sq {
            *best = Some(EdgeCandidate::vertex([c, a], 0.0, side));
            *max_grad_sq = vert_grad_sq;
        }
    }
}

fn gradient_exit_edge(
    face: [usize; 3],
    pv: [[f64; 3]; 3],
    start_point: [f64; 3],
    unit_dir: [f64; 3],
) -> Option<([usize; 2], f64, [f64; 3], &'static str)> {
    let mut best: Option<(f64, [usize; 2], f64)> = None;
    for i in 0..3 {
        let tri = [pv[i], pv[(i + 1) % 3], pv[(i + 2) % 3]];
        if !dir_enters_01(tri, unit_dir) {
            continue;
        }
        let edge = [face[i], face[(i + 1) % 3]];
        let position =
            match line_line_cross(sub(tri[0], start_point), sub(tri[1], start_point), unit_dir) {
                Some(value) => value,
                None => {
                    let position = if dot(sub(tri[1], tri[0]), unit_dir) >= 0.0 {
                        0.0
                    } else {
                        1.0
                    };
                    return Some((
                        edge,
                        position,
                        edge_point(tri[0], tri[1], position),
                        "vertex",
                    ));
                }
            };
        let clamped = position.clamp(0.0, 1.0);
        let miss = (position - clamped).abs() * distance_sq(tri[0], tri[1]).sqrt();
        if best.is_none_or(|(best_miss, _, _)| miss < best_miss) {
            best = Some((miss, edge, clamped));
        }
    }
    let (_, edge, position) = best?;
    let point = edge_point(
        pv[face_corner(face, edge[0])?],
        pv[face_corner(face, edge[1])?],
        position,
    );
    let kind = if position <= 1e-12 || position >= 1.0 - 1e-12 {
        "vertex"
    } else {
        "edge"
    };
    Some((edge, position, point, kind))
}

fn compute_enter_01_cross(
    triangle: [[f64; 3]; 3],
    unit_dir: [f64; 3],
    point: [f64; 3],
) -> Option<f64> {
    if !dir_enters_01(triangle, unit_dir) {
        return None;
    }
    line_line_cross(sub(triangle[0], point), sub(triangle[1], point), unit_dir)
}

fn directed_face(faces: &[[usize; 3]], from: usize, to: usize) -> Option<DirectedFace> {
    for face in faces {
        for i in 0..3 {
            if face[i] == from && face[(i + 1) % 3] == to {
                return Some(DirectedFace {
                    third: face[(i + 2) % 3],
                });
            }
        }
    }
    None
}

fn validate_edge(edge: [i64; 2], vertex_count: usize) -> Result<[usize; 2], GeometryError> {
    let mut output = [0_usize; 2];
    for (index, value) in edge.into_iter().enumerate() {
        if value < 0 || value as usize >= vertex_count {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "edge",
                value: format!("{edge:?} for {vertex_count} vertices"),
            });
        }
        output[index] = value as usize;
    }
    if output[0] == output[1] {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "edge",
            value: "edge_endpoints_must_differ".to_string(),
        });
    }
    Ok(output)
}

fn fallback_lowest_vertex(
    face: [usize; 3],
    pv: [[f64; 3]; 3],
    vv: [f64; 3],
    start_point: [f64; 3],
    start_value: f64,
) -> Result<([usize; 2], [f64; 3]), GeometryError> {
    let mut best: Option<(f64, usize)> = None;
    for i in 0..3 {
        if vv[i] <= start_value {
            let dist_sq = distance_sq(pv[i], start_point);
            let grad_sq = if dist_sq > 0.0 {
                (vv[i] - start_value).powi(2) / dist_sq
            } else {
                f64::MAX
            };
            if best.is_none_or(|(best_grad_sq, _)| grad_sq > best_grad_sq) {
                best = Some((grad_sq, i));
            }
        }
    }
    let Some((_, corner)) = best else {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "triangle_step",
            value: "no_descending_edge_or_vertex".to_string(),
        });
    };
    Ok(([face[corner], face[(corner + 1) % 3]], pv[corner]))
}

fn triangle_gradient(a: [f64; 3], b: [f64; 3], c: [f64; 3], vb: f64, vc: f64) -> [f64; 3] {
    let e0 = sub(b, a);
    let e1 = sub(c, a);
    let d00 = dot(e0, e0);
    let d01 = dot(e0, e1);
    let d11 = dot(e1, e1);
    let denominator = d00 * d11 - d01 * d01;
    if denominator.abs() <= 1e-18 {
        return [0.0, 0.0, 0.0];
    }
    let alpha = (vb * d11 - vc * d01) / denominator;
    let beta = (vc * d00 - vb * d01) / denominator;
    add(scale(e0, alpha), scale(e1, beta))
}

fn dir_enters_01(triangle: [[f64; 3]; 3], dir: [f64; 3]) -> bool {
    let edge = sub(triangle[1], triangle[0]);
    let edge_len = distance_sq(edge, [0.0, 0.0, 0.0]).sqrt();
    if edge_len <= 1e-12 {
        return false;
    }
    let u01 = scale(edge, 1.0 / edge_len);
    let ort_dir = sub(dir, scale(u01, dot(dir, u01)));
    dot(ort_dir, sub(triangle[2], triangle[0])) > 0.0
}

fn line_line_cross(b: [f64; 3], c: [f64; 3], unit_dir: [f64; 3]) -> Option<f64> {
    let d = sub(c, b);
    let gort = sub(d, scale(unit_dir, dot(d, unit_dir)));
    let god = dot(gort, d);
    if god <= 0.0 {
        return None;
    }
    Some(-dot(gort, b) / god)
}

fn validate_barycentric(weights: [f64; 3]) -> Result<(), GeometryError> {
    if weights.iter().any(|value| !value.is_finite()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "start_barycentric",
            value: "weights_must_be_finite".to_string(),
        });
    }
    let sum = weights[0] + weights[1] + weights[2];
    if (sum - 1.0).abs() > 1e-8 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "start_barycentric",
            value: format!("weights_sum_must_be_1_got_{sum}"),
        });
    }
    Ok(())
}

fn interpolate_triangle(points: [[f64; 3]; 3], weights: [f64; 3]) -> [f64; 3] {
    add(
        add(scale(points[0], weights[0]), scale(points[1], weights[1])),
        scale(points[2], weights[2]),
    )
}

fn edge_point(a: [f64; 3], b: [f64; 3], position: f64) -> [f64; 3] {
    add(scale(a, 1.0 - position), scale(b, position))
}

fn face_corner(face: [usize; 3], vertex: usize) -> Option<usize> {
    face.into_iter().position(|value| value == vertex)
}
