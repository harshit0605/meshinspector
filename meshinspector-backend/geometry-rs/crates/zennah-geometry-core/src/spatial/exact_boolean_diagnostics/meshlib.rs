use super::super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOperand, ExactBooleanOperation,
    ExactBooleanOutputFaceSource,
};
use super::super::exact_boolean_topology::{
    meshlib_topology_rewrite_plan, ExactMeshlibTopologyRewritePlan,
};
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_meshlib_near_stitch::{
    exact_meshlib_near_stitch_plan_with_prepared_parts, exact_meshlib_near_stitch_plan_with_source,
    ExactMeshlibNearStitchPlan, ExactMeshlibNearStitchSourceInput,
};
use super::super::exact_meshlib_rewrite_apply::{
    exact_meshlib_prepared_base_record_rewrite_apply_plan,
    exact_meshlib_record_rewrite_apply_plan_with_copied_edges,
    ExactMeshlibPreparedBaseRecordRewriteApplyPlan, ExactMeshlibRecordRewriteApplyPlan,
};
use super::super::exact_splice_apply::{
    ExactMeshlibCopiedEdgeTranslationInput, ExactMeshlibPreparedBaseTopologyInput,
};
use super::super::exact_stitch::{ExactStitchEdgePair, ExactStitchPlan};
use super::copied_edges::{
    exact_meshlib_copied_edge_plan, exact_meshlib_copied_edge_translation_input,
    exact_meshlib_near_stitch_source_input, ExactMeshlibCopiedEdgePlan,
};
use super::export::{mesh_export_health, mesh_export_stats, packed_mesh_export};
use crate::{GeometryError, MeshHealth, MeshStats};
mod copied_records;
mod near_stitch;
pub use copied_records::{
    MeshlibCopiedFaceRecordCandidateDiagnostic, MeshlibCopiedFaceRecordDiagnostic,
    MeshlibCopiedPrevNextEdgeUpdateDiagnostic,
};
pub(super) use near_stitch::near_stitch_failure_details;
pub use near_stitch::{
    MeshlibNearStitchFailureDiagnostic, MeshlibNearStitchLinkedEdgeDiagnostic,
    MeshlibNearStitchRingDiagnostic, MeshlibNearStitchSourceLookupDiagnostic,
    MeshlibNearStitchTargetSnapshotDiagnostic,
};
mod prepared_base_export;
pub(in crate::spatial) use prepared_base_export::meshlib_prepared_base_export;
mod prepared_base_records;
pub use prepared_base_records::{
    MeshlibFaceExportFailureDiagnostic, MeshlibPreparedBaseRecordRewriteDiagnostics,
    MeshlibPreparedSourceRecordReplayDiagnostic, MeshlibRecordRewriteFailedCommandDiagnostic,
    MeshlibRecordRewriteTargetDiagnostic,
};
mod prepared_parts;
use prepared_parts::{
    meshlib_prepare_connect_parts, MeshlibPreparedConnectParts, MeshlibPreparedConnectPartsSummary,
};

pub(in crate::spatial) struct MeshlibRewriteDiagnosticsInput<'a> {
    pub(in crate::spatial) first_cut: &'a ExactCutMeshResult,
    pub(in crate::spatial) second_cut: &'a ExactCutMeshResult,
    pub(in crate::spatial) first_added_face_ranges: &'a [[usize; 2]],
    pub(in crate::spatial) second_added_face_ranges: &'a [[usize; 2]],
    pub(in crate::spatial) stitch_plan: Option<&'a ExactStitchPlan>,
    pub(in crate::spatial) assembly: &'a ExactBooleanAssemblyResult,
    pub(in crate::spatial) operation: ExactBooleanOperation,
    pub(in crate::spatial) epsilon: f64,
}

pub(super) struct MeshlibRewriteDiagnostics {
    pub topology_rewrite: ExactMeshlibTopologyRewritePlan,
    pub copied_edges: ExactMeshlibCopiedEdgePlan,
    pub near_stitch: ExactMeshlibNearStitchPlan,
    pub record_rewrite_apply: ExactMeshlibRecordRewriteApplyPlan,
    pub prepared_connect_summary: MeshlibPreparedConnectSummaryDiagnostics,
    pub prepared_base_record_rewrite: MeshlibPreparedBaseRecordRewriteDiagnostics,
    pub record_rewrite_exported_mesh_stats: Option<MeshStats>,
    pub record_rewrite_exported_mesh_health: Option<MeshHealth>,
    pub record_rewrite_packed_mesh_stats: Option<MeshStats>,
    pub record_rewrite_packed_mesh_health: Option<MeshHealth>,
}

