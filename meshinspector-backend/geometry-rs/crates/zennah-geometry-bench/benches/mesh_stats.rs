use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zennah_geometry_core::{
    apply_brush_strokes, closest_points_on_mesh, falloff_weights, first_ray_hit, first_ray_hits,
    laplacian_smooth_vertices, local_offset_vertices, marching_tetrahedra, mesh_stats,
    orient_faces_consistently, point_mesh_distances, project_vertices_to_sdf,
    ray_thickness_at_vertices, refine_vertices_with_sdf, sdf_boolean_marching_tetrahedra,
    sdf_boolean_values, sdf_grid_values, sdf_offset_marching_tetrahedra,
    sdf_shell_marching_tetrahedra, self_intersecting_faces, signed_point_mesh_distances,
    smooth_vertices_with_falloff, weighted_laplacian_smooth_vertices, winding_numbers,
    SdfBooleanOperation, SmoothFalloffOptions,
};

fn grid_mesh(size: usize) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let mut vertices = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            vertices.push([x as f64, y as f64, 0.0]);
        }
    }
    let mut faces = Vec::with_capacity((size - 1) * (size - 1) * 2);
    for y in 0..(size - 1) {
        for x in 0..(size - 1) {
            let a = (y * size + x) as i64;
            let b = a + 1;
            let c = a + size as i64;
            let d = c + 1;
            faces.push([a, c, b]);
            faces.push([b, c, d]);
        }
    }
    (vertices, faces)
}

fn bench_mesh_stats(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(256);
    criterion.bench_function("mesh_stats_grid_256", |bencher| {
        bencher.iter(|| mesh_stats(black_box(&vertices), black_box(&faces)).unwrap())
    });
}

fn crossing_pairs(count: usize) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let mut vertices = Vec::with_capacity(count * 6);
    let mut faces = Vec::with_capacity(count * 2);
    for index in 0..count {
        let offset = index as f64 * 4.0;
        let start = vertices.len() as i64;
        vertices.extend_from_slice(&[
            [offset - 1.0, 0.0, 0.0],
            [offset + 1.0, 0.0, 0.0],
            [offset, 1.0, 0.0],
            [offset, -0.5, -1.0],
            [offset, -0.5, 1.0],
            [offset, 1.2, 0.0],
        ]);
        faces.push([start, start + 1, start + 2]);
        faces.push([start + 3, start + 4, start + 5]);
    }
    (vertices, faces)
}

fn bench_self_intersections(criterion: &mut Criterion) {
    let (vertices, faces) = crossing_pairs(4096);
    criterion.bench_function("self_intersections_crossing_pairs_8192_faces", |bencher| {
        bencher.iter(|| {
            self_intersecting_faces(black_box(&vertices), black_box(&faces), 1e-8).unwrap()
        })
    });
}

fn query_grid(size: usize, z: f64) -> Vec<[f64; 3]> {
    let mut points = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            points.push([x as f64 + 0.25, y as f64 + 0.25, z]);
        }
    }
    points
}

fn bench_point_mesh_distances(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(256);
    let points = query_grid(64, 1.0);
    criterion.bench_function("point_mesh_distances_grid_4096_points", |bencher| {
        bencher.iter(|| {
            point_mesh_distances(black_box(&points), black_box(&vertices), black_box(&faces))
                .unwrap()
        })
    });
}

fn bench_closest_points_on_mesh(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(256);
    let points = query_grid(64, 1.0);
    criterion.bench_function("closest_points_on_mesh_grid_4096_points", |bencher| {
        bencher.iter(|| {
            closest_points_on_mesh(black_box(&points), black_box(&vertices), black_box(&faces))
                .unwrap()
        })
    });
}

fn bench_first_ray_hit(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(256);
    criterion.bench_function("first_ray_hit_grid_256", |bencher| {
        bencher.iter(|| {
            first_ray_hit(
                black_box(&vertices),
                black_box(&faces),
                [128.25, 128.25, 8.0],
                [0.0, 0.0, -1.0],
                1e-8,
                &[],
            )
            .unwrap()
        })
    });
}

fn ray_grid(size: usize, z: f64) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    let mut origins = Vec::with_capacity(size * size);
    let mut directions = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            origins.push([x as f64 + 0.25, y as f64 + 0.25, z]);
            directions.push([0.0, 0.0, -1.0]);
        }
    }
    (origins, directions)
}

