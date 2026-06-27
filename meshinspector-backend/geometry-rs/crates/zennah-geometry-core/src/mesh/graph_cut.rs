use super::base::{edge_face_map, safe_normalize_vector, validate_faces};
use super::overhang::overhang_face_normal;
use crate::math::{cross, dot, norm, sub};
use crate::GeometryError;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

enum GraphCutCurvaturePreference {
    Geodesic,
    Convex,
    Concave,
}

impl GraphCutCurvaturePreference {
    fn angle_sin_factor(self) -> f64 {
        match self {
            GraphCutCurvaturePreference::Geodesic => 0.0,
            GraphCutCurvaturePreference::Convex => -2.0,
            GraphCutCurvaturePreference::Concave => 2.0,
        }
    }
}

fn parse_graph_cut_curvature_preference(
    value: &str,
) -> Result<GraphCutCurvaturePreference, GeometryError> {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized.as_str() {
        "" | "none" | "geodesic" | "shortest" | "shortest_boundary" => {
            Ok(GraphCutCurvaturePreference::Geodesic)
        }
        "convex" => Ok(GraphCutCurvaturePreference::Convex),
        "concave" => Ok(GraphCutCurvaturePreference::Concave),
        _ => Err(GeometryError::InvalidSelectionParameter {
            field: "curvature_preference",
            value: normalized,
        }),
    }
}

pub fn graph_cut_select_region(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    source_face_ids: &[usize],
    sink_face_ids: &[usize],
    boundary_weight: f64,
) -> Result<Vec<i64>, GeometryError> {
    graph_cut_select_region_with_curvature_preference(
        vertices,
        faces_i64,
        source_face_ids,
        sink_face_ids,
        boundary_weight,
        "geodesic",
    )
}

pub fn graph_cut_select_region_with_curvature_preference(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    source_face_ids: &[usize],
    sink_face_ids: &[usize],
    boundary_weight: f64,
    curvature_preference: &str,
) -> Result<Vec<i64>, GeometryError> {
    if !boundary_weight.is_finite() || boundary_weight <= 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "boundary_weight",
            value: boundary_weight.to_string(),
        });
    }
    let curvature_preference = parse_graph_cut_curvature_preference(curvature_preference)?;

    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut source_faces = vec![false; faces.len()];
    let mut sink_faces = vec![false; faces.len()];
    mark_graph_cut_seed_faces(
        source_face_ids,
        faces.len(),
        "source_face_ids",
        &mut source_faces,
    )?;
    mark_graph_cut_seed_faces(sink_face_ids, faces.len(), "sink_face_ids", &mut sink_faces)?;

    let selected = segment_faces_by_metric_cut(
        vertices,
        &faces,
        &source_faces,
        &sink_faces,
        boundary_weight,
        curvature_preference.angle_sin_factor(),
    );
    Ok(selected
        .into_iter()
        .enumerate()
        .filter_map(|(face_index, is_selected)| is_selected.then_some(face_index as i64))
        .collect())
}

pub fn graph_cut_select_region_auto_not_region(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    source_face_ids: &[usize],
    uncertainty_distance_mm: f64,
    boundary_weight: f64,
) -> Result<Vec<i64>, GeometryError> {
    graph_cut_select_region_auto_not_region_with_curvature_preference(
        vertices,
        faces_i64,
        source_face_ids,
        uncertainty_distance_mm,
        boundary_weight,
        "geodesic",
    )
}

pub fn graph_cut_select_region_auto_not_region_with_curvature_preference(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    source_face_ids: &[usize],
    uncertainty_distance_mm: f64,
    boundary_weight: f64,
    curvature_preference: &str,
) -> Result<Vec<i64>, GeometryError> {
    if !uncertainty_distance_mm.is_finite() || uncertainty_distance_mm < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "uncertainty_distance_mm",
            value: uncertainty_distance_mm.to_string(),
        });
    }
    if !boundary_weight.is_finite() || boundary_weight <= 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "boundary_weight",
            value: boundary_weight.to_string(),
        });
    }
    let curvature_preference = parse_graph_cut_curvature_preference(curvature_preference)?;

    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut source_faces = vec![false; faces.len()];
    mark_graph_cut_seed_faces(
        source_face_ids,
        faces.len(),
        "source_face_ids",
        &mut source_faces,
    )?;
    if !source_faces.iter().any(|is_source| *is_source) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "source_face_ids",
            value: "empty".to_string(),
        });
    }

    let sink_faces =
        auto_not_region_seed_faces(vertices, &faces, &source_faces, uncertainty_distance_mm);
    let selected = segment_faces_by_metric_cut(
        vertices,
        &faces,
        &source_faces,
        &sink_faces,
        boundary_weight,
        curvature_preference.angle_sin_factor(),
    );
    Ok(selected
        .into_iter()
        .enumerate()
        .filter_map(|(face_index, is_selected)| is_selected.then_some(face_index as i64))
        .collect())
}