struct MeshlibPreparedBaseRewritePlan {
    apply: ExactMeshlibPreparedBaseRecordRewriteApplyPlan,
    near_stitch: ExactMeshlibNearStitchPlan,
    prepared_connect_summary: MeshlibPreparedConnectSummaryDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MeshlibPreparedConnectSummaryDiagnostics {
    pub(super) base_faces: usize,
    pub(super) incoming_faces: usize,
    pub(super) base_vertices: usize,
    pub(super) incoming_vertices: usize,
    pub(super) unstitched_vertices: usize,
    pub(super) unstitched_faces: usize,
    pub(super) path_pairs: usize,
    pub(super) path_count_mismatch: bool,
    pub(super) path_length_mismatches: usize,
    pub(super) path_closed_mismatches: usize,
    pub(super) path_coordinate_mismatches: usize,
    pub(super) path_same_direction_edges: usize,
    pub(super) path_reversed_edges: usize,
    pub(super) base_mapped_cut_path_edges: usize,
    pub(super) incoming_mapped_cut_path_edges: usize,
    pub(super) base_missing_cut_path_edges: usize,
    pub(super) incoming_missing_cut_path_edges: usize,
    pub(super) base_cut_paths_complete: bool,
    pub(super) incoming_cut_paths_complete: bool,
}

impl From<MeshlibPreparedConnectPartsSummary> for MeshlibPreparedConnectSummaryDiagnostics {
    fn from(summary: MeshlibPreparedConnectPartsSummary) -> Self {
        Self {
            base_faces: summary.base_faces,
            incoming_faces: summary.incoming_faces,
            base_vertices: summary.base_vertices,
            incoming_vertices: summary.incoming_vertices,
            unstitched_vertices: summary.unstitched_vertices,
            unstitched_faces: summary.unstitched_faces,
            path_pairs: summary.path_pairs,
            path_count_mismatch: summary.path_count_mismatch,
            path_length_mismatches: summary.path_length_mismatches,
            path_closed_mismatches: summary.path_closed_mismatches,
            path_coordinate_mismatches: summary.path_coordinate_mismatches,
            path_same_direction_edges: summary.path_same_direction_edges,
            path_reversed_edges: summary.path_reversed_edges,
            base_mapped_cut_path_edges: summary.base_mapped_cut_path_edges,
            incoming_mapped_cut_path_edges: summary.incoming_mapped_cut_path_edges,
            base_missing_cut_path_edges: summary.base_missing_cut_path_edges,
            incoming_missing_cut_path_edges: summary.incoming_missing_cut_path_edges,
            base_cut_paths_complete: summary.base_cut_paths_complete,
            incoming_cut_paths_complete: summary.incoming_cut_paths_complete,
        }
    }
}

fn operand_label(operand: ExactBooleanOperand) -> &'static str {
    match operand {
        ExactBooleanOperand::First => "first",
        ExactBooleanOperand::Second => "second",
    }
}

