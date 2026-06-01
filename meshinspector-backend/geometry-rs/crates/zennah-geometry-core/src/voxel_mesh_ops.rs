use crate::{
    extract_surface_mesh_from_sdf_cells, finalized_marching_tetrahedra,
    finalized_sdf_boolean_marching_tetrahedra, finalized_sdf_offset_marching_tetrahedra,
    finalized_sdf_shell_marching_tetrahedra, mesh_bounds, mesh_health, refine_vertices_with_sdf,
    sdf_boolean_values, sdf_grid_values, sdf_offset_values, sdf_shell_values, GeometryError,
    GridMeshExtractionOptions, MeshArrays, SdfBooleanOperation, VoxelBooleanMeshOptions,
    VoxelMeshExtractor, VoxelMeshOptions, VoxelRebuildReport, VoxelRebuildResult,
};

const DEFAULT_BOOLEAN_ORIGIN_PHASE: [f64; 3] = [0.125, 0.125, 0.125];

struct SdfField {
    values: Vec<f32>,
    origin: [f64; 3],
    shape: [usize; 3],
}

pub fn extract_grid_mesh(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    options: GridMeshExtractionOptions,
) -> Result<MeshArrays, GeometryError> {
    let mesh = match options.extractor {
        VoxelMeshExtractor::Marching => {
            finalized_marching_tetrahedra(values, origin, shape, options.voxel_size, 0.0)?
        }
        VoxelMeshExtractor::Cells => {
            extract_surface_mesh_from_sdf_cells(values, origin, shape, options.voxel_size, 0.0)?
        }
    };
    maybe_refine_grid_mesh(mesh, values, origin, shape, options)
}

pub fn voxel_offset_mesh(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    offset_mm: f64,
    options: VoxelMeshOptions,
) -> Result<MeshArrays, GeometryError> {
    let voxel_size = options.voxel_size;
    let padding = options
        .padding_mm
        .unwrap_or_else(|| (offset_mm.abs() + voxel_size).max(voxel_size));
    let field = sample_sdf_for_mesh(vertices, faces, voxel_size, padding, [0.0; 3])?;
    if options.extractor == VoxelMeshExtractor::Marching && !options.refine {
        return finalized_sdf_offset_marching_tetrahedra(
            &field.values,
            field.origin,
            field.shape,
            voxel_size,
            offset_mm,
            0.0,
        );
    }
    let offset_values = sdf_offset_values(&field.values, offset_mm)?;
    extract_grid_mesh(
        &offset_values,
        field.origin,
        field.shape,
        mesh_extraction_options(options),
    )
}

pub fn voxel_shell_mesh(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    wall_thickness_mm: f64,
    options: VoxelMeshOptions,
) -> Result<MeshArrays, GeometryError> {
    let voxel_size = options.voxel_size;
    let padding = options
        .padding_mm
        .unwrap_or_else(|| (wall_thickness_mm + voxel_size).max(voxel_size));
    let field = sample_sdf_for_mesh(vertices, faces, voxel_size, padding, [0.0; 3])?;
    if options.extractor == VoxelMeshExtractor::Marching && !options.refine {
        return finalized_sdf_shell_marching_tetrahedra(
            &field.values,
            field.origin,
            field.shape,
            voxel_size,
            wall_thickness_mm,
            0.0,
        );
    }
    let shell_values = sdf_shell_values(&field.values, wall_thickness_mm)?;
    extract_grid_mesh(
        &shell_values,
        field.origin,
        field.shape,
        mesh_extraction_options(options),
    )
}

