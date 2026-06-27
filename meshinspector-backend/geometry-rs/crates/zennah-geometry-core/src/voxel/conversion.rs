use crate::deform::laplacian_smooth_vertices;
use crate::grid::{grid_index, grid_value_count};
use crate::math::{add, norm, scale};
use crate::mesh::{validate_faces, vertex_neighbor_list, vertex_normals_from_faces};
use crate::repair::{find_disoriented_faces, FindDisorientationRayMode};
use crate::{
    GeometryError, MeshArrays, VoxelDualMeshSettings, VoxelMaxDerivResult, VoxelMaxDerivSettings,
    VoxelSmartMeshResult,
};
use std::collections::HashMap;

use super::conversion_polynomial::{
    fit_polynomial_least_squares, meshlib_dense_value_at, polynomial_derivative,
    polynomial_interval_min_arg, pseudo_index, smooth_shift_vectors,
};
use super::marching::{dual_contouring, marching_tetrahedra};

pub fn voxel_to_mesh_simple_values(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    iso_value: f32,
    level_set: bool,
) -> Result<MeshArrays, GeometryError> {
    if shape.iter().any(|dimension| *dimension == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "voxel_size",
            value: format!("{voxel_size:?}"),
        });
    }
    if !iso_value.is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "iso_value",
            value: iso_value.to_string(),
        });
    }
    let expected_values = grid_value_count(shape)?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }

    let mut marching_values = vec![0.0_f32; expected_values];
    let xy = shape[0] * shape[1];
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                let meshlib_index = x + y * shape[0] + z * xy;
                let value = values[meshlib_index];
                if value.is_nan() {
                    return Err(GeometryError::InvalidVoxelValue {
                        index: meshlib_index,
                        value,
                    });
                }
                marching_values[grid_index([x, y, z], shape)] =
                    if level_set { value } else { -value };
            }
        }
    }

    let marching_iso = if level_set { iso_value } else { -iso_value };
    let mut mesh =
        marching_tetrahedra(&marching_values, [0.0, 0.0, 0.0], shape, 1.0, marching_iso)?;
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] *= voxel_size[axis];
        }
    }
    Ok(MeshArrays {
        vertices: mesh.vertices,
        faces: mesh.faces,
    })
}

pub fn voxel_to_mesh_dual_values(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    iso_value: f32,
    level_set: bool,
) -> Result<MeshArrays, GeometryError> {
    if shape.iter().any(|dimension| *dimension == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "voxel_size",
            value: format!("{voxel_size:?}"),
        });
    }
    if !iso_value.is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "iso_value",
            value: iso_value.to_string(),
        });
    }
    let expected_values = grid_value_count(shape)?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }

    let mut dual_values = vec![0.0_f32; expected_values];
    let xy = shape[0] * shape[1];
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                let meshlib_index = x + y * shape[0] + z * xy;
                let value = values[meshlib_index];
                if value.is_nan() {
                    return Err(GeometryError::InvalidVoxelValue {
                        index: meshlib_index,
                        value,
                    });
                }
                dual_values[grid_index([x, y, z], shape)] = if level_set { value } else { -value };
            }
        }
    }

    let dual_iso = if level_set { iso_value } else { -iso_value };
    let mut mesh = dual_contouring(&dual_values, [0.0, 0.0, 0.0], shape, 1.0, dual_iso)?;
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] *= voxel_size[axis];
        }
    }
    Ok(mesh)
}

pub fn voxel_to_mesh_dual_values_with_settings(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    settings: VoxelDualMeshSettings,
) -> Result<MeshArrays, GeometryError> {
    if !settings.adaptivity.is_finite() || !(0.0..=1.0).contains(&settings.adaptivity) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "adaptivity",
            value: settings.adaptivity.to_string(),
        });
    }

    let mut mesh = voxel_to_mesh_dual_values(
        values,
        shape,
        voxel_size,
        settings.iso_value,
        settings.level_set,
    )?;
    if settings.relax_disoriented_triangles {
        mesh = relax_disoriented_mesh_triangles(mesh)?;
    }
    if settings.adaptivity > 0.0 {
        mesh = adapt_planar_dual_mesh(mesh);
    }
    if mesh.vertices.len() > settings.max_vertices {
        return Err(GeometryError::MeshVerticesLimitExceeded {
            vertices: mesh.vertices.len(),
            limit: settings.max_vertices,
        });
    }
    if mesh.faces.len() > settings.max_faces {
        return Err(GeometryError::MeshFacesLimitExceeded {
            faces: mesh.faces.len(),
            limit: settings.max_faces,
        });
    }
    Ok(mesh)
}

