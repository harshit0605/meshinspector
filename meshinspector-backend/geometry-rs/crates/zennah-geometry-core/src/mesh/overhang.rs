use super::base::{edge_face_map, normalize_axis_vector, safe_normalize_vector, validate_faces};
use super::graph_cut::segment_faces_by_edge_length_cut;
use super::overlap::{face_center, ray_intersects_any_face_except};
use crate::math::{cross, dot, norm, scale, sub};
use crate::GeometryError;
use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap, HashMap, VecDeque};

pub fn select_camera_facing_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    camera_direction: [f64; 3],
    min_dot: f64,
) -> Result<Vec<i64>, GeometryError> {
    if !min_dot.is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "min_dot",
            value: min_dot.to_string(),
        });
    }
    if !camera_direction.iter().all(|value| value.is_finite()) || norm(camera_direction) < 1e-8 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "camera_direction",
            value: format!("{camera_direction:?}"),
        });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    let camera_forward = normalize_axis_vector(camera_direction)?;
    let toward_camera = scale(camera_forward, -1.0);
    let threshold = min_dot.max(0.0);
    Ok(faces
        .iter()
        .enumerate()
        .filter_map(|(face_id, face)| {
            let normal = overhang_face_normal(vertices, face);
            (dot(normal, toward_camera) > threshold).then_some(face_id as i64)
        })
        .collect())
}

pub fn select_overhang_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    axis: [f64; 3],
    layer_height_mm: f64,
    max_overhang_distance_mm: f64,
    hops: i64,
) -> Result<Vec<i64>, GeometryError> {
    if !axis.iter().all(|value| value.is_finite()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "axis",
            value: format!("{axis:?}"),
        });
    }
    if !layer_height_mm.is_finite() || layer_height_mm <= 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "layer_height_mm",
            value: layer_height_mm.to_string(),
        });
    }
    if !max_overhang_distance_mm.is_finite() || max_overhang_distance_mm <= 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_overhang_distance_mm",
            value: max_overhang_distance_mm.to_string(),
        });
    }
    if hops < 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "hops",
            value: hops.to_string(),
        });
    }

    let axis = normalize_axis_vector(axis)?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(Vec::new());
    }
    let vertex_projections = vertices
        .iter()
        .map(|vertex| dot(*vertex, axis))
        .collect::<Vec<_>>();
    let min_projection = vertex_projections
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let basement_top = min_projection + layer_height_mm;
    let min_cos = -max_overhang_distance_mm / layer_height_mm.hypot(max_overhang_distance_mm);

    let mut selected = vec![false; faces.len()];
    for (face_index, face) in faces.iter().enumerate() {
        if face
            .iter()
            .all(|vertex_id| vertex_projections[*vertex_id] <= basement_top)
        {
            continue;
        }
        let normal = overhang_face_normal(vertices, face);
        if dot(axis, normal) < min_cos {
            selected[face_index] = true;
        }
    }

    if hops > 0 {
        let edge_adjacency = face_edge_adjacency(&faces);
        let mut smooth_faces = selected.clone();
        expand_selected_faces(&mut smooth_faces, &edge_adjacency, hops as usize);
        shrink_selected_faces(&mut smooth_faces, &edge_adjacency, hops as usize);
        for (face_index, is_smooth) in smooth_faces.into_iter().enumerate() {
            selected[face_index] |= is_smooth;
        }
    }

    let edge_map = edge_face_map(&faces);
    let components = selected_face_components_per_vertex(vertices.len(), &faces, &selected);
    let mut retained = BTreeSet::<usize>::new();
    for component in components {
        if component_axis_height(vertices, &faces, &component, axis) <= layer_height_mm
            && component_lateral_width(vertices, &faces, &edge_map, &component, axis)
                < max_overhang_distance_mm
        {
            continue;
        }
        retained.extend(component);
    }

    Ok(retained.into_iter().map(|face_id| face_id as i64).collect())
}