pub(super) fn meshlib_rewrite_diagnostics(
    input: MeshlibRewriteDiagnosticsInput<'_>,
) -> Result<MeshlibRewriteDiagnostics, GeometryError> {
    let topology_rewrite = meshlib_topology_rewrite_plan(input.assembly, input.operation);
    let copied_edges = exact_meshlib_copied_edge_plan(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
    );
    let near_stitch = exact_meshlib_near_stitch_plan_with_source(
        input.assembly,
        topology_rewrite.base_operand,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
        exact_meshlib_near_stitch_source_input(
            input.first_cut,
            input.second_cut,
            input.assembly,
            topology_rewrite.incoming_operand,
            &topology_rewrite.record_rewrite_command_edges,
        ),
    );
    let record_rewrite_apply = exact_meshlib_record_rewrite_apply_plan_with_copied_edges(
        &input.assembly.faces,
        &input.assembly.face_sources,
        &topology_rewrite.record_rewrite_command_edges,
        &near_stitch.commands,
        exact_meshlib_copied_edge_translation_input(
            input.first_cut,
            input.second_cut,
            input.assembly,
            topology_rewrite.incoming_operand,
            &topology_rewrite.record_rewrite_command_edges,
        ),
    );
    let prepared_base = meshlib_prepared_base_rewrite_plan(&input, &topology_rewrite);
    let prepared_base_exported_mesh_stats = mesh_export_stats(
        &prepared_base.apply.vertices,
        &prepared_base.apply.apply.exported_face_indices,
        prepared_base.apply.apply.export_failed_faces,
    )?;
    let prepared_base_exported_mesh_health = mesh_export_health(
        &prepared_base.apply.vertices,
        &prepared_base.apply.apply.exported_face_indices,
        prepared_base.apply.apply.export_failed_faces,
        input.epsilon,
        super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET,
    )?;
    let prepared_base_packed_export = packed_mesh_export(
        &prepared_base.apply.vertices,
        &prepared_base.apply.apply.exported_face_indices,
        prepared_base.apply.apply.export_failed_faces,
    )?;
    let prepared_base_packed_mesh_stats = if let Some(export) = &prepared_base_packed_export {
        crate::mesh::mesh_stats(&export.vertices, &export.faces).map(Some)?
    } else {
        None
    };
    let prepared_base_packed_mesh_health = if let Some(export) = &prepared_base_packed_export {
        crate::mesh::mesh_health(
            &export.vertices,
            &export.faces,
            true,
            Some(super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET),
            input.epsilon,
        )
        .map(Some)?
    } else {
        None
    };
    let prepared_base_record_rewrite = MeshlibPreparedBaseRecordRewriteDiagnostics::from_apply_plan(
        &prepared_base.apply,
        &prepared_base.near_stitch,
        prepared_base_exported_mesh_stats,
        prepared_base_exported_mesh_health,
        prepared_base_packed_mesh_stats,
        prepared_base_packed_mesh_health,
    );
    let record_rewrite_exported_mesh_stats = mesh_export_stats(
        &input.assembly.vertices,
        &record_rewrite_apply.exported_face_indices,
        record_rewrite_apply.export_failed_faces,
    )?;
    let record_rewrite_exported_mesh_health = mesh_export_health(
        &input.assembly.vertices,
        &record_rewrite_apply.exported_face_indices,
        record_rewrite_apply.export_failed_faces,
        input.epsilon,
        super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET,
    )?;
    let packed_export = packed_mesh_export(
        &input.assembly.vertices,
        &record_rewrite_apply.exported_face_indices,
        record_rewrite_apply.export_failed_faces,
    )?;
    let record_rewrite_packed_mesh_stats = if let Some(export) = &packed_export {
        crate::mesh::mesh_stats(&export.vertices, &export.faces).map(Some)?
    } else {
        None
    };
    let record_rewrite_packed_mesh_health = if let Some(export) = &packed_export {
        crate::mesh::mesh_health(
            &export.vertices,
            &export.faces,
            true,
            Some(super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET),
            input.epsilon,
        )
        .map(Some)?
    } else {
        None
    };

    Ok(MeshlibRewriteDiagnostics {
        topology_rewrite,
        copied_edges,
        near_stitch,
        record_rewrite_apply,
        prepared_connect_summary: prepared_base.prepared_connect_summary,
        prepared_base_record_rewrite,
        record_rewrite_exported_mesh_stats,
        record_rewrite_exported_mesh_health,
        record_rewrite_packed_mesh_stats,
        record_rewrite_packed_mesh_health,
    })
}

