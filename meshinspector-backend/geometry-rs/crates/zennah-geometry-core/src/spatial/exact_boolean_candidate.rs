use super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperation};
use super::exact_boolean_assembly::{
    exact_assemble_boolean_from_cut_meshes_with_stitch,
    exact_assemble_difference_with_coplanar_sampling,
    exact_assemble_intersection_with_coplanar_sampling,
    exact_assemble_union_with_coplanar_first_wins,
};
use super::exact_boolean_paths::mapped_prepare_result_cut_paths;
use super::exact_cut_apply::ExactCutMeshResult;
use super::exact_cut_pair::ExactCutShadowRepairPath;
use super::exact_fill_apply::{exact_fill_cut_holes, ExactCutHoleFillResult};
use super::exact_splice::{exact_topology_splice_plan, ExactTopologySplicePlan};
use super::exact_splice_apply::{exact_topology_splice_apply_plan, ExactTopologySpliceApplyPlan};
use super::exact_stitch::{exact_stitch_plan_from_cut_meshes, ExactStitchPlan};
use crate::mesh::{mesh_health, mesh_stats};
use crate::GeometryError;
use std::collections::BTreeMap;

const PAIRED_COPLANAR_SELF_INTERSECTION_FACE_BUDGET: usize = 20_000;

pub(super) struct ExactBooleanPipelineParts {
    pub(super) first_cut: ExactCutHoleFillResult,
    pub(super) second_cut: ExactCutHoleFillResult,
    pub(super) stitch_plan: ExactStitchPlan,
    pub(super) topology_splice_plan: ExactTopologySplicePlan,
    pub(super) topology_splice_apply_plan: ExactTopologySpliceApplyPlan,
    pub(super) assembly: ExactBooleanAssemblyResult,
    pub(super) first_shadow_repair_paths: Vec<ExactCutShadowRepairPath>,
    pub(super) second_shadow_repair_paths: Vec<ExactCutShadowRepairPath>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct PairedCoplanarCandidateDiagnostics {
    pub(super) stitch_compatible: bool,
    pub(super) first_prepare_part_dividable: bool,
    pub(super) second_prepare_part_dividable: bool,
    pub(super) prepare_first_face_indices: Vec<usize>,
    pub(super) prepare_second_face_indices: Vec<usize>,
    pub(super) selected_first_face_indices: Vec<usize>,
    pub(super) selected_second_face_indices: Vec<usize>,
    pub(super) first_cut_path_side_components: [usize; 2],
    pub(super) second_cut_path_side_components: [usize; 2],
    pub(super) first_cut_path_overlap_components: usize,
    pub(super) second_cut_path_overlap_components: usize,
    pub(super) result_cut_paths_complete: bool,
    pub(super) prepare_result_cut_paths_complete: bool,
    pub(super) output_faces: usize,
    pub(super) output_area: f64,
    pub(super) output_volume: f64,
    pub(super) output_self_intersections: Option<usize>,
    pub(super) output_self_intersections_available: bool,
    pub(super) boundary_edges: usize,
    pub(super) nonmanifold_edges: usize,
    pub(super) duplicate_output_faces: usize,
}

impl PairedCoplanarCandidateDiagnostics {
    pub(super) fn topology_promotable(&self) -> bool {
        let result_cut_paths_ready =
            self.prepare_result_cut_paths_complete || self.result_cut_paths_complete;
        self.stitch_compatible
            && self.first_prepare_part_dividable
            && self.second_prepare_part_dividable
            && result_cut_paths_ready
            && self.boundary_edges == 0
            && self.nonmanifold_edges == 0
            && self.duplicate_output_faces == 0
    }

    pub(super) fn promotable_against(
        &self,
        reference_volume: f64,
        reference_area: f64,
        epsilon: f64,
    ) -> bool {
        self.topology_promotable()
            && self.preserves_reference_volume(reference_volume, epsilon)
            && self.preserves_reference_area(reference_area, epsilon)
    }

    pub(super) fn preserves_reference_volume(&self, reference_volume: f64, epsilon: f64) -> bool {
        let tolerance = (epsilon.abs() * 1_000_000.0).max(1e-6) * reference_volume.abs().max(1.0);
        (self.output_volume - reference_volume).abs() <= tolerance
    }

