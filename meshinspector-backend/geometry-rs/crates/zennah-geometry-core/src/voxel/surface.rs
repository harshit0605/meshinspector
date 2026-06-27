use crate::grid::{grid_index, grid_value_count};
use crate::{GeometryError, MeshArrays};
use std::collections::HashMap;

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