fn meshlib_prepared_base_rewrite_plan(
    input: &MeshlibRewriteDiagnosticsInput<'_>,
    topology_rewrite: &ExactMeshlibTopologyRewritePlan,
) -> MeshlibPreparedBaseRewritePlan {
    let empty_face_sources: &[ExactBooleanOutputFaceSource] = &[];
    let base_prepared_faces = filtered_prepared_faces(input, topology_rewrite.base_operand);
    let incoming_prepared_faces = filtered_prepared_faces(input, topology_rewrite.incoming_operand);
    let prepared_connect_parts = meshlib_prepare_connect_parts(input, topology_rewrite);
    let prepared_connect_summary = prepared_connect_parts.summary(input.epsilon, input.stitch_plan);
    debug_assert_eq!(
        prepared_connect_summary.base_faces,
        base_prepared_faces.len(),
        "MeshLib preparePart mirror must copy the same base face mask as the record rewrite path"
    );
    debug_assert_eq!(
        prepared_connect_summary.incoming_faces,
        incoming_prepared_faces.len(),
        "MeshLib preparePart mirror must copy the same incoming face mask as the record rewrite path"
    );
    let base_contour_vertex_maps =
        prepared_connect_base_contour_vertex_maps(input, topology_rewrite, &prepared_connect_parts);
    let (incoming_contour_vertex_maps, incoming_contour_vertex_map_source_indices) =
        prepared_connect_incoming_contour_vertex_maps(
            input,
            topology_rewrite,
            &prepared_connect_parts,
        );
    let prepared_base_vertex_count = prepared_connect_parts.base.vertices.len();
    let mut prepared_base_copied_edges = exact_meshlib_copied_edge_translation_input(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
    );
    prepared_base_copied_edges.vertex_map = &[];
    prepared_base_copied_edges.contour_vertex_maps = incoming_contour_vertex_maps.clone();
    prepared_base_copied_edges.contour_vertex_map_source_indices =
        incoming_contour_vertex_map_source_indices.clone();
    let prepared_base_copied_edges = ExactMeshlibCopiedEdgeTranslationInput {
        prepared_faces: &incoming_prepared_faces,
        face_sources: empty_face_sources,
        append_prepared_faces: true,
        first_virtual_vertex: prepared_base_vertex_count,
        ..prepared_base_copied_edges
    };
    let prepared_base_incoming_source = exact_meshlib_near_stitch_source_input(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
    );
    let prepared_base_incoming_source = ExactMeshlibNearStitchSourceInput {
        prepared_faces: &incoming_prepared_faces,
        vertex_map: &[],
        contour_vertex_maps: incoming_contour_vertex_maps,
        contour_vertex_map_source_indices: incoming_contour_vertex_map_source_indices,
        first_virtual_vertex: prepared_base_vertex_count,
        ..prepared_base_incoming_source
    };
    let base_contour_vertex_map_source_indices =
        prepared_connect_contour_source_indices(input, topology_rewrite.base_operand);
    let prepared_base_source = exact_meshlib_near_stitch_source_input(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.base_operand,
        &topology_rewrite.record_rewrite_command_edges,
    );
    let prepared_base_source = ExactMeshlibNearStitchSourceInput {
        prepared_faces: &base_prepared_faces,
        vertex_map: &prepared_connect_parts.base.cut_to_part_vertices,
        contour_vertex_maps: base_contour_vertex_maps.clone(),
        contour_vertex_map_source_indices: base_contour_vertex_map_source_indices,
        first_virtual_vertex: prepared_base_vertex_count,
        ..prepared_base_source
    };
    let near_stitch = exact_meshlib_near_stitch_plan_with_prepared_parts(
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
        prepared_base_source,
        prepared_base_incoming_source,
    );
    let apply = exact_meshlib_prepared_base_record_rewrite_apply_plan(
        prepared_base_topology_input(
            input.first_cut,
            input.second_cut,
            input.assembly,
            topology_rewrite.base_operand,
            &base_prepared_faces,
            Some(&prepared_connect_parts),
            base_contour_vertex_maps,
        ),
        &topology_rewrite.record_rewrite_command_edges,
        &near_stitch.commands,
        Some(prepared_base_copied_edges),
    );
    MeshlibPreparedBaseRewritePlan {
        apply,
        near_stitch,
        prepared_connect_summary: prepared_connect_summary.into(),
    }
}

fn prepared_base_topology_input<'a>(
    first_cut: &'a ExactCutMeshResult,
    second_cut: &'a ExactCutMeshResult,
    assembly: &'a ExactBooleanAssemblyResult,
    base_operand: ExactBooleanOperand,
    prepared_faces: &'a [usize],
    prepared_connect_parts: Option<&'a MeshlibPreparedConnectParts>,
    contour_vertex_maps: Vec<([usize; 2], [usize; 2])>,
) -> ExactMeshlibPreparedBaseTopologyInput<'a> {
    if let Some(prepared_connect_parts) = prepared_connect_parts {
        let cut_mesh = match base_operand {
            ExactBooleanOperand::First => first_cut,
            ExactBooleanOperand::Second => second_cut,
        };
        return ExactMeshlibPreparedBaseTopologyInput {
            cut_mesh,
            prepared_faces,
            vertex_map: &prepared_connect_parts.base.cut_to_part_vertices,
            contour_vertex_maps,
            output_vertices: &prepared_connect_parts.base.vertices,
            operand: base_operand,
            first_virtual_vertex: prepared_connect_parts.base.vertices.len(),
            flip_orientation: match base_operand {
                ExactBooleanOperand::First => assembly.flipped_first,
                ExactBooleanOperand::Second => assembly.flipped_second,
            },
        };
    }
    let (cut_mesh, vertex_map, flip_orientation) = match base_operand {
        ExactBooleanOperand::First => (
            first_cut,
            assembly.first_output_vertex_for_cut_vertex.as_slice(),
            assembly.flipped_first,
        ),
        ExactBooleanOperand::Second => (
            second_cut,
            assembly.second_output_vertex_for_cut_vertex.as_slice(),
            assembly.flipped_second,
        ),
    };
    ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh,
        prepared_faces,
        vertex_map,
        contour_vertex_maps,
        output_vertices: &assembly.vertices,
        operand: base_operand,
        first_virtual_vertex: assembly.vertices.len(),
        flip_orientation,
    }
}