pub fn voxel_boolean_mesh(
    left_vertices: &[[f64; 3]],
    left_faces: &[[i64; 3]],
    right_vertices: &[[f64; 3]],
    right_faces: &[[i64; 3]],
    operation: SdfBooleanOperation,
    options: VoxelBooleanMeshOptions,
) -> Result<MeshArrays, GeometryError> {
    let voxel_size = options.voxel_size;
    let padding = options.padding_mm.unwrap_or(voxel_size);
    let (bbox_min, bbox_max) = combined_bounds(left_vertices, right_vertices);
    let (origin, shape) = grid_origin_and_shape(
        bbox_min,
        bbox_max,
        voxel_size,
        padding,
        options.origin_phase,
    )?;
    let left_values = sdf_grid_values(left_vertices, left_faces, origin, shape, voxel_size, 0.5)?;
    let right_values =
        sdf_grid_values(right_vertices, right_faces, origin, shape, voxel_size, 0.5)?;
    if options.extractor == VoxelMeshExtractor::Marching && !options.refine {
        return finalized_sdf_boolean_marching_tetrahedra(
            &left_values,
            &right_values,
            operation,
            origin,
            shape,
            voxel_size,
            0.0,
        );
    }

    let values = sdf_boolean_values(&left_values, &right_values, operation)?;
    extract_grid_mesh(&values, origin, shape, boolean_extraction_options(options))
}

pub fn global_thicken_mesh(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    min_target_thickness_mm: f64,
) -> Result<MeshArrays, GeometryError> {
    if !min_target_thickness_mm.is_finite() || min_target_thickness_mm <= 0.0 {
        return Err(GeometryError::InvalidWallThickness {
            wall_thickness_mm: min_target_thickness_mm,
        });
    }
    let offset_mm = min_target_thickness_mm / 2.0;
    let voxel_size = service_global_thicken_voxel_size(vertices, min_target_thickness_mm);
    voxel_offset_mesh(
        vertices,
        faces,
        offset_mm,
        VoxelMeshOptions {
            voxel_size,
            padding_mm: None,
            extractor: VoxelMeshExtractor::Marching,
            refine: false,
        },
    )
}

pub fn voxel_rebuild_via_sdf(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    offset_mm: f64,
    options: VoxelMeshOptions,
) -> Result<VoxelRebuildResult, GeometryError> {
    let before = mesh_health(vertices, faces, true, Some(50_000), 1e-8)?;
    let rebuilt = voxel_offset_mesh(vertices, faces, offset_mm, options)?;
    let after = mesh_health(&rebuilt.vertices, &rebuilt.faces, true, Some(50_000), 1e-8)?;
    let output_vertex_count = rebuilt.vertices.len();
    let output_face_count = rebuilt.faces.len();
    Ok(VoxelRebuildResult {
        vertices: rebuilt.vertices,
        faces: rebuilt.faces,
        report: VoxelRebuildReport {
            input_vertex_count: vertices.len(),
            input_face_count: faces.len(),
            output_vertex_count,
            output_face_count,
            input_boundary_edge_count: before.boundary_edge_count,
            output_boundary_edge_count: after.boundary_edge_count,
            input_nonmanifold_edge_count: before.nonmanifold_edge_count,
            output_nonmanifold_edge_count: after.nonmanifold_edge_count,
            input_self_intersections: before.self_intersections,
            output_self_intersections: after.self_intersections,
            voxel_size_mm: options.voxel_size,
            offset_mm,
            extractor: extractor_name(options.extractor).to_string(),
            refine: options.refine,
        },
    })
}

pub fn default_boolean_origin_phase() -> [f64; 3] {
    DEFAULT_BOOLEAN_ORIGIN_PHASE
}

fn sample_sdf_for_mesh(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    voxel_size: f64,
    padding: f64,
    origin_phase: [f64; 3],
) -> Result<SdfField, GeometryError> {
    let (bbox_min, bbox_max) = mesh_bounds(vertices);
    let (origin, shape) =
        grid_origin_and_shape(bbox_min, bbox_max, voxel_size, padding, origin_phase)?;
    let values = sdf_grid_values(vertices, faces, origin, shape, voxel_size, 0.5)?;
    Ok(SdfField {
        values,
        origin,
        shape,
    })
}

