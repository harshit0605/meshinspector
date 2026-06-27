use std::collections::{BTreeMap, BTreeSet};

use crate::math::{add, cross, dot, norm, scale, sub};

use super::sampling::{squared_distance, validate_distance_limit, validate_point_rows};

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudMeshProjectionResult {
    pub points: Vec<[f64; 3]>,
    pub squared_distances: Vec<f64>,
    pub face_indices: Vec<i64>,
    pub vertex_indices: Vec<i64>,
    pub normals: Vec<[f64; 3]>,
    pub boundary_flags: Vec<bool>,
}

pub fn point_cloud_project_to_mesh(
    query_points: &[[f64; 3]],
    mesh_vertices: &[[f64; 3]],
    mesh_faces: &[[i64; 3]],
    up_dist_limit_sq: f64,
    lo_dist_limit_sq: f64,
    point_transform: Option<[f64; 16]>,
    mesh_transform: Option<[f64; 16]>,
    face_mask: Option<&[bool]>,
) -> Result<PointCloudMeshProjectionResult, String> {
    validate_point_rows("query points", query_points, true)?;
    validate_point_rows("mesh vertices", mesh_vertices, true)?;
    validate_distance_limit("up_dist_limit_sq", up_dist_limit_sq)?;
    validate_distance_limit("lo_dist_limit_sq", lo_dist_limit_sq)?;
    let face_region = ProjectionFaceRegion::new(mesh_faces, face_mask)?;
    let projection_points = projection_space_points(query_points, point_transform, mesh_transform)?;
    let closest =
        crate::closest_points_on_mesh(&projection_points, mesh_vertices, &face_region.faces)
            .map_err(|error| error.to_string())?;
    let projection_topology = ProjectionMeshTopology::new(mesh_vertices, mesh_faces, face_mask)?;

    let mut points = Vec::with_capacity(query_points.len());
    let mut squared_distances = Vec::with_capacity(query_points.len());
    let mut face_indices = Vec::with_capacity(query_points.len());
    let mut vertex_indices = Vec::with_capacity(query_points.len());
    let mut normals = Vec::with_capacity(query_points.len());
    let mut boundary_flags = Vec::with_capacity(query_points.len());

    for (query_index, query) in projection_points.iter().enumerate() {
        let distance_sq = closest.distances[query_index] * closest.distances[query_index];
        let face_index = closest.face_indices[query_index];
        if face_index < 0 || distance_sq >= up_dist_limit_sq {
            points.push([0.0; 3]);
            squared_distances.push(up_dist_limit_sq);
            face_indices.push(-1);
            vertex_indices.push(-1);
            normals.push([0.0; 3]);
            boundary_flags.push(false);
            continue;
        }

        let point = closest.closest_points[query_index];
        let original_face_index = face_region.original_index(face_index as usize);
        let face = mesh_faces[original_face_index];
        points.push(point);
        squared_distances.push(squared_distance(*query, point));
        face_indices.push(original_face_index as i64);
        vertex_indices.push(closest_vertex(mesh_vertices, face, point));
        normals.push(projection_topology.pseudonormal(original_face_index, point));
        boundary_flags.push(projection_topology.point_on_boundary(face, point));
    }

    Ok(PointCloudMeshProjectionResult {
        points,
        squared_distances,
        face_indices,
        vertex_indices,
        normals,
        boundary_flags,
    })
}

struct ProjectionFaceRegion {
    faces: Vec<[i64; 3]>,
    original_face_indices: Vec<usize>,
}

impl ProjectionFaceRegion {
    fn new(faces: &[[i64; 3]], face_mask: Option<&[bool]>) -> Result<Self, String> {
        if let Some(mask) = face_mask {
            if mask.len() != faces.len() {
                return Err("face_mask length must match mesh_faces length".to_string());
            }
        }
        let mut region_faces = Vec::new();
        let mut original_face_indices = Vec::new();
        for (face_index, face) in faces.iter().enumerate() {
            if face_mask.map(|mask| mask[face_index]).unwrap_or(true) {
                region_faces.push(*face);
                original_face_indices.push(face_index);
            }
        }
        Ok(Self {
            faces: region_faces,
            original_face_indices,
        })
    }

    fn original_index(&self, region_face_index: usize) -> usize {
        self.original_face_indices[region_face_index]
    }
}