pub fn relax_disoriented_mesh_triangles(mut mesh: MeshArrays) -> Result<MeshArrays, GeometryError> {
    if mesh.faces.is_empty() || !mesh_is_closed(&mesh)? {
        return Ok(mesh);
    }
    let disoriented = find_disoriented_faces(
        &mesh.vertices,
        &mesh.faces,
        FindDisorientationRayMode::Shallowest,
        1e-8,
    )?;
    for face_index in disoriented {
        if let Some(face) = mesh.faces.get_mut(face_index) {
            face.swap(1, 2);
        }
    }
    Ok(mesh)
}

fn mesh_is_closed(mesh: &MeshArrays) -> Result<bool, GeometryError> {
    let faces = validate_faces(&mesh.faces, mesh.vertices.len())?;
    let mut edge_counts: HashMap<(usize, usize), usize> = HashMap::new();
    for face in faces {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let edge = if a < b { (a, b) } else { (b, a) };
            *edge_counts.entry(edge).or_insert(0) += 1;
        }
    }
    Ok(edge_counts.values().all(|count| *count == 2))
}

fn adapt_planar_dual_mesh(mesh: MeshArrays) -> MeshArrays {
    if mesh.vertices.len() < 4 || mesh.faces.len() <= 2 {
        return mesh;
    }

    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }

    let eps = 1e-9_f64;
    let Some(plane_axis) = (0..3).find(|axis| (max[*axis] - min[*axis]).abs() <= eps) else {
        return mesh;
    };
    let (u_axis, v_axis) = match plane_axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let u_span = max[u_axis] - min[u_axis];
    let v_span = max[v_axis] - min[v_axis];
    if u_span <= eps || v_span <= eps {
        return mesh;
    }

    let Some(source_normal) = first_face_normal(&mesh) else {
        return mesh;
    };
    let mesh_area = mesh_area(&mesh);
    let rectangle_area = u_span * v_span;
    if (mesh_area - rectangle_area).abs() > rectangle_area.max(1.0) * 1e-6 {
        return mesh;
    }

    let plane_value = 0.5 * (min[plane_axis] + max[plane_axis]);
    let mut vertices = vec![[0.0; 3]; 4];
    let corners = [
        (min[u_axis], min[v_axis]),
        (max[u_axis], min[v_axis]),
        (max[u_axis], max[v_axis]),
        (min[u_axis], max[v_axis]),
    ];
    for (index, (u, v)) in corners.into_iter().enumerate() {
        vertices[index][plane_axis] = plane_value;
        vertices[index][u_axis] = u;
        vertices[index][v_axis] = v;
    }

    let mut faces = vec![[0, 1, 2], [0, 2, 3]];
    let adapted = MeshArrays {
        vertices: vertices.clone(),
        faces: faces.clone(),
    };
    if let Some(adapted_normal) = first_face_normal(&adapted) {
        let dot = source_normal[0] * adapted_normal[0]
            + source_normal[1] * adapted_normal[1]
            + source_normal[2] * adapted_normal[2];
        if dot < 0.0 {
            faces = vec![[0, 2, 1], [0, 3, 2]];
        }
    }

    MeshArrays { vertices, faces }
}

fn mesh_area(mesh: &MeshArrays) -> f64 {
    mesh.faces
        .iter()
        .filter_map(|face| triangle_vertices(mesh, face))
        .map(|[a, b, c]| 0.5 * norm(cross(sub(b, a), sub(c, a))))
        .sum()
}

fn first_face_normal(mesh: &MeshArrays) -> Option<[f64; 3]> {
    mesh.faces.iter().find_map(|face| {
        let [a, b, c] = triangle_vertices(mesh, face)?;
        let normal = cross(sub(b, a), sub(c, a));
        (norm(normal) > 1e-12).then_some(normal)
    })
}