fn bench_first_ray_hits(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(256);
    let (origins, directions) = ray_grid(64, 8.0);
    criterion.bench_function("first_ray_hits_grid_4096_rays", |bencher| {
        bencher.iter(|| {
            first_ray_hits(
                black_box(&vertices),
                black_box(&faces),
                black_box(&origins),
                black_box(&directions),
                1e-8,
                &[],
            )
            .unwrap()
        })
    });
}

fn bench_ray_thickness(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(96);
    criterion.bench_function("ray_thickness_grid_9216_vertices", |bencher| {
        bencher.iter(|| {
            ray_thickness_at_vertices(black_box(&vertices), black_box(&faces), 1e-5).unwrap()
        })
    });
}

fn cube_mesh() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    (
        vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
        vec![
            [0, 2, 1],
            [0, 3, 2],
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
        ],
    )
}

fn bench_sdf_grid_values(criterion: &mut Criterion) {
    let (vertices, faces) = cube_mesh();
    criterion.bench_function("sdf_grid_values_cube_4096_samples", |bencher| {
        bencher.iter(|| {
            sdf_grid_values(
                black_box(&vertices),
                black_box(&faces),
                [-2.0, -2.0, -2.0],
                [16, 16, 16],
                4.0 / 15.0,
                0.5,
            )
            .unwrap()
        })
    });
}

fn bench_sdf_boolean_values(criterion: &mut Criterion) {
    let left = vec![0.25_f32; 128 * 128 * 64];
    let right = vec![-0.5_f32; 128 * 128 * 64];
    criterion.bench_function("sdf_boolean_values_1m_samples", |bencher| {
        bencher.iter(|| {
            sdf_boolean_values(
                black_box(&left),
                black_box(&right),
                SdfBooleanOperation::Difference,
            )
            .unwrap()
        })
    });
}

fn bench_sdf_boolean_marching_tetrahedra(criterion: &mut Criterion) {
    let (vertices, faces) = cube_mesh();
    let left = sdf_grid_values(
        &vertices,
        &faces,
        [-1.5, -1.5, -1.5],
        [25, 25, 25],
        0.125,
        0.5,
    )
    .unwrap();
    let right: Vec<f32> = left.iter().map(|value| *value - 0.2).collect();
    criterion.bench_function("sdf_boolean_marching_tetrahedra_cube_25_grid", |bencher| {
        bencher.iter(|| {
            sdf_boolean_marching_tetrahedra(
                black_box(&left),
                black_box(&right),
                SdfBooleanOperation::Union,
                [-1.5, -1.5, -1.5],
                [25, 25, 25],
                0.125,
                0.0,
            )
            .unwrap()
        })
    });
}

fn bench_sdf_offset_marching_tetrahedra(criterion: &mut Criterion) {
    let (vertices, faces) = cube_mesh();
    let values = sdf_grid_values(
        &vertices,
        &faces,
        [-1.5, -1.5, -1.5],
        [25, 25, 25],
        0.125,
        0.5,
    )
    .unwrap();
    criterion.bench_function("sdf_offset_marching_tetrahedra_cube_25_grid", |bencher| {
        bencher.iter(|| {
            sdf_offset_marching_tetrahedra(
                black_box(&values),
                [-1.5, -1.5, -1.5],
                [25, 25, 25],
                0.125,
                0.25,
                0.0,
            )
            .unwrap()
        })
    });
}

fn bench_sdf_shell_marching_tetrahedra(criterion: &mut Criterion) {
    let (vertices, faces) = cube_mesh();
    let values = sdf_grid_values(
        &vertices,
        &faces,
        [-2.0, -2.0, -2.0],
        [33, 33, 33],
        0.125,
        0.5,
    )
    .unwrap();
    criterion.bench_function("sdf_shell_marching_tetrahedra_cube_33_grid", |bencher| {
        bencher.iter(|| {
            sdf_shell_marching_tetrahedra(
                black_box(&values),
                [-2.0, -2.0, -2.0],
                [33, 33, 33],
                0.125,
                0.75,
                0.0,
            )
            .unwrap()
        })
    });
}

fn projection_points(size: usize, radius: f64) -> Vec<[f64; 3]> {
    let mut points = Vec::with_capacity(size * size * 6);
    for y_index in 0..size {
        let y = -1.0 + 2.0 * y_index as f64 / (size - 1) as f64;
        for z_index in 0..size {
            let z = -1.0 + 2.0 * z_index as f64 / (size - 1) as f64;
            points.push([radius, y, z]);
            points.push([-radius, y, z]);
            points.push([y, radius, z]);
            points.push([y, -radius, z]);
            points.push([y, z, radius]);
            points.push([y, z, -radius]);
        }
    }
    points
}

