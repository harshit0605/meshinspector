use crate::grid::grid_value_count;
use crate::mesh::validate_faces;
use crate::GeometryError;
use rayon::prelude::*;

use super::bvh::build_flat_bvh;
use super::closest::closest_point_with_bvh;

pub fn unsigned_sdf_grid_values(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
) -> Result<Vec<f32>, GeometryError> {
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
            closest_point_with_bvh(point, &bvh, &triangles)
                .distance_sq
                .sqrt() as f32
        })
        .collect();

    Ok(output)
}
