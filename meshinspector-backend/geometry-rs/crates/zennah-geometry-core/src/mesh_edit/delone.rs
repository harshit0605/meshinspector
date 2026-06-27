use crate::{GeometryError, MeshArrays};

use super::input;
use super::topology::{ordered_edge, EdgeState};
use super::{delone_edge_satisfied, edge_incident_faces, flip_edge, opposite_vertex};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct MakeDeloneEdgeFlipsOptions {
    pub num_iters: usize,
    pub region_faces: Option<Vec<usize>>,
    pub max_deviation_after_flip: Option<f64>,
    pub max_angle_change: Option<f64>,
    pub critical_tri_aspect_ratio: Option<f64>,
    pub not_flippable_edges: Vec<[usize; 2]>,
    pub vert_region: Option<Vec<usize>>,
}

impl Default for MakeDeloneEdgeFlipsOptions {
    fn default() -> Self {
        Self {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MakeDeloneEdgeFlipsResult {
    pub mesh: MeshArrays,
    pub flips_done: usize,
    pub region_faces: Vec<usize>,
    pub region_face_count: usize,
}

pub fn make_delone_edge_flips(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    options: MakeDeloneEdgeFlipsOptions,
) -> Result<MakeDeloneEdgeFlipsResult, GeometryError> {
    let mut faces = input::validate_faces(faces_i64, vertices.len())?;
    if let Some(max_deviation) = options.max_deviation_after_flip {
        if max_deviation < 0.0 || !max_deviation.is_finite() {
            return Err(GeometryError::InvalidMeshEditInput {
                field: "max_deviation_after_flip",
                value: max_deviation,
            });
        }
    }
    if let Some(max_angle_change) = options.max_angle_change {
        if max_angle_change < 0.0 || !max_angle_change.is_finite() {
            return Err(GeometryError::InvalidMeshEditInput {
                field: "max_angle_change",
                value: max_angle_change,
            });
        }
    }
    if let Some(critical_tri_aspect_ratio) = options.critical_tri_aspect_ratio {
        if critical_tri_aspect_ratio < 0.0 || !critical_tri_aspect_ratio.is_finite() {
            return Err(GeometryError::InvalidMeshEditInput {
                field: "critical_tri_aspect_ratio",
                value: critical_tri_aspect_ratio,
            });
        }
    }
    let not_flippable_edges =
        normalize_not_flippable_edges(&options.not_flippable_edges, vertices.len())?;
    let vert_region = normalize_vert_region(options.vert_region.as_deref(), vertices.len())?;
    let mut edge_state = EdgeState::from_faces(&faces);
    let mut region = input::initial_region(faces.len(), options.region_faces.as_deref())?;
    let mut flips_done = 0_usize;

    for _ in 0..options.num_iters {
        let incident = edge_incident_faces(&faces);
        let mut edge_set = incident.keys().copied().collect::<BTreeSet<_>>();
        let mut candidates = BTreeSet::<[usize; 2]>::new();
        for (edge, face_indices) in &incident {
            let edge = *edge;
            if face_indices.len() != 2 {
                continue;
            }
            if !region.get(face_indices[0]).copied().unwrap_or(false)
                || !region.get(face_indices[1]).copied().unwrap_or(false)
            {
                continue;
            }
            if not_flippable_edges.contains(&edge) {
                continue;
            }
            if !delone_edge_matches_vert_region(
                &faces,
                edge,
                [face_indices[0], face_indices[1]],
                vert_region.as_ref(),
            ) {
                continue;
            }
            if !delone_edge_satisfied_with_settings(
                vertices,
                &faces,
                edge,
                [face_indices[0], face_indices[1]],
                &options,
            ) {
                candidates.insert(edge);
            }
        }

        let flips_before_iter = flips_done;
        for edge in candidates {
            let Some((face_pair, new_edge)) = delone_flippable_face_pair(
                &faces,
                &region,
                edge,
                &incident,
                &edge_set,
                &not_flippable_edges,
                vert_region.as_ref(),
            ) else {
                continue;
            };
            if delone_edge_satisfied_with_settings(vertices, &faces, edge, face_pair, &options) {
                continue;
            }
            flip_edge(&mut faces, &mut region, &mut edge_state, edge, face_pair);
            edge_set.remove(&edge);
            edge_set.insert(new_edge);
            flips_done += 1;
        }
        if flips_done == flips_before_iter {
            break;
        }
    }

    let region_faces: Vec<usize> = region
        .iter()
        .enumerate()
        .filter_map(|(index, selected)| selected.then_some(index))
        .collect();
    Ok(MakeDeloneEdgeFlipsResult {
        mesh: MeshArrays {
            vertices: vertices.to_vec(),
            faces: faces
                .into_iter()
                .map(|face| [face[0] as i64, face[1] as i64, face[2] as i64])
                .collect(),
        },
        flips_done,
        region_face_count: region_faces.len(),
        region_faces,
    })
}

fn delone_edge_satisfied_with_settings(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    edge: [usize; 2],
    face_pair: [usize; 2],
    options: &MakeDeloneEdgeFlipsOptions,
) -> bool {
    delone_edge_satisfied_with_flip_limits(
        vertices,
        faces,
        edge,
        face_pair,
        options.max_deviation_after_flip,
        options.max_angle_change,
        options.critical_tri_aspect_ratio,
    )
}

pub(in crate::mesh_edit) fn delone_edge_satisfied_with_flip_limits(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    edge: [usize; 2],
    face_pair: [usize; 2],
    max_deviation_after_flip: Option<f64>,
    max_angle_change: Option<f64>,
    critical_tri_aspect_ratio: Option<f64>,
) -> bool {
    if let Some(max_deviation) = max_deviation_after_flip {
        if let Some(deviation_sq) = deviation_sq_after_flip(vertices, faces, edge, face_pair) {
            if deviation_sq > max_deviation * max_deviation {
                return true;
            }
        }
    }
    if let Some(mut max_angle_change) = max_angle_change {
        if let Some(critical_tri_aspect_ratio) = critical_tri_aspect_ratio {
            if let Some(max_aspect) =
                current_delone_tri_aspect_ratio(vertices, faces, edge, face_pair)
            {
                if max_aspect > critical_tri_aspect_ratio {
                    max_angle_change = f64::INFINITY;
                }
            }
        }
        return delone_edge_satisfied_with_angle_limit(
            vertices,
            faces,
            edge,
            face_pair,
            max_angle_change,
        );
    }
    delone_edge_satisfied(vertices, faces, edge, face_pair)
}

fn current_delone_tri_aspect_ratio(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    edge: [usize; 2],
    face_pair: [usize; 2],
) -> Option<f64> {
    let (a_index, c_index, d_index) =
        oriented_edge_with_opposite_for_delone(faces[face_pair[0]], edge)?;
    let b_index = opposite_vertex(faces[face_pair[1]], edge)?;
    Some(
        triangle_aspect_ratio(vertices[a_index], vertices[c_index], vertices[d_index]).max(
            triangle_aspect_ratio(vertices[c_index], vertices[a_index], vertices[b_index]),
        ),
    )
}

pub(in crate::mesh_edit) fn deviation_sq_after_flip(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    edge: [usize; 2],
    face_pair: [usize; 2],
) -> Option<f64> {
    let first_opposite = opposite_vertex(faces[face_pair[0]], edge)?;
    let second_opposite = opposite_vertex(faces[face_pair[1]], edge)?;
    Some(segment_segment_distance_sq(
        vertices[edge[0]],
        vertices[edge[1]],
        vertices[first_opposite],
        vertices[second_opposite],
    ))
}

fn delone_flippable_face_pair(
    faces: &[[usize; 3]],
    region: &[bool],
    edge: [usize; 2],
    incident: &BTreeMap<[usize; 2], Vec<usize>>,
    edge_set: &BTreeSet<[usize; 2]>,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    vert_region: Option<&BTreeSet<usize>>,
) -> Option<([usize; 2], [usize; 2])> {
    if not_flippable_edges.contains(&edge) {
        return None;
    }
    let face_indices = incident.get(&edge)?;
    if face_indices.len() != 2 {
        return None;
    }
    let face_pair = [face_indices[0], face_indices[1]];
    if !delone_face_contains_edge(faces[face_pair[0]], edge)
        || !delone_face_contains_edge(faces[face_pair[1]], edge)
    {
        return None;
    }
    if !region.get(face_indices[0]).copied().unwrap_or(false)
        || !region.get(face_indices[1]).copied().unwrap_or(false)
    {
        return None;
    }
    let first_opposite = opposite_vertex(faces[face_indices[0]], edge)?;
    let second_opposite = opposite_vertex(faces[face_indices[1]], edge)?;
    if first_opposite == second_opposite {
        return None;
    }
    if !delone_edge_matches_vert_region(faces, edge, face_pair, vert_region) {
        return None;
    }
    let new_edge = ordered_edge(first_opposite, second_opposite);
    if edge_set.contains(&new_edge) {
        return None;
    }
    Some((face_pair, new_edge))
}

fn delone_face_contains_edge(face: [usize; 3], edge: [usize; 2]) -> bool {
    face.contains(&edge[0]) && face.contains(&edge[1])
}

fn normalize_not_flippable_edges(
    edges: &[[usize; 2]],
    vertex_count: usize,
) -> Result<BTreeSet<[usize; 2]>, GeometryError> {
    let mut normalized = BTreeSet::new();
    for edge in edges {
        if edge[0] == edge[1] || edge[0] >= vertex_count || edge[1] >= vertex_count {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "not_flippable_edges",
                value: format!("{edge:?}"),
            });
        }
        normalized.insert(ordered_edge(edge[0], edge[1]));
    }
    Ok(normalized)
}

fn normalize_vert_region(
    vertices: Option<&[usize]>,
    vertex_count: usize,
) -> Result<Option<BTreeSet<usize>>, GeometryError> {
    let Some(vertices) = vertices else {
        return Ok(None);
    };
    let mut normalized = BTreeSet::new();
    for &vertex in vertices {
        if vertex >= vertex_count {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "vert_region",
                value: vertex.to_string(),
            });
        }
        normalized.insert(vertex);
    }
    Ok(Some(normalized))
}