fn grid_origin_and_shape(
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
    voxel_size: f64,
    padding: f64,
    origin_phase: [f64; 3],
) -> Result<([f64; 3], [usize; 3]), GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    let mut origin = [0.0_f64; 3];
    let mut shape = [0_usize; 3];
    for axis in 0..3 {
        origin[axis] = bbox_min[axis] - padding + origin_phase[axis] * voxel_size;
        let maximum = bbox_max[axis] + padding + origin_phase[axis] * voxel_size;
        let dimension = ((maximum - origin[axis]) / voxel_size).ceil() as usize + 1;
        shape[axis] = dimension.max(2);
    }
    Ok((origin, shape))
}

fn service_global_thicken_voxel_size(vertices: &[[f64; 3]], min_target_thickness_mm: f64) -> f64 {
    let (bbox_min, bbox_max) = mesh_bounds(vertices);
    let diagonal_sq: f64 = (0..3)
        .map(|axis| {
            let delta = bbox_max[axis] - bbox_min[axis];
            delta * delta
        })
        .sum();
    (diagonal_sq.sqrt() * 0.0025).max(min_target_thickness_mm / 4.0)
}

fn combined_bounds(left: &[[f64; 3]], right: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let (left_min, left_max) = mesh_bounds(left);
    let (right_min, right_max) = mesh_bounds(right);
    let mut bbox_min = [0.0_f64; 3];
    let mut bbox_max = [0.0_f64; 3];
    for axis in 0..3 {
        bbox_min[axis] = left_min[axis].min(right_min[axis]);
        bbox_max[axis] = left_max[axis].max(right_max[axis]);
    }
    (bbox_min, bbox_max)
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

fn boolean_extraction_options(options: VoxelBooleanMeshOptions) -> GridMeshExtractionOptions {
    GridMeshExtractionOptions {
        voxel_size: options.voxel_size,
        extractor: options.extractor,
        refine: options.refine,
        smooth_iterations: 1,
        smooth_strength: 0.2,
        projection_iterations: 3,
    }
}

fn extractor_name(extractor: VoxelMeshExtractor) -> &'static str {
    match extractor {
        VoxelMeshExtractor::Marching => "marching",
        VoxelMeshExtractor::Cells => "cells",
    }
}

fn maybe_refine_grid_mesh(
    mesh: MeshArrays,
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    options: GridMeshExtractionOptions,
) -> Result<MeshArrays, GeometryError> {
    if !options.refine || mesh.faces.is_empty() {
        return Ok(mesh);
    }
    let refined_vertices = refine_vertices_with_sdf(
        &mesh.vertices,
        &mesh.faces,
        values,
        origin,
        shape,
        options.voxel_size,
        0.0,
        options.smooth_iterations,
        options.smooth_strength,
        options.projection_iterations,
    )?;
    let refined = MeshArrays {
        vertices: refined_vertices,
        faces: mesh.faces.clone(),
    };
    if refinement_preserves_topology(&mesh, &refined)? {
        Ok(refined)
    } else {
        Ok(mesh)
    }
}

fn refinement_preserves_topology(
    source: &MeshArrays,
    refined: &MeshArrays,
) -> Result<bool, GeometryError> {
    if !source.faces.is_empty() && refined.faces.is_empty() {
        return Ok(false);
    }
    let source_health = mesh_health(&source.vertices, &source.faces, true, Some(50_000), 1e-8)?;
    let refined_health = mesh_health(&refined.vertices, &refined.faces, true, Some(50_000), 1e-8)?;
    if source_health.is_closed && !refined_health.is_closed {
        return Ok(false);
    }
    if refined_health.boundary_edge_count > source_health.boundary_edge_count {
        return Ok(false);
    }
    if refined_health.nonmanifold_edge_count > source_health.nonmanifold_edge_count {
        return Ok(false);
    }
    if let (Some(source_intersections), Some(refined_intersections)) = (
        source_health.self_intersections,
        refined_health.self_intersections,
    ) {
        return Ok(refined_intersections <= source_intersections);
    }
    Ok(true)
}