fn bench_project_vertices_to_sdf(criterion: &mut Criterion) {
    let (vertices, faces) = cube_mesh();
    let values = sdf_grid_values(
        &vertices,
        &faces,
        [-1.5, -1.5, -1.5],
        [25, 25, 25],
        0.125,
        0.5,
    )
    .unwrap();
    let points = projection_points(37, 1.12);
    criterion.bench_function("project_vertices_to_sdf_8214_points", |bencher| {
        bencher.iter(|| {
            project_vertices_to_sdf(
                black_box(&points),
                black_box(&values),
                [-1.5, -1.5, -1.5],
                [25, 25, 25],
                0.125,
                0.0,
                3,
            )
            .unwrap()
        })
    });
}

fn projection_face_mesh(size: usize, radius: f64) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let mut vertices = Vec::with_capacity(size * size);
    for y_index in 0..size {
        let y = -1.0 + 2.0 * y_index as f64 / (size - 1) as f64;
        for x_index in 0..size {
            let x = -1.0 + 2.0 * x_index as f64 / (size - 1) as f64;
            vertices.push([x, y, radius]);
        }
    }

    let mut faces = Vec::with_capacity((size - 1) * (size - 1) * 2);
    for y in 0..(size - 1) {
        for x in 0..(size - 1) {
            let a = (y * size + x) as i64;
            let b = a + 1;
            let c = a + size as i64;
            let d = c + 1;
            faces.push([a, c, b]);
            faces.push([b, c, d]);
        }
    }
    (vertices, faces)
}

fn bench_refine_vertices_with_sdf(criterion: &mut Criterion) {
    let (cube_vertices, cube_faces) = cube_mesh();
    let values = sdf_grid_values(
        &cube_vertices,
        &cube_faces,
        [-1.5, -1.5, -1.5],
        [25, 25, 25],
        0.125,
        0.5,
    )
    .unwrap();
    let (vertices, faces) = projection_face_mesh(96, 1.12);
    criterion.bench_function("refine_vertices_with_sdf_9216_vertices", |bencher| {
        bencher.iter(|| {
            refine_vertices_with_sdf(
                black_box(&vertices),
                black_box(&faces),
                black_box(&values),
                [-1.5, -1.5, -1.5],
                [25, 25, 25],
                0.125,
                0.0,
                2,
                0.25,
                3,
            )
            .unwrap()
        })
    });
}

fn bench_laplacian_smooth_vertices(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(96);
    criterion.bench_function("laplacian_smooth_vertices_9216_vertices", |bencher| {
        bencher.iter(|| {
            laplacian_smooth_vertices(black_box(&vertices), black_box(&faces), 2, 0.25).unwrap()
        })
    });
}

fn bench_weighted_laplacian_smooth_vertices(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(96);
    let mut weights = vec![0.0_f32; vertices.len()];
    for (index, weight) in weights.iter_mut().enumerate() {
        *weight = if index % 3 == 0 { 1.0 } else { 0.35 };
    }
    criterion.bench_function(
        "weighted_laplacian_smooth_vertices_9216_vertices",
        |bencher| {
            bencher.iter(|| {
                weighted_laplacian_smooth_vertices(
                    black_box(&vertices),
                    black_box(&faces),
                    black_box(&weights),
                    2,
                    0.25,
                    0.02,
                )
                .unwrap()
            })
        },
    );
}

fn bench_marching_tetrahedra(criterion: &mut Criterion) {
    let (vertices, faces) = cube_mesh();
    let values = sdf_grid_values(
        &vertices,
        &faces,
        [-1.4, -1.4, -1.4],
        [15, 15, 15],
        0.2,
        0.5,
    )
    .unwrap();
    criterion.bench_function("marching_tetrahedra_cube_15_grid", |bencher| {
        bencher.iter(|| {
            marching_tetrahedra(
                black_box(&values),
                [-1.4, -1.4, -1.4],
                [15, 15, 15],
                0.2,
                0.0,
            )
            .unwrap()
        })
    });
}

fn bench_falloff_weights(criterion: &mut Criterion) {
    let (vertices, _) = grid_mesh(128);
    let seeds: Vec<i64> = (0..128).step_by(2).map(|value| value as i64).collect();
    criterion.bench_function("falloff_weights_16384_vertices_64_seeds", |bencher| {
        bencher.iter(|| falloff_weights(black_box(&vertices), black_box(&seeds), 8.0, 3.0).unwrap())
    });
}

