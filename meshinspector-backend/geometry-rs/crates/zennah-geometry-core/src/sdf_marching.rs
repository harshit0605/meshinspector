use crate::grid::{grid_value_count, sample_sdf_value};
use crate::math::{add, cross, norm, scale, sub};
use crate::{
    basic_repair, marching_tetrahedra, orient_faces_consistently, sdf_boolean_marching_tetrahedra,
    sdf_offset_marching_tetrahedra, sdf_shell_marching_tetrahedra, GeometryError, MeshArrays,
    SdfBooleanOperation,
};

enum SdfSampler<'a> {
    Values(&'a [f32]),
    Boolean {
        left: &'a [f32],
        right: &'a [f32],
        operation: SdfBooleanOperation,
    },
    Offset {
        values: &'a [f32],
        offset_mm: f64,
    },
    Shell {
        values: &'a [f32],
        wall_thickness_mm: f64,
    },
}

impl SdfSampler<'_> {
    fn sample(&self, origin: [f64; 3], shape: [usize; 3], voxel_size: f64, point: [f64; 3]) -> f32 {
        match self {
            Self::Values(values) => sample_sdf_value(values, origin, shape, voxel_size, point),
            Self::Boolean {
                left,
                right,
                operation,
            } => {
                let left_value = sample_sdf_value(left, origin, shape, voxel_size, point);
                let right_value = sample_sdf_value(right, origin, shape, voxel_size, point);
                match operation {
                    SdfBooleanOperation::Union => left_value.min(right_value),
                    SdfBooleanOperation::Intersection => left_value.max(right_value),
                    SdfBooleanOperation::Difference => left_value.max(-right_value),
                }
            }
            Self::Offset { values, offset_mm } => {
                (sample_sdf_value(values, origin, shape, voxel_size, point) as f64 - offset_mm)
                    as f32
            }
            Self::Shell {
                values,
                wall_thickness_mm,
            } => {
                let value = sample_sdf_value(values, origin, shape, voxel_size, point) as f64;
                value.max(-(value + wall_thickness_mm)) as f32
            }
        }
    }
}

pub fn finalized_marching_tetrahedra(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f32,
) -> Result<MeshArrays, GeometryError> {
    let raw = marching_tetrahedra(values, origin, shape, voxel_size, iso_value)?;
    finalize_marching_mesh(
        raw.vertices,
        raw.faces,
        SdfSampler::Values(values),
        origin,
        shape,
        voxel_size,
    )
}

pub fn finalized_sdf_boolean_marching_tetrahedra(
    left: &[f32],
    right: &[f32],
    operation: SdfBooleanOperation,
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f32,
) -> Result<MeshArrays, GeometryError> {
    validate_sdf_values(left, shape, voxel_size)?;
    if left.len() != right.len() {
        return Err(GeometryError::MismatchedSdfValueCount {
            left: left.len(),
            right: right.len(),
        });
    }
    let raw = sdf_boolean_marching_tetrahedra(
        left, right, operation, origin, shape, voxel_size, iso_value,
    )?;
    finalize_marching_mesh(
        raw.vertices,
        raw.faces,
        SdfSampler::Boolean {
            left,
            right,
            operation,
        },
        origin,
        shape,
        voxel_size,
    )
}

pub fn finalized_sdf_offset_marching_tetrahedra(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    offset_mm: f64,
    iso_value: f32,
) -> Result<MeshArrays, GeometryError> {
    let raw =
        sdf_offset_marching_tetrahedra(values, origin, shape, voxel_size, offset_mm, iso_value)?;
    finalize_marching_mesh(
        raw.vertices,
        raw.faces,
        SdfSampler::Offset { values, offset_mm },
        origin,
        shape,
        voxel_size,
    )
}

pub fn finalized_sdf_shell_marching_tetrahedra(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    wall_thickness_mm: f64,
    iso_value: f32,
) -> Result<MeshArrays, GeometryError> {
    let raw = sdf_shell_marching_tetrahedra(
        values,
        origin,
        shape,
        voxel_size,
        wall_thickness_mm,
        iso_value,
    )?;
    finalize_marching_mesh(
        raw.vertices,
        raw.faces,
        SdfSampler::Shell {
            values,
            wall_thickness_mm,
        },
        origin,
        shape,
        voxel_size,
    )
}

