use crate::deform::laplacian_smooth_vertices;
use crate::grid::{grid_index, grid_value_count, sample_sdf_gradient, sample_sdf_value};
use crate::math::{cross, norm, sub};
use crate::{GeometryError, MarchingTetrahedraResult, MeshArrays, SdfBooleanOperation};
use rayon::prelude::*;
use std::collections::HashMap;

const CUBE_CORNERS: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1],
];

const TETRAHEDRA: [[usize; 4]; 6] = [
    [0, 5, 1, 6],
    [0, 1, 2, 6],
    [0, 2, 3, 6],
    [0, 3, 7, 6],
    [0, 7, 4, 6],
    [0, 4, 5, 6],
];

const SURFACE_FACE_QUADS: [[[usize; 3]; 4]; 6] = [
    [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
    [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]],
    [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
    [[0, 1, 0], [0, 1, 1], [1, 1, 1], [1, 1, 0]],
    [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]],
    [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
];

const SURFACE_NEIGHBOR_OFFSETS: [[isize; 3]; 6] = [
    [-1, 0, 0],
    [1, 0, 0],
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
];

#[derive(Debug, Clone, Copy)]
struct MarchingGrid {
    origin: [f64; 3],
    voxel_size: f64,
    iso_value: f32,
}

pub fn sdf_boolean_values(
    left: &[f32],
    right: &[f32],
    operation: SdfBooleanOperation,
) -> Result<Vec<f32>, GeometryError> {
    if left.len() != right.len() {
        return Err(GeometryError::MismatchedSdfValueCount {
            left: left.len(),
            right: right.len(),
        });
    }

    let output = left
        .par_iter()
        .zip(right.par_iter())
        .map(|(left_value, right_value)| match operation {
            SdfBooleanOperation::Union => (*left_value).min(*right_value),
            SdfBooleanOperation::Intersection => (*left_value).max(*right_value),
            SdfBooleanOperation::Difference => (*left_value).max(-*right_value),
        })
        .collect();
    Ok(output)
}

pub fn sdf_offset_values(values: &[f32], offset_mm: f64) -> Result<Vec<f32>, GeometryError> {
    if !offset_mm.is_finite() {
        return Err(GeometryError::InvalidSdfOffset { offset_mm });
    }
    Ok(values
        .par_iter()
        .map(|value| (*value as f64 - offset_mm) as f32)
        .collect())
}

pub fn sdf_shell_values(values: &[f32], wall_thickness_mm: f64) -> Result<Vec<f32>, GeometryError> {
    if !wall_thickness_mm.is_finite() || wall_thickness_mm <= 0.0 {
        return Err(GeometryError::InvalidWallThickness { wall_thickness_mm });
    }
    Ok(values
        .par_iter()
        .map(|value| {
            let inner_void = *value as f64 + wall_thickness_mm;
            (*value as f64).max(-inner_void) as f32
        })
        .collect())
}

pub fn extract_surface_mesh_from_sdf_cells(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f32,
) -> Result<MeshArrays, GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    let expected_values = grid_value_count(shape)?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    if shape.iter().any(|dimension| *dimension < 2) {
        return Ok(MeshArrays {
            vertices: Vec::new(),
            faces: Vec::new(),
        });
    }

    let cell_shape = [shape[0] - 1, shape[1] - 1, shape[2] - 1];
    let cell_count = grid_value_count(cell_shape)?;
    let mut occupied = vec![false; cell_count];
    let mut any_occupied = false;
    for i in 0..cell_shape[0] {
        for j in 0..cell_shape[1] {
            for k in 0..cell_shape[2] {
                let average = cell_average(values, shape, [i, j, k]);
                let inside = average <= iso_value;
                occupied[grid_index([i, j, k], cell_shape)] = inside;
                any_occupied |= inside;
            }
        }
    }
    if !any_occupied {
        return Ok(MeshArrays {
            vertices: Vec::new(),
            faces: Vec::new(),
        });
    }

    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut faces: Vec<[i64; 3]> = Vec::new();
    let mut vertex_map: HashMap<[usize; 3], usize> = HashMap::new();

    for i in 0..cell_shape[0] {
        for j in 0..cell_shape[1] {
            for k in 0..cell_shape[2] {
                let base = [i, j, k];
                if !occupied[grid_index(base, cell_shape)] {
                    continue;
                }
                for (face_index, offset) in SURFACE_NEIGHBOR_OFFSETS.iter().enumerate() {
                    if neighbor_occupied(base, *offset, cell_shape, &occupied) {
                        continue;
                    }
                    let mut quad = [0_usize; 4];
                    for (corner_index, corner) in SURFACE_FACE_QUADS[face_index].iter().enumerate()
                    {
                        let key = [
                            base[0] + corner[0],
                            base[1] + corner[1],
                            base[2] + corner[2],
                        ];
                        quad[corner_index] = surface_vertex_index(
                            key,
                            origin,
                            voxel_size,
                            &mut vertices,
                            &mut vertex_map,
                        );
                    }
                    faces.push([quad[0] as i64, quad[1] as i64, quad[2] as i64]);
                    faces.push([quad[0] as i64, quad[2] as i64, quad[3] as i64]);
                }
            }
        }
    }

    Ok(MeshArrays { vertices, faces })
}

