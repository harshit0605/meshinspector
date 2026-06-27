use crate::math::{add, cross, dot, norm, sub};
use crate::{GeometryError, MeshHealth, MeshStats};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::collections::{BTreeSet, HashMap, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeFaceEntry {
    pub edge: [usize; 2],
    pub face_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSelectionObject {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub source_vertex_indices: Vec<usize>,
    pub source_face_indices: Vec<usize>,
    pub vertex_uvs: Option<Vec<[f64; 2]>>,
    pub vertex_colors: Option<Vec<[u8; 4]>>,
    pub face_colors: Option<Vec<[u8; 4]>>,
    pub texture_per_face: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshSelectionAttributes {
    pub vertex_uvs: Option<Vec<[f64; 2]>>,
    pub vertex_colors: Option<Vec<[u8; 4]>>,
    pub face_colors: Option<Vec<[u8; 4]>>,
    pub texture_per_face: Option<Vec<i64>>,
}

pub fn extract_selected_faces_as_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    selected_face_ids: &[usize],
) -> Result<MeshSelectionObject, GeometryError> {
    extract_selected_faces_as_mesh_with_attributes(
        vertices,
        faces_i64,
        selected_face_ids,
        MeshSelectionAttributes::default(),
    )
}

pub fn extract_selected_faces_as_mesh_with_attributes(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    selected_face_ids: &[usize],
    attributes: MeshSelectionAttributes,
) -> Result<MeshSelectionObject, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if selected_face_ids.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "selected_face_ids",
            value: "must not be empty".to_string(),
        });
    }

    let mut source_face_indices = BTreeSet::new();
    for face_id in selected_face_ids {
        if *face_id >= faces.len() {
            return Err(GeometryError::FaceRegionIndexOutOfBounds {
                index: *face_id,
                face_count: faces.len(),
            });
        }
        source_face_indices.insert(*face_id);
    }

    let mut vertices_out = Vec::new();
    let mut faces_out = Vec::new();
    let mut source_vertex_indices = Vec::new();
    let mut vertex_map: HashMap<usize, usize> = HashMap::new();

    for source_face_id in source_face_indices.iter().copied() {
        let mut face = [0_i64; 3];
        for (corner, source_vertex_id) in faces[source_face_id].iter().copied().enumerate() {
            let target_vertex_id = if let Some(target_vertex_id) = vertex_map.get(&source_vertex_id)
            {
                *target_vertex_id
            } else {
                let target_vertex_id = vertices_out.len();
                vertex_map.insert(source_vertex_id, target_vertex_id);
                vertices_out.push(vertices[source_vertex_id]);
                source_vertex_indices.push(source_vertex_id);
                target_vertex_id
            };
            face[corner] = target_vertex_id as i64;
        }
        faces_out.push(face);
    }

    Ok(MeshSelectionObject {
        vertex_uvs: remap_vertex_attribute(
            attributes.vertex_uvs,
            &source_vertex_indices,
            vertices.len(),
            "vertex_uvs",
        )?,
        vertex_colors: remap_vertex_attribute(
            attributes.vertex_colors,
            &source_vertex_indices,
            vertices.len(),
            "vertex_colors",
        )?,
        face_colors: remap_face_attribute(
            attributes.face_colors,
            &source_face_indices.iter().copied().collect::<Vec<_>>(),
            faces.len(),
            "face_colors",
        )?,
        texture_per_face: remap_face_attribute(
            attributes.texture_per_face,
            &source_face_indices.iter().copied().collect::<Vec<_>>(),
            faces.len(),
            "texture_per_face",
        )?,
        vertices: vertices_out,
        faces: faces_out,
        source_vertex_indices,
        source_face_indices: source_face_indices.into_iter().collect(),
    })
}