fn validate_sdf_values(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: f64,
) -> Result<(), GeometryError> {
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
    Ok(())
}

fn finalize_marching_mesh(
    vertices: Vec<[f64; 3]>,
    faces: Vec<[i64; 3]>,
    sampler: SdfSampler<'_>,
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
) -> Result<MeshArrays, GeometryError> {
    if vertices.is_empty() || faces.is_empty() {
        return Ok(MeshArrays {
            vertices: Vec::new(),
            faces: Vec::new(),
        });
    }
    validate_sdf_values(
        match &sampler {
            SdfSampler::Values(values)
            | SdfSampler::Offset { values, .. }
            | SdfSampler::Shell { values, .. } => values,
            SdfSampler::Boolean { left, .. } => left,
        },
        shape,
        voxel_size,
    )?;
    if shape.iter().any(|dimension| *dimension < 2) {
        return Ok(MeshArrays {
            vertices: Vec::new(),
            faces: Vec::new(),
        });
    }

    let oriented = orient_faces_with_sdf(&vertices, &faces, &sampler, origin, shape, voxel_size)?;
    let repaired = basic_repair(&vertices, &oriented, (voxel_size * 1e-8).max(1e-10), 1e-12)?;
    Ok(MeshArrays {
        vertices: repaired.vertices,
        faces: repaired.faces,
    })
}

fn orient_faces_with_sdf(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    sampler: &SdfSampler<'_>,
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
) -> Result<Vec<[i64; 3]>, GeometryError> {
    let orientation = orient_faces_consistently(faces)?;
    let mut oriented = orientation.faces;
    if orientation.component_faces.is_empty() {
        return Ok(oriented);
    }

    let step = (voxel_size * 0.25).max(1e-7);
    for component_window in orientation.component_offsets.windows(2) {
        let component = &orientation.component_faces[component_window[0]..component_window[1]];
        let mut deltas = Vec::<f64>::new();
        for face_index in component {
            let face = checked_face(oriented[*face_index], *face_index, vertices.len())?;
            let pa = vertices[face[0]];
            let pb = vertices[face[1]];
            let pc = vertices[face[2]];
            let normal = cross(sub(pb, pa), sub(pc, pa));
            let length = norm(normal);
            if length <= 1e-12 {
                continue;
            }

            let unit_normal = scale(normal, 1.0 / length);
            let centroid = scale(add(add(pa, pb), pc), 1.0 / 3.0);
            let outside = sampler.sample(
                origin,
                shape,
                voxel_size,
                add(centroid, scale(unit_normal, step)),
            );
            let inside = sampler.sample(
                origin,
                shape,
                voxel_size,
                sub(centroid, scale(unit_normal, step)),
            );
            let delta = (outside - inside) as f64;
            if delta.is_finite() {
                deltas.push(delta);
            }
        }

        if !deltas.is_empty() && median(&mut deltas) < 0.0 {
            for face_index in component {
                let face = oriented[*face_index];
                oriented[*face_index] = [face[0], face[2], face[1]];
            }
        }
    }

    Ok(oriented)
}

fn checked_face(
    face: [i64; 3],
    face_index: usize,
    vertex_count: usize,
) -> Result<[usize; 3], GeometryError> {
    let mut converted = [0_usize; 3];
    for (corner, vertex) in face.iter().enumerate() {
        if *vertex < 0 {
            return Err(GeometryError::NegativeFaceIndex {
                face: face_index,
                vertex: *vertex,
            });
        }
        let index = *vertex as usize;
        if index >= vertex_count {
            return Err(GeometryError::FaceIndexOutOfBounds {
                face: face_index,
                vertex: *vertex,
                vertex_count,
            });
        }
        converted[corner] = index;
    }
    Ok(converted)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    let midpoint = values.len() / 2;
    if values.len() % 2 == 1 {
        values[midpoint]
    } else {
        (values[midpoint - 1] + values[midpoint]) * 0.5
    }
}
