use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::helpers::{
    boundary_vertices_in_region, collapse_plan, distance_sq, edge_allowed_by_edges_to_collapse,
    edge_blocked_by_not_flippable,
};
use super::qem::{
    boundary_policy_rejects_edge, compare_candidates, compare_qem_candidates,
    shortest_collapse_pos, sum_forms_for_boundary_policy,
};
use super::types::{
    CollapsePlan, DecimateMeshOptions, DecimateMeshStrategy, EdgeCandidate, QemCandidate,
    QuadraticForm,
};
use crate::mesh_edit::{delone, edge_incident_faces, opposite_vertex, topology};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FlipCandidate {
    pub(super) edge: [usize; 2],
    pub(super) face_pair: [usize; 2],
    pub(super) new_edge: [usize; 2],
    pub(super) twin: Option<FlipTwin>,
    pub(super) deviation_sq: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FlipTwin {
    pub(super) edge: [usize; 2],
    pub(super) face_pair: [usize; 2],
    pub(super) new_edge: [usize; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum CandidatePlan {
    Shortest(EdgeCandidate),
    Qem(QemCandidate),
}

impl CandidatePlan {
    pub(super) fn edge(self) -> [usize; 2] {
        match self {
            CandidatePlan::Shortest(candidate) => candidate.edge,
            CandidatePlan::Qem(candidate) => candidate.edge,
        }
    }

    fn collapse_pos(self) -> [f64; 3] {
        match self {
            CandidatePlan::Shortest(_) => {
                unreachable!("shortest candidate uses option-based collapse position")
            }
            CandidatePlan::Qem(candidate) => candidate.collapse_pos,
        }
    }

    pub(super) fn collapse_form(self) -> Option<QuadraticForm> {
        match self {
            CandidatePlan::Shortest(_) => None,
            CandidatePlan::Qem(candidate) => Some(candidate.collapse_form),
        }
    }

    pub(super) fn error_introduced(self) -> f64 {
        match self {
            CandidatePlan::Shortest(candidate) => candidate.length_sq.sqrt(),
            CandidatePlan::Qem(candidate) => candidate.error_sq.sqrt(),
        }
    }

    pub(super) fn error_sq(self) -> f64 {
        match self {
            CandidatePlan::Shortest(candidate) => candidate.length_sq,
            CandidatePlan::Qem(candidate) => candidate.error_sq,
        }
    }
}

pub(super) fn next_delone_flip_candidate(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    options: &DecimateMeshOptions,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
    twin_map: &BTreeMap<[usize; 2], [usize; 2]>,
    flipped_edges: &BTreeSet<[usize; 2]>,
    collapse_error_sq: Option<f64>,
) -> Option<FlipCandidate> {
    if options.max_angle_change < 0.0 {
        return None;
    }
    let max_allowed_error_sq =
        collapse_error_sq.unwrap_or_else(|| squared_limit(options.max_error));
    let incident = edge_incident_faces(faces);
    let edge_set = incident.keys().copied().collect::<BTreeSet<_>>();
    let boundary_vertices =
        (!options.touch_near_bd_edges).then(|| boundary_vertices_in_region(faces, region));
    let mut best: Option<FlipCandidate> = None;

    for (edge, face_indices) in &incident {
        let edge = *edge;
        if face_indices.len() != 2 {
            continue;
        }
        if !region.get(face_indices[0]).copied().unwrap_or(false)
            && !region.get(face_indices[1]).copied().unwrap_or(false)
        {
            continue;
        }
        if not_flippable_edges.contains(&edge) {
            continue;
        }
        if flipped_edges.contains(&edge) {
            continue;
        }
        if !edge_allowed_by_edges_to_collapse(edge, edges_to_collapse) {
            continue;
        }
        if boundary_vertices
            .as_ref()
            .map(|vertices| vertices.contains(&edge[0]) || vertices.contains(&edge[1]))
            .unwrap_or(false)
        {
            continue;
        }
        let Some((face_pair, new_edge)) = flip_face_pair_and_new_edge(
            faces,
            region,
            &edge_set,
            edge,
            [face_indices[0], face_indices[1]],
        ) else {
            continue;
        };
        if delone::delone_edge_satisfied_with_flip_limits(
            vertices,
            faces,
            edge,
            face_pair,
            None,
            Some(options.max_angle_change),
            Some(options.critical_tri_aspect_ratio),
        ) {
            continue;
        }
        let Some(deviation_sq) = delone::deviation_sq_after_flip(vertices, faces, edge, face_pair)
        else {
            continue;
        };
        if !deviation_sq.is_finite() || deviation_sq > max_allowed_error_sq {
            continue;
        }
        if let Some(collapse_error_sq) = collapse_error_sq {
            if deviation_sq >= collapse_error_sq {
                continue;
            }
        }
        let twin = if let Some(twin_edge) = twin_map.get(&edge).copied() {
            if twin_edge == edge {
                continue;
            }
            if flipped_edges.contains(&twin_edge) {
                continue;
            }
            let Some(twin_face_indices) = incident.get(&twin_edge) else {
                continue;
            };
            if twin_face_indices.len() != 2 {
                continue;
            }
            let Some((twin_face_pair, twin_new_edge)) = flip_face_pair_and_new_edge(
                faces,
                region,
                &edge_set,
                twin_edge,
                [twin_face_indices[0], twin_face_indices[1]],
            ) else {
                continue;
            };
            Some(FlipTwin {
                edge: twin_edge,
                face_pair: twin_face_pair,
                new_edge: twin_new_edge,
            })
        } else {
            None
        };
        let candidate = FlipCandidate {
            edge,
            face_pair,
            new_edge,
            twin,
            deviation_sq,
        };
        if best
            .map(|current| compare_flip_candidates(candidate, current) == Ordering::Less)
            .unwrap_or(true)
        {
            best = Some(candidate);
        }
    }

    best
}

fn flip_face_pair_and_new_edge(
    faces: &[[usize; 3]],
    region: &[bool],
    edge_set: &BTreeSet<[usize; 2]>,
    edge: [usize; 2],
    face_pair: [usize; 2],
) -> Option<([usize; 2], [usize; 2])> {
    if !region.get(face_pair[0]).copied().unwrap_or(false)
        && !region.get(face_pair[1]).copied().unwrap_or(false)
    {
        return None;
    }
    if !faces[face_pair[0]].contains(&edge[0])
        || !faces[face_pair[0]].contains(&edge[1])
        || !faces[face_pair[1]].contains(&edge[0])
        || !faces[face_pair[1]].contains(&edge[1])
    {
        return None;
    }
    let first_opposite = opposite_vertex(faces[face_pair[0]], edge)?;
    let second_opposite = opposite_vertex(faces[face_pair[1]], edge)?;
    if first_opposite == second_opposite {
        return None;
    }
    let new_edge = topology::ordered_edge(first_opposite, second_opposite);
    if edge_set.contains(&new_edge) {
        return None;
    }
    Some((face_pair, new_edge))
}

pub(super) fn remap_twin_map_after_flip(
    twin_map: &mut BTreeMap<[usize; 2], [usize; 2]>,
    edge: [usize; 2],
    new_edge: [usize; 2],
    twin_edge: [usize; 2],
    twin_new_edge: [usize; 2],
) {
    let primary_value = twin_map.remove(&edge);
    let twin_value = twin_map.remove(&twin_edge);
    if primary_value == Some(twin_edge) && twin_value == Some(edge) && new_edge != twin_new_edge {
        twin_map.insert(new_edge, twin_new_edge);
        twin_map.insert(twin_new_edge, new_edge);
    }
}

fn compare_flip_candidates(left: FlipCandidate, right: FlipCandidate) -> Ordering {
    left.deviation_sq
        .total_cmp(&right.deviation_sq)
        .then_with(|| left.edge.cmp(&right.edge))
}

pub(super) fn next_collapse_plan(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    tracked_region: &[bool],
    vertex_forms: Option<&[QuadraticForm]>,
    options: &DecimateMeshOptions,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
    skipped_edges: &mut BTreeSet<[usize; 2]>,
) -> Option<(CandidatePlan, CollapsePlan)> {
    loop {
        let boundary_vertices = (!options.touch_near_bd_edges || !options.touch_bd_verts)
            .then(|| boundary_vertices_in_region(faces, region));
        let candidate = match options.strategy {
            DecimateMeshStrategy::ShortestEdgeFirst => {
                CandidatePlan::Shortest(shortest_candidate_edge(
                    vertices,
                    faces,
                    region,
                    skipped_edges,
                    boundary_vertices.as_ref(),
                    options,
                    not_flippable_edges,
                    edges_to_collapse,
                )?)
            }
            DecimateMeshStrategy::MinimizeError => CandidatePlan::Qem(qem_candidate_edge(
                vertices,
                faces,
                region,
                vertex_forms?,
                skipped_edges,
                boundary_vertices.as_ref(),
                options,
                not_flippable_edges,
                edges_to_collapse,
            )?),
        };
        let collapse_pos = match candidate {
            CandidatePlan::Shortest(shortest) => {
                shortest_collapse_pos(vertices, shortest.edge, boundary_vertices.as_ref(), options)
            }
            CandidatePlan::Qem(_) => candidate.collapse_pos(),
        };
        let Some(plan) = collapse_plan(
            vertices,
            faces,
            region,
            tracked_region,
            candidate.edge(),
            collapse_pos,
            options,
        ) else {
            skipped_edges.insert(candidate.edge());
            continue;
        };
        break Some((candidate, plan));
    }
}

fn shortest_candidate_edge(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    skipped_edges: &BTreeSet<[usize; 2]>,
    boundary_vertices: Option<&BTreeSet<usize>>,
    options: &DecimateMeshOptions,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
) -> Option<EdgeCandidate> {
    let max_error_sq = squared_limit(options.max_error);
    let mut seen = BTreeSet::<[usize; 2]>::new();
    let mut best: Option<EdgeCandidate> = None;
    for (face_index, face) in faces.iter().copied().enumerate() {
        if !region[face_index] {
            continue;
        }
        for edge in topology::oriented_face_edges(face) {
            let edge = topology::ordered_edge(edge[0], edge[1]);
            if skipped_edges.contains(&edge) || !seen.insert(edge) {
                continue;
            }
            if !edge_allowed_by_edges_to_collapse(edge, edges_to_collapse) {
                continue;
            }
            if !options.collapse_near_not_flippable
                && edge_blocked_by_not_flippable(edge, not_flippable_edges)
            {
                continue;
            }
            if boundary_vertices
                .map(|vertices| boundary_policy_rejects_edge(edge, vertices, options))
                .unwrap_or(false)
            {
                continue;
            }
            let length_sq = distance_sq(vertices[edge[0]], vertices[edge[1]]);
            if !length_sq.is_finite() || length_sq > max_error_sq {
                continue;
            }
            let candidate = EdgeCandidate { edge, length_sq };
            if best
                .map(|current| compare_candidates(candidate, current) == Ordering::Less)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
    }
    best
}

fn squared_limit(value: f64) -> f64 {
    if value >= f64::MAX.sqrt() {
        f64::INFINITY
    } else {
        value * value
    }
}

fn qem_candidate_edge(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    region: &[bool],
    vertex_forms: &[QuadraticForm],
    skipped_edges: &BTreeSet<[usize; 2]>,
    boundary_vertices: Option<&BTreeSet<usize>>,
    options: &DecimateMeshOptions,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
) -> Option<QemCandidate> {
    let max_error_sq = squared_limit(options.max_error);
    let mut seen = BTreeSet::<[usize; 2]>::new();
    let mut best: Option<QemCandidate> = None;
    for (face_index, face) in faces.iter().copied().enumerate() {
        if !region[face_index] {
            continue;
        }
        for edge in topology::oriented_face_edges(face) {
            let edge = topology::ordered_edge(edge[0], edge[1]);
            if skipped_edges.contains(&edge) || !seen.insert(edge) {
                continue;
            }
            if !edge_allowed_by_edges_to_collapse(edge, edges_to_collapse) {
                continue;
            }
            if !options.collapse_near_not_flippable
                && edge_blocked_by_not_flippable(edge, not_flippable_edges)
            {
                continue;
            }
            if boundary_vertices
                .map(|vertices| boundary_policy_rejects_edge(edge, vertices, options))
                .unwrap_or(false)
            {
                continue;
            }
            let (collapse_form, collapse_pos) = sum_forms_for_boundary_policy(
                vertex_forms[edge[0]],
                vertices[edge[0]],
                vertex_forms[edge[1]],
                vertices[edge[1]],
                edge,
                boundary_vertices,
                options,
            );
            if !collapse_form.c.is_finite() || collapse_form.c > max_error_sq {
                continue;
            }
            let candidate = QemCandidate {
                edge,
                error_sq: collapse_form.c,
                collapse_pos,
                collapse_form,
            };
            if best
                .map(|current| compare_qem_candidates(candidate, current) == Ordering::Less)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
    }
    best
}