fn prepared_connect_base_contour_vertex_maps(
    input: &MeshlibRewriteDiagnosticsInput<'_>,
    topology_rewrite: &ExactMeshlibTopologyRewritePlan,
    prepared_connect_parts: &MeshlibPreparedConnectParts,
) -> Vec<([usize; 2], [usize; 2])> {
    let cut_mesh = operand_cut_mesh(input, topology_rewrite.base_operand);
    prepared_part_self_contour_vertex_maps(cut_mesh, &prepared_connect_parts.base)
}

fn prepared_connect_incoming_contour_vertex_maps(
    input: &MeshlibRewriteDiagnosticsInput<'_>,
    topology_rewrite: &ExactMeshlibTopologyRewritePlan,
    prepared_connect_parts: &MeshlibPreparedConnectParts,
) -> (Vec<([usize; 2], [usize; 2])>, Vec<Option<usize>>) {
    let cut_mesh = operand_cut_mesh(input, topology_rewrite.incoming_operand);
    if let Some(stitch_plan) = input.stitch_plan {
        return prepared_part_connect_contour_vertex_maps_from_stitch(
            stitch_plan,
            topology_rewrite.base_operand,
            topology_rewrite.incoming_operand,
            &prepared_connect_parts.base,
            &prepared_connect_parts.incoming,
        );
    }
    (
        prepared_part_connect_contour_vertex_maps(
            cut_mesh,
            &prepared_connect_parts.base,
            &prepared_connect_parts.incoming,
        ),
        prepared_connect_contour_source_indices(input, topology_rewrite.incoming_operand),
    )
}

fn prepared_part_self_contour_vertex_maps(
    cut_mesh: &ExactCutMeshResult,
    part: &prepared_parts::MeshlibPreparedPart,
) -> Vec<([usize; 2], [usize; 2])> {
    cut_mesh
        .cut_edge_paths
        .iter()
        .zip(&part.remapped_cut_edge_paths)
        .flat_map(|(source_path, mapped_path)| {
            source_path.iter().copied().zip(mapped_path.iter().copied())
        })
        .collect()
}

fn prepared_part_connect_contour_vertex_maps(
    incoming_cut_mesh: &ExactCutMeshResult,
    base: &prepared_parts::MeshlibPreparedPart,
    incoming: &prepared_parts::MeshlibPreparedPart,
) -> Vec<([usize; 2], [usize; 2])> {
    incoming_cut_mesh
        .cut_edge_paths
        .iter()
        .zip(&base.remapped_cut_edge_paths)
        .zip(&incoming.remapped_cut_edge_paths)
        .flat_map(|((incoming_source_path, base_path), incoming_path)| {
            incoming_source_path
                .iter()
                .copied()
                .zip(base_path.iter().copied())
                .take(incoming_path.len())
        })
        .collect()
}

fn prepared_part_connect_contour_vertex_maps_from_stitch(
    stitch_plan: &ExactStitchPlan,
    base_operand: ExactBooleanOperand,
    incoming_operand: ExactBooleanOperand,
    base: &prepared_parts::MeshlibPreparedPart,
    incoming: &prepared_parts::MeshlibPreparedPart,
) -> (Vec<([usize; 2], [usize; 2])>, Vec<Option<usize>>) {
    let mut maps = Vec::new();
    let mut source_indices = Vec::new();
    for path in &stitch_plan.paths {
        for pair_index in &path.pair_indices {
            let Some(pair) = stitch_plan.pairs.get(*pair_index) else {
                continue;
            };
            let base_edge = stitch_pair_edge_for_operand(pair, base_operand);
            let incoming_edge = stitch_pair_edge_for_operand(pair, incoming_operand);
            let Some(output_edge) = base.cut_to_part_directed_edges.get(&base_edge).copied() else {
                continue;
            };
            if !incoming
                .cut_to_part_directed_edges
                .contains_key(&incoming_edge)
            {
                continue;
            }
            maps.push((incoming_edge, output_edge));
            source_indices.push(Some(stitch_pair_edge_index_for_operand(
                pair,
                incoming_operand,
            )));
        }
    }
    (maps, source_indices)
}

