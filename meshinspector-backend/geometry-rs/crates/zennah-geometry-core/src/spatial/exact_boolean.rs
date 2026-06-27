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
    ExactBooleanPipelineDiagnostics, MeshlibCopiedFaceRecordCandidateDiagnostic,
    MeshlibCopiedFaceRecordDiagnostic, MeshlibCopiedPrevNextEdgeUpdateDiagnostic,
    MeshlibFaceExportFailureDiagnostic, MeshlibNearStitchFailureDiagnostic,
    MeshlibNearStitchLinkedEdgeDiagnostic, MeshlibNearStitchRingDiagnostic,
    MeshlibNearStitchSourceLookupDiagnostic, MeshlibNearStitchTargetSnapshotDiagnostic,
    MeshlibPreparedBaseRecordRewriteDiagnostics, MeshlibPreparedSourceRecordReplayDiagnostic,
    MeshlibRecordRewriteFailedCommandDiagnostic, MeshlibRecordRewriteTargetDiagnostic,
};
use super::exact_boolean_output::{exact_boolean_public_output_mesh, ExactBooleanOutputMeshInput};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactBooleanOutputMeshSource {
    Assembly,
    MeshlibPreparedBaseExport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactBooleanOutputMesh {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub source: ExactBooleanOutputMeshSource,
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
    pub output: ExactBooleanOutputMesh,
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
                candidate.first_shadow_repair_paths.clone(),
                candidate.second_shadow_repair_paths.clone(),
                operation,
                epsilon,
            )
        })
        .transpose()?;
    let paired_coplanar_candidate_diagnostics = paired_coplanar_candidate_parts
        .as_ref()
        .map(|parts| paired_coplanar_candidate_diagnostics_from_parts(parts, operation, epsilon))
        .transpose()?;
    let active_output_stats = mesh_stats(
        &active_parts.assembly.vertices,
        &active_parts.assembly.faces,
    )?;
    let active_output_volume = active_output_stats.volume_mm3;
    let active_output_area = active_output_stats.surface_area_mm2;
    let mut paired_coplanar_candidate_output = paired_coplanar_candidate_parts
        .as_ref()
        .map(|parts| {
            exact_boolean_public_output_mesh(ExactBooleanOutputMeshInput {
                first_cut: &parts.first_cut,
                second_cut: &parts.second_cut,
                assembly: &parts.assembly,
                operation,
                epsilon,
            })
        })
        .transpose()?;
    let use_paired_coplanar_candidate =
        paired_coplanar_candidate_diagnostics
            .as_ref()
            .is_some_and(|diagnostics| {
                paired_coplanar_candidate_topology_promotable(operation, diagnostics)
                    || diagnostics.promotable_against(
                        active_output_volume,
                        active_output_area,
                        epsilon,
                    )
            })
            || paired_coplanar_candidate_output
                .as_ref()
                .is_some_and(|output| {
                    paired_public_output_promotable(
                        output,
                        active_output_volume,
                        active_output_area,
                        epsilon,
                    )
                });
    let (parts, output) =
        if use_paired_coplanar_candidate {
            let parts = paired_coplanar_candidate_parts
                .take()
                .expect("paired candidate promotion requires paired candidate parts");
            let output = paired_coplanar_candidate_output.take().unwrap_or(
                exact_boolean_public_output_mesh(ExactBooleanOutputMeshInput {
                    first_cut: &parts.first_cut,
                    second_cut: &parts.second_cut,
                    assembly: &parts.assembly,
                    operation,
                    epsilon,
                })?,
            );
            (parts, output)
        } else {
            let output = exact_boolean_public_output_mesh(ExactBooleanOutputMeshInput {
                first_cut: &active_parts.first_cut,
                second_cut: &active_parts.second_cut,
                assembly: &active_parts.assembly,
                operation,
                epsilon,
            })?;
            (active_parts, output)
        };
    let paired_coplanar_candidate_parts_for_diagnostics = if use_paired_coplanar_candidate {
        Some(&parts)
    } else {
        paired_coplanar_candidate_parts.as_ref()
    };
    let diagnostics = exact_boolean_pipeline_diagnostics(ExactBooleanPipelineDiagnosticInputs {
        original_first_vertices: first_vertices,
        original_first_faces: first_faces_i64,
        original_second_vertices: second_vertices,
        original_second_faces: second_faces_i64,
        first_cut: &parts.first_cut,
        second_cut: &parts.second_cut,
        stitch_plan: &parts.stitch_plan,
        topology_splice_plan: &parts.topology_splice_plan,
        topology_splice_apply_plan: &parts.topology_splice_apply_plan,
        assembly: &parts.assembly,
        output: &output,
        coplanar_cut_trial: cut_meshes.coplanar_cut_trial.as_ref(),
        paired_coplanar_candidate: paired_coplanar_candidate_diagnostics.as_ref(),
        paired_coplanar_candidate_parts: paired_coplanar_candidate_parts_for_diagnostics,
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
        output,
    })
}

fn paired_public_output_promotable(
    output: &ExactBooleanOutputMesh,
    reference_volume: f64,
    reference_area: f64,
    epsilon: f64,
) -> bool {
    output.source == ExactBooleanOutputMeshSource::MeshlibPreparedBaseExport
        && mesh_stats(&output.vertices, &output.faces).is_ok_and(|stats| {
            metric_close(stats.volume_mm3, reference_volume, epsilon)
                && metric_close(stats.surface_area_mm2, reference_area, epsilon)
        })
}

fn paired_coplanar_candidate_topology_promotable(
    operation: ExactBooleanOperation,
    diagnostics: &super::exact_boolean_candidate::PairedCoplanarCandidateDiagnostics,
) -> bool {
    matches!(
        operation,
        ExactBooleanOperation::Union | ExactBooleanOperation::Intersection
    ) && diagnostics.topology_promotable()
}

fn metric_close(value: f64, reference: f64, epsilon: f64) -> bool {
    let tolerance = (epsilon.abs() * 1_000_000.0).max(1e-6) * reference.abs().max(1.0);
    (value - reference).abs() <= tolerance
}

#[cfg(test)]
mod tests;
