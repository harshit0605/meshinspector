use crate::deform::laplacian_smooth_vertices;
use crate::grid::{grid_index, grid_value_count, sample_sdf_gradient, sample_sdf_value};
use crate::math::{cross, norm, sub};
use crate::{GeometryError, MarchingTetrahedraResult, MeshArrays, SdfBooleanOperation};
use rayon::prelude::*;
use std::collections::HashMap;

use super::ops::{sdf_boolean_values, sdf_offset_values, sdf_shell_values};

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

const CUBE_EDGES: [[usize; 2]; 12] = [
    [0, 1],
    [1, 2],
    [2, 3],
    [3, 0],
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 4],
    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7],
];

#[derive(Debug, Clone, Copy)]
struct MarchingGrid {
    origin: [f64; 3],
    voxel_size: f64,
    iso_value: f32,
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

pub fn dual_contouring(
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
    for (index, value) in values.iter().copied().enumerate() {
        if value.is_nan() {
            return Err(GeometryError::InvalidVoxelValue { index, value });
        }
    }

    let cell_shape = [shape[0] - 1, shape[1] - 1, shape[2] - 1];
    let cell_count = grid_value_count(cell_shape)?;
    let mut cell_vertices = vec![None; cell_count];
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let marching_grid = MarchingGrid {
        origin,
        voxel_size,
        iso_value,
    };

    for x in 0..cell_shape[0] {
        for y in 0..cell_shape[1] {
            for z in 0..cell_shape[2] {
                let corner_keys = cube_corner_keys(x, y, z);
                let mut corner_values = [0.0_f32; 8];
                for (corner_index, key) in corner_keys.iter().enumerate() {
                    corner_values[corner_index] = values[grid_index(*key, shape)];
                }
                if !dual_cell_crosses_iso(&corner_values, iso_value) {
                    continue;
                }
                let mut crossing_sum = [0.0_f64; 3];
                let mut crossing_count = 0_usize;
                for [corner_a, corner_b] in CUBE_EDGES {
                    let value_a = corner_values[corner_a];
                    let value_b = corner_values[corner_b];
                    if !dual_edge_crosses_iso(value_a, value_b, iso_value) {
                        continue;
                    }
                    let crossing = dual_edge_crossing_point(
                        corner_keys[corner_a],
                        value_a,
                        corner_keys[corner_b],
                        value_b,
                        marching_grid,
                    );
                    for axis in 0..3 {
                        crossing_sum[axis] += crossing[axis];
                    }
                    crossing_count += 1;
                }
                if crossing_count == 0 {
                    continue;
                }
                let vertex = [
                    crossing_sum[0] / crossing_count as f64,
                    crossing_sum[1] / crossing_count as f64,
                    crossing_sum[2] / crossing_count as f64,
                ];
                let vertex_index = vertices.len();
                vertices.push(vertex);
                cell_vertices[grid_index([x, y, z], cell_shape)] = Some(vertex_index);
            }
        }
    }

    for axis in 0..3 {
        let mut edge_shape = shape;
        edge_shape[axis] -= 1;
        for x in 0..edge_shape[0] {
            for y in 0..edge_shape[1] {
                for z in 0..edge_shape[2] {
                    let edge_start = [x, y, z];
                    let mut edge_end = edge_start;
                    edge_end[axis] += 1;
                    let value_a = values[grid_index(edge_start, shape)];
                    let value_b = values[grid_index(edge_end, shape)];
                    if !dual_edge_crosses_iso(value_a, value_b, iso_value) {
                        continue;
                    }
                    let Some(cell_keys) = dual_face_cell_keys(axis, edge_start, cell_shape) else {
                        continue;
                    };
                    let mut quad = [0_usize; 4];
                    let mut complete = true;
                    for (index, cell_key) in cell_keys.iter().enumerate() {
                        match cell_vertices[grid_index(*cell_key, cell_shape)] {
                            Some(vertex_index) => quad[index] = vertex_index,
                            None => {
                                complete = false;
                                break;
                            }
                        }
                    }
                    if !complete {
                        continue;
                    }
                    let start_inside = value_a < iso_value;
                    let end_inside = value_b < iso_value;
                    let normal_points_positive_axis = start_inside && !end_inside;
                    add_dual_quad(quad, normal_points_positive_axis, &vertices, &mut faces);
                }
            }
        }
    }

    Ok(MeshArrays { vertices, faces })
}

fn cube_corner_keys(i: usize, j: usize, k: usize) -> [[usize; 3]; 8] {
    CUBE_CORNERS.map(|corner| [i + corner[0], j + corner[1], k + corner[2]])
}

fn dual_cell_crosses_iso(values: &[f32; 8], iso_value: f32) -> bool {
    let mut has_inside = false;
    let mut has_outside = false;
    for value in values {
        has_inside |= *value < iso_value;
        has_outside |= *value >= iso_value;
    }
    has_inside && has_outside
}

fn dual_edge_crosses_iso(value_a: f32, value_b: f32, iso_value: f32) -> bool {
    (value_a < iso_value && value_b >= iso_value) || (value_b < iso_value && value_a >= iso_value)
}

fn dual_edge_crossing_point(
    key_a: [usize; 3],
    value_a: f32,
    key_b: [usize; 3],
    value_b: f32,
    grid: MarchingGrid,
) -> [f64; 3] {
    let denom = value_b - value_a;
    let t = if denom.abs() < 1e-12 {
        0.5
    } else {
        ((grid.iso_value - value_a) / denom).clamp(0.0, 1.0) as f64
    };
    let point_a = point_for_grid_key(key_a, grid.origin, grid.voxel_size);
    let point_b = point_for_grid_key(key_b, grid.origin, grid.voxel_size);
    [
        point_a[0] + (point_b[0] - point_a[0]) * t,
        point_a[1] + (point_b[1] - point_a[1]) * t,
        point_a[2] + (point_b[2] - point_a[2]) * t,
    ]
}

fn dual_face_cell_keys(
    axis: usize,
    edge_start: [usize; 3],
    cell_shape: [usize; 3],
) -> Option<[[usize; 3]; 4]> {
    let [x, y, z] = edge_start;
    match axis {
        0 if y > 0 && y < cell_shape[1] && z > 0 && z < cell_shape[2] => {
            Some([[x, y - 1, z - 1], [x, y, z - 1], [x, y, z], [x, y - 1, z]])
        }
        1 if x > 0 && x < cell_shape[0] && z > 0 && z < cell_shape[2] => {
            Some([[x - 1, y, z - 1], [x - 1, y, z], [x, y, z], [x, y, z - 1]])
        }
        2 if x > 0 && x < cell_shape[0] && y > 0 && y < cell_shape[1] => {
            Some([[x - 1, y - 1, z], [x, y - 1, z], [x, y, z], [x - 1, y, z]])
        }
        _ => None,
    }
}

fn add_dual_quad(
    quad: [usize; 4],
    normal_points_positive_axis: bool,
    vertices: &[[f64; 3]],
    faces: &mut Vec<[i64; 3]>,
) {
    if normal_points_positive_axis {
        add_marching_face(quad[0], quad[1], quad[2], vertices, faces);
        add_marching_face(quad[0], quad[2], quad[3], vertices, faces);
    } else {
        add_marching_face(quad[0], quad[2], quad[1], vertices, faces);
        add_marching_face(quad[0], quad[3], quad[2], vertices, faces);
    }
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