fn remap_vertex_attribute<T: Copy>(
    values: Option<Vec<T>>,
    source_vertex_indices: &[usize],
    source_vertex_count: usize,
    field: &'static str,
) -> Result<Option<Vec<T>>, GeometryError> {
    remap_indexed_attribute(values, source_vertex_indices, source_vertex_count, field)
}

fn remap_face_attribute<T: Copy>(
    values: Option<Vec<T>>,
    source_face_indices: &[usize],
    source_face_count: usize,
    field: &'static str,
) -> Result<Option<Vec<T>>, GeometryError> {
    remap_indexed_attribute(values, source_face_indices, source_face_count, field)
}

fn remap_indexed_attribute<T: Copy>(
    values: Option<Vec<T>>,
    source_indices: &[usize],
    source_count: usize,
    field: &'static str,
) -> Result<Option<Vec<T>>, GeometryError> {
    let Some(values) = values else {
        return Ok(None);
    };
    if values.is_empty() {
        return Ok(None);
    }
    if values.len() < source_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!(
                "expected at least {source_count} entries for MeshLib cloneRegion attribute remap, got {}",
                values.len()
            ),
        });
    }
    Ok(Some(
        source_indices.iter().map(|index| values[*index]).collect(),
    ))
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
    faces.iter().map(|face| face_area(vertices, face)).sum()
}

pub(super) fn face_area(vertices: &[[f64; 3]], face: &[usize; 3]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    norm(cross(sub(b, a), sub(c, a))) * 0.5
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

pub(crate) fn edge_face_map(faces: &[[usize; 3]]) -> FxHashMap<(usize, usize), Vec<usize>> {
    let mut edges: FxHashMap<(usize, usize), Vec<usize>> = FxHashMap::default();
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
        .par_iter()
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

pub fn select_boundary_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<i64>, GeometryError> {
    let mut face_ids = BTreeSet::<usize>::new();
    for entry in ordered_edge_face_entries(faces_i64, vertices.len())? {
        if entry.face_indices.len() == 1 {
            face_ids.insert(entry.face_indices[0]);
        }
    }
    Ok(face_ids.into_iter().map(|face_id| face_id as i64).collect())
}

pub fn select_boundary_edges(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[i64; 2]>, GeometryError> {
    boundary_edges_for_mesh(vertices, faces_i64)
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
    edge_map: &FxHashMap<(usize, usize), Vec<usize>>,
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
    // Build adjacency as flat Vecs (push, then sort + dedup) instead of a
    // BTreeSet per vertex — avoids one tree allocation per vertex (millions on
    // large meshes) and keeps neighbor data contiguous for the smoothing /
    // thickness loops that iterate it. Output is identical: sorted ascending,
    // deduped — exactly the order a BTreeSet yielded, so consumers are unaffected.
    let mut neighbors = vec![Vec::<usize>::new(); vertex_count];
    for face in faces {
        let [a, b, c] = *face;
        neighbors[a].push(b);
        neighbors[a].push(c);
        neighbors[b].push(a);
        neighbors[b].push(c);
        neighbors[c].push(a);
        neighbors[c].push(b);
    }
    neighbors.par_iter_mut().for_each(|items| {
        items.sort_unstable();
        items.dedup();
    });
    neighbors
}

pub(crate) fn vertex_normals(
    vertex_count: usize,
    triangles: &[[[f64; 3]; 3]],
    faces: &[[usize; 3]],
) -> Vec<[f64; 3]> {
    let mut normals = vec![[0.0; 3]; vertex_count];
    let area_weighted: Vec<[f64; 3]> = triangles
        .par_iter()
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
    normals.into_par_iter().map(safe_normalize_vector).collect()
}

pub(crate) fn vertex_normals_from_faces(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
) -> Vec<[f64; 3]> {
    let mut normals = vec![[0.0; 3]; vertices.len()];
    let area_weighted: Vec<[f64; 3]> = faces
        .par_iter()
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
    normals.into_par_iter().map(safe_normalize_vector).collect()
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
