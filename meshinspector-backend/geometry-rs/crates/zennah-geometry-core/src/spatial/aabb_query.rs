use super::bvh;
use super::bvh::{build_flat_bvh, overlapping_face_pairs, FlatBvh};
use crate::mesh::validate_faces;
use crate::GeometryError;

#[derive(Debug, Clone)]
pub struct AabbQueryTree {
    bvh: FlatBvh,
    face_count: usize,
    leaf_size: usize,
}

impl AabbQueryTree {
    pub fn build(
        vertices: &[[f64; 3]],
        faces_i64: &[[i64; 3]],
        leaf_size: usize,
    ) -> Result<Self, GeometryError> {
        let faces = validate_faces(faces_i64, vertices.len())?;
        let clamped_leaf_size = leaf_size.max(1);
        if faces.is_empty() {
            return Ok(Self {
                bvh: FlatBvh {
                    nodes: Vec::new(),
                    face_indices: Vec::new(),
                },
                face_count: 0,
                leaf_size: clamped_leaf_size,
            });
        }
        let triangles: Vec<[[f64; 3]; 3]> = faces
            .iter()
            .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
            .collect();
        Ok(Self {
            bvh: build_flat_bvh(&triangles, clamped_leaf_size),
            face_count: faces.len(),
            leaf_size: clamped_leaf_size,
        })
    }

    pub fn face_count(&self) -> usize {
        self.face_count
    }

    pub fn leaf_size(&self) -> usize {
        self.leaf_size
    }

    pub fn ray_candidate_faces(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
    ) -> Vec<usize> {
        bvh::ray_candidate_faces(&self.bvh, origin, direction, max_distance)
    }

    pub fn overlapping_face_pairs(&self, epsilon: f64) -> Vec<(usize, usize)> {
        overlapping_face_pairs(&self.bvh, epsilon)
    }

    pub fn closest_candidate_faces(&self, point: [f64; 3], current_best_sq: f64) -> Vec<usize> {
        bvh::closest_candidate_faces(&self.bvh, point, current_best_sq)
    }
}

pub fn point_aabb_distance_sq(point: [f64; 3], bbox_min: [f64; 3], bbox_max: [f64; 3]) -> f64 {
    bvh::point_aabb_distance_sq(point, bbox_min, bbox_max)
}

pub fn ray_intersects_aabb(
    origin: [f64; 3],
    direction: [f64; 3],
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
    max_distance: f64,
) -> bool {
    bvh::ray_intersects_aabb(origin, direction, bbox_min, bbox_max, max_distance)
}

pub fn aabb_ray_candidate_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    origin: [f64; 3],
    direction: [f64; 3],
    leaf_size: usize,
    max_distance: f64,
) -> Result<Vec<usize>, GeometryError> {
    Ok(
        AabbQueryTree::build(vertices, faces_i64, leaf_size)?.ray_candidate_faces(
            origin,
            direction,
            max_distance,
        ),
    )
}

pub fn aabb_overlapping_face_pairs(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<Vec<(usize, usize)>, GeometryError> {
    Ok(AabbQueryTree::build(vertices, faces_i64, leaf_size)?.overlapping_face_pairs(epsilon))
}

pub fn aabb_closest_candidate_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    point: [f64; 3],
    current_best_sq: f64,
    leaf_size: usize,
) -> Result<Vec<usize>, GeometryError> {
    Ok(AabbQueryTree::build(vertices, faces_i64, leaf_size)?
        .closest_candidate_faces(point, current_best_sq))
}
