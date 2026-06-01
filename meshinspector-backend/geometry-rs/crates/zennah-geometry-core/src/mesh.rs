use crate::math::{add, cross, dot, norm, sub};
use crate::{GeometryError, MeshHealth, MeshStats};
use std::collections::{BTreeSet, HashMap, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeFaceEntry {
    pub edge: [usize; 2],
    pub face_indices: Vec<usize>,
}

pub fn mesh_stats(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<MeshStats, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let (bbox_min, bbox_max) = bounds(vertices);
    let bbox_size = [
        bbox_max[0] - bbox_min[0],
        bbox_max[1] - bbox_min[1],
        bbox_max[2] - bbox_min[2],
    ];
    let edge_map = edge_face_map(&faces);
    let boundary_edge_count = edge_map
        .values()
        .filter(|face_ids| face_ids.len() == 1)
        .count();
    Ok(MeshStats {
        bbox_min,
        bbox_max,
        bbox_size,
        surface_area_mm2: surface_area(vertices, &faces),
        volume_mm3: signed_volume(vertices, &faces).abs(),
        vertex_count: vertices.len(),
        face_count: faces.len(),
        connected_components: connected_face_components(&faces, &edge_map),
        boundary_edge_count,
    })
}

pub fn boundary_loops(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<Vec<usize>>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let edge_map = edge_face_map(&faces);
    let mut adjacency: HashMap<usize, BTreeSet<usize>> = HashMap::new();
    for ((a, b), face_ids) in edge_map {
        if face_ids.len() != 1 {
            continue;
        }
        adjacency.entry(a).or_default().insert(b);
        adjacency.entry(b).or_default().insert(a);
    }
    if adjacency.is_empty() {
        return Ok(Vec::new());
    }

    let mut loops = Vec::new();
    let mut seen = BTreeSet::new();
    for start in adjacency.keys().copied().collect::<BTreeSet<_>>() {
        if seen.contains(&start) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        seen.insert(start);
        let mut component = Vec::new();
        while let Some(vertex_id) = queue.pop_front() {
            component.push(vertex_id);
            if let Some(neighbors) = adjacency.get(&vertex_id) {
                for neighbor in neighbors {
                    if seen.insert(*neighbor) {
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
        loops.push(component);
    }
    Ok(loops)
}

pub fn mesh_health(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    detect_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> Result<MeshHealth, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let edge_map = edge_face_map(&faces);
    let boundary_edge_count = edge_map
        .values()
        .filter(|face_ids| face_ids.len() == 1)
        .count();
    let nonmanifold_edge_count = edge_map
        .values()
        .filter(|face_ids| face_ids.len() > 2)
        .count();
    let holes_count = boundary_loops(vertices, faces_i64)?.len();
    let should_detect_intersections = detect_self_intersections
        && max_self_intersection_faces.is_none_or(|limit| faces.len() <= limit);
    let intersecting_faces = if should_detect_intersections {
        Some(crate::spatial::self_intersecting_faces(
            vertices, faces_i64, epsilon,
        )?)
    } else {
        None
    };

    Ok(MeshHealth {
        is_closed: boundary_edge_count == 0 && nonmanifold_edge_count == 0,
        holes_count,
        boundary_edge_count,
        nonmanifold_edge_count,
        self_intersections: intersecting_faces.as_ref().map(Vec::len),
        self_intersections_available: intersecting_faces.is_some(),
    })
}

pub(crate) fn validate_faces(
    faces: &[[i64; 3]],
    vertex_count: usize,
) -> Result<Vec<[usize; 3]>, GeometryError> {
    let mut output = Vec::with_capacity(faces.len());
    for (face_index, face) in faces.iter().enumerate() {
        let mut converted = [0_usize; 3];
        for (corner, value) in face.iter().enumerate() {
            if *value < 0 {
                return Err(GeometryError::NegativeFaceIndex {
                    face: face_index,
                    vertex: *value,
                });
            }
            let vertex = *value as usize;
            if vertex >= vertex_count {
                return Err(GeometryError::FaceIndexOutOfBounds {
                    face: face_index,
                    vertex: *value,
                    vertex_count,
                });
            }
            converted[corner] = vertex;
        }
        output.push(converted);
    }
    Ok(output)
}

pub(crate) fn bounds(vertices: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    if vertices.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }
    let mut bbox_min = vertices[0];
    let mut bbox_max = vertices[0];
    for vertex in vertices.iter().skip(1) {
        for axis in 0..3 {
            bbox_min[axis] = bbox_min[axis].min(vertex[axis]);
            bbox_max[axis] = bbox_max[axis].max(vertex[axis]);
        }
    }
    (bbox_min, bbox_max)
}

pub(crate) fn surface_area(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> f64 {
    faces
        .iter()
        .map(|face| {
            let a = vertices[face[0]];
            let b = vertices[face[1]];
            let c = vertices[face[2]];
            norm(cross(sub(b, a), sub(c, a))) * 0.5
        })
        .sum()
}

pub(crate) fn signed_volume(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> f64 {
    faces
        .iter()
        .map(|face| {
            let a = vertices[face[0]];
            let b = vertices[face[1]];
            let c = vertices[face[2]];
            dot(a, cross(b, c)) / 6.0
        })
        .sum()
}

pub(crate) fn edge_face_map(faces: &[[usize; 3]]) -> HashMap<(usize, usize), Vec<usize>> {
    let mut edges: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (face_index, face) in faces.iter().enumerate() {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let key = if a <= b { (a, b) } else { (b, a) };
            edges.entry(key).or_default().push(face_index);
        }
    }
    edges
}

pub fn mesh_bounds(vertices: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    bounds(vertices)
}

pub fn safe_normalize_vectors(vectors: &[[f64; 3]]) -> Vec<[f64; 3]> {
    vectors.iter().copied().map(safe_normalize_vector).collect()
}

pub fn normalize_axis_vector(axis: [f64; 3]) -> Result<[f64; 3], GeometryError> {
    let magnitude = norm(axis);
    if magnitude < 1e-8 {
        Err(GeometryError::DirectionTooSmall)
    } else {
        Ok([
            axis[0] / magnitude,
            axis[1] / magnitude,
            axis[2] / magnitude,
        ])
    }
}

pub fn face_normals_for_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    Ok(faces
        .iter()
        .map(|face| {
            let a = vertices[face[0]];
            let b = vertices[face[1]];
            let c = vertices[face[2]];
            safe_normalize_vector(cross(sub(b, a), sub(c, a)))
        })
        .collect())
}

pub fn vertex_normals_for_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    Ok(vertex_normals_from_faces(vertices, &faces))
}

pub fn mesh_surface_area(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<f64, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    Ok(surface_area(vertices, &faces))
}

pub fn mesh_signed_volume(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<f64, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    Ok(signed_volume(vertices, &faces))
}

pub fn mesh_volume(vertices: &[[f64; 3]], faces_i64: &[[i64; 3]]) -> Result<f64, GeometryError> {
    Ok(mesh_signed_volume(vertices, faces_i64)?.abs())
}

pub fn ordered_edge_face_entries(
    faces_i64: &[[i64; 3]],
    vertex_count: usize,
) -> Result<Vec<EdgeFaceEntry>, GeometryError> {
    let faces = validate_faces(faces_i64, vertex_count)?;
    let mut edge_positions: HashMap<(usize, usize), usize> = HashMap::new();
    let mut entries: Vec<EdgeFaceEntry> = Vec::new();
    for (face_index, face) in faces.iter().enumerate() {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let edge = if a <= b { (a, b) } else { (b, a) };
            if let Some(entry_index) = edge_positions.get(&edge) {
                entries[*entry_index].face_indices.push(face_index);
            } else {
                edge_positions.insert(edge, entries.len());
                entries.push(EdgeFaceEntry {
                    edge: [edge.0, edge.1],
                    face_indices: vec![face_index],
                });
            }
        }
    }
    Ok(entries)
}

pub fn boundary_edges_for_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[i64; 2]>, GeometryError> {
    Ok(ordered_edge_face_entries(faces_i64, vertices.len())?
        .into_iter()
        .filter(|entry| entry.face_indices.len() == 1)
        .map(|entry| [entry.edge[0] as i64, entry.edge[1] as i64])
        .collect())
}

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

pub fn vertex_neighbors_for_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<Vec<i64>>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    Ok(vertex_neighbor_list(vertices.len(), &faces)
        .into_iter()
        .map(|neighbors| neighbors.into_iter().map(|value| value as i64).collect())
        .collect())
}

pub(crate) fn connected_face_components(
    faces: &[[usize; 3]],
    edge_map: &HashMap<(usize, usize), Vec<usize>>,
) -> usize {
    if faces.is_empty() {
        return 0;
    }
    let mut adjacency = vec![Vec::<usize>::new(); faces.len()];
    for face_ids in edge_map.values() {
        if face_ids.len() < 2 {
            continue;
        }
        for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                let a = face_ids[i];
                let b = face_ids[j];
                adjacency[a].push(b);
                adjacency[b].push(a);
            }
        }
    }
    let mut seen = vec![false; faces.len()];
    let mut components = 0;
    for start in 0..faces.len() {
        if seen[start] {
            continue;
        }
        components += 1;
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        while let Some(face) = queue.pop_front() {
            for neighbor in &adjacency[face] {
                if !seen[*neighbor] {
                    seen[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
    }
    components
}

pub(crate) fn vertex_neighbor_list(vertex_count: usize, faces: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let mut neighbors = vec![BTreeSet::<usize>::new(); vertex_count];
    for face in faces {
        let [a, b, c] = *face;
        neighbors[a].insert(b);
        neighbors[a].insert(c);
        neighbors[b].insert(a);
        neighbors[b].insert(c);
        neighbors[c].insert(a);
        neighbors[c].insert(b);
    }
    neighbors
        .into_iter()
        .map(|items| items.into_iter().collect())
        .collect()
}

pub(crate) fn vertex_normals(
    vertex_count: usize,
    triangles: &[[[f64; 3]; 3]],
    faces: &[[usize; 3]],
) -> Vec<[f64; 3]> {
    let mut normals = vec![[0.0; 3]; vertex_count];
    let area_weighted: Vec<[f64; 3]> = triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = *triangle;
            cross(sub(b, a), sub(c, a))
        })
        .collect();
    for corner in 0..3 {
        for (face_index, face) in faces.iter().enumerate() {
            let vertex_id = face[corner];
            normals[vertex_id] = add(normals[vertex_id], area_weighted[face_index]);
        }
    }
    normals.into_iter().map(safe_normalize_vector).collect()
}

pub(crate) fn vertex_normals_from_faces(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
) -> Vec<[f64; 3]> {
    let mut normals = vec![[0.0; 3]; vertices.len()];
    let area_weighted: Vec<[f64; 3]> = faces
        .iter()
        .map(|face| {
            let a = vertices[face[0]];
            let b = vertices[face[1]];
            let c = vertices[face[2]];
            cross(sub(b, a), sub(c, a))
        })
        .collect();
    for corner in 0..3 {
        for (face_index, face) in faces.iter().enumerate() {
            let vertex_id = face[corner];
            normals[vertex_id] = add(normals[vertex_id], area_weighted[face_index]);
        }
    }
    normals.into_iter().map(safe_normalize_vector).collect()
}

pub fn safe_normalize_vector(vector: [f64; 3]) -> [f64; 3] {
    let magnitude = norm(vector);
    if magnitude < 1e-12 {
        [0.0; 3]
    } else {
        [
            vector[0] / magnitude,
            vector[1] / magnitude,
            vector[2] / magnitude,
        ]
    }
}

pub(crate) fn vertex_face_adjacency(vertex_count: usize, faces: &[[usize; 3]]) -> Vec<Vec<i64>> {
    let mut adjacency = vec![Vec::new(); vertex_count];
    for (face_index, face) in faces.iter().enumerate() {
        for vertex_id in face {
            adjacency[*vertex_id].push(face_index as i64);
        }
    }
    adjacency
}
