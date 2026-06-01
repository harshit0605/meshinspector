use crate::math::{add, scale, sub};
use crate::mesh::{validate_faces, vertex_neighbor_list};
use crate::GeometryError;
use rayon::prelude::*;

pub fn taubin_smooth_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    iterations: i64,
    lamb: f64,
    nu: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let neighbors = vertex_neighbor_list(vertices.len(), &faces);
    let step_count = iterations.max(0) as usize;
    let shrink = if lamb.is_finite() {
        lamb.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let dilate = if nu.is_finite() { nu } else { 0.0 };
    let mut smoothed = vertices.to_vec();

    for pass in 0..step_count {
        let previous = smoothed.clone();
        let factor = if pass % 2 == 0 { shrink } else { -dilate };
        smoothed
            .par_iter_mut()
            .enumerate()
            .for_each(|(vertex_index, vertex)| {
                let neighbor_ids = &neighbors[vertex_index];
                if neighbor_ids.is_empty() {
                    return;
                }
                let mut target = [0.0_f64; 3];
                for neighbor_id in neighbor_ids {
                    target = add(target, previous[*neighbor_id]);
                }
                let mean = scale(target, 1.0 / neighbor_ids.len() as f64);
                *vertex = add(
                    previous[vertex_index],
                    scale(sub(mean, previous[vertex_index]), factor),
                );
            });
    }

    Ok(smoothed)
}
