use crate::{
    mesh_bounds, voxel_shell_mesh, GeometryError, MeshArrays, VoxelMeshExtractor, VoxelMeshOptions,
};

pub fn service_hollow_mesh(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    wall_thickness_mm: f64,
) -> Result<MeshArrays, GeometryError> {
    let voxel_size = service_hollow_voxel_size(vertices, wall_thickness_mm)?;
    voxel_shell_mesh(
        vertices,
        faces,
        wall_thickness_mm,
        VoxelMeshOptions {
            voxel_size,
            padding_mm: None,
            extractor: VoxelMeshExtractor::Marching,
            refine: false,
        },
    )
}

pub fn service_hollow_voxel_size(
    vertices: &[[f64; 3]],
    wall_thickness_mm: f64,
) -> Result<f64, GeometryError> {
    if !wall_thickness_mm.is_finite() || wall_thickness_mm <= 0.0 {
        return Err(GeometryError::InvalidWallThickness { wall_thickness_mm });
    }
    let (bbox_min, bbox_max) = mesh_bounds(vertices);
    let diagonal_sq: f64 = (0..3)
        .map(|axis| {
            let delta = bbox_max[axis] - bbox_min[axis];
            delta * delta
        })
        .sum();
    Ok((diagonal_sq.sqrt() * 5e-3).max(wall_thickness_mm / 4.0))
}