pub fn sdf_boolean_marching_tetrahedra(
    left: &[f32],
    right: &[f32],
    operation: SdfBooleanOperation,
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f32,
) -> Result<MarchingTetrahedraResult, GeometryError> {
    let values = sdf_boolean_values(left, right, operation)?;
    marching_tetrahedra(&values, origin, shape, voxel_size, iso_value)
}

pub fn sdf_offset_marching_tetrahedra(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    offset_mm: f64,
    iso_value: f32,
) -> Result<MarchingTetrahedraResult, GeometryError> {
    if !offset_mm.is_finite() {
        return Err(GeometryError::InvalidSdfOffset { offset_mm });
    }
    let offset_values = sdf_offset_values(values, offset_mm)?;
    marching_tetrahedra(&offset_values, origin, shape, voxel_size, iso_value)
}

pub fn sdf_shell_marching_tetrahedra(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    wall_thickness_mm: f64,
    iso_value: f32,
) -> Result<MarchingTetrahedraResult, GeometryError> {
    if !wall_thickness_mm.is_finite() || wall_thickness_mm <= 0.0 {
        return Err(GeometryError::InvalidWallThickness { wall_thickness_mm });
    }
    let shell_values = sdf_shell_values(values, wall_thickness_mm)?;
    marching_tetrahedra(&shell_values, origin, shape, voxel_size, iso_value)
}

pub fn project_vertices_to_sdf(
    vertices: &[[f64; 3]],
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f64,
    iterations: i64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    let expected_values = grid_value_count(shape)?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    if shape.iter().any(|dimension| *dimension < 2) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }

    let mut projected = vertices.to_vec();
    let step_count = iterations.max(1) as usize;
    for _ in 0..step_count {
        projected.par_iter_mut().for_each(|vertex| {
            let distance =
                sample_sdf_value(values, origin, shape, voxel_size, *vertex) as f64 - iso_value;
            let gradient = sample_sdf_gradient(values, origin, shape, voxel_size, *vertex);
            for axis in 0..3 {
                vertex[axis] -= gradient[axis] as f64 * distance;
            }
        });
    }
    Ok(projected)
}

#[allow(clippy::too_many_arguments)]
pub fn refine_vertices_with_sdf(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f64,
    smooth_iterations: i64,
    smooth_strength: f64,
    projection_iterations: i64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let smoothed =
        laplacian_smooth_vertices(vertices, faces_i64, smooth_iterations, smooth_strength)?;
    project_vertices_to_sdf(
        &smoothed,
        values,
        origin,
        shape,
        voxel_size,
        iso_value,
        projection_iterations,
    )
}