fn delone_edge_matches_vert_region(
    faces: &[[usize; 3]],
    edge: [usize; 2],
    face_pair: [usize; 2],
    vert_region: Option<&BTreeSet<usize>>,
) -> bool {
    let Some(vert_region) = vert_region else {
        return true;
    };
    let Some(first_opposite) = opposite_vertex(faces[face_pair[0]], edge) else {
        return false;
    };
    let Some(second_opposite) = opposite_vertex(faces[face_pair[1]], edge) else {
        return false;
    };
    vert_region.contains(&edge[0])
        || vert_region.contains(&edge[1])
        || vert_region.contains(&first_opposite)
        || vert_region.contains(&second_opposite)
}

pub(super) fn quadrangle_satisfied(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> bool {
    quadrangle_satisfied_with_angle_limit(a, b, c, d, f64::INFINITY)
}

fn quadrangle_satisfied_with_angle_limit(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
    max_angle_change: f64,
) -> bool {
    let dir_abd = cross(subtract(b, a), subtract(d, a));
    let dir_dbc = cross(subtract(b, d), subtract(c, d));
    if dot(dir_abd, dir_dbc) < 0.0 {
        return true;
    }
    const NO_ANGLE_CHANGE_LIMIT: f64 = 2.0 * std::f64::consts::PI;
    if max_angle_change < NO_ANGLE_CHANGE_LIMIT {
        let old_angle = dihedral_angle(dir_abd, dir_dbc, subtract(d, b));
        let dir_abc = cross(subtract(b, a), subtract(c, a));
        let dir_acd = cross(subtract(c, a), subtract(d, a));
        let new_angle = dihedral_angle(dir_abc, dir_acd, subtract(a, c));
        if (old_angle - new_angle).abs() > max_angle_change {
            return true;
        }
    }
    if !is_unfold_quadrangle_convex(a, b, c, d) {
        return true;
    }

    let metric_ac = circumcircle_diameter_sq(a, c, d).max(circumcircle_diameter_sq(c, a, b));
    let metric_bd = circumcircle_diameter_sq(b, d, a).max(circumcircle_diameter_sq(d, b, c));
    const EPS: f64 = 1e-7;
    if !metric_ac.is_finite() {
        return metric_ac <= metric_bd;
    }
    metric_ac <= metric_bd + EPS * (metric_ac + metric_bd)
}

fn delone_edge_satisfied_with_angle_limit(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    edge: [usize; 2],
    face_pair: [usize; 2],
    max_angle_change: f64,
) -> bool {
    let Some((a_index, c_index, d_index)) =
        oriented_edge_with_opposite_for_delone(faces[face_pair[0]], edge)
    else {
        return true;
    };
    let Some(b_index) = opposite_vertex(faces[face_pair[1]], edge) else {
        return true;
    };
    quadrangle_satisfied_with_angle_limit(
        vertices[a_index],
        vertices[b_index],
        vertices[c_index],
        vertices[d_index],
        max_angle_change,
    )
}

fn oriented_edge_with_opposite_for_delone(
    face: [usize; 3],
    edge: [usize; 2],
) -> Option<(usize, usize, usize)> {
    for offset in 0..3 {
        let current = face[offset];
        let next = face[(offset + 1) % 3];
        if (current == edge[0] && next == edge[1]) || (current == edge[1] && next == edge[0]) {
            return Some((current, next, face[(offset + 2) % 3]));
        }
    }
    None
}

fn is_unfold_quadrangle_convex(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> bool {
    let x = shortest_path_in_quadrangle(a, b, c, d);
    x > 0.0 && x < 1.0
}

fn shortest_path_in_quadrangle(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let vec_b = subtract(b, a);
    let vec_c = subtract(c, a);
    let vec_d = subtract(d, a);
    let unfold_b = [length(vec_b), 0.0];
    let unfold_c = unfold_on_plane(vec_b, vec_c, unfold_b, true);
    let unfold_d = unfold_on_plane(vec_c, vec_d, unfold_c, true);
    line_intersection(unfold_c, unfold_b, unfold_d).clamp(0.0, 1.0)
}

fn unfold_on_plane(b: [f64; 3], c: [f64; 3], d: [f64; 2], to_left_from_origin: bool) -> [f64; 2] {
    let dot_bc = dot(b, c);
    let cross_bc = length(cross(b, c));
    let dd = dot2(d, d);
    if dd <= 0.0 {
        return [0.0, 0.0];
    }
    let orthogonal = if to_left_from_origin {
        [-d[1], d[0]]
    } else {
        [d[1], -d[0]]
    };
    [
        (dot_bc * d[0] + cross_bc * orthogonal[0]) / dd,
        (dot_bc * d[1] + cross_bc * orthogonal[1]) / dd,
    ]
}

fn line_intersection(b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> f64 {
    let c1 = cross2(d, c);
    let c2 = cross2(subtract2(c, b), subtract2(d, b));
    let denominator = c1 + c2;
    if denominator == 0.0 {
        return 0.0;
    }
    c1 / denominator
}

fn circumcircle_diameter_sq(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = distance_sq(b, a);
    let ca = distance_sq(a, c);
    let bc = distance_sq(c, b);
    if ab <= 0.0 {
        return ca;
    }
    if ca <= 0.0 {
        return bc;
    }
    if bc <= 0.0 {
        return ab;
    }
    let area_sq = length_sq(cross(subtract(b, a), subtract(c, a)));
    if area_sq <= 0.0 {
        return f64::INFINITY;
    }
    ab * ca * bc / area_sq
}

fn triangle_aspect_ratio(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let bc = distance(b, c);
    let ca = distance(c, a);
    let ab = distance(a, b);
    let half_perimeter = (bc + ca + ab) / 2.0;
    let denominator = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if denominator <= 0.0 {
        return f64::MAX;
    }
    bc * ca * ab / denominator
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    distance_sq(a, b).sqrt()
}

fn distance_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn segment_segment_distance_sq(p1: [f64; 3], q1: [f64; 3], p2: [f64; 3], q2: [f64; 3]) -> f64 {
    let d1 = subtract(q1, p1);
    let d2 = subtract(q2, p2);
    let r = subtract(p1, p2);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let f = dot(d2, r);
    const EPS: f64 = 1e-12;

    let (s, t) = if a <= EPS && e <= EPS {
        (0.0, 0.0)
    } else if a <= EPS {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= EPS {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denominator = a * e - b * b;
            let s = if denominator.abs() > EPS {
                ((b * f - c * e) / denominator).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t_nominal = (b * s + f) / e;
            if t_nominal < 0.0 {
                ((-c / a).clamp(0.0, 1.0), 0.0)
            } else if t_nominal > 1.0 {
                (((b - c) / a).clamp(0.0, 1.0), 1.0)
            } else {
                (s, t_nominal)
            }
        }
    };

    let c1 = add(p1, scale(d1, s));
    let c2 = add(p2, scale(d2, t));
    distance_sq(c1, c2)
}

fn dihedral_angle(left_norm: [f64; 3], right_norm: [f64; 3], edge_vector: [f64; 3]) -> f64 {
    let edge_len = length(edge_vector);
    if edge_len <= 0.0 {
        return 0.0;
    }
    let edge_dir = scale(edge_vector, 1.0 / edge_len);
    let sin = dot(edge_dir, cross(left_norm, right_norm));
    let cos = dot(left_norm, right_norm);
    sin.atan2(cos)
}

fn length(vector: [f64; 3]) -> f64 {
    length_sq(vector).sqrt()
}

fn length_sq(vector: [f64; 3]) -> f64 {
    dot(vector, vector)
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(vector: [f64; 3], amount: f64) -> [f64; 3] {
    [vector[0] * amount, vector[1] * amount, vector[2] * amount]
}

fn subtract2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn dot2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
