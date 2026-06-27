use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateNonManifoldVerticesReport {
    pub input_nonmanifold_vertex_count: usize,
    pub output_nonmanifold_vertex_count: usize,
    pub duplicated_vertex_count: usize,
    pub input_vertex_count: usize,
    pub output_vertex_count: usize,
    pub input_face_count: usize,
    pub output_face_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateNonManifoldVerticesResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: DuplicateNonManifoldVerticesReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VertexDuplication {
    pub src_vertex: usize,
    pub dup_vertex: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateNonManifoldVertexIdsResult {
    pub faces: Vec<[i64; 3]>,
    pub duplications: Vec<VertexDuplication>,
    pub input_nonmanifold_vertex_count: usize,
    pub output_nonmanifold_vertex_count: usize,
}

#[derive(Clone, Debug)]
struct IncidentItem {
    face: usize,
    src_vertex: usize,
}

pub fn duplicate_nonmanifold_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<DuplicateNonManifoldVerticesResult, GeometryError> {
    duplicate_nonmanifold_vertices_with_region(vertices, faces_i64, None)
}

pub fn duplicate_nonmanifold_vertices_in_region(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    region_face_indices: &[usize],
) -> Result<DuplicateNonManifoldVerticesResult, GeometryError> {
    duplicate_nonmanifold_vertices_with_region(vertices, faces_i64, Some(region_face_indices))
}

pub fn duplicate_nonmanifold_vertex_ids_with_last_valid_vertex(
    faces_i64: &[[i64; 3]],
    region_face_indices: Option<&[usize]>,
    last_valid_vertex: Option<usize>,
) -> Result<DuplicateNonManifoldVertexIdsResult, GeometryError> {
    let faces = validate_face_ids(faces_i64)?;
    let region = normalize_face_region(region_face_indices, faces.len())?;
    let input_vertex_count = max_face_vertex(&faces).map_or(0, |vertex| vertex + 1);
    let input_nonmanifold_vertex_count =
        non_manifold_vertex_count_in_components(input_vertex_count, &faces, region.as_deref());
    let mut output_faces = faces_i64.to_vec();
    let mut duplications = Vec::new();
    let mut last_used_vertex =
        last_valid_vertex.unwrap_or_else(|| max_face_vertex(&faces).unwrap_or(0));
    duplicate_nonmanifold_vertex_ids_meshlib_pass(
        &mut output_faces,
        region.as_deref(),
        &mut last_used_vertex,
        &mut duplications,
    )?;

    let output_faces_usize = validate_face_ids(&output_faces)?;
    let output_vertex_count = max_face_vertex(&output_faces_usize).map_or(0, |vertex| vertex + 1);
    let output_nonmanifold_vertex_count = non_manifold_vertex_count_in_components(
        output_vertex_count,
        &output_faces_usize,
        region.as_deref(),
    );
    Ok(DuplicateNonManifoldVertexIdsResult {
        faces: output_faces,
        duplications,
        input_nonmanifold_vertex_count,
        output_nonmanifold_vertex_count,
    })
}

fn duplicate_nonmanifold_vertices_with_region(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    region_face_indices: Option<&[usize]>,
) -> Result<DuplicateNonManifoldVerticesResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let id_result = duplicate_nonmanifold_vertex_ids_with_last_valid_vertex(
        faces_i64,
        region_face_indices,
        Some(vertices.len().saturating_sub(1)),
    )?;
    let mut output_vertices = vertices.to_vec();
    for duplication in &id_result.duplications {
        if duplication.dup_vertex == output_vertices.len() {
            output_vertices.push(output_vertices[duplication.src_vertex]);
        }
    }
    Ok(DuplicateNonManifoldVerticesResult {
        vertices: output_vertices.clone(),
        faces: id_result.faces.clone(),
        report: DuplicateNonManifoldVerticesReport {
            input_nonmanifold_vertex_count: id_result.input_nonmanifold_vertex_count,
            output_nonmanifold_vertex_count: id_result.output_nonmanifold_vertex_count,
            duplicated_vertex_count: id_result.duplications.len(),
            input_vertex_count: vertices.len(),
            output_vertex_count: output_vertices.len(),
            input_face_count: faces_i64.len(),
            output_face_count: id_result.faces.len(),
        },
    })
}

pub fn non_manifold_vertex_count(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<usize, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    Ok(non_manifold_vertex_count_in_components(
        vertices.len(),
        &faces,
        None,
    ))
}

fn non_manifold_vertex_count_in_components(
    vertex_count: usize,
    faces: &[[usize; 3]],
    region_face_indices: Option<&[usize]>,
) -> usize {
    let mut incident_faces = vec![Vec::new(); vertex_count];
    for face_index in face_indices(faces.len(), region_face_indices) {
        let face = faces[face_index];
        for vertex in unique_face_vertices(face) {
            incident_faces[vertex].push(face_index);
        }
    }
    let mut output = 0_usize;
    for (vertex, face_ids) in incident_faces.iter().enumerate() {
        if face_ids.len() <= 1 {
            continue;
        }
        let components = meshlib_incident_face_chains(vertex, face_ids, faces);
        if components.len() > 1 {
            output += 1;
        }
    }
    output
}

fn normalize_face_region(
    region_face_indices: Option<&[usize]>,
    face_count: usize,
) -> Result<Option<Vec<usize>>, GeometryError> {
    let Some(indices) = region_face_indices else {
        return Ok(None);
    };
    let mut normalized = indices.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    if let Some(out_of_range) = normalized.iter().find(|index| **index >= face_count) {
        return Err(GeometryError::FaceRegionIndexOutOfBounds {
            index: *out_of_range,
            face_count,
        });
    }
    Ok(Some(normalized))
}

fn duplicate_nonmanifold_vertex_ids_meshlib_pass(
    faces: &mut Vec<[i64; 3]>,
    region_face_indices: Option<&[usize]>,
    last_used_vertex: &mut usize,
    duplications: &mut Vec<VertexDuplication>,
) -> Result<(), GeometryError> {
    let mut incident_items = preprocess_incident_items(faces, region_face_indices)?;
    if incident_items.is_empty() {
        return Ok(());
    }

    let mut path = Vec::new();
    let mut closed_path = Vec::new();
    let mut visited_vertices = HashSet::new();
    let mut pos_end = 0_usize;
    while pos_end != incident_items.len() {
        let pos_begin = pos_end;
        pos_end += 1;
        while pos_end < incident_items.len()
            && incident_items[pos_begin].src_vertex == incident_items[pos_end].src_vertex
        {
            pos_end += 1;
        }

        let mut last_unvisited = pos_end - pos_begin;
        let mut found_chains = 0_usize;
        while last_unvisited > 0 {
            for vertex in &path {
                visited_vertices.remove(vertex);
            }

            let mut orientation = true;
            let Some(first_vertex) = first_incident_neighbor_i64(
                faces[incident_items[pos_begin].face],
                incident_items[pos_begin].src_vertex,
            ) else {
                break;
            };
            visited_vertices.insert(first_vertex);
            let mut next_vertex = take_next_incident_meshlib(
                faces,
                &mut incident_items,
                pos_begin,
                &mut last_unvisited,
                first_vertex,
                orientation,
            );
            if next_vertex.is_none() {
                orientation = false;
                next_vertex = take_next_incident_meshlib(
                    faces,
                    &mut incident_items,
                    pos_begin,
                    &mut last_unvisited,
                    first_vertex,
                    orientation,
                );
            }
            let Some(mut next_vertex) = next_vertex else {
                break;
            };
            visited_vertices.insert(next_vertex);
            path = vec![first_vertex, next_vertex];

            loop {
                next_vertex = match take_next_incident_meshlib(
                    faces,
                    &mut incident_items,
                    pos_begin,
                    &mut last_unvisited,
                    next_vertex,
                    orientation,
                ) {
                    Some(vertex) => vertex,
                    None => {
                        if orientation {
                            orientation = false;
                            if let Some(vertex) = take_next_incident_meshlib(
                                faces,
                                &mut incident_items,
                                pos_begin,
                                &mut last_unvisited,
                                first_vertex,
                                orientation,
                            ) {
                                path.reverse();
                                vertex
                            } else {
                                if found_chains > 0 {
                                    duplicate_meshlib_path(
                                        faces,
                                        &mut incident_items,
                                        pos_begin,
                                        pos_end,
                                        last_unvisited,
                                        &path,
                                        last_used_vertex,
                                        duplications,
                                    );
                                }
                                break;
                            }
                        } else {
                            if found_chains > 0 {
                                duplicate_meshlib_path(
                                    faces,
                                    &mut incident_items,
                                    pos_begin,
                                    pos_end,
                                    last_unvisited,
                                    &path,
                                    last_used_vertex,
                                    duplications,
                                );
                            }
                            break;
                        }
                    }
                };

                if visited_vertices.contains(&next_vertex) {
                    path.push(next_vertex);
                    extract_closed_path(&mut path, &mut closed_path);
                    for vertex in &closed_path {
                        visited_vertices.remove(vertex);
                    }
                    if found_chains > 0 {
                        duplicate_meshlib_path(
                            faces,
                            &mut incident_items,
                            pos_begin,
                            pos_end,
                            last_unvisited,
                            &closed_path,
                            last_used_vertex,
                            duplications,
                        );
                    }
                    found_chains += 1;
                    if path.is_empty() {
                        break;
                    }
                }
                path.push(next_vertex);
                visited_vertices.insert(next_vertex);
            }
            found_chains += 1;
        }
    }
    Ok(())
}

fn preprocess_incident_items(
    faces: &[[i64; 3]],
    region_face_indices: Option<&[usize]>,
) -> Result<Vec<IncidentItem>, GeometryError> {
    let mut incident_items = Vec::with_capacity(3 * faces.len());
    for face_index in face_indices(faces.len(), region_face_indices) {
        let face = faces[face_index];
        if face[0] == face[1] || face[1] == face[2] || face[2] == face[0] {
            continue;
        }
        for vertex in face {
            if vertex < 0 {
                return Err(GeometryError::NegativeFaceIndex {
                    face: face_index,
                    vertex,
                });
            }
            incident_items.push(IncidentItem {
                face: face_index,
                src_vertex: vertex as usize,
            });
        }
    }
    incident_items.sort_by_key(|item| item.src_vertex);
    Ok(incident_items)
}

fn take_next_incident_meshlib(
    faces: &[[i64; 3]],
    incident_items: &mut [IncidentItem],
    pos_begin: usize,
    last_unvisited: &mut usize,
    current: usize,
    orientation: bool,
) -> Option<usize> {
    if *last_unvisited == 0 {
        return None;
    }
    for offset in 0..*last_unvisited {
        let item_index = pos_begin + offset;
        let item = &incident_items[item_index];
        let next =
            next_incident_vertex_i64(faces[item.face], item.src_vertex, current, orientation);
        if let Some(next) = next {
            *last_unvisited -= 1;
            incident_items.swap(item_index, pos_begin + *last_unvisited);
            return Some(next);
        }
    }
    None
}

fn duplicate_meshlib_path(
    faces: &mut [[i64; 3]],
    incident_items: &mut [IncidentItem],
    pos_begin: usize,
    pos_end: usize,
    last_unvisited: usize,
    path: &[usize],
    last_used_vertex: &mut usize,
    duplications: &mut Vec<VertexDuplication>,
) {
    if path.len() < 2 {
        return;
    }
    let src_vertex = incident_items[pos_begin].src_vertex;
    *last_used_vertex += 1;
    let dup_vertex = *last_used_vertex;
    duplications.push(VertexDuplication {
        src_vertex,
        dup_vertex,
    });

    for edge in path.windows(2) {
        for item in incident_items
            .iter_mut()
            .take(pos_end)
            .skip(pos_begin + last_unvisited)
        {
            let face = &mut faces[item.face];
            let mut neighbors = Vec::with_capacity(2);
            let mut already_duplicated = true;
            for vertex in face.iter().copied() {
                if vertex == src_vertex as i64 {
                    already_duplicated = false;
                } else {
                    neighbors.push(vertex as usize);
                }
            }
            if already_duplicated || neighbors.len() != 2 {
                continue;
            }
            if neighbors.contains(&edge[0]) && neighbors.contains(&edge[1]) {
                for vertex in face {
                    if *vertex == src_vertex as i64 {
                        *vertex = dup_vertex as i64;
                        break;
                    }
                }
                item.src_vertex = dup_vertex;
                break;
            }
        }
    }
}

fn extract_closed_path(path: &mut Vec<usize>, closed_path: &mut Vec<usize>) {
    closed_path.clear();
    let Some(last_vertex) = path.last().copied() else {
        return;
    };
    for index in 0..path.len() {
        if path[index] == last_vertex {
            closed_path.extend_from_slice(&path[index..]);
            path.truncate(index);
            break;
        }
    }
}

fn validate_face_ids(faces_i64: &[[i64; 3]]) -> Result<Vec<[usize; 3]>, GeometryError> {
    faces_i64
        .iter()
        .enumerate()
        .map(|(face_index, face)| {
            let mut output = [0_usize; 3];
            for (corner_index, vertex) in face.iter().enumerate() {
                if *vertex < 0 {
                    return Err(GeometryError::NegativeFaceIndex {
                        face: face_index,
                        vertex: *vertex,
                    });
                }
                output[corner_index] = *vertex as usize;
            }
            Ok(output)
        })
        .collect()
}

fn max_face_vertex(faces: &[[usize; 3]]) -> Option<usize> {
    faces.iter().flat_map(|face| face.iter().copied()).max()
}

fn face_indices(face_count: usize, region_face_indices: Option<&[usize]>) -> Vec<usize> {
    match region_face_indices {
        Some(indices) => indices.to_vec(),
        None => (0..face_count).collect(),
    }
}

fn meshlib_incident_face_chains(
    vertex: usize,
    incident_faces: &[usize],
    faces: &[[usize; 3]],
) -> Vec<Vec<usize>> {
    let mut remaining = incident_faces.to_vec();
    remaining.sort_unstable();
    let mut components = Vec::new();

    while !remaining.is_empty() {
        let Some(first_neighbor) = first_incident_neighbor(faces[remaining[0]], vertex) else {
            remaining.remove(0);
            continue;
        };
        let mut path = vec![first_neighbor];
        let mut visited = HashMap::from([(first_neighbor, 0_usize)]);
        let mut chain_faces = Vec::new();
        let mut orientation = true;

        let Some((mut next_vertex, face_id)) =
            take_next_incident_face(vertex, first_neighbor, orientation, &mut remaining, faces)
                .or_else(|| {
                    orientation = false;
                    take_next_incident_face(
                        vertex,
                        first_neighbor,
                        orientation,
                        &mut remaining,
                        faces,
                    )
                })
        else {
            remaining.remove(0);
            continue;
        };
        chain_faces.push(face_id);
        path.push(next_vertex);
        visited.insert(next_vertex, path.len() - 1);

        loop {
            let current = if orientation {
                *path.last().unwrap_or(&first_neighbor)
            } else {
                path[0]
            };
            let mut step =
                take_next_incident_face(vertex, current, orientation, &mut remaining, faces);
            if step.is_none() && orientation {
                orientation = false;
                step = take_next_incident_face(
                    vertex,
                    first_neighbor,
                    orientation,
                    &mut remaining,
                    faces,
                );
                if step.is_some() {
                    path.reverse();
                    visited = path
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(index, neighbor)| (neighbor, index))
                        .collect();
                }
            }

            let Some((candidate, face_id)) = step else {
                components.push(chain_faces);
                break;
            };
            next_vertex = candidate;
            chain_faces.push(face_id);

            if let Some(repeated_at) = visited.get(&next_vertex).copied() {
                components.push(chain_faces[repeated_at..].to_vec());
                break;
            }
            path.push(next_vertex);
            visited.insert(next_vertex, path.len() - 1);
        }
    }
    components
}

