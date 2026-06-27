use crate::math::{add, cross, dot, norm, scale, sub};
use crate::mesh::{safe_normalize_vector, validate_faces};
use crate::{GeometryError, MeshArrays};

pub fn offset_verts_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    offsets: &[f32],
) -> Result<MeshArrays, GeometryError> {
    if offsets.len() != vertices.len() {
        return Err(GeometryError::WeightCountDoesNotMatchVertices {
            weights: offsets.len(),
            vertices: vertices.len(),
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    let normals = meshlib_pseudonormals(vertices, &faces);
    let shifted_vertices = vertices
        .iter()
        .zip(normals)
        .zip(offsets)
        .map(|((vertex, normal), offset)| add(*vertex, scale(normal, f64::from(*offset))))
        .collect();

    Ok(MeshArrays {
        vertices: shifted_vertices,
        faces: faces_i64.to_vec(),
    })
}

fn meshlib_pseudonormals(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> Vec<[f64; 3]> {
    let mut normals = vec![[0.0; 3]; vertices.len()];
    for face in faces {
        let a = vertices[face[0]];
        let b = vertices[face[1]];
        let c = vertices[face[2]];
        let face_normal = safe_normalize_vector(cross(sub(b, a), sub(c, a)));
        if face_normal == [0.0; 3] {
            continue;
        }

        let corners = [(face[0], a, b, c), (face[1], b, c, a), (face[2], c, a, b)];
        for (vertex_id, vertex, next, previous) in corners {
            let angle = corner_angle(sub(next, vertex), sub(previous, vertex));
            normals[vertex_id] = add(normals[vertex_id], scale(face_normal, angle));
        }
    }
    normals.into_iter().map(safe_normalize_vector).collect()
}

fn corner_angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    let magnitude = norm(a) * norm(b);
    if magnitude <= 1e-12 {
        return 0.0;
    }
    (dot(a, b) / magnitude).clamp(-1.0, 1.0).acos()
}
