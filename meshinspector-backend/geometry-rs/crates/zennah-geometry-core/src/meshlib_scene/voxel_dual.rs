use crate::types::{GeometryError, MeshArrays, VoxelDualMeshSettings};
use crate::voxel::voxel_to_mesh_dual_values_with_settings;

pub fn meshlib_vdb_payload_to_dual_mesh(
    model_bytes: &[u8],
    dimensions: [usize; 3],
    voxel_size: [f32; 3],
    iso_value: f32,
) -> Result<MeshArrays, GeometryError> {
    meshlib_vdb_payload_to_dual_mesh_with_settings(
        model_bytes,
        dimensions,
        voxel_size,
        VoxelDualMeshSettings {
            iso_value,
            level_set: true,
            ..VoxelDualMeshSettings::default()
        },
    )
}

pub fn meshlib_vdb_payload_to_dual_mesh_with_settings(
    model_bytes: &[u8],
    dimensions: [usize; 3],
    voxel_size: [f32; 3],
    settings: VoxelDualMeshSettings,
) -> Result<MeshArrays, GeometryError> {
    let mut parsed =
        super::voxel_vdb::parse_meshlib_vdb_voxel_model(model_bytes, dimensions, voxel_size)
            .map_err(|reason| GeometryError::InvalidVdbPayload { reason })?;
    if parsed.values.is_empty() {
        return Err(GeometryError::EmptyVoxelValues);
    }
    super::voxel_vdb::pad_meshlib_vdb_voxels_for_meshing(&mut parsed)
        .map_err(|reason| GeometryError::InvalidVdbPayload { reason })?;
    let mut mesh = voxel_to_mesh_dual_values_with_settings(
        &parsed.values,
        parsed.dimensions,
        [
            f64::from(parsed.voxel_size[0]),
            f64::from(parsed.voxel_size[1]),
            f64::from(parsed.voxel_size[2]),
        ],
        VoxelDualMeshSettings {
            level_set: parsed.grid_level_set,
            ..settings
        },
    )?;
    apply_voxel_grid_origin(&mut mesh, parsed.origin, parsed.voxel_size);
    Ok(mesh)
}

fn apply_voxel_grid_origin(mesh: &mut MeshArrays, origin: [i32; 3], voxel_size: [f32; 3]) {
    if origin == [0, 0, 0] {
        return;
    }
    let shift = [
        f64::from(origin[0]) * f64::from(voxel_size[0]),
        f64::from(origin[1]) * f64::from(voxel_size[1]),
        f64::from(origin[2]) * f64::from(voxel_size[2]),
    ];
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] += shift[axis];
        }
    }
}
