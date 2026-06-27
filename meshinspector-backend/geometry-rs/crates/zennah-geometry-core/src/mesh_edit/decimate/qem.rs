use crate::mesh_edit::topology;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::helpers::{add, distance_sq, face_normal, invert_3x3, mat_vec, midpoint, sub};
use super::types::{DecimateMeshOptions, EdgeCandidate, QemCandidate, QuadraticForm};

pub(super) fn boundary_policy_rejects_edge(
    edge: [usize; 2],
    boundary_vertices: &BTreeSet<usize>,
    options: &DecimateMeshOptions,
) -> bool {
    let first_boundary = boundary_vertices.contains(&edge[0]);
    let second_boundary = boundary_vertices.contains(&edge[1]);
    if !options.touch_near_bd_edges {
        return first_boundary || second_boundary;
    }
    !options.touch_bd_verts && first_boundary && second_boundary
}

pub(super) fn shortest_collapse_pos(
    vertices: &[[f64; 3]],
    edge: [usize; 2],
    boundary_vertices: Option<&BTreeSet<usize>>,
    options: &DecimateMeshOptions,
) -> [f64; 3] {
    if options.touch_near_bd_edges && !options.touch_bd_verts {
        if let Some(boundary_vertices) = boundary_vertices {
            let first_boundary = boundary_vertices.contains(&edge[0]);
            let second_boundary = boundary_vertices.contains(&edge[1]);
            if first_boundary && !second_boundary {
                return vertices[edge[0]];
            }
            if second_boundary && !first_boundary {
                return vertices[edge[1]];
            }
        }
    }
    if options.optimize_vertex_pos {
        midpoint(vertices[edge[0]], vertices[edge[1]])
    } else {
        vertices[edge[0]]
    }
}

pub(super) fn sum_forms_for_boundary_policy(
    left_form: QuadraticForm,
    left_pos: [f64; 3],
    right_form: QuadraticForm,
    right_pos: [f64; 3],
    edge: [usize; 2],
    boundary_vertices: Option<&BTreeSet<usize>>,
    options: &DecimateMeshOptions,
) -> (QuadraticForm, [f64; 3]) {
    if options.touch_near_bd_edges && !options.touch_bd_verts {
        if let Some(boundary_vertices) = boundary_vertices {
            let first_boundary = boundary_vertices.contains(&edge[0]);
            let second_boundary = boundary_vertices.contains(&edge[1]);
            if first_boundary && !second_boundary {
                return sum_forms_at(left_form, left_pos, right_form, right_pos, left_pos);
            }
            if second_boundary && !first_boundary {
                return sum_forms_at(left_form, left_pos, right_form, right_pos, right_pos);
            }
        }
    }
    sum_forms(
        left_form,
        left_pos,
        right_form,
        right_pos,
        !options.optimize_vertex_pos,
    )
}

pub(super) fn compare_qem_candidates(left: QemCandidate, right: QemCandidate) -> Ordering {
    left.error_sq
        .partial_cmp(&right.error_sq)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.edge.cmp(&right.edge))
}

pub(super) fn compare_candidates(left: EdgeCandidate, right: EdgeCandidate) -> Ordering {
    left.length_sq
        .partial_cmp(&right.length_sq)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.edge.cmp(&right.edge))
}

pub(super) fn compute_vertex_forms(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    angle_weighted_dist_to_plane: bool,
    stabilizer: f64,
) -> Vec<QuadraticForm> {
    let mut forms = vec![QuadraticForm::zero(); vertices.len()];
    let region_vertices: BTreeSet<usize> = faces
        .iter()
        .copied()
        .enumerate()
        .filter(|(face_index, _)| region[*face_index])
        .flat_map(|(_, face)| face)
        .collect();

    for face in faces.iter().copied() {
        let normal = face_normal(vertices, face);
        if distance_sq(normal, [0.0, 0.0, 0.0]) <= 1e-24 {
            continue;
        }
        for vertex in face {
            if region_vertices.contains(&vertex) {
                let weight = if angle_weighted_dist_to_plane {
                    corner_angle(vertices, face, vertex) / std::f64::consts::PI
                } else {
                    1.0
                };
                forms[vertex].add_dist_to_plane(normal, weight);
            }
        }
    }

    for edge in boundary_edges_in_region(faces, region) {
        let direction = sub(vertices[edge[1]], vertices[edge[0]]);
        forms[edge[0]].add_dist_to_line(direction);
        forms[edge[1]].add_dist_to_line(direction);
    }

    for edge in not_flippable_edges.iter().copied() {
        let direction = sub(vertices[edge[1]], vertices[edge[0]]);
        forms[edge[0]].add_dist_to_line(direction);
        forms[edge[1]].add_dist_to_line(direction);
    }

    for vertex in region_vertices {
        forms[vertex].add_dist_to_origin(stabilizer);
    }
    forms
}

