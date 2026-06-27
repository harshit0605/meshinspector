use crate::grid::grid_value_count;
use crate::math::{dot, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use rayon::prelude::*;

use super::bvh::build_flat_bvh;
use super::closest::closest_point_with_bvh;

pub fn weighted_sdf_grid_values(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    vertex_weights: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    winding_threshold: f64,
) -> Result<Vec<f32>, GeometryError> {
    if vertex_weights.len() != vertices.len() {
        return Err(GeometryError::WeightCountDoesNotMatchVertices {
            weights: vertex_weights.len(),
            vertices: vertices.len(),
        });
    }
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    let total = grid_value_count(shape)?;
    if faces.is_empty() {
        return Ok(vec![f32::INFINITY; total]);
    }

    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let winding_eval = super::fast_winding::WindingEvaluator::new(&triangles, &bvh);
    let yz_plane = shape[1] * shape[2];
    let output = (0..total)
        .into_par_iter()
        .map(|index| {
            let ix = index / yz_plane;
            let remainder = index % yz_plane;
            let iy = remainder / shape[2];
            let iz = remainder % shape[2];
            let point = [
                origin[0] + ix as f64 * voxel_size,
                origin[1] + iy as f64 * voxel_size,
                origin[2] + iz as f64 * voxel_size,
            ];
            let closest = closest_point_with_bvh(point, &bvh, &triangles);
            if closest.face_index < 0 {
                return f32::INFINITY;
            }
            let face = faces[closest.face_index as usize];
            let barycentric =
                barycentric_coordinates(closest.point, triangles[closest.face_index as usize]);
            let interpolated_weight = barycentric[0] * vertex_weights[face[0]] as f64
                + barycentric[1] * vertex_weights[face[1]] as f64
                + barycentric[2] * vertex_weights[face[2]] as f64;
            let winding = winding_eval.winding_at(point);
            let sign = if winding.abs() >= winding_threshold {
                -1.0
            } else {
                1.0
            };
            (closest.distance_sq.sqrt() * sign - interpolated_weight) as f32
        })
        .collect();

    Ok(output)
}

fn barycentric_coordinates(point: [f64; 3], triangle: [[f64; 3]; 3]) -> [f64; 3] {
    let [a, b, c] = triangle;
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let v2 = sub(point, a);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-12 {
        return [1.0, 0.0, 0.0];
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    [1.0 - v - w, v, w]
}