fn mark_graph_cut_seed_faces(
    face_ids: &[usize],
    face_count: usize,
    field: &'static str,
    output: &mut [bool],
) -> Result<(), GeometryError> {
    for face_id in face_ids {
        if *face_id >= face_count {
            return Err(GeometryError::InvalidSelectionParameter {
                field,
                value: face_id.to_string(),
            });
        }
        output[*face_id] = true;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FaceDistanceQueueEntry {
    cost: f64,
    face_id: usize,
}

impl Eq for FaceDistanceQueueEntry {}

impl Ord for FaceDistanceQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| self.face_id.cmp(&other.face_id))
    }
}

impl PartialOrd for FaceDistanceQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn auto_not_region_seed_faces(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    source_faces: &[bool],
    uncertainty_distance_mm: f64,
) -> Vec<bool> {
    let face_count = faces.len();
    let mut graph = vec![Vec::<(usize, f64)>::new(); face_count];
    for ((a, b), face_ids) in edge_face_map(faces) {
        if face_ids.len() < 2 {
            continue;
        }
        let edge_length = norm(sub(vertices[a], vertices[b]));
        for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                graph[face_ids[i]].push((face_ids[j], edge_length));
                graph[face_ids[j]].push((face_ids[i], edge_length));
            }
        }
    }

    let mut distances = vec![f64::INFINITY; face_count];
    let mut queue = BinaryHeap::<FaceDistanceQueueEntry>::new();
    for (face_id, is_source) in source_faces.iter().enumerate() {
        if *is_source {
            distances[face_id] = 0.0;
            queue.push(FaceDistanceQueueEntry { cost: 0.0, face_id });
        }
    }
    while let Some(entry) = queue.pop() {
        if entry.cost > distances[entry.face_id] {
            continue;
        }
        for (neighbor, weight) in &graph[entry.face_id] {
            let next = entry.cost + *weight;
            if next < distances[*neighbor] {
                distances[*neighbor] = next;
                queue.push(FaceDistanceQueueEntry {
                    cost: next,
                    face_id: *neighbor,
                });
            }
        }
    }

    let mut sink_faces = vec![false; face_count];
    for (face_id, distance) in distances.iter().copied().enumerate() {
        if !source_faces[face_id] && distance >= uncertainty_distance_mm {
            sink_faces[face_id] = true;
        }
    }
    if sink_faces.iter().any(|is_sink| *is_sink) {
        return sink_faces;
    }

    let fallback_distance = distances
        .iter()
        .copied()
        .enumerate()
        .filter(|(face_id, _)| !source_faces[*face_id])
        .map(|(_, distance)| distance)
        .max_by(|a, b| a.total_cmp(b));
    if let Some(fallback_distance) = fallback_distance {
        for (face_id, distance) in distances.iter().copied().enumerate() {
            if !source_faces[face_id] && distance == fallback_distance {
                sink_faces[face_id] = true;
            }
        }
    }
    sink_faces
}

#[derive(Debug, Clone, Copy)]
struct FlowEdge {
    to: usize,
    rev: usize,
    capacity: f64,
}

pub(super) fn segment_faces_by_edge_length_cut(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    source_faces: &[bool],
    sink_faces: &[bool],
    boundary_weight: f64,
) -> Vec<bool> {
    segment_faces_by_metric_cut(
        vertices,
        faces,
        source_faces,
        sink_faces,
        boundary_weight,
        0.0,
    )
}

fn segment_faces_by_metric_cut(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    source_faces: &[bool],
    sink_faces: &[bool],
    boundary_weight: f64,
    curvature_factor: f64,
) -> Vec<bool> {
    const INF_CAPACITY: f64 = 1.0e30;
    const FLOW_EPSILON: f64 = 1.0e-12;

    let face_count = faces.len();
    let source = face_count;
    let sink = face_count + 1;
    let mut graph = vec![Vec::<FlowEdge>::new(); face_count + 2];
    let face_normals = if curvature_factor == 0.0 {
        Vec::new()
    } else {
        faces
            .iter()
            .map(|face| overhang_face_normal(vertices, face))
            .collect::<Vec<_>>()
    };

    for (face_index, is_source) in source_faces.iter().enumerate() {
        if *is_source && !sink_faces[face_index] {
            add_directed_flow_edge(&mut graph, source, face_index, INF_CAPACITY);
        }
    }
    for (face_index, is_sink) in sink_faces.iter().enumerate() {
        if *is_sink && !source_faces[face_index] {
            add_directed_flow_edge(&mut graph, face_index, sink, INF_CAPACITY);
        }
    }

    for ((a, b), face_ids) in edge_face_map(faces) {
        if face_ids.len() < 2 {
            continue;
        }
        for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                let capacity = graph_cut_edge_metric(
                    vertices,
                    faces,
                    &face_normals,
                    (a, b),
                    face_ids[i],
                    face_ids[j],
                    curvature_factor,
                ) * boundary_weight;
                if capacity <= FLOW_EPSILON {
                    continue;
                }
                add_undirected_flow_edge(&mut graph, face_ids[i], face_ids[j], capacity);
            }
        }
    }

    dinic_max_flow(&mut graph, source, sink);
    residual_reachable_faces(&graph, source, face_count, FLOW_EPSILON)
}