fn corner_angle(vertices: &[[f64; 3]], face: [usize; 3], vertex: usize) -> f64 {
    let Some(corner) = face.iter().position(|candidate| *candidate == vertex) else {
        return 0.0;
    };
    let origin = vertices[vertex];
    let previous = vertices[face[(corner + 2) % 3]];
    let next = vertices[face[(corner + 1) % 3]];
    let first = sub(previous, origin);
    let second = sub(next, origin);
    let denominator =
        distance_sq(first, [0.0, 0.0, 0.0]).sqrt() * distance_sq(second, [0.0, 0.0, 0.0]).sqrt();
    if denominator <= 1e-24 {
        return 0.0;
    }
    (dot(first, second) / denominator).clamp(-1.0, 1.0).acos()
}

pub(super) fn boundary_edges_in_region(faces: &[[usize; 3]], region: &[bool]) -> Vec<[usize; 2]> {
    let mut counts = BTreeMap::<[usize; 2], usize>::new();
    for (face_index, face) in faces.iter().copied().enumerate() {
        if !region[face_index] {
            continue;
        }
        for edge in topology::oriented_face_edges(face) {
            *counts
                .entry(topology::ordered_edge(edge[0], edge[1]))
                .or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(edge, count)| (count == 1).then_some(edge))
        .collect()
}

fn sum_forms(
    left_form: QuadraticForm,
    left_pos: [f64; 3],
    right_form: QuadraticForm,
    right_pos: [f64; 3],
    min_among_endpoints: bool,
) -> (QuadraticForm, [f64; 3]) {
    let mut sum = left_form.add(right_form);
    let collapse_pos = if min_among_endpoints {
        let left_error = left_form.c + right_form.eval(sub(left_pos, right_pos));
        let right_error = left_form.eval(sub(left_pos, right_pos)) + right_form.c;
        if left_error <= right_error {
            sum.c = left_error;
            left_pos
        } else {
            sum.c = right_error;
            right_pos
        }
    } else {
        let center = midpoint(left_pos, right_pos);
        let rhs = add(
            mat_vec(left_form.a, sub(left_pos, center)),
            mat_vec(right_form.a, sub(right_pos, center)),
        );
        let collapse_pos = invert_3x3(sum.a)
            .map(|inverse| add(mat_vec(inverse, rhs), center))
            .unwrap_or_else(|| {
                let left_error = left_form.c + right_form.eval(sub(left_pos, right_pos));
                let right_error = left_form.eval(sub(left_pos, right_pos)) + right_form.c;
                if left_error <= right_error {
                    left_pos
                } else {
                    right_pos
                }
            });
        sum.c = left_form.eval(sub(left_pos, collapse_pos))
            + right_form.eval(sub(right_pos, collapse_pos));
        collapse_pos
    };
    (sum, collapse_pos)
}

fn sum_forms_at(
    left_form: QuadraticForm,
    left_pos: [f64; 3],
    right_form: QuadraticForm,
    right_pos: [f64; 3],
    collapse_pos: [f64; 3],
) -> (QuadraticForm, [f64; 3]) {
    let mut sum = left_form.add(right_form);
    sum.c =
        left_form.eval(sub(left_pos, collapse_pos)) + right_form.eval(sub(right_pos, collapse_pos));
    (sum, collapse_pos)
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}