fn bench_smooth_vertices_with_falloff(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(96);
    let seeds: Vec<i64> = (0..64).map(|value| value as i64).collect();
    criterion.bench_function("smooth_vertices_with_falloff_9216_vertices", |bencher| {
        bencher.iter(|| {
            smooth_vertices_with_falloff(
                black_box(&vertices),
                black_box(&faces),
                black_box(&seeds),
                SmoothFalloffOptions {
                    falloff_mm: 8.0,
                    iterations: 2,
                    strength: 0.25,
                    active_threshold: 0.02,
                    cutoff_multiplier: 3.0,
                },
            )
            .unwrap()
        })
    });
}

fn bench_local_offset_vertices(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(96);
    let seeds: Vec<i64> = (0..64).map(|value| value as i64).collect();
    criterion.bench_function("local_offset_vertices_9216_vertices", |bencher| {
        bencher.iter(|| {
            local_offset_vertices(
                black_box(&vertices),
                black_box(&faces),
                black_box(&seeds),
                8.0,
                0.25,
                3.0,
            )
            .unwrap()
        })
    });
}

fn bench_apply_brush_strokes(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(96);
    let seeds: Vec<i64> = (0..64).map(|value| value as i64).collect();
    let mut flat_seeds = Vec::with_capacity(seeds.len() * 3);
    flat_seeds.extend_from_slice(&seeds);
    flat_seeds.extend(seeds.iter().map(|value| value + 32));
    flat_seeds.extend_from_slice(&seeds);
    criterion.bench_function("apply_brush_strokes_9216_vertices", |bencher| {
        bencher.iter(|| {
            apply_brush_strokes(
                black_box(&vertices),
                black_box(&faces),
                &[0, 1, 2],
                &[0, 64, 128, 192],
                black_box(&flat_seeds),
                &[0, 0, 0],
                &[0, 0, 0, 0],
                &[],
                &[0, 0, 0, 0],
                &[],
                &[0.18, 0.07, 0.0],
                &[8.0, 6.0, 8.0],
                &[1, 1, 2],
                &[0.5, 0.5, 0.25],
                3.0,
            )
            .unwrap()
        })
    });
}

fn orientation_strip_faces(count: usize) -> Vec<[i64; 3]> {
    let mut faces = Vec::with_capacity(count * 2);
    for index in 0..count {
        let start = (index * 4) as i64;
        faces.push([start, start + 1, start + 2]);
        faces.push([start + 1, start + 2, start + 3]);
    }
    faces
}

fn bench_orient_faces_consistently(criterion: &mut Criterion) {
    let faces = orientation_strip_faces(4096);
    criterion.bench_function("orient_faces_consistently_8192_faces", |bencher| {
        bencher.iter(|| orient_faces_consistently(black_box(&faces)).unwrap())
    });
}

fn bench_winding_numbers(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(64);
    let points = query_grid(32, 1.0);
    criterion.bench_function("winding_numbers_grid_1024_points", |bencher| {
        bencher.iter(|| {
            winding_numbers(black_box(&points), black_box(&vertices), black_box(&faces)).unwrap()
        })
    });
}

fn bench_signed_point_mesh_distances(criterion: &mut Criterion) {
    let (vertices, faces) = grid_mesh(64);
    let points = query_grid(32, 1.0);
    criterion.bench_function("signed_point_mesh_distances_grid_1024_points", |bencher| {
        bencher.iter(|| {
            signed_point_mesh_distances(
                black_box(&points),
                black_box(&vertices),
                black_box(&faces),
                0.5,
            )
            .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_mesh_stats,
    bench_self_intersections,
    bench_point_mesh_distances,
    bench_closest_points_on_mesh,
    bench_winding_numbers,
    bench_signed_point_mesh_distances,
    bench_first_ray_hit,
    bench_first_ray_hits,
    bench_ray_thickness,
    bench_sdf_grid_values,
    bench_sdf_boolean_values,
    bench_sdf_boolean_marching_tetrahedra,
    bench_sdf_offset_marching_tetrahedra,
    bench_sdf_shell_marching_tetrahedra,
    bench_project_vertices_to_sdf,
    bench_refine_vertices_with_sdf,
    bench_laplacian_smooth_vertices,
    bench_weighted_laplacian_smooth_vertices,
    bench_falloff_weights,
    bench_smooth_vertices_with_falloff,
    bench_local_offset_vertices,
    bench_apply_brush_strokes,
    bench_marching_tetrahedra,
    bench_orient_faces_consistently
);
criterion_main!(benches);
