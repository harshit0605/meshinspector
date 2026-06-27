use super::base::{face_area, ordered_edge_face_entries, validate_faces};
use crate::GeometryError;
use std::collections::{BTreeSet, VecDeque};

pub fn face_adjacency_for_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<Vec<i64>>, GeometryError> {
    let face_count = faces_i64.len();
    let mut adjacency = vec![Vec::<i64>::new(); face_count];
    for entry in ordered_edge_face_entries(faces_i64, vertices.len())? {
        if entry.face_indices.len() < 2 {
            continue;
        }
        for i in 0..entry.face_indices.len() {
            for j in (i + 1)..entry.face_indices.len() {
                let a = entry.face_indices[i];
                let b = entry.face_indices[j];
                adjacency[a].push(b as i64);
                adjacency[b].push(a as i64);
            }
        }
    }
    Ok(adjacency)
}

pub fn connected_face_components_for_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<Vec<i64>>, GeometryError> {
    let adjacency = face_adjacency_for_mesh(vertices, faces_i64)?;
    let mut seen = vec![false; adjacency.len()];
    let mut components = Vec::new();
    for start in 0..adjacency.len() {
        if seen[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        let mut component = Vec::new();
        while let Some(face_id) = queue.pop_front() {
            component.push(face_id as i64);
            for neighbor in &adjacency[face_id] {
                let neighbor = *neighbor as usize;
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
        components.push(component);
    }
    Ok(components)
}

pub fn select_largest_component_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    min_area_mm2: f64,
) -> Result<Vec<i64>, GeometryError> {
    if !min_area_mm2.is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "min_area_mm2",
            value: min_area_mm2.to_string(),
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    let adjacency = face_adjacency_for_mesh(vertices, faces_i64)?;
    let mut seen = vec![false; adjacency.len()];
    let mut largest_component = Vec::<usize>::new();
    let mut largest_area = f64::NEG_INFINITY;

    for start in 0..adjacency.len() {
        if seen[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        let mut component = Vec::<usize>::new();
        let mut component_area = 0.0;

        while let Some(face_id) = queue.pop_front() {
            component.push(face_id);
            component_area += face_area(vertices, &faces[face_id]);
            for neighbor in &adjacency[face_id] {
                let neighbor = *neighbor as usize;
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }

        if component_area > largest_area {
            largest_area = component_area;
            largest_component = component;
        }
    }

    if largest_component.is_empty() || largest_area < min_area_mm2 {
        return Ok(Vec::new());
    }

    largest_component.sort_unstable();
    Ok(largest_component
        .into_iter()
        .map(|face_id| face_id as i64)
        .collect())
}

pub fn expand_face_selection_to_components(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    seed_face_ids: &[usize],
) -> Result<Vec<i64>, GeometryError> {
    if seed_face_ids.is_empty() {
        return Ok(Vec::new());
    }
    let adjacency = face_adjacency_for_mesh(vertices, faces_i64)?;
    let face_count = adjacency.len();
    for seed in seed_face_ids {
        if *seed >= face_count {
            return Err(GeometryError::FaceRegionIndexOutOfBounds {
                index: *seed,
                face_count,
            });
        }
    }

    let mut selected = BTreeSet::<usize>::new();
    for seed in seed_face_ids {
        if selected.contains(seed) {
            continue;
        }
        let mut queue = VecDeque::from([*seed]);
        selected.insert(*seed);
        while let Some(face_id) = queue.pop_front() {
            for neighbor in &adjacency[face_id] {
                let neighbor = *neighbor as usize;
                if selected.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }
    Ok(selected.into_iter().map(|face_id| face_id as i64).collect())
}
