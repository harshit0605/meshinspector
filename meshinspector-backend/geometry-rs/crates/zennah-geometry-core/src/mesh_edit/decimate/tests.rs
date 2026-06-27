use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Closed UV sphere with >2000 faces so the fast path engages.
fn uv_sphere(stacks: usize, slices: usize, radius: f64) -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    let mut vertices = Vec::new();
    for i in 1..stacks {
        let theta = std::f64::consts::PI * i as f64 / stacks as f64;
        for j in 0..slices {
            let phi = std::f64::consts::TAU * j as f64 / slices as f64;
            vertices.push([
                radius * theta.sin() * phi.cos(),
                radius * theta.sin() * phi.sin(),
                radius * theta.cos(),
            ]);
        }
    }
    let top = vertices.len();
    vertices.push([0.0, 0.0, radius]);
    let bottom = vertices.len();
    vertices.push([0.0, 0.0, -radius]);

    let vid = |i: usize, j: usize| (i - 1) * slices + (j % slices);
    let mut faces = Vec::new();
    for i in 1..(stacks - 1) {
        for j in 0..slices {
            let a = vid(i, j);
            let b = vid(i, j + 1);
            let c = vid(i + 1, j);
            let d = vid(i + 1, j + 1);
            faces.push([a, b, c]);
            faces.push([b, d, c]);
        }
    }
    for j in 0..slices {
        faces.push([top, vid(1, j + 1), vid(1, j)]);
        faces.push([bottom, vid(stacks - 1, j), vid(stacks - 1, j + 1)]);
    }
    (vertices, faces)
}

fn run_both(options: &DecimateMeshOptions) -> (DecimateMeshState, DecimateMeshState) {
    let (vertices, faces) = uv_sphere(34, 64, 5.0);
    assert!(
        faces.len() >= 2_000,
        "sphere too small to trigger fast path"
    );
    let region = vec![true; faces.len()];
    let empty_nf = BTreeSet::new();
    let none_etc: Option<BTreeSet<[usize; 2]>> = None;
    let empty_twin = BTreeMap::new();

    let reference = decimate_mesh_serial_state_reference(
        vertices.clone(),
        faces.clone(),
        region.clone(),
        region.clone(),
        options,
        &empty_nf,
        &none_etc,
        &empty_twin,
        None,
        None,
    );
    let fast = fast::decimate_mesh_serial_state_fast(
        vertices,
        faces,
        region.clone(),
        region,
        options,
        empty_nf,
        none_etc,
        empty_twin,
        None,
        None,
    );
    (reference, fast)
}

fn assert_equivalent(reference: &DecimateMeshState, fast: &DecimateMeshState, label: &str) {
    assert_eq!(
        reference.verts_deleted, fast.verts_deleted,
        "{label}: verts_deleted mismatch"
    );
    assert_eq!(
        reference.faces_deleted, fast.faces_deleted,
        "{label}: faces_deleted mismatch"
    );
    assert_eq!(
        reference.faces.len(),
        fast.faces.len(),
        "{label}: face count mismatch"
    );
    assert_eq!(
        reference.faces, fast.faces,
        "{label}: face topology mismatch"
    );
    assert_eq!(
        reference.vertices.len(),
        fast.vertices.len(),
        "{label}: vertex count mismatch"
    );
    for (index, (lhs, rhs)) in reference
        .vertices
        .iter()
        .zip(fast.vertices.iter())
        .enumerate()
    {
        assert_eq!(lhs, rhs, "{label}: vertex {index} position mismatch");
    }
}

#[test]
fn fast_matches_reference_minimize_error_target_count() {
    let mut options = DecimateMeshOptions {
        strategy: DecimateMeshStrategy::MinimizeError,
        max_deleted_faces: 1_500,
        ..DecimateMeshOptions::default()
    };
    options.max_error = f64::MAX;
    let (reference, fast) = run_both(&options);
    assert!(reference.faces_deleted > 0, "no collapses happened");
    assert_equivalent(&reference, &fast, "minimize_error/target_count");
}

#[test]
fn fast_matches_reference_minimize_error_max_error() {
    let options = DecimateMeshOptions {
        strategy: DecimateMeshStrategy::MinimizeError,
        max_error: 0.05,
        max_deleted_faces: 4_000,
        ..DecimateMeshOptions::default()
    };
    let (reference, fast) = run_both(&options);
    assert!(reference.faces_deleted > 0, "no collapses happened");
    assert_equivalent(&reference, &fast, "minimize_error/max_error");
}

#[test]
fn fast_matches_reference_shortest_edge_first() {
    let options = DecimateMeshOptions {
        strategy: DecimateMeshStrategy::ShortestEdgeFirst,
        max_error: 0.6,
        max_deleted_faces: 3_000,
        ..DecimateMeshOptions::default()
    };
    let (reference, fast) = run_both(&options);
    assert!(reference.faces_deleted > 0, "no collapses happened");
    assert_equivalent(&reference, &fast, "shortest_edge_first");
}

#[test]
fn fast_matches_reference_under_heavy_unbounded_reduction() {
    let options = DecimateMeshOptions {
        strategy: DecimateMeshStrategy::MinimizeError,
        max_error: 1e9,
        max_deleted_faces: 30_000,
        max_deleted_vertices: 20_000,
        ..DecimateMeshOptions::default()
    };
    let (reference, fast) = run_both(&options);
    assert!(
        reference.faces_deleted > 1_000,
        "expected a substantial reduction"
    );
    assert_equivalent(&reference, &fast, "heavy/unbounded");
}

#[test]
fn fast_matches_reference_with_aspect_and_edge_len_guards() {
    let options = DecimateMeshOptions {
        strategy: DecimateMeshStrategy::MinimizeError,
        max_error: f64::MAX,
        max_deleted_faces: 2_500,
        max_triangle_aspect_ratio: 8.0,
        max_edge_len: 3.0,
        ..DecimateMeshOptions::default()
    };
    let (reference, fast) = run_both(&options);
    assert_equivalent(&reference, &fast, "aspect+edge_len guards");
}