fn graph_cut_edge_metric(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    face_normals: &[[f64; 3]],
    edge: (usize, usize),
    first_face: usize,
    second_face: usize,
    curvature_factor: f64,
) -> f64 {
    let edge_length = norm(sub(vertices[edge.0], vertices[edge.1]));
    if curvature_factor == 0.0 {
        return edge_length;
    }
    edge_length
        * (curvature_factor
            * graph_cut_dihedral_angle_sin(
                vertices,
                faces,
                face_normals,
                edge,
                first_face,
                second_face,
            ))
        .exp()
}

fn graph_cut_dihedral_angle_sin(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    face_normals: &[[f64; 3]],
    edge: (usize, usize),
    first_face: usize,
    second_face: usize,
) -> f64 {
    let first_forward = face_has_oriented_edge(faces[first_face], edge.0, edge.1);
    let second_forward = face_has_oriented_edge(faces[second_face], edge.0, edge.1);
    let (left_face, right_face) = match (first_forward, second_forward) {
        (true, false) => (first_face, second_face),
        (false, true) => (second_face, first_face),
        _ => return 0.0,
    };
    let edge_direction = safe_normalize_vector(sub(vertices[edge.1], vertices[edge.0]));
    dot(
        edge_direction,
        cross(face_normals[left_face], face_normals[right_face]),
    )
}

fn face_has_oriented_edge(face: [usize; 3], from: usize, to: usize) -> bool {
    (face[0] == from && face[1] == to)
        || (face[1] == from && face[2] == to)
        || (face[2] == from && face[0] == to)
}

fn add_directed_flow_edge(graph: &mut [Vec<FlowEdge>], from: usize, to: usize, capacity: f64) {
    let from_rev = graph[to].len();
    let to_rev = graph[from].len();
    graph[from].push(FlowEdge {
        to,
        rev: from_rev,
        capacity,
    });
    graph[to].push(FlowEdge {
        to: from,
        rev: to_rev,
        capacity: 0.0,
    });
}

fn add_undirected_flow_edge(graph: &mut [Vec<FlowEdge>], a: usize, b: usize, capacity: f64) {
    add_directed_flow_edge(graph, a, b, capacity);
    add_directed_flow_edge(graph, b, a, capacity);
}

fn dinic_max_flow(graph: &mut [Vec<FlowEdge>], source: usize, sink: usize) -> f64 {
    const FLOW_EPSILON: f64 = 1.0e-12;
    let mut total_flow = 0.0;
    loop {
        let levels = dinic_levels(graph, source);
        if levels[sink] < 0 {
            break;
        }
        let mut next_edge = vec![0_usize; graph.len()];
        loop {
            let pushed = dinic_dfs(source, sink, f64::INFINITY, &levels, &mut next_edge, graph);
            if pushed <= FLOW_EPSILON {
                break;
            }
            total_flow += pushed;
        }
    }
    total_flow
}

fn dinic_levels(graph: &[Vec<FlowEdge>], source: usize) -> Vec<i32> {
    const FLOW_EPSILON: f64 = 1.0e-12;
    let mut levels = vec![-1_i32; graph.len()];
    let mut queue = VecDeque::from([source]);
    levels[source] = 0;
    while let Some(node) = queue.pop_front() {
        for edge in &graph[node] {
            if edge.capacity > FLOW_EPSILON && levels[edge.to] < 0 {
                levels[edge.to] = levels[node] + 1;
                queue.push_back(edge.to);
            }
        }
    }
    levels
}

fn dinic_dfs(
    node: usize,
    sink: usize,
    pushed: f64,
    levels: &[i32],
    next_edge: &mut [usize],
    graph: &mut [Vec<FlowEdge>],
) -> f64 {
    const FLOW_EPSILON: f64 = 1.0e-12;
    if pushed <= FLOW_EPSILON {
        return 0.0;
    }
    if node == sink {
        return pushed;
    }

    while next_edge[node] < graph[node].len() {
        let edge_index = next_edge[node];
        let edge = graph[node][edge_index];
        if edge.capacity > FLOW_EPSILON && levels[edge.to] == levels[node] + 1 {
            let flow = dinic_dfs(
                edge.to,
                sink,
                pushed.min(edge.capacity),
                levels,
                next_edge,
                graph,
            );
            if flow > FLOW_EPSILON {
                graph[node][edge_index].capacity -= flow;
                graph[edge.to][edge.rev].capacity += flow;
                return flow;
            }
        }
        next_edge[node] += 1;
    }
    0.0
}

fn residual_reachable_faces(
    graph: &[Vec<FlowEdge>],
    source: usize,
    face_count: usize,
    epsilon: f64,
) -> Vec<bool> {
    let mut reachable_nodes = vec![false; graph.len()];
    let mut queue = VecDeque::from([source]);
    reachable_nodes[source] = true;
    while let Some(node) = queue.pop_front() {
        for edge in &graph[node] {
            if edge.capacity > epsilon && !reachable_nodes[edge.to] {
                reachable_nodes[edge.to] = true;
                queue.push_back(edge.to);
            }
        }
    }
    reachable_nodes.into_iter().take(face_count).collect()
}
