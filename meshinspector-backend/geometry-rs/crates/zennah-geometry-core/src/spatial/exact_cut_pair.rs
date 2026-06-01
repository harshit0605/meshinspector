use super::exact_coplanar::coplanar_overlap_contours;
use super::exact_cut_apply::{exact_cut_mesh_by_contours, ExactCutMeshResult};
use super::exact_lone_retry::{
    exact_lone_subdivision_pair_prepass, exact_pair_intersection_contours_with_coplanar,
};
use super::exact_one_mesh::{exact_one_mesh_intersection_contours, ExactOneMeshContours};
use super::exact_stitch::exact_stitch_plan_from_cut_meshes;
use crate::GeometryError;

#[derive(Debug, Clone, PartialEq)]
pub struct ExactMeshPairCutMeshes {
    pub first: ExactCutMeshResult,
    pub second: ExactCutMeshResult,
    pub(super) coplanar_cut_trial: Option<ExactCoplanarContourCutTrial>,
    pub(super) paired_coplanar_candidate: Option<ExactCoplanarCutCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactCoplanarContourCutTrial {
    pub contours: usize,
    pub contour_edges: usize,
    pub first_cut_edges: usize,
    pub second_cut_edges: usize,
    pub paired_contours: usize,
    pub paired_contour_edges: usize,
    pub paired_first_cut_edges: usize,
    pub paired_second_cut_edges: usize,
    pub paired_stitch_cut_path_length_mismatches: usize,
    pub paired_stitch_unmatched_first_edges: usize,
    pub paired_stitch_unmatched_second_edges: usize,
    pub paired_duplicate_first_path_edges: usize,
    pub paired_duplicate_second_path_edges: usize,
    pub first_skipped_source_faces: Vec<usize>,
    pub second_skipped_source_faces: Vec<usize>,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ExactCoplanarCutCandidate {
    pub first: ExactCutMeshResult,
    pub second: ExactCutMeshResult,
}

struct CoplanarContourCutTrialResult {
    first: ExactCutMeshResult,
    second: ExactCutMeshResult,
    summary: ExactCoplanarContourCutTrial,
    paired_candidate: ExactCoplanarCutCandidate,
}

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
                paired_coplanar_candidate: Some(trial.paired_candidate),
            });
        }
        stable.coplanar_cut_trial = Some(trial.summary);
        stable.paired_coplanar_candidate = Some(trial.paired_candidate);
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

fn coplanar_contour_cut_trial(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
    stable: &ExactMeshPairCutMeshes,
) -> Result<Option<CoplanarContourCutTrialResult>, GeometryError> {
    let coplanar = coplanar_overlap_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        epsilon,
    )?;
    let contours = coplanar.merged_contours.first.len();
    if contours == 0 {
        return Ok(None);
    }
    let contour_edges = coplanar
        .merged_contours
        .first
        .iter()
        .map(|contour| contour.intersections.len())
        .sum();
    let combined = exact_pair_intersection_contours_with_coplanar(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    let first =
        exact_cut_mesh_by_contours(first_vertices, first_faces_i64, &combined.first, epsilon)?;
    let second =
        exact_cut_mesh_by_contours(second_vertices, second_faces_i64, &combined.second, epsilon)?;
    let mut paired_combined = exact_one_mesh_intersection_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    append_one_mesh_contours(
        &mut paired_combined,
        coplanar.paired_merged_contours.clone(),
    );
    let paired_first = exact_cut_mesh_by_contours(
        first_vertices,
        first_faces_i64,
        &paired_combined.first,
        epsilon,
    )?;
    let paired_second = exact_cut_mesh_by_contours(
        second_vertices,
        second_faces_i64,
        &paired_combined.second,
        epsilon,
    )?;
    let paired_stitch_plan =
        exact_stitch_plan_from_cut_meshes(&paired_first, &paired_second, epsilon);
    let accepted = coplanar_trial_is_no_regression(stable, &first, &second);
    let summary = ExactCoplanarContourCutTrial {
        contours,
        contour_edges,
        first_cut_edges: first.cut_edges.len(),
        second_cut_edges: second.cut_edges.len(),
        paired_contours: coplanar.paired_merged_contours.first.len(),
        paired_contour_edges: paired_contour_edges(&coplanar.paired_merged_contours.first),
        paired_first_cut_edges: paired_first.cut_edges.len(),
        paired_second_cut_edges: paired_second.cut_edges.len(),
        paired_stitch_cut_path_length_mismatches: cut_path_length_mismatches(
            &paired_first.cut_edge_paths,
            &paired_second.cut_edge_paths,
        ),
        paired_stitch_unmatched_first_edges: paired_stitch_plan.unmatched_first_edges.len(),
        paired_stitch_unmatched_second_edges: paired_stitch_plan.unmatched_second_edges.len(),
        paired_duplicate_first_path_edges: duplicate_path_edges(&paired_first.cut_edge_paths),
        paired_duplicate_second_path_edges: duplicate_path_edges(&paired_second.cut_edge_paths),
        first_skipped_source_faces: first.skipped_source_faces.clone(),
        second_skipped_source_faces: second.skipped_source_faces.clone(),
        accepted,
    };
    Ok(Some(CoplanarContourCutTrialResult {
        first,
        second,
        summary,
        paired_candidate: ExactCoplanarCutCandidate {
            first: paired_first,
            second: paired_second,
        },
    }))
}

fn paired_contour_edges(contours: &[super::exact_one_mesh::ExactOneMeshContour]) -> usize {
    contours
        .iter()
        .map(|contour| contour.intersections.len())
        .sum()
}

fn append_one_mesh_contours(target: &mut ExactOneMeshContours, mut source: ExactOneMeshContours) {
    target.first.append(&mut source.first);
    target.second.append(&mut source.second);
    target
        .coordinates_in_first_space
        .append(&mut source.coordinates_in_first_space);
}

fn cut_path_length_mismatches(
    first_paths: &[Vec<[usize; 2]>],
    second_paths: &[Vec<[usize; 2]>],
) -> usize {
    let shared_mismatches = first_paths
        .iter()
        .zip(second_paths)
        .filter(|(first, second)| first.len() != second.len())
        .count();
    shared_mismatches + first_paths.len().abs_diff(second_paths.len())
}

fn duplicate_path_edges(paths: &[Vec<[usize; 2]>]) -> usize {
    paths
        .iter()
        .map(|path| {
            let mut seen = std::collections::BTreeSet::new();
            path.iter()
                .copied()
                .map(ordered_edge)
                .filter(|edge| !seen.insert(*edge))
                .count()
        })
        .sum()
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn coplanar_trial_is_no_regression(
    stable: &ExactMeshPairCutMeshes,
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
) -> bool {
    first.skipped_source_faces.is_empty()
        && second.skipped_source_faces.is_empty()
        && first.cut_edges.len() >= stable.first.cut_edges.len()
        && second.cut_edges.len() >= stable.second.cut_edges.len()
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
    for skipped_face in &mut cut_mesh.skipped_source_faces {
        if let Some(mapped) = source_map.get(*skipped_face) {
            *skipped_face = *mapped;
        }
    }
    cut_mesh.skipped_source_faces.sort_unstable();
    cut_mesh.skipped_source_faces.dedup();
}