pub fn select_outer_layer_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    epsilon: f64,
) -> Result<Vec<i64>, GeometryError> {
    if !epsilon.is_finite() || epsilon < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "epsilon",
            value: epsilon.to_string(),
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(Vec::new());
    }

    let triangles = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect::<Vec<_>>();
    let ray_epsilon = epsilon.max(1e-12);
    let mut outer_seeds = vec![false; faces.len()];
    let mut inner_seeds = vec![false; faces.len()];

    for (face_index, triangle) in triangles.iter().enumerate() {
        let normal = safe_normalize_vector(cross(
            sub(triangle[1], triangle[0]),
            sub(triangle[2], triangle[0]),
        ));
        if norm(normal) < 1e-12 {
            continue;
        }
        let center = face_center(triangle);
        let outer =
            !ray_intersects_any_face_except(&triangles, center, normal, face_index, ray_epsilon);
        let inner = !ray_intersects_any_face_except(
            &triangles,
            center,
            [-normal[0], -normal[1], -normal[2]],
            face_index,
            ray_epsilon,
        );
        if outer == inner {
            continue;
        }
        if outer {
            outer_seeds[face_index] = true;
        } else {
            inner_seeds[face_index] = true;
        }
    }

    let selected =
        segment_faces_by_edge_length_cut(vertices, &faces, &outer_seeds, &inner_seeds, 1.0);
    Ok(selected
        .into_iter()
        .enumerate()
        .filter_map(|(face_index, is_selected)| is_selected.then_some(face_index as i64))
        .collect())
}

pub(super) fn overhang_face_normal(vertices: &[[f64; 3]], face: &[usize; 3]) -> [f64; 3] {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    safe_normalize_vector(cross(sub(b, a), sub(c, a)))
}

fn face_edge_adjacency(faces: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let edge_map = edge_face_map(faces);
    let mut adjacency = vec![BTreeSet::<usize>::new(); faces.len()];
    for face_ids in edge_map.values() {
        if face_ids.len() < 2 {
            continue;
        }
        for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                adjacency[face_ids[i]].insert(face_ids[j]);
                adjacency[face_ids[j]].insert(face_ids[i]);
            }
        }
    }
    adjacency
        .into_iter()
        .map(|neighbors| neighbors.into_iter().collect())
        .collect()
}

fn expand_selected_faces(selected: &mut [bool], adjacency: &[Vec<usize>], hops: usize) {
    for _ in 0..hops {
        let mut next = selected.to_vec();
        for (face_index, is_selected) in selected.iter().enumerate() {
            if !*is_selected {
                continue;
            }
            for neighbor in &adjacency[face_index] {
                next[*neighbor] = true;
            }
        }
        selected.copy_from_slice(&next);
    }
}

fn shrink_selected_faces(selected: &mut [bool], adjacency: &[Vec<usize>], hops: usize) {
    for _ in 0..hops {
        let mut next = selected.to_vec();
        for (face_index, is_selected) in selected.iter().enumerate() {
            if !*is_selected {
                continue;
            }
            if adjacency[face_index]
                .iter()
                .any(|neighbor| !selected[*neighbor])
            {
                next[face_index] = false;
            }
        }
        selected.copy_from_slice(&next);
    }
}

