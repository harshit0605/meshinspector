use super::{weighted_inner_offset_preview_vertices, weighted_inner_offset_vertices};

fn boxed_mesh(half_x: f64, half_y: f64, half_z: f64) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let vertices = vec![
        [-half_x, -half_y, -half_z],
        [half_x, -half_y, -half_z],
        [half_x, half_y, -half_z],
        [-half_x, half_y, -half_z],
        [-half_x, -half_y, half_z],
        [half_x, -half_y, half_z],
        [half_x, half_y, half_z],
        [-half_x, half_y, half_z],
    ];
    let faces = vec![
        [0, 3, 2],
        [0, 2, 1],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    (vertices, faces)
}

#[test]
fn thin_slab_offset_never_crosses_opposite_wall() {
    let (vertices, faces) = boxed_mesh(2.0, 2.0, 0.1);
    let regions = vec!["head".to_string()];

    let displaced = weighted_inner_offset_vertices(
        &vertices,
        &faces,
        &regions,
        &[0, 1],
        &[0],
        &["head".to_string()],
        0.8,
    )
    .unwrap();

    for (top, bottom) in [(4usize, 0usize), (5, 1), (6, 2), (7, 3)] {
        assert!(
            displaced[top][2] > displaced[bottom][2],
            "top vertex {top} folded through bottom vertex {bottom}: {} <= {}",
            displaced[top][2],
            displaced[bottom][2]
        );
    }
}

#[test]
fn preview_freezes_protected_vertices_and_moves_unprotected_ones() {
    let (vertices, faces) = boxed_mesh(2.0, 2.0, 2.0);
    let regions = vec!["head".to_string()];

    let displaced = weighted_inner_offset_preview_vertices(
        &vertices,
        &faces,
        &regions,
        &[0, 1],
        &[0],
        &["head".to_string()],
        0.5,
    )
    .unwrap();

    let moved = |index: usize| -> f64 {
        ((vertices[index][0] - displaced[index][0]).powi(2)
            + (vertices[index][1] - displaced[index][1]).powi(2)
            + (vertices[index][2] - displaced[index][2]).powi(2))
        .sqrt()
    };
    assert!(
        moved(0) < 1e-9,
        "protected vertex 0 moved {} mm in preview mode",
        moved(0)
    );
    assert!(
        moved(6) > 1e-3,
        "far unprotected vertex 6 should still offset inward, moved {}",
        moved(6)
    );
}

#[test]
fn thick_cube_keeps_full_requested_offset() {
    let (vertices, faces) = boxed_mesh(2.0, 2.0, 2.0);
    let regions = vec!["head".to_string()];

    let displaced = weighted_inner_offset_vertices(
        &vertices,
        &faces,
        &regions,
        &[0, 1],
        &[0],
        &["head".to_string()],
        0.5,
    )
    .unwrap();

    assert_eq!(displaced.len(), vertices.len());
    for (index, (before, after)) in vertices.iter().zip(displaced.iter()).enumerate() {
        let moved = ((before[0] - after[0]).powi(2)
            + (before[1] - after[1]).powi(2)
            + (before[2] - after[2]).powi(2))
        .sqrt();
        assert!(moved > 1e-4, "vertex {index} did not move inward at all");
    }
}

#[test]
fn protected_shell_keeps_material_instead_of_crumbling() {
    use super::protected_hollow_mesh;
    use crate::{mesh_volume, VoxelMeshExtractor, VoxelMeshOptions};

    // Coarse torus: center radius 10, tube radius 2.
    let (nu, nv) = (48usize, 16usize);
    let (big_r, small_r) = (10.0_f64, 2.0_f64);
    let mut vertices = Vec::with_capacity(nu * nv);
    for i in 0..nu {
        let u = std::f64::consts::TAU * i as f64 / nu as f64;
        for j in 0..nv {
            let v = std::f64::consts::TAU * j as f64 / nv as f64;
            vertices.push([
                (big_r + small_r * v.cos()) * u.cos(),
                (big_r + small_r * v.cos()) * u.sin(),
                small_r * v.sin(),
            ]);
        }
    }
    let vid = |i: usize, j: usize| ((i % nu) * nv + (j % nv)) as i64;
    let mut faces = Vec::with_capacity(nu * nv * 2);
    for i in 0..nu {
        for j in 0..nv {
            faces.push([vid(i, j), vid(i + 1, j), vid(i + 1, j + 1)]);
            faces.push([vid(i, j), vid(i + 1, j + 1), vid(i, j + 1)]);
        }
    }

    // Protect the bore-facing half of the tube.
    let bore_vertices: Vec<i64> = vertices
        .iter()
        .enumerate()
        .filter(|(_, vertex)| (vertex[0] * vertex[0] + vertex[1] * vertex[1]).sqrt() < big_r)
        .map(|(index, _)| index as i64)
        .collect();
    let region_ids = vec!["inner_band".to_string()];
    let vertex_offsets = vec![0, bore_vertices.len() as i64];

    let full_volume = mesh_volume(&vertices, &faces).unwrap();
    let options = VoxelMeshOptions {
        voxel_size: 0.4,
        padding_mm: Some(1.2),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let shell = protected_hollow_mesh(
        &vertices,
        &faces,
        &region_ids,
        &vertex_offsets,
        &bore_vertices,
        &["inner_band".to_string()],
        0.8,
        options,
    )
    .unwrap();
    let shell_volume = mesh_volume(&shell.vertices, &shell.faces).unwrap();

    assert!(
        shell_volume > full_volume * 0.25,
        "protected shell crumbled: kept {shell_volume:.1} of {full_volume:.1} mm3"
    );
    assert!(
        shell_volume < full_volume * 0.99,
        "shell removed no material: {shell_volume:.1} of {full_volume:.1} mm3"
    );
    let components =
        crate::mesh::connected_face_components_for_mesh(&shell.vertices, &shell.faces).unwrap();
    assert!(
        components.len() <= 4,
        "shell fragmented into {} components",
        components.len()
    );
}
