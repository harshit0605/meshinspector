mod coplanar_trial;
mod source_preserving;
mod types;

#[cfg(test)]
mod tests;

use coplanar_trial::coplanar_contour_cut_trial;
pub use types::ExactMeshPairCutMeshes;
pub(in crate::spatial) use types::{ExactCoplanarContourCutTrial, ExactCutShadowRepairPath};

use super::exact_cut_apply::{exact_cut_mesh_by_contours, ExactCutMeshResult};
use super::exact_lone_retry::exact_lone_subdivision_pair_prepass;
use super::exact_one_mesh::exact_one_mesh_intersection_contours;
use crate::GeometryError;

pub fn exact_mesh_pair_cut_meshes(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<ExactMeshPairCutMeshes, GeometryError> {
    let prepass = exact_lone_subdivision_pair_prepass(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    let retry_had_subdivisions =
        !prepass.first_subdivisions.is_empty() || !prepass.second_subdivisions.is_empty();
    let mut retry = ExactMeshPairCutMeshes {
        first: exact_cut_mesh_by_contours(
            &prepass.first_vertices,
            &prepass.first_faces,
            &prepass.contours.first,
            epsilon,
        )?,
        second: exact_cut_mesh_by_contours(
            &prepass.second_vertices,
            &prepass.second_faces,
            &prepass.contours.second,
            epsilon,
        )?,
        coplanar_cut_trial: None,
        paired_coplanar_candidate: None,
    };
    remap_cut_source_faces(&mut retry.first, &prepass.first_source_face_for_faces);
    remap_cut_source_faces(&mut retry.second, &prepass.second_source_face_for_faces);

    let needs_baseline_check = retry_had_subdivisions && retry_needs_baseline_check(&retry);
    let mut stable = if needs_baseline_check {
        let baseline = baseline_cut_meshes(
            first_vertices,
            first_faces_i64,
            second_vertices,
            second_faces_i64,
            leaf_size,
            epsilon,
        )?;
        if retry_lost_cut_edges(&baseline, &retry) || retry_has_new_skipped_faces(&baseline, &retry)
        {
            baseline
        } else {
            retry
        }
    } else {
        retry
    };

    if let Some(trial) = coplanar_contour_cut_trial(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
        &stable,
    )? {
        if trial.summary.accepted {
            return Ok(ExactMeshPairCutMeshes {
                first: trial.first,
                second: trial.second,
                coplanar_cut_trial: Some(trial.summary),
                paired_coplanar_candidate: trial.paired_candidate,
            });
        }
        stable.coplanar_cut_trial = Some(trial.summary);
        stable.paired_coplanar_candidate = trial.paired_candidate;
    }

    Ok(stable)
}

fn baseline_cut_meshes(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<ExactMeshPairCutMeshes, GeometryError> {
    let contours = exact_one_mesh_intersection_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    Ok(ExactMeshPairCutMeshes {
        first: exact_cut_mesh_by_contours(
            first_vertices,
            first_faces_i64,
            &contours.first,
            epsilon,
        )?,
        second: exact_cut_mesh_by_contours(
            second_vertices,
            second_faces_i64,
            &contours.second,
            epsilon,
        )?,
        coplanar_cut_trial: None,
        paired_coplanar_candidate: None,
    })
}

fn retry_lost_cut_edges(baseline: &ExactMeshPairCutMeshes, retry: &ExactMeshPairCutMeshes) -> bool {
    (!baseline.first.cut_edges.is_empty() && retry.first.cut_edges.is_empty())
        || (!baseline.second.cut_edges.is_empty() && retry.second.cut_edges.is_empty())
}

fn retry_needs_baseline_check(retry: &ExactMeshPairCutMeshes) -> bool {
    retry.first.cut_edges.is_empty()
        || retry.second.cut_edges.is_empty()
        || !retry.first.skipped_source_faces.is_empty()
        || !retry.second.skipped_source_faces.is_empty()
}

fn retry_has_new_skipped_faces(
    baseline: &ExactMeshPairCutMeshes,
    retry: &ExactMeshPairCutMeshes,
) -> bool {
    (!retry.first.skipped_source_faces.is_empty() && baseline.first.skipped_source_faces.is_empty())
        || (!retry.second.skipped_source_faces.is_empty()
            && baseline.second.skipped_source_faces.is_empty())
}

fn remap_cut_source_faces(cut_mesh: &mut ExactCutMeshResult, source_map: &[usize]) {
    for source_face in &mut cut_mesh.source_face_for_faces {
        if let Some(mapped) = source_map.get(*source_face) {
            *source_face = *mapped;
        }
    }
    for event in &mut cut_mesh.cut_face_source_events {
        if let Some(mapped) = source_map.get(event.source_face) {
            event.source_face = *mapped;
        }
    }
    for skipped_face in &mut cut_mesh.skipped_source_faces {
        if let Some(mapped) = source_map.get(*skipped_face) {
            *skipped_face = *mapped;
        }
    }
    for path_source_faces in &mut cut_mesh.cut_edge_path_source_faces {
        for source_face in path_source_faces.iter_mut().flatten() {
            if let Some(mapped) = source_map.get(*source_face) {
                *source_face = *mapped;
            }
        }
    }
    for path_source_faces in &mut cut_mesh.collapsed_cut_segment_path_source_faces {
        for source_face in path_source_faces.iter_mut().flatten() {
            if let Some(mapped) = source_map.get(*source_face) {
                *source_face = *mapped;
            }
        }
    }
    cut_mesh.skipped_source_faces.sort_unstable();
    cut_mesh.skipped_source_faces.dedup();
}