fn triangle_vertices(mesh: &MeshArrays, face: &[i64; 3]) -> Option<[[f64; 3]; 3]> {
    let a = usize::try_from(face[0]).ok()?;
    let b = usize::try_from(face[1]).ok()?;
    let c = usize::try_from(face[2]).ok()?;
    Some([
        *mesh.vertices.get(a)?,
        *mesh.vertices.get(b)?,
        *mesh.vertices.get(c)?,
    ])
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn voxel_move_mesh_to_max_deriv_values(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    settings: VoxelMaxDerivSettings,
) -> Result<VoxelMaxDerivResult, GeometryError> {
    if shape.iter().any(|dimension| *dimension == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "voxel_size",
            value: format!("{voxel_size:?}"),
        });
    }
    let expected_values = grid_value_count(shape)?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    for (index, value) in values.iter().copied().enumerate() {
        if value.is_nan() {
            return Err(GeometryError::InvalidVoxelValue { index, value });
        }
    }
    if !(3..=6).contains(&settings.degree) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "degree",
            value: format!(
                "{} (MeshLib MoveMeshToVoxelMaxDeriv requires degree in [3, 6])",
                settings.degree
            ),
        });
    }
    if settings.sample_points <= settings.degree {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "sample_points",
            value: settings.sample_points.to_string(),
        });
    }
    for (field, value) in [
        ("outlier_threshold", settings.outlier_threshold),
        (
            "intermediate_smooth_force",
            settings.intermediate_smooth_force,
        ),
        (
            "preparation_smooth_force",
            settings.preparation_smooth_force,
        ),
        ("final_relax_force", settings.final_relax_force),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(GeometryError::InvalidSelectionParameter {
                field,
                value: value.to_string(),
            });
        }
    }

    let faces_usize = validate_faces(faces, vertices.len())?;
    let mut current = if settings.preparation_smooth_force > 0.0 && !faces.is_empty() {
        laplacian_smooth_vertices(vertices, faces, 1, settings.preparation_smooth_force)?
    } else {
        vertices.to_vec()
    };
    if current.is_empty() || faces_usize.is_empty() || settings.iters == 0 {
        return Ok(VoxelMaxDerivResult {
            vertices: current,
            corrected_indices: Vec::new(),
        });
    }

    let min_voxel_size = voxel_size[0].min(voxel_size[1]).min(voxel_size[2]);
    let neighbors = vertex_neighbor_list(current.len(), &faces_usize);
    let mut corrected = vec![false; current.len()];

    for _ in 0..settings.iters {
        let normals = vertex_normals_from_faces(&current, &faces_usize);
        let mut shifts = vec![[0.0_f64; 3]; current.len()];

        for (vertex_index, (point, normal)) in current.iter().zip(normals.iter()).enumerate() {
            if norm(*normal) < 1e-12 {
                continue;
            }
            let mut samples = Vec::with_capacity(settings.sample_points);
            for sample_index in 0..settings.sample_points {
                let offset = pseudo_index(sample_index, settings.sample_points);
                let sample_point = add(*point, scale(*normal, offset * min_voxel_size));
                samples.push(meshlib_dense_value_at(
                    values,
                    shape,
                    voxel_size,
                    sample_point,
                ));
            }
            let coeffs = fit_polynomial_least_squares(&samples, settings.degree)?;
            let arg_min_d = pseudo_index(2, settings.sample_points);
            let arg_max_d = pseudo_index(settings.sample_points - 3, settings.sample_points - 1);
            let derivative = polynomial_derivative(&coeffs);
            let min_x = polynomial_interval_min_arg(&derivative, arg_min_d, arg_max_d);
            if min_x.abs() < settings.outlier_threshold {
                corrected[vertex_index] = true;
                shifts[vertex_index] = scale(*normal, min_x.clamp(-0.1, 0.1) * min_voxel_size);
            }
        }

        let smoothed_shifts = smooth_shift_vectors(
            &shifts,
            &neighbors,
            settings.smooth_shift_iterations,
            settings.intermediate_smooth_force,
        );
        for (point, shift) in current.iter_mut().zip(smoothed_shifts) {
            *point = add(*point, shift);
        }

        if settings.final_relax_iterations > 0
            && settings.final_relax_force > 0.0
            && !faces.is_empty()
        {
            current = laplacian_smooth_vertices(
                &current,
                faces,
                settings.final_relax_iterations as i64,
                settings.final_relax_force,
            )?;
        }
    }

    let corrected_indices = corrected
        .into_iter()
        .enumerate()
        .filter_map(|(index, was_corrected)| was_corrected.then_some(index))
        .collect();
    Ok(VoxelMaxDerivResult {
        vertices: current,
        corrected_indices,
    })
}

pub fn voxel_to_mesh_smart_values(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    iso_value: f32,
    level_set: bool,
    settings: VoxelMaxDerivSettings,
) -> Result<VoxelSmartMeshResult, GeometryError> {
    let simple = voxel_to_mesh_simple_values(values, shape, voxel_size, iso_value, level_set)?;
    let refined = voxel_move_mesh_to_max_deriv_values(
        &simple.vertices,
        &simple.faces,
        values,
        shape,
        voxel_size,
        settings,
    )?;

    Ok(VoxelSmartMeshResult {
        vertices: refined.vertices,
        faces: simple.faces,
        corrected_indices: refined.corrected_indices,
    })
}