    fn preserves_reference_area(&self, reference_area: f64, epsilon: f64) -> bool {
        let tolerance = (epsilon.abs() * 1_000_000.0).max(1e-6) * reference_area.abs().max(1.0);
        (self.output_area - reference_area).abs() <= tolerance
    }
}

pub(super) fn exact_boolean_pipeline_parts_from_cut_meshes(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> Result<ExactBooleanPipelineParts, GeometryError> {
    let first_cut = exact_fill_cut_holes(first, epsilon)?;
    let second_cut = exact_fill_cut_holes(second, epsilon)?;
    let stitch_plan = exact_stitch_plan_from_cut_meshes(&first_cut.mesh, &second_cut.mesh, epsilon);
    let assembly = exact_assemble_boolean_from_cut_meshes_with_stitch(
        &first_cut.mesh,
        &second_cut.mesh,
        Some(&stitch_plan),
        operation,
        epsilon,
    )?;
    let topology_splice_plan =
        exact_topology_splice_plan(&assembly.faces, &assembly.stitched_edge_sources);
    let topology_splice_apply_plan = exact_topology_splice_apply_plan(
        &assembly.faces,
        &topology_splice_plan,
        &assembly.stitched_edge_paths,
    );

    Ok(ExactBooleanPipelineParts {
        first_cut,
        second_cut,
        stitch_plan,
        topology_splice_plan,
        topology_splice_apply_plan,
        assembly,
        first_shadow_repair_paths: Vec::new(),
        second_shadow_repair_paths: Vec::new(),
    })
}

pub(super) fn exact_boolean_paired_coplanar_pipeline_parts_from_cut_meshes(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    first_shadow_repair_paths: Vec<ExactCutShadowRepairPath>,
    second_shadow_repair_paths: Vec<ExactCutShadowRepairPath>,
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> Result<ExactBooleanPipelineParts, GeometryError> {
    let first_cut = exact_fill_cut_holes(first, epsilon)?;
    let second_cut = exact_fill_cut_holes(second, epsilon)?;
    let stitch_plan = exact_stitch_plan_from_cut_meshes(&first_cut.mesh, &second_cut.mesh, epsilon);
    let assembly = match operation {
        ExactBooleanOperation::Union => exact_assemble_union_with_coplanar_first_wins(
            &first_cut.mesh,
            &second_cut.mesh,
            Some(&stitch_plan),
            epsilon,
        )?,
        ExactBooleanOperation::Intersection => exact_assemble_intersection_with_coplanar_sampling(
            &first_cut.mesh,
            &second_cut.mesh,
            Some(&stitch_plan),
            epsilon,
        )?,
        ExactBooleanOperation::DifferenceAB | ExactBooleanOperation::DifferenceBA => {
            exact_assemble_difference_with_coplanar_sampling(
                &first_cut.mesh,
                &second_cut.mesh,
                Some(&stitch_plan),
                operation,
                epsilon,
            )?
        }
        _ => {
            return exact_boolean_pipeline_parts_from_cut_meshes(first, second, operation, epsilon);
        }
    };
    let topology_splice_plan =
        exact_topology_splice_plan(&assembly.faces, &assembly.stitched_edge_sources);
    let topology_splice_apply_plan = exact_topology_splice_apply_plan(
        &assembly.faces,
        &topology_splice_plan,
        &assembly.stitched_edge_paths,
    );

    Ok(ExactBooleanPipelineParts {
        first_cut,
        second_cut,
        stitch_plan,
        topology_splice_plan,
        topology_splice_apply_plan,
        assembly,
        first_shadow_repair_paths,
        second_shadow_repair_paths,
    })
}

pub(super) fn paired_coplanar_candidate_diagnostics_from_parts(
    parts: &ExactBooleanPipelineParts,
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> Result<PairedCoplanarCandidateDiagnostics, GeometryError> {
    let assembly = &parts.assembly;
    let stats = mesh_stats(&assembly.vertices, &assembly.faces)?;
    let health = mesh_health(
        &assembly.vertices,
        &assembly.faces,
        true,
        Some(PAIRED_COPLANAR_SELF_INTERSECTION_FACE_BUDGET),
        epsilon,
    )?;
    let (_, duplicate_output_faces) = duplicate_face_counts(&assembly.faces);
    let prepare_result_cut = mapped_prepare_result_cut_paths(
        operation,
        &parts.first_cut.mesh,
        &parts.second_cut.mesh,
        &assembly.prepare_first_faces,
        &assembly.prepare_second_faces,
    );

    Ok(PairedCoplanarCandidateDiagnostics {
        stitch_compatible: parts.stitch_plan.compatible,
        first_prepare_part_dividable: assembly.first_cut_paths_consistent,
        second_prepare_part_dividable: assembly.second_cut_paths_consistent,
        prepare_first_face_indices: assembly.prepare_first_faces.clone(),
        prepare_second_face_indices: assembly.prepare_second_faces.clone(),
        selected_first_face_indices: assembly.selected_first_faces.clone(),
        selected_second_face_indices: assembly.selected_second_faces.clone(),
        first_cut_path_side_components: assembly.first_cut_path_side_components,
        second_cut_path_side_components: assembly.second_cut_path_side_components,
        first_cut_path_overlap_components: assembly.first_cut_path_overlap_components,
        second_cut_path_overlap_components: assembly.second_cut_path_overlap_components,
        result_cut_paths_complete: assembly.result_cut_paths_complete,
        prepare_result_cut_paths_complete: prepare_result_cut.complete,
        output_faces: assembly.faces.len(),
        output_area: stats.surface_area_mm2,
        output_volume: stats.volume_mm3,
        output_self_intersections: health.self_intersections,
        output_self_intersections_available: health.self_intersections_available,
        boundary_edges: health.boundary_edge_count,
        nonmanifold_edges: health.nonmanifold_edge_count,
        duplicate_output_faces,
    })
}

fn duplicate_face_counts(faces: &[[i64; 3]]) -> (usize, usize) {
    let mut counts = BTreeMap::<[i64; 3], usize>::new();
    for face in faces {
        let mut sorted = *face;
        sorted.sort_unstable();
        *counts.entry(sorted).or_default() += 1;
    }
    let groups = counts.values().filter(|&&count| count > 1).count();
    let duplicates = counts.values().map(|count| count.saturating_sub(1)).sum();
    (groups, duplicates)
}