pub fn marching_tetrahedra(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f32,
) -> Result<MarchingTetrahedraResult, GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    let expected_values = grid_value_count(shape)?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    if shape.iter().any(|dimension| *dimension < 2) {
        return Ok(MarchingTetrahedraResult {
            vertices: Vec::new(),
            faces: Vec::new(),
        });
    }

    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut faces: Vec<[i64; 3]> = Vec::new();
    let mut edge_vertex_map: HashMap<([usize; 3], [usize; 3]), usize> = HashMap::new();
    let marching_grid = MarchingGrid {
        origin,
        voxel_size,
        iso_value,
    };

    for i in 0..(shape[0] - 1) {
        for j in 0..(shape[1] - 1) {
            for k in 0..(shape[2] - 1) {
                let corner_keys = cube_corner_keys(i, j, k);
                let mut corner_values = [0.0_f32; 8];
                for (corner_index, key) in corner_keys.iter().enumerate() {
                    corner_values[corner_index] = values[grid_index(*key, shape)];
                }

                for tetrahedron in TETRAHEDRA {
                    let mut tet_keys = [[0_usize; 3]; 4];
                    let mut tet_values = [0.0_f32; 4];
                    for index in 0..4 {
                        tet_keys[index] = corner_keys[tetrahedron[index]];
                        tet_values[index] = corner_values[tetrahedron[index]];
                    }

                    let mut inside = Vec::with_capacity(4);
                    let mut outside = Vec::with_capacity(4);
                    for (index, value) in tet_values.iter().enumerate() {
                        if *value < iso_value {
                            inside.push(index);
                        } else {
                            outside.push(index);
                        }
                    }
                    match inside.len() {
                        0 | 4 => {}
                        1 => {
                            let inside_index = inside[0];
                            let tri = [
                                marching_edge_vertex(
                                    tet_keys[inside_index],
                                    tet_values[inside_index],
                                    tet_keys[outside[0]],
                                    tet_values[outside[0]],
                                    marching_grid,
                                    &mut vertices,
                                    &mut edge_vertex_map,
                                ),
                                marching_edge_vertex(
                                    tet_keys[inside_index],
                                    tet_values[inside_index],
                                    tet_keys[outside[1]],
                                    tet_values[outside[1]],
                                    marching_grid,
                                    &mut vertices,
                                    &mut edge_vertex_map,
                                ),
                                marching_edge_vertex(
                                    tet_keys[inside_index],
                                    tet_values[inside_index],
                                    tet_keys[outside[2]],
                                    tet_values[outside[2]],
                                    marching_grid,
                                    &mut vertices,
                                    &mut edge_vertex_map,
                                ),
                            ];
                            add_marching_face(tri[0], tri[1], tri[2], &vertices, &mut faces);
                        }
                        3 => {
                            let outside_index = outside[0];
                            let tri = [
                                marching_edge_vertex(
                                    tet_keys[inside[0]],
                                    tet_values[inside[0]],
                                    tet_keys[outside_index],
                                    tet_values[outside_index],
                                    marching_grid,
                                    &mut vertices,
                                    &mut edge_vertex_map,
                                ),
                                marching_edge_vertex(
                                    tet_keys[inside[1]],
                                    tet_values[inside[1]],
                                    tet_keys[outside_index],
                                    tet_values[outside_index],
                                    marching_grid,
                                    &mut vertices,
                                    &mut edge_vertex_map,
                                ),
                                marching_edge_vertex(
                                    tet_keys[inside[2]],
                                    tet_values[inside[2]],
                                    tet_keys[outside_index],
                                    tet_values[outside_index],
                                    marching_grid,
                                    &mut vertices,
                                    &mut edge_vertex_map,
                                ),
                            ];
                            add_marching_face(tri[0], tri[2], tri[1], &vertices, &mut faces);
                        }
                        2 => {
                            let i0 = inside[0];
                            let i1 = inside[1];
                            let o0 = outside[0];
                            let o1 = outside[1];
                            let v00 = marching_edge_vertex(
                                tet_keys[i0],
                                tet_values[i0],
                                tet_keys[o0],
                                tet_values[o0],
                                marching_grid,
                                &mut vertices,
                                &mut edge_vertex_map,
                            );
                            let v10 = marching_edge_vertex(
                                tet_keys[i1],
                                tet_values[i1],
                                tet_keys[o0],
                                tet_values[o0],
                                marching_grid,
                                &mut vertices,
                                &mut edge_vertex_map,
                            );
                            let v11 = marching_edge_vertex(
                                tet_keys[i1],
                                tet_values[i1],
                                tet_keys[o1],
                                tet_values[o1],
                                marching_grid,
                                &mut vertices,
                                &mut edge_vertex_map,
                            );
                            let v01 = marching_edge_vertex(
                                tet_keys[i0],
                                tet_values[i0],
                                tet_keys[o1],
                                tet_values[o1],
                                marching_grid,
                                &mut vertices,
                                &mut edge_vertex_map,
                            );
                            add_marching_face(v00, v10, v11, &vertices, &mut faces);
                            add_marching_face(v00, v11, v01, &vertices, &mut faces);
                        }
                        _ => unreachable!("tetrahedra have four corners"),
                    }
                }
            }
        }
    }

    Ok(MarchingTetrahedraResult { vertices, faces })
}