struct ProjectionMeshTopology<'a> {
    vertices: &'a [[f64; 3]],
    faces: &'a [[i64; 3]],
    face_normals: Vec<[f64; 3]>,
    edge_faces: BTreeMap<[i64; 2], Vec<usize>>,
    vertex_faces: BTreeMap<i64, Vec<usize>>,
    boundary_edges: BTreeSet<[i64; 2]>,
}

impl<'a> ProjectionMeshTopology<'a> {
    fn new(
        vertices: &'a [[f64; 3]],
        faces: &'a [[i64; 3]],
        face_mask: Option<&[bool]>,
    ) -> Result<Self, String> {
        if let Some(mask) = face_mask {
            if mask.len() != faces.len() {
                return Err("face_mask length must match mesh_faces length".to_string());
            }
        }
        let mut edge_faces = BTreeMap::<[i64; 2], Vec<usize>>::new();
        let mut vertex_faces = BTreeMap::<i64, Vec<usize>>::new();
        let mut face_normals = Vec::with_capacity(faces.len());

        for (face_index, face) in faces.iter().enumerate() {
            validate_face_indices(vertices, *face)?;
            face_normals.push(face_normal(vertices, *face));
            if !face_mask.map(|mask| mask[face_index]).unwrap_or(true) {
                continue;
            }
            for vertex_index in face {
                vertex_faces
                    .entry(*vertex_index)
                    .or_default()
                    .push(face_index);
            }
            for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
                edge_faces
                    .entry(sorted_edge(edge[0], edge[1]))
                    .or_default()
                    .push(face_index);
            }
        }

        let boundary_edges = edge_faces
            .iter()
            .filter_map(|(edge, face_indices)| (face_indices.len() == 1).then_some(*edge))
            .collect();

        Ok(Self {
            vertices,
            faces,
            face_normals,
            edge_faces,
            vertex_faces,
            boundary_edges,
        })
    }

    fn pseudonormal(&self, face_index: usize, point: [f64; 3]) -> [f64; 3] {
        let face = self.faces[face_index];
        let barycentric = barycentric_coordinates(self.vertices, face, point);
        if let Some(vertex_index) = barycentric_vertex(face, barycentric) {
            return self.vertex_pseudonormal(vertex_index, face_index);
        }
        if let Some(edge) = barycentric_edge(face, barycentric) {
            return self.edge_pseudonormal(edge, face_index);
        }
        self.face_normals[face_index]
    }

    fn point_on_boundary(&self, face: [i64; 3], point: [f64; 3]) -> bool {
        [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
            .into_iter()
            .any(|edge| {
                self.boundary_edges.contains(&sorted_edge(edge[0], edge[1]))
                    && point_segment_distance_sq(
                        point,
                        self.vertices[edge[0] as usize],
                        self.vertices[edge[1] as usize],
                    ) <= 1e-18
            })
    }

    fn vertex_pseudonormal(&self, vertex_index: i64, fallback_face_index: usize) -> [f64; 3] {
        let Some(face_indices) = self.vertex_faces.get(&vertex_index) else {
            return self.face_normals[fallback_face_index];
        };
        let mut sum = [0.0; 3];
        for face_index in face_indices {
            let face = self.faces[*face_index];
            let angle = face_vertex_angle(self.vertices, face, vertex_index);
            sum = add(sum, scale(self.face_normals[*face_index], angle));
        }
        normalize_or(sum, self.face_normals[fallback_face_index])
    }

    fn edge_pseudonormal(&self, edge: [i64; 2], fallback_face_index: usize) -> [f64; 3] {
        let Some(face_indices) = self.edge_faces.get(&sorted_edge(edge[0], edge[1])) else {
            return self.face_normals[fallback_face_index];
        };
        let sum = face_indices.iter().fold([0.0; 3], |acc, face_index| {
            add(acc, self.face_normals[*face_index])
        });
        normalize_or(sum, self.face_normals[fallback_face_index])
    }
}

fn validate_face_indices(vertices: &[[f64; 3]], face: [i64; 3]) -> Result<(), String> {
    for vertex_index in face {
        if vertex_index < 0 {
            return Err("mesh faces must reference non-negative vertex indices".to_string());
        }
        if vertex_index as usize >= vertices.len() {
            return Err("mesh faces must reference existing vertex indices".to_string());
        }
    }
    Ok(())
}

const BARYCENTRIC_EPS: f64 = 1e-12;