fn take_next_incident_face(
    vertex: usize,
    current: usize,
    orientation: bool,
    remaining: &mut Vec<usize>,
    faces: &[[usize; 3]],
) -> Option<(usize, usize)> {
    let position = remaining.iter().position(|face_id| {
        next_incident_vertex(faces[*face_id], vertex, current, orientation).is_some()
    })?;
    let face_id = remaining.remove(position);
    let next = next_incident_vertex(faces[face_id], vertex, current, orientation)?;
    Some((next, face_id))
}

fn next_incident_vertex(
    face: [usize; 3],
    vertex: usize,
    current: usize,
    orientation: bool,
) -> Option<usize> {
    if orientation {
        if face[0] == vertex && face[1] == current {
            return Some(face[2]);
        }
        if face[1] == vertex && face[2] == current {
            return Some(face[0]);
        }
        if face[2] == vertex && face[0] == current {
            return Some(face[1]);
        }
    } else {
        if face[1] == vertex && face[0] == current {
            return Some(face[2]);
        }
        if face[2] == vertex && face[1] == current {
            return Some(face[0]);
        }
        if face[0] == vertex && face[2] == current {
            return Some(face[1]);
        }
    }
    None
}

fn next_incident_vertex_i64(
    face: [i64; 3],
    vertex: usize,
    current: usize,
    orientation: bool,
) -> Option<usize> {
    let face = [
        usize::try_from(face[0]).ok()?,
        usize::try_from(face[1]).ok()?,
        usize::try_from(face[2]).ok()?,
    ];
    next_incident_vertex(face, vertex, current, orientation)
}

fn first_incident_neighbor(face: [usize; 3], vertex: usize) -> Option<usize> {
    face.into_iter().find(|candidate| *candidate != vertex)
}

fn first_incident_neighbor_i64(face: [i64; 3], vertex: usize) -> Option<usize> {
    for candidate in face {
        let candidate = usize::try_from(candidate).ok()?;
        if candidate != vertex {
            return Some(candidate);
        }
    }
    None
}

fn unique_face_vertices(face: [usize; 3]) -> Vec<usize> {
    let mut vertices = vec![face[0], face[1], face[2]];
    vertices.sort_unstable();
    vertices.dedup();
    vertices
}