fn cube_corner_keys(i: usize, j: usize, k: usize) -> [[usize; 3]; 8] {
    CUBE_CORNERS.map(|corner| [i + corner[0], j + corner[1], k + corner[2]])
}

fn cell_average(values: &[f32], shape: [usize; 3], base: [usize; 3]) -> f32 {
    let mut total = 0.0_f32;
    for dx in 0..=1 {
        for dy in 0..=1 {
            for dz in 0..=1 {
                total += values[grid_index([base[0] + dx, base[1] + dy, base[2] + dz], shape)];
            }
        }
    }
    total / 8.0
}

fn neighbor_occupied(
    base: [usize; 3],
    offset: [isize; 3],
    cell_shape: [usize; 3],
    occupied: &[bool],
) -> bool {
    let mut neighbor = [0_usize; 3];
    for axis in 0..3 {
        let value = base[axis] as isize + offset[axis];
        if value < 0 || value >= cell_shape[axis] as isize {
            return false;
        }
        neighbor[axis] = value as usize;
    }
    occupied[grid_index(neighbor, cell_shape)]
}

fn surface_vertex_index(
    key: [usize; 3],
    origin: [f64; 3],
    voxel_size: f64,
    vertices: &mut Vec<[f64; 3]>,
    vertex_map: &mut HashMap<[usize; 3], usize>,
) -> usize {
    if let Some(index) = vertex_map.get(&key) {
        return *index;
    }
    let index = vertices.len();
    vertices.push([
        origin[0] + key[0] as f64 * voxel_size,
        origin[1] + key[1] as f64 * voxel_size,
        origin[2] + key[2] as f64 * voxel_size,
    ]);
    vertex_map.insert(key, index);
    index
}

fn point_for_grid_key(key: [usize; 3], origin: [f64; 3], voxel_size: f64) -> [f64; 3] {
    [
        origin[0] + key[0] as f64 * voxel_size,
        origin[1] + key[1] as f64 * voxel_size,
        origin[2] + key[2] as f64 * voxel_size,
    ]
}

fn marching_edge_vertex(
    key_a: [usize; 3],
    value_a: f32,
    key_b: [usize; 3],
    value_b: f32,
    grid: MarchingGrid,
    vertices: &mut Vec<[f64; 3]>,
    edge_vertex_map: &mut HashMap<([usize; 3], [usize; 3]), usize>,
) -> usize {
    let edge_key = if key_a <= key_b {
        (key_a, key_b)
    } else {
        (key_b, key_a)
    };
    if let Some(existing) = edge_vertex_map.get(&edge_key) {
        return *existing;
    }

    let denom = value_b - value_a;
    let t = if denom.abs() < 1e-12 {
        0.5
    } else {
        ((grid.iso_value - value_a) / denom).clamp(0.0, 1.0) as f64
    };
    let point_a = point_for_grid_key(key_a, grid.origin, grid.voxel_size);
    let point_b = point_for_grid_key(key_b, grid.origin, grid.voxel_size);
    let point = [
        point_a[0] + (point_b[0] - point_a[0]) * t,
        point_a[1] + (point_b[1] - point_a[1]) * t,
        point_a[2] + (point_b[2] - point_a[2]) * t,
    ];
    let index = vertices.len();
    vertices.push(point);
    edge_vertex_map.insert(edge_key, index);
    index
}

fn add_marching_face(
    a: usize,
    b: usize,
    c: usize,
    vertices: &[[f64; 3]],
    faces: &mut Vec<[i64; 3]>,
) {
    if a == b || b == c || a == c {
        return;
    }
    let pa = vertices[a];
    let pb = vertices[b];
    let pc = vertices[c];
    if norm(cross(sub(pb, pa), sub(pc, pa))) < 1e-12 {
        return;
    }
    faces.push([a as i64, b as i64, c as i64]);
}