fn barycentric_coordinates(vertices: &[[f64; 3]], face: [i64; 3], point: [f64; 3]) -> [f64; 3] {
    let a = vertices[face[0] as usize];
    let b = vertices[face[1] as usize];
    let c = vertices[face[2] as usize];
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let v2 = sub(point, a);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denom = d00 * d11 - d01 * d01;
    if denom <= 0.0 {
        return [1.0 / 3.0; 3];
    }
    let inv_denom = 1.0 / denom;
    let b_weight = ((d11 * d20 - d01 * d21) * inv_denom).clamp(0.0, 1.0);
    let c_weight = ((d00 * d21 - d01 * d20) * inv_denom).clamp(0.0, 1.0 - b_weight);
    [1.0 - b_weight - c_weight, b_weight, c_weight]
}

fn barycentric_vertex(face: [i64; 3], barycentric: [f64; 3]) -> Option<i64> {
    for index in 0..3 {
        if barycentric[index] >= 1.0 - BARYCENTRIC_EPS
            && (0..3).all(|other| other == index || barycentric[other] <= BARYCENTRIC_EPS)
        {
            return Some(face[index]);
        }
    }
    None
}

fn barycentric_edge(face: [i64; 3], barycentric: [f64; 3]) -> Option<[i64; 2]> {
    if barycentric[0] <= BARYCENTRIC_EPS {
        return Some(sorted_edge(face[1], face[2]));
    }
    if barycentric[1] <= BARYCENTRIC_EPS {
        return Some(sorted_edge(face[2], face[0]));
    }
    if barycentric[2] <= BARYCENTRIC_EPS {
        return Some(sorted_edge(face[0], face[1]));
    }
    None
}

fn face_vertex_angle(vertices: &[[f64; 3]], face: [i64; 3], vertex_index: i64) -> f64 {
    let center = vertices[vertex_index as usize];
    let mut directions = Vec::with_capacity(2);
    for face_vertex in face {
        if face_vertex != vertex_index {
            directions.push(sub(vertices[face_vertex as usize], center));
        }
    }
    if directions.len() != 2 {
        return 0.0;
    }
    norm(cross(directions[0], directions[1])).atan2(dot(directions[0], directions[1]))
}

fn normalize_or(vector: [f64; 3], fallback: [f64; 3]) -> [f64; 3] {
    let length = norm(vector);
    if length == 0.0 {
        fallback
    } else {
        scale(vector, 1.0 / length)
    }
}

fn closest_vertex(vertices: &[[f64; 3]], face: [i64; 3], point: [f64; 3]) -> i64 {
    face.into_iter()
        .min_by(|left, right| {
            let left = *left as usize;
            let right = *right as usize;
            squared_distance(vertices[left], point)
                .total_cmp(&squared_distance(vertices[right], point))
                .then_with(|| left.cmp(&right))
        })
        .unwrap_or(-1)
}

fn face_normal(vertices: &[[f64; 3]], face: [i64; 3]) -> [f64; 3] {
    let a = vertices[face[0] as usize];
    let b = vertices[face[1] as usize];
    let c = vertices[face[2] as usize];
    let normal = cross(sub(b, a), sub(c, a));
    let normal_length = norm(normal);
    if normal_length == 0.0 {
        [0.0; 3]
    } else {
        scale(normal, 1.0 / normal_length)
    }
}

fn point_segment_distance_sq(point: [f64; 3], start: [f64; 3], end: [f64; 3]) -> f64 {
    let segment = sub(end, start);
    let length_sq = dot(segment, segment);
    if length_sq == 0.0 {
        return squared_distance(point, start);
    }
    let t = (dot(sub(point, start), segment) / length_sq).clamp(0.0, 1.0);
    squared_distance(
        point,
        [
            start[0] + segment[0] * t,
            start[1] + segment[1] * t,
            start[2] + segment[2] * t,
        ],
    )
}

fn projection_space_points(
    query_points: &[[f64; 3]],
    point_transform: Option<[f64; 16]>,
    mesh_transform: Option<[f64; 16]>,
) -> Result<Vec<[f64; 3]>, String> {
    if let Some(transform) = point_transform {
        validate_affine_transform("point_transform", transform)?;
    }
    let mesh_inverse = match mesh_transform {
        Some(transform) => Some(inverse_rigid_transform(transform)?),
        None => None,
    };
    Ok(query_points
        .iter()
        .map(|point| {
            let transformed = point_transform
                .map(|transform| transform_point(transform, *point))
                .unwrap_or(*point);
            mesh_inverse
                .map(|transform| transform_point(transform, transformed))
                .unwrap_or(transformed)
        })
        .collect())
}