fn selected_face_components_per_vertex(
    vertex_count: usize,
    faces: &[[usize; 3]],
    selected: &[bool],
) -> Vec<Vec<usize>> {
    let mut vertex_faces = vec![Vec::<usize>::new(); vertex_count];
    for (face_index, face) in faces.iter().enumerate() {
        if !selected[face_index] {
            continue;
        }
        for vertex_id in face {
            vertex_faces[*vertex_id].push(face_index);
        }
    }

    let mut seen = vec![false; faces.len()];
    let mut components = Vec::new();
    for start in 0..faces.len() {
        if !selected[start] || seen[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        let mut component = Vec::new();
        while let Some(face_index) = queue.pop_front() {
            component.push(face_index);
            for vertex_id in &faces[face_index] {
                for neighbor in &vertex_faces[*vertex_id] {
                    if !seen[*neighbor] {
                        seen[*neighbor] = true;
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
        components.push(component);
    }
    components
}

fn component_axis_height(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    component: &[usize],
    axis: [f64; 3],
) -> f64 {
    let mut min_projection = f64::INFINITY;
    let mut max_projection = f64::NEG_INFINITY;
    for face_id in component {
        for vertex_id in &faces[*face_id] {
            let projection = dot(vertices[*vertex_id], axis);
            min_projection = min_projection.min(projection);
            max_projection = max_projection.max(projection);
        }
    }
    max_projection - min_projection
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WidthQueueEntry {
    cost: f64,
    vertex_id: usize,
}

impl Eq for WidthQueueEntry {}

impl Ord for WidthQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| self.vertex_id.cmp(&other.vertex_id))
    }
}

impl PartialOrd for WidthQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn component_lateral_width(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    edge_map: &rustc_hash::FxHashMap<(usize, usize), Vec<usize>>,
    component: &[usize],
    axis: [f64; 3],
) -> f64 {
    let selected_faces = component.iter().copied().collect::<BTreeSet<_>>();
    let mut region_edges = HashMap::<(usize, usize), f64>::new();
    let mut boundary_vertices = BTreeSet::<usize>::new();
    for face_id in component {
        let face = faces[*face_id];
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let edge = if a <= b { (a, b) } else { (b, a) };
            region_edges
                .entry(edge)
                .or_insert_with(|| lateral_edge_length(vertices[a], vertices[b], axis));
            if edge_map.get(&edge).is_none_or(|face_ids| {
                face_ids
                    .iter()
                    .any(|candidate| !selected_faces.contains(candidate))
            }) {
                boundary_vertices.insert(a);
                boundary_vertices.insert(b);
            }
        }
    }

    let fallback_width = region_edges.values().copied().fold(0.0_f64, f64::max);
    if region_edges.is_empty() || boundary_vertices.is_empty() {
        return fallback_width;
    }

    let mut graph = HashMap::<usize, Vec<(usize, f64)>>::new();
    for ((a, b), weight) in &region_edges {
        graph.entry(*a).or_default().push((*b, *weight));
        graph.entry(*b).or_default().push((*a, *weight));
    }

    let mut distances = HashMap::<usize, f64>::new();
    let mut queue = BinaryHeap::<WidthQueueEntry>::new();
    for vertex_id in boundary_vertices {
        distances.insert(vertex_id, 0.0);
        queue.push(WidthQueueEntry {
            cost: 0.0,
            vertex_id,
        });
    }
    while let Some(entry) = queue.pop() {
        if entry.cost > *distances.get(&entry.vertex_id).unwrap_or(&f64::INFINITY) {
            continue;
        }
        for (neighbor, weight) in graph.get(&entry.vertex_id).into_iter().flatten() {
            let next = entry.cost + *weight;
            if next < *distances.get(neighbor).unwrap_or(&f64::INFINITY) {
                distances.insert(*neighbor, next);
                queue.push(WidthQueueEntry {
                    cost: next,
                    vertex_id: *neighbor,
                });
            }
        }
    }

    let max_boundary_distance = distances.values().copied().fold(0.0_f64, f64::max);
    if max_boundary_distance > 0.0 {
        2.0 * max_boundary_distance
    } else {
        fallback_width
    }
}

fn lateral_edge_length(a: [f64; 3], b: [f64; 3], axis: [f64; 3]) -> f64 {
    let edge = sub(b, a);
    let edge_length_sq = dot(edge, edge);
    let axial = dot(edge, axis);
    (edge_length_sq - axial * axial).max(0.0).sqrt()
}
