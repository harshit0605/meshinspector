use crate::mesh::{mesh_health, mesh_stats};
use crate::{GeometryError, MeshHealth, MeshStats};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PackedMeshExport {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
}

pub(super) fn mesh_export_stats(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    failed_faces: usize,
) -> Result<Option<MeshStats>, GeometryError> {
    if failed_faces > 0 {
        return Ok(None);
    }
    mesh_stats(vertices, faces).map(Some)
}

pub(super) fn mesh_export_health(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    failed_faces: usize,
    epsilon: f64,
    self_intersection_budget: usize,
) -> Result<Option<MeshHealth>, GeometryError> {
    if failed_faces > 0 {
        return Ok(None);
    }
    mesh_health(
        vertices,
        faces,
        true,
        Some(self_intersection_budget),
        epsilon,
    )
    .map(Some)
}

pub(super) fn packed_mesh_export(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    failed_faces: usize,
) -> Result<Option<PackedMeshExport>, GeometryError> {
    if failed_faces > 0 {
        return Ok(None);
    }
    let mut vertex_map = BTreeMap::<usize, usize>::new();
    let mut packed_vertices = Vec::new();
    let mut packed_faces = Vec::with_capacity(faces.len());
    for (face_index, face) in faces.iter().enumerate() {
        let mut packed_face = [0_i64; 3];
        for (corner, vertex) in face.iter().copied().enumerate() {
            if vertex < 0 {
                return Err(GeometryError::NegativeFaceIndex {
                    face: face_index,
                    vertex,
                });
            }
            let source_vertex = vertex as usize;
            if source_vertex >= vertices.len() {
                return Err(GeometryError::FaceIndexOutOfBounds {
                    face: face_index,
                    vertex,
                    vertex_count: vertices.len(),
                });
            }
            let packed_vertex = match vertex_map.get(&source_vertex).copied() {
                Some(mapped) => mapped,
                None => {
                    let mapped = packed_vertices.len();
                    vertex_map.insert(source_vertex, mapped);
                    packed_vertices.push(vertices[source_vertex]);
                    mapped
                }
            };
            packed_face[corner] = packed_vertex as i64;
        }
        packed_faces.push(packed_face);
    }
    Ok(Some(PackedMeshExport {
        vertices: packed_vertices,
        faces: packed_faces,
    }))
}