fn inverse_rigid_transform(transform: [f64; 16]) -> Result<[f64; 16], String> {
    validate_affine_transform("mesh_transform", transform)?;
    let rows = [
        [transform[0], transform[1], transform[2]],
        [transform[4], transform[5], transform[6]],
        [transform[8], transform[9], transform[10]],
    ];
    for row in rows {
        if (dot(row, row) - 1.0).abs() > 1e-8 {
            return Err("mesh_transform must be rigid for point-cloud mesh projection".to_string());
        }
    }
    for left in 0..3 {
        for right in (left + 1)..3 {
            if dot(rows[left], rows[right]).abs() > 1e-8 {
                return Err(
                    "mesh_transform must be rigid for point-cloud mesh projection".to_string(),
                );
            }
        }
    }

    let translation = [transform[3], transform[7], transform[11]];
    let inv_linear = [
        [rows[0][0], rows[1][0], rows[2][0]],
        [rows[0][1], rows[1][1], rows[2][1]],
        [rows[0][2], rows[1][2], rows[2][2]],
    ];
    let inv_translation = [
        -dot(inv_linear[0], translation),
        -dot(inv_linear[1], translation),
        -dot(inv_linear[2], translation),
    ];
    Ok([
        inv_linear[0][0],
        inv_linear[0][1],
        inv_linear[0][2],
        inv_translation[0],
        inv_linear[1][0],
        inv_linear[1][1],
        inv_linear[1][2],
        inv_translation[1],
        inv_linear[2][0],
        inv_linear[2][1],
        inv_linear[2][2],
        inv_translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ])
}

fn validate_affine_transform(name: &str, transform: [f64; 16]) -> Result<(), String> {
    if !transform.iter().all(|value| value.is_finite()) {
        return Err(format!("{name} values must be finite"));
    }
    if transform[12].abs() > 1e-12
        || transform[13].abs() > 1e-12
        || transform[14].abs() > 1e-12
        || (transform[15] - 1.0).abs() > 1e-12
    {
        return Err(format!("{name} must be a row-major affine 4x4 transform"));
    }
    Ok(())
}

fn transform_point(transform: [f64; 16], point: [f64; 3]) -> [f64; 3] {
    [
        transform[0] * point[0] + transform[1] * point[1] + transform[2] * point[2] + transform[3],
        transform[4] * point[0] + transform[5] * point[1] + transform[6] * point[2] + transform[7],
        transform[8] * point[0]
            + transform[9] * point[1]
            + transform[10] * point[2]
            + transform[11],
    ]
}

