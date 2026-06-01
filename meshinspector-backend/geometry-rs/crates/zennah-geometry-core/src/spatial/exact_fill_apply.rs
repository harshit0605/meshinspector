use super::exact_cut_apply::ExactCutMeshResult;
use super::exact_fill_plan::{
    exact_planar_hole_fill_plan, execute_exact_planar_hole_fill_plan, ExactPlanarHoleFillPlan,
};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ExactCutHoleFillPlan {
    pub representative_edge: [usize; 2],
    pub boundary_loop: Vec<usize>,
    pub boundary_edges: Vec<[usize; 2]>,
    pub source_face: usize,
    pub fill_plan: ExactPlanarHoleFillPlan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactCutHoleFillResult {
    pub mesh: ExactCutMeshResult,
    pub fill_plans: Vec<ExactCutHoleFillPlan>,
    pub added_face_ranges: Vec<[usize; 2]>,
}

pub fn exact_cut_hole_fill_plans(
    cut_mesh: &ExactCutMeshResult,
    epsilon: f64,
) -> Result<Vec<ExactCutHoleFillPlan>, GeometryError> {
    let faces = validate_faces(&cut_mesh.faces, cut_mesh.vertices.len())?;
    let cut_edges = cut_mesh
        .cut_edges
        .iter()
        .map(|edge| ordered_edge(*edge))
        .collect::<BTreeSet<_>>();
    if cut_edges.is_empty() {
        return Ok(Vec::new());
    }

    let boundary_edges = cut_boundary_edges(&faces, &cut_edges);
    let loops = directed_boundary_loops(&boundary_edges);
    Ok(loops
        .into_iter()
        .filter_map(|boundary| {
            let fill_plan =
                exact_planar_hole_fill_plan(&cut_mesh.vertices, &boundary.vertices, epsilon)?;
            let source_face = source_face_for_boundary(cut_mesh, &boundary);
            let representative_edge = boundary.boundary_edges[0];
            Some(ExactCutHoleFillPlan {
                representative_edge,
                boundary_loop: boundary.vertices,
                boundary_edges: boundary.boundary_edges,
                source_face,
                fill_plan,
            })
        })
        .collect())
}

pub fn exact_fill_cut_holes(
    cut_mesh: &ExactCutMeshResult,
    epsilon: f64,
) -> Result<ExactCutHoleFillResult, GeometryError> {
    let fill_plans = exact_cut_hole_fill_plans(cut_mesh, epsilon)?;
    let mut mesh = cut_mesh.clone();
    let mut added_face_ranges = Vec::with_capacity(fill_plans.len());
    for fill_plan in &fill_plans {
        let execution =
            execute_exact_planar_hole_fill_plan(&fill_plan.fill_plan, fill_plan.source_face);
        let start = mesh.faces.len();
        mesh.faces.extend(execution.faces);
        mesh.source_face_for_faces
            .extend(execution.source_face_for_faces);
        added_face_ranges.push([start, mesh.faces.len()]);
    }
    Ok(ExactCutHoleFillResult {
        mesh,
        fill_plans,
        added_face_ranges,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BoundaryEdge {
    from: usize,
    to: usize,
    face_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundaryLoop {
    vertices: Vec<usize>,
    boundary_edges: Vec<[usize; 2]>,
    adjacent_faces: Vec<usize>,
}

fn cut_boundary_edges(faces: &[[usize; 3]], cut_edges: &BTreeSet<[usize; 2]>) -> Vec<BoundaryEdge> {
    let mut edge_occurrences = BTreeMap::<[usize; 2], Vec<BoundaryEdge>>::new();
    for (face_index, face) in faces.iter().enumerate() {
        for edge_index in 0..3 {
            let from = face[edge_index];
            let to = face[(edge_index + 1) % 3];
            let key = ordered_edge([from, to]);
            if cut_edges.contains(&key) {
                edge_occurrences.entry(key).or_default().push(BoundaryEdge {
                    from: to,
                    to: from,
                    face_index,
                });
            }
        }
    }

    edge_occurrences
        .into_values()
        .filter_map(|edges| (edges.len() == 1).then_some(edges[0]))
        .collect()
}

fn directed_boundary_loops(edges: &[BoundaryEdge]) -> Vec<BoundaryLoop> {
    let mut outgoing = BTreeMap::<usize, Vec<BoundaryEdge>>::new();
    for edge in edges {
        outgoing.entry(edge.from).or_default().push(*edge);
    }
    for candidates in outgoing.values_mut() {
        candidates.sort();
    }

    let edge_set = edges
        .iter()
        .map(|edge| (edge.from, edge.to))
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::<(usize, usize)>::new();
    let mut loops = Vec::new();
    for edge in edges.iter().copied() {
        if seen.contains(&(edge.from, edge.to)) {
            continue;
        }
        if let Some(boundary_loop) = trace_boundary_loop(edge, &outgoing, &edge_set, &mut seen) {
            loops.push(boundary_loop);
        }
    }
    loops
}

fn trace_boundary_loop(
    start: BoundaryEdge,
    outgoing: &BTreeMap<usize, Vec<BoundaryEdge>>,
    edge_set: &BTreeSet<(usize, usize)>,
    seen: &mut BTreeSet<(usize, usize)>,
) -> Option<BoundaryLoop> {
    let mut vertices = vec![start.from];
    let mut boundary_edges = Vec::new();
    let mut adjacent_faces = Vec::new();
    let mut current = start;

    loop {
        if !edge_set.contains(&(current.from, current.to)) {
            return None;
        }
        if !seen.insert((current.from, current.to)) {
            return None;
        }
        boundary_edges.push([current.from, current.to]);
        vertices.push(current.to);
        adjacent_faces.push(current.face_index);
        if current.to == start.from {
            vertices.pop();
            return (vertices.len() >= 3).then_some(BoundaryLoop {
                vertices,
                boundary_edges,
                adjacent_faces,
            });
        }

        current = next_boundary_edge(current.to, outgoing, seen)?;
    }
}

fn next_boundary_edge(
    vertex: usize,
    outgoing: &BTreeMap<usize, Vec<BoundaryEdge>>,
    seen: &BTreeSet<(usize, usize)>,
) -> Option<BoundaryEdge> {
    let next_edges = outgoing.get(&vertex)?;
    let mut candidates = next_edges
        .iter()
        .copied()
        .filter(|edge| !seen.contains(&(edge.from, edge.to)));
    let next = candidates.next()?;
    candidates.next().is_none().then_some(next)
}

fn source_face_for_boundary(cut_mesh: &ExactCutMeshResult, boundary: &BoundaryLoop) -> usize {
    boundary
        .adjacent_faces
        .iter()
        .filter_map(|face| cut_mesh.source_face_for_faces.get(*face))
        .copied()
        .next()
        .unwrap_or_else(|| boundary.adjacent_faces.first().copied().unwrap_or(0))
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}
