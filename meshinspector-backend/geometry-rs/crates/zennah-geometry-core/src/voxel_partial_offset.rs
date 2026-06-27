use crate::hollow::{region_index_by_id, validate_region_offsets, validate_region_vertex_index};
use crate::{
    extract_grid_mesh, finalized_marching_tetrahedra, mesh_bounds, sdf_offset_values,
    unsigned_sdf_grid_values, voxel_boolean_mesh, GeometryError, GridMeshExtractionOptions,
    MeshArrays, SdfBooleanOperation, VoxelBooleanMeshOptions, VoxelMeshExtractor, VoxelMeshOptions,
};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn voxel_partial_offset_mesh(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    selected_region_ids: &[String],
    offset_mm: f64,
    options: VoxelMeshOptions,
) -> Result<MeshArrays, GeometryError> {
    if !offset_mm.is_finite() || offset_mm <= 0.0 {
        return Err(GeometryError::InvalidSdfOffset { offset_mm });
    }
    if selected_region_ids.is_empty() {
        return Err(GeometryError::UnknownRegionIds { ids: Vec::new() });
    }
    let selected_part = selected_region_mesh_part(
        vertices,
        faces,
        region_ids,
        vertex_offsets,
        vertex_indices,
        selected_region_ids,
    )?;
    let voxel_size = options.voxel_size;
    let padding = options
        .padding_mm
        .unwrap_or_else(|| (offset_mm.abs() + voxel_size).max(voxel_size));
    let offset_part = offset_mesh_part_unsigned(
        &selected_part.vertices,
        &selected_part.faces,
        offset_mm,
        voxel_size,
        padding,
        options,
    )?;
    voxel_boolean_mesh(
        vertices,
        faces,
        &offset_part.vertices,
        &offset_part.faces,
        SdfBooleanOperation::Union,
        VoxelBooleanMeshOptions {
            voxel_size,
            padding_mm: Some(padding),
            origin_phase: crate::default_boolean_origin_phase(),
            extractor: options.extractor,
            refine: options.refine,
        },
    )
}

fn selected_region_mesh_part(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    selected_region_ids: &[String],
) -> Result<MeshArrays, GeometryError> {
    let ranges = validate_region_offsets(vertex_offsets, vertex_indices.len(), region_ids.len())?;
    let region_by_id = region_index_by_id(region_ids);
    let mut selected_vertices = vec![false; vertices.len()];
    for region_id in selected_region_ids {
        let Some(region_index) = region_by_id.get(region_id) else {
            return Err(GeometryError::UnknownRegionIds {
                ids: vec![region_id.clone()],
            });
        };
        for index in &vertex_indices[ranges[*region_index].clone()] {
            let vertex_index = validate_region_vertex_index(*index, vertices.len())?;
            selected_vertices[vertex_index] = true;
        }
    }

    let mut remap = HashMap::<usize, i64>::new();
    let mut part_vertices = Vec::<[f64; 3]>::new();
    let mut part_faces = Vec::<[i64; 3]>::new();
    for face in faces {
        let indices = [
            validate_region_vertex_index(face[0], vertices.len())?,
            validate_region_vertex_index(face[1], vertices.len())?,
            validate_region_vertex_index(face[2], vertices.len())?,
        ];
        if !indices.iter().all(|index| selected_vertices[*index]) {
            continue;
        }
        let mut mapped = [0_i64; 3];
        for (slot, index) in indices.iter().enumerate() {
            let mapped_index = if let Some(existing) = remap.get(index) {
                *existing
            } else {
                let next_index = part_vertices.len() as i64;
                part_vertices.push(vertices[*index]);
                remap.insert(*index, next_index);
                next_index
            };
            mapped[slot] = mapped_index;
        }
        part_faces.push(mapped);
    }

    if part_faces.is_empty() {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "selected_region_faces",
            value: 0.0,
        });
    }
    Ok(MeshArrays {
        vertices: part_vertices,
        faces: part_faces,
    })
}

fn offset_mesh_part_unsigned(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    offset_mm: f64,
    voxel_size: f64,
    padding: f64,
    options: VoxelMeshOptions,
) -> Result<MeshArrays, GeometryError> {
    let (bbox_min, bbox_max) = mesh_bounds(vertices);
    let (origin, shape) = grid_origin_and_shape(bbox_min, bbox_max, voxel_size, padding)?;
    let values = unsigned_sdf_grid_values(vertices, faces, origin, shape, voxel_size)?;
    let offset_values = sdf_offset_values(&values, offset_mm)?;
    if options.extractor == VoxelMeshExtractor::Marching && !options.refine {
        return finalized_marching_tetrahedra(&offset_values, origin, shape, voxel_size, 0.0);
    }
    extract_grid_mesh(
        &offset_values,
        origin,
        shape,
        mesh_extraction_options(options),
    )
}

fn grid_origin_and_shape(
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
    voxel_size: f64,
    padding: f64,
) -> Result<([f64; 3], [usize; 3]), GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    let mut origin = [0.0_f64; 3];
    let mut shape = [0_usize; 3];
    for axis in 0..3 {
        origin[axis] = bbox_min[axis] - padding;
        let maximum = bbox_max[axis] + padding;
        let dimension = ((maximum - origin[axis]) / voxel_size).ceil() as usize + 1;
        shape[axis] = dimension.max(2);
    }
    Ok((origin, shape))
}

fn mesh_extraction_options(options: VoxelMeshOptions) -> GridMeshExtractionOptions {
    GridMeshExtractionOptions {
        voxel_size: options.voxel_size,
        extractor: options.extractor,
        refine: options.refine,
        smooth_iterations: 1,
        smooth_strength: 0.2,
        projection_iterations: 3,
    }
}