fn sorted_edge(left: i64, right: i64) -> [i64; 2] {
    if left <= right {
        [left, right]
    } else {
        [right, left]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_triangle() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
        (
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2]],
        )
    }

    fn trihedral_corner() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
        (
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            vec![[0, 1, 2], [0, 3, 1], [0, 2, 3]],
        )
    }

    fn stacked_triangles() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
        (
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 2.0],
                [1.0, 0.0, 2.0],
                [0.0, 1.0, 2.0],
            ],
            vec![[0, 1, 2], [3, 4, 5]],
        )
    }

    #[test]
    fn point_cloud_project_to_mesh_matches_meshlib_projection_payload_shape() {
        let (vertices, faces) = single_triangle();
        let result = point_cloud_project_to_mesh(
            &[[0.25, 0.25, 1.0]],
            &vertices,
            &faces,
            f64::MAX,
            0.0,
            None,
            None,
            None,
        )
        .expect("projection should succeed");

        assert_eq!(result.points, vec![[0.25, 0.25, 0.0]]);
        assert_eq!(result.squared_distances, vec![1.0]);
        assert_eq!(result.face_indices, vec![0]);
        assert_eq!(result.vertex_indices, vec![0]);
        assert_eq!(result.normals, vec![[0.0, 0.0, 1.0]]);
        assert_eq!(result.boundary_flags, vec![false]);
    }

    #[test]
    fn point_cloud_project_to_mesh_uses_meshlib_style_strict_upper_limit() {
        let (vertices, faces) = single_triangle();
        let result = point_cloud_project_to_mesh(
            &[[0.25, 0.25, 1.0]],
            &vertices,
            &faces,
            1.0,
            0.0,
            None,
            None,
            None,
        )
        .expect("projection should succeed");

        assert_eq!(result.points, vec![[0.0; 3]]);
        assert_eq!(result.squared_distances, vec![1.0]);
        assert_eq!(result.face_indices, vec![-1]);
        assert_eq!(result.vertex_indices, vec![-1]);
    }

    #[test]
    fn point_cloud_project_to_mesh_marks_boundary_edge_hits_like_mesh_or_points() {
        let (vertices, faces) = single_triangle();
        let result = point_cloud_project_to_mesh(
            &[[0.5, 0.0, 1.0]],
            &vertices,
            &faces,
            f64::MAX,
            0.0,
            None,
            None,
            None,
        )
        .expect("projection should succeed");

        assert_eq!(result.points, vec![[0.5, 0.0, 0.0]]);
        assert_eq!(result.boundary_flags, vec![true]);
    }

    #[test]
    fn point_cloud_project_to_mesh_returns_meshlib_edge_pseudonormal() {
        let (vertices, faces) = trihedral_corner();
        let result = point_cloud_project_to_mesh(
            &[[0.5, -0.2, -0.2]],
            &vertices,
            &faces,
            f64::MAX,
            0.0,
            None,
            None,
            None,
        )
        .expect("projection should succeed");

        assert_eq!(result.points, vec![[0.5, 0.0, 0.0]]);
        let inv_sqrt_2 = 1.0 / 2.0_f64.sqrt();
        assert_eq!(result.normals, vec![[0.0, inv_sqrt_2, inv_sqrt_2]]);
    }

    #[test]
    fn point_cloud_project_to_mesh_returns_meshlib_vertex_pseudonormal() {
        let (vertices, faces) = trihedral_corner();
        let result = point_cloud_project_to_mesh(
            &[[-0.2, -0.2, -0.2]],
            &vertices,
            &faces,
            f64::MAX,
            0.0,
            None,
            None,
            None,
        )
        .expect("projection should succeed");

        assert_eq!(result.points, vec![[0.0, 0.0, 0.0]]);
        let inv_sqrt_3 = 1.0 / 3.0_f64.sqrt();
        assert_eq!(result.normals, vec![[inv_sqrt_3, inv_sqrt_3, inv_sqrt_3]]);
    }

    #[test]
    fn point_cloud_project_to_mesh_uses_meshlib_style_face_region_mask() {
        let (vertices, faces) = stacked_triangles();
        let face_mask = [false, true];

        let result = point_cloud_project_to_mesh(
            &[[0.25, 0.25, 0.1]],
            &vertices,
            &faces,
            f64::MAX,
            0.0,
            None,
            None,
            Some(&face_mask),
        )
        .expect("projection should succeed");

        assert_eq!(result.points, vec![[0.25, 0.25, 2.0]]);
        assert_eq!(result.face_indices, vec![1]);
        assert_eq!(result.vertex_indices, vec![3]);
        assert_eq!(result.squared_distances, vec![3.61]);
    }

    #[test]
    fn point_cloud_project_to_mesh_applies_meshlib_style_rigid_reference_transform() {
        let (vertices, faces) = single_triangle();
        let mesh_transform = [
            1.0, 0.0, 0.0, 10.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];

        let result = point_cloud_project_to_mesh(
            &[[10.25, 0.25, 1.0]],
            &vertices,
            &faces,
            f64::MAX,
            0.0,
            None,
            Some(mesh_transform),
            None,
        )
        .expect("projection should succeed");

        assert_eq!(result.points, vec![[0.25, 0.25, 0.0]]);
        assert_eq!(result.squared_distances, vec![1.0]);
        assert_eq!(result.face_indices, vec![0]);
    }

    #[test]
    fn point_cloud_project_to_mesh_rejects_non_rigid_mesh_transform() {
        let (vertices, faces) = single_triangle();
        let non_rigid_transform = [
            2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];

        let error = point_cloud_project_to_mesh(
            &[[0.25, 0.25, 1.0]],
            &vertices,
            &faces,
            f64::MAX,
            0.0,
            None,
            Some(non_rigid_transform),
            None,
        )
        .expect_err("non-rigid mesh transform should be rejected");

        assert!(error.contains("must be rigid"));
    }
}
