pub use super::exact_boolean_assembly::{
    assemble_classified_boolean, exact_assemble_boolean_from_cut_meshes,
};
use super::exact_boolean_candidate::{
    exact_boolean_paired_coplanar_pipeline_parts_from_cut_meshes,
    exact_boolean_pipeline_parts_from_cut_meshes, paired_coplanar_candidate_diagnostics_from_parts,
};
use super::exact_boolean_diagnostics::{
    exact_boolean_pipeline_diagnostics, ExactBooleanPipelineDiagnosticInputs,
};
#[cfg(test)]
pub(super) use super::exact_boolean_diagnostics::{
    meshlib_result_cut_path_summary, MeshlibResultCutPathSummary,
};
pub use super::exact_boolean_diagnostics::{
    ExactBooleanPipelineDiagnostics, MeshlibFaceExportFailureDiagnostic,
    MeshlibNearStitchFailureDiagnostic, MeshlibNearStitchLinkedEdgeDiagnostic,
    MeshlibNearStitchRingDiagnostic, MeshlibNearStitchSourceLookupDiagnostic,
    MeshlibNearStitchTargetSnapshotDiagnostic, MeshlibPreparedSourceRecordReplayDiagnostic,
};
use super::exact_cut_pair::exact_mesh_pair_cut_meshes;
use super::exact_fill_apply::ExactCutHoleFillResult;
use super::exact_splice::ExactTopologySplicePlan;
use super::exact_splice_apply::ExactTopologySpliceApplyPlan;
use super::exact_stitch::{ExactStitchPath, ExactStitchPlan};
use crate::mesh::mesh_stats;
use crate::GeometryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactBooleanOperation {
    Union,
    Intersection,
    DifferenceAB,
    DifferenceBA,
    InsideA,
    InsideB,
    OutsideA,
    OutsideB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExactBooleanOperand {
    First,
    Second,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactBooleanOutputFaceSource {
    pub operand: ExactBooleanOperand,
    pub cut_face: usize,
    pub source_face: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactBooleanStitchedEdgeSource {
    pub output_edge: [usize; 2],
    pub first_output_edge: Option<[usize; 2]>,
    pub second_output_edge: Option<[usize; 2]>,
    pub first_stitch_edge: Option<[usize; 2]>,
    pub second_stitch_edge: Option<[usize; 2]>,
    pub first_stitch_edge_synthetic: bool,
    pub second_stitch_edge_synthetic: bool,
    pub first_edge_index: usize,
    pub second_edge_index: usize,
    pub first_cut_edge: [usize; 2],
    pub second_cut_edge: [usize; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactBooleanAssemblyResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub face_sources: Vec<ExactBooleanOutputFaceSource>,
    pub first_output_vertex_for_cut_vertex: Vec<Option<usize>>,
    pub second_output_vertex_for_cut_vertex: Vec<Option<usize>>,
    pub stitched_edge_sources: Vec<ExactBooleanStitchedEdgeSource>,
    pub stitched_edge_paths: Vec<ExactStitchPath>,
    pub prepare_first_faces: Vec<usize>,
    pub prepare_second_faces: Vec<usize>,
    pub selected_first_faces: Vec<usize>,
    pub selected_second_faces: Vec<usize>,
    pub flipped_first: bool,
    pub flipped_second: bool,
    pub first_cut_paths_consistent: bool,
    pub second_cut_paths_consistent: bool,
    pub first_cut_path_side_components: [usize; 2],
    pub second_cut_path_side_components: [usize; 2],
    pub first_cut_path_overlap_components: usize,
    pub second_cut_path_overlap_components: usize,
    pub result_cut_paths: Vec<Vec<[usize; 2]>>,
    pub result_cut_path_closed: Vec<bool>,
    pub result_cut_paths_complete: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactBooleanPipelineResult {
    pub first_cut: ExactCutHoleFillResult,
    pub second_cut: ExactCutHoleFillResult,
    pub stitch_plan: ExactStitchPlan,
    pub topology_splice_plan: ExactTopologySplicePlan,
    pub topology_splice_apply_plan: ExactTopologySpliceApplyPlan,
    pub diagnostics: ExactBooleanPipelineDiagnostics,
    pub assembly: ExactBooleanAssemblyResult,
}

pub fn exact_boolean_from_meshes(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    operation: ExactBooleanOperation,
    leaf_size: usize,
    epsilon: f64,
) -> Result<ExactBooleanPipelineResult, GeometryError> {
    let cut_meshes = exact_mesh_pair_cut_meshes(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    let active_parts = exact_boolean_pipeline_parts_from_cut_meshes(
        &cut_meshes.first,
        &cut_meshes.second,
        operation,
        epsilon,
    )?;
    let mut paired_coplanar_candidate_parts = cut_meshes
        .paired_coplanar_candidate
        .as_ref()
        .map(|candidate| {
            exact_boolean_paired_coplanar_pipeline_parts_from_cut_meshes(
                &candidate.first,
                &candidate.second,
                operation,
                epsilon,
            )
        })
        .transpose()?;
    let paired_coplanar_candidate_diagnostics = paired_coplanar_candidate_parts
        .as_ref()
        .map(|parts| paired_coplanar_candidate_diagnostics_from_parts(parts, epsilon))
        .transpose()?;
    let active_output_stats = mesh_stats(
        &active_parts.assembly.vertices,
        &active_parts.assembly.faces,
    )?;
    let active_output_volume = active_output_stats.volume_mm3;
    let active_output_area = active_output_stats.surface_area_mm2;
    let use_paired_coplanar_candidate =
        paired_coplanar_candidate_diagnostics
            .as_ref()
            .is_some_and(|diagnostics| {
                diagnostics.promotable_against(
                    active_output_volume,
                    active_output_area,
                    epsilon,
                    operation,
                )
            });
    let parts = if use_paired_coplanar_candidate {
        paired_coplanar_candidate_parts
            .take()
            .expect("paired candidate diagnostics require paired candidate parts")
    } else {
        active_parts
    };
    let diagnostics = exact_boolean_pipeline_diagnostics(ExactBooleanPipelineDiagnosticInputs {
        first_cut: &parts.first_cut,
        second_cut: &parts.second_cut,
        stitch_plan: &parts.stitch_plan,
        topology_splice_plan: &parts.topology_splice_plan,
        topology_splice_apply_plan: &parts.topology_splice_apply_plan,
        assembly: &parts.assembly,
        coplanar_cut_trial: cut_meshes.coplanar_cut_trial.as_ref(),
        paired_coplanar_candidate: paired_coplanar_candidate_diagnostics.as_ref(),
        active_output_volume,
        operation,
        epsilon,
    })?;
    Ok(ExactBooleanPipelineResult {
        first_cut: parts.first_cut,
        second_cut: parts.second_cut,
        stitch_plan: parts.stitch_plan,
        topology_splice_plan: parts.topology_splice_plan,
        topology_splice_apply_plan: parts.topology_splice_apply_plan,
        diagnostics,
        assembly: parts.assembly,
    })
}

#[cfg(test)]
mod tests;