fn prepared_connect_contour_source_indices(
    input: &MeshlibRewriteDiagnosticsInput<'_>,
    operand: ExactBooleanOperand,
) -> Vec<Option<usize>> {
    cut_path_edge_source_indices(operand_cut_mesh(input, operand))
}

fn stitch_pair_edge_for_operand(
    pair: &ExactStitchEdgePair,
    operand: ExactBooleanOperand,
) -> [usize; 2] {
    match operand {
        ExactBooleanOperand::First => pair.first_edge,
        ExactBooleanOperand::Second => pair.second_edge,
    }
}

fn stitch_pair_edge_index_for_operand(
    pair: &ExactStitchEdgePair,
    operand: ExactBooleanOperand,
) -> usize {
    match operand {
        ExactBooleanOperand::First => pair.first_edge_index,
        ExactBooleanOperand::Second => pair.second_edge_index,
    }
}

fn cut_path_edge_source_indices(cut_mesh: &ExactCutMeshResult) -> Vec<Option<usize>> {
    let mut lookup = CutEdgeOccurrenceLookup::new(&cut_mesh.cut_edges);
    let mut indices = Vec::new();
    for path in &cut_mesh.cut_edge_paths {
        for edge in path {
            indices.push(lookup.take(*edge));
        }
    }
    indices
}

fn operand_cut_mesh<'a>(
    input: &'a MeshlibRewriteDiagnosticsInput<'_>,
    operand: ExactBooleanOperand,
) -> &'a ExactCutMeshResult {
    match operand {
        ExactBooleanOperand::First => input.first_cut,
        ExactBooleanOperand::Second => input.second_cut,
    }
}

struct CutEdgeOccurrenceLookup {
    directed: std::collections::BTreeMap<[usize; 2], Vec<usize>>,
    undirected: std::collections::BTreeMap<[usize; 2], Vec<usize>>,
    used: std::collections::BTreeSet<usize>,
}

impl CutEdgeOccurrenceLookup {
    fn new(cut_edges: &[[usize; 2]]) -> Self {
        let mut lookup = Self {
            directed: std::collections::BTreeMap::new(),
            undirected: std::collections::BTreeMap::new(),
            used: std::collections::BTreeSet::new(),
        };
        for (index, edge) in cut_edges.iter().copied().enumerate() {
            lookup.directed.entry(edge).or_default().push(index);
            lookup
                .undirected
                .entry(ordered_edge(edge))
                .or_default()
                .push(index);
        }
        lookup
    }

    fn take(&mut self, edge: [usize; 2]) -> Option<usize> {
        self.take_from_indices(self.directed.get(&edge).cloned())
            .or_else(|| self.take_from_indices(self.undirected.get(&ordered_edge(edge)).cloned()))
    }

    fn take_from_indices(&mut self, indices: Option<Vec<usize>>) -> Option<usize> {
        indices?.into_iter().find(|index| self.used.insert(*index))
    }
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn filtered_prepared_faces(
    input: &MeshlibRewriteDiagnosticsInput<'_>,
    operand: ExactBooleanOperand,
) -> Vec<usize> {
    let (prepared_faces, added_face_ranges) = match operand {
        ExactBooleanOperand::First => (
            input.assembly.prepare_first_faces.as_slice(),
            input.first_added_face_ranges,
        ),
        ExactBooleanOperand::Second => (
            input.assembly.prepare_second_faces.as_slice(),
            input.second_added_face_ranges,
        ),
    };
    prepared_faces
        .iter()
        .copied()
        .filter(|face| !face_in_ranges(*face, added_face_ranges))
        .collect()
}

fn face_in_ranges(face: usize, ranges: &[[usize; 2]]) -> bool {
    ranges
        .iter()
        .any(|[start, end]| (*start..*end).contains(&face))
}
