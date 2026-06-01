use super::exact_boolean::ExactBooleanOutputFaceSource;
use super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
use super::exact_meshlib_near_stitch::{
    ExactMeshlibNearStitchEdgeUpdateCommand, ExactMeshlibNearStitchEndpoint,
};
use super::exact_splice_apply::{
    output_topology_from_prepared_base, ExactMeshlibPreparedBaseTopologyInput,
};
use super::exact_splice_apply::{
    ExactMeshlibCopiedEdgeTranslationInput, ExactMeshlibFaceExportFailureDiagnostic,
    ExactMeshlibNearStitchCandidateDiagnostics, ExactMeshlibPreparedSourceRecordReplayDiagnostic,
    ExactMeshlibRecordRewriteTargetDiagnostic, OutputFaceTopology,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExactMeshlibRecordRewriteApplyStatus {
    Applied,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactMeshlibRecordRewriteApplyEntry {
    pub command: ExactMeshlibRecordRewriteCommand,
    pub status: ExactMeshlibRecordRewriteApplyStatus,
    pub error: Option<&'static str>,
    pub target_diagnostic: Option<ExactMeshlibRecordRewriteTargetDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExactMeshlibNearStitchUpdateStatus {
    Applied,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactMeshlibNearStitchUpdateEntry {
    pub command: ExactMeshlibNearStitchEdgeUpdateCommand,
    pub status: ExactMeshlibNearStitchUpdateStatus,
    pub error: Option<&'static str>,
    pub candidate_diagnostics: Option<ExactMeshlibNearStitchCandidateDiagnostics>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactMeshlibRecordRewriteApplyPlan {
    pub entries: Vec<ExactMeshlibRecordRewriteApplyEntry>,
    pub near_stitch_update_entries: Vec<ExactMeshlibNearStitchUpdateEntry>,
    pub commands: usize,
    pub applied_commands: usize,
    pub failed_commands: usize,
    pub failed_missing_target_edges: usize,
    pub failed_closed_target_edges: usize,
    pub failed_missing_source_edges: usize,
    pub failed_other_commands: usize,
    pub near_stitch_update_commands: usize,
    pub applied_near_stitch_updates: usize,
    pub failed_near_stitch_updates: usize,
    pub failed_near_stitch_start_updates: usize,
    pub failed_near_stitch_end_updates: usize,
    pub failed_missing_near_stitch_previous_edges: usize,
    pub failed_missing_near_stitch_next_edges: usize,
    pub failed_near_stitch_origin_mismatches: usize,
    pub failed_near_stitch_previous_left_faces: usize,
    pub failed_near_stitch_next_right_faces: usize,
    pub failed_other_near_stitch_updates: usize,
    pub prepared_synthetic_target_edges: usize,
    pub translated_face_records: usize,
    pub translated_copied_edge_records: usize,
    pub translated_copied_face_records: usize,
    pub mapped_source_record_replays: usize,
    pub mapped_source_record_replays_on_near_stitch_targets: usize,
    pub mapped_source_record_replay_attempts: usize,
    pub mapped_source_record_replay_attempts_on_near_stitch_targets: usize,
    pub skipped_mapped_source_record_replays: usize,
    pub mapped_source_record_replay_details: Vec<ExactMeshlibPreparedSourceRecordReplayDiagnostic>,
    pub failed_copied_edge_records: usize,
    pub refreshed_face_records: usize,
    pub synthetic_side_edges: usize,
    pub exported_faces: usize,
    pub export_failed_faces: usize,
    pub export_non_triangular_faces: usize,
    pub export_left_ring_not_closed_faces: usize,
    pub export_missing_origin_faces: usize,
    pub export_face_record_left_mismatch_faces: usize,
    pub export_face_left_ring_mismatch_faces: usize,
    pub export_other_failed_faces: usize,
    pub export_failed_face_indices: Vec<usize>,
    pub export_failed_face_details: Vec<ExactMeshlibFaceExportFailureDiagnostic>,
    pub exported_face_indices: Vec<[i64; 3]>,
    pub topology_edges_before_rewrite: usize,
    pub topology_edges_after_rewrite: usize,
    pub export_changed_faces: bool,
    pub ready_for_export: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactMeshlibPreparedBaseRecordRewriteApplyPlan {
    pub apply: ExactMeshlibRecordRewriteApplyPlan,
    pub prepared_faces: usize,
    pub prepared_vertices: usize,
    pub virtual_vertices: usize,
    pub prepared_face_sources: usize,
}

#[cfg(test)]
pub(super) fn exact_meshlib_record_rewrite_apply_plan(
    faces: &[[i64; 3]],
    commands: &[ExactMeshlibRecordRewriteCommand],
    near_stitch_updates: &[ExactMeshlibNearStitchEdgeUpdateCommand],
) -> ExactMeshlibRecordRewriteApplyPlan {
    exact_meshlib_record_rewrite_apply_plan_from_topology(
        faces,
        OutputFaceTopology::from_faces(faces),
        commands,
        near_stitch_updates,
        None,
    )
}

#[cfg(test)]
pub(super) fn exact_meshlib_record_rewrite_apply_plan_with_sources(
    faces: &[[i64; 3]],
    face_sources: &[ExactBooleanOutputFaceSource],
    commands: &[ExactMeshlibRecordRewriteCommand],
    near_stitch_updates: &[ExactMeshlibNearStitchEdgeUpdateCommand],
) -> ExactMeshlibRecordRewriteApplyPlan {
    exact_meshlib_record_rewrite_apply_plan_from_topology(
        faces,
        OutputFaceTopology::from_faces_with_sources(faces, face_sources),
        commands,
        near_stitch_updates,
        None,
    )
}

pub(super) fn exact_meshlib_record_rewrite_apply_plan_with_copied_edges(
    faces: &[[i64; 3]],
    face_sources: &[ExactBooleanOutputFaceSource],
    commands: &[ExactMeshlibRecordRewriteCommand],
    near_stitch_updates: &[ExactMeshlibNearStitchEdgeUpdateCommand],
    copied_edges: ExactMeshlibCopiedEdgeTranslationInput<'_>,
) -> ExactMeshlibRecordRewriteApplyPlan {
    exact_meshlib_record_rewrite_apply_plan_from_topology(
        faces,
        OutputFaceTopology::from_faces_with_sources(faces, face_sources),
        commands,
        near_stitch_updates,
        Some(copied_edges),
    )
}

#[cfg(test)]
pub(super) fn exact_meshlib_record_rewrite_apply_plan_with_prepared_base(
    base: ExactMeshlibPreparedBaseTopologyInput<'_>,
    commands: &[ExactMeshlibRecordRewriteCommand],
    near_stitch_updates: &[ExactMeshlibNearStitchEdgeUpdateCommand],
    copied_edges: Option<ExactMeshlibCopiedEdgeTranslationInput<'_>>,
) -> ExactMeshlibRecordRewriteApplyPlan {
    exact_meshlib_prepared_base_record_rewrite_apply_plan(
        base,
        commands,
        near_stitch_updates,
        copied_edges,
    )
    .apply
}

pub(super) fn exact_meshlib_prepared_base_record_rewrite_apply_plan(
    base: ExactMeshlibPreparedBaseTopologyInput<'_>,
    commands: &[ExactMeshlibRecordRewriteCommand],
    near_stitch_updates: &[ExactMeshlibNearStitchEdgeUpdateCommand],
    copied_edges: Option<ExactMeshlibCopiedEdgeTranslationInput<'_>>,
) -> ExactMeshlibPreparedBaseRecordRewriteApplyPlan {
    match output_topology_from_prepared_base(base) {
        Ok(prepared) => {
            let prepared_faces = prepared.faces.len();
            let prepared_vertices = prepared.vertices.len();
            let virtual_vertices = prepared.virtual_vertices;
            let prepared_face_sources = prepared.face_sources.len();
            let copied_edges = copied_edges.map(|mut copied_edges| {
                copied_edges.first_virtual_vertex = prepared.vertices.len();
                copied_edges
            });
            let apply = exact_meshlib_record_rewrite_apply_plan_from_topology(
                &prepared.faces,
                Ok(prepared.topology),
                commands,
                near_stitch_updates,
                copied_edges,
            );
            ExactMeshlibPreparedBaseRecordRewriteApplyPlan {
                apply,
                prepared_faces,
                prepared_vertices,
                virtual_vertices,
                prepared_face_sources,
            }
        }
        Err(error) => {
            let apply = exact_meshlib_record_rewrite_apply_plan_from_topology(
                &[],
                Err(error),
                commands,
                near_stitch_updates,
                copied_edges,
            );
            ExactMeshlibPreparedBaseRecordRewriteApplyPlan {
                apply,
                prepared_faces: 0,
                prepared_vertices: 0,
                virtual_vertices: 0,
                prepared_face_sources: 0,
            }
        }
    }
}

fn exact_meshlib_record_rewrite_apply_plan_from_topology(
    faces: &[[i64; 3]],
    mut output_topology: Result<OutputFaceTopology, &'static str>,
    commands: &[ExactMeshlibRecordRewriteCommand],
    near_stitch_updates: &[ExactMeshlibNearStitchEdgeUpdateCommand],
    copied_edges: Option<ExactMeshlibCopiedEdgeTranslationInput<'_>>,
) -> ExactMeshlibRecordRewriteApplyPlan {
    let topology_edges_before_rewrite = output_topology
        .as_ref()
        .map(OutputFaceTopology::not_lone_undirected_edge_count)
        .unwrap_or(0);
    let mut entries = Vec::with_capacity(commands.len());
    let mut applied_commands = 0;
    let mut failed_commands = 0;
    let mut failed_missing_target_edges = 0;
    let mut failed_closed_target_edges = 0;
    let mut failed_missing_source_edges = 0;
    let mut failed_other_commands = 0;
    let mut near_stitch_update_entries = Vec::with_capacity(near_stitch_updates.len());
    let mut applied_near_stitch_updates = 0;
    let mut failed_near_stitch_updates = 0;
    let mut failed_near_stitch_start_updates = 0;
    let mut failed_near_stitch_end_updates = 0;
    let mut failed_missing_near_stitch_previous_edges = 0;
    let mut failed_missing_near_stitch_next_edges = 0;
    let mut failed_near_stitch_origin_mismatches = 0;
    let mut failed_near_stitch_previous_left_faces = 0;
    let mut failed_near_stitch_next_right_faces = 0;
    let mut failed_other_near_stitch_updates = 0;
    let synthetic_side_edges = commands.iter().map(|command| command.synthetic_sides).sum();

    let mut failed_copied_edge_records = 0;
    let mut prepared_copied_edges = None;
    if let Some(copied_edges) = copied_edges {
        prepare_meshlib_record_rewrite_maps(&mut output_topology, commands);
        match output_topology
            .as_mut()
            .map_err(|error| *error)
            .and_then(|topology| topology.prepare_meshlib_copied_edges(copied_edges))
        {
            Ok(prepared) => {
                prepared_copied_edges = Some(prepared);
            }
            Err(_) => {
                failed_copied_edge_records = 1;
            }
        }
    }

    for command in commands {
        let result = output_topology
            .as_mut()
            .map_err(|error| *error)
            .and_then(|topology| topology.apply_meshlib_record_rewrite_command(command));
        let (status, error, target_diagnostic) = match result {
            Ok(target_diagnostic) => {
                applied_commands += 1;
                (
                    ExactMeshlibRecordRewriteApplyStatus::Applied,
                    None,
                    Some(target_diagnostic),
                )
            }
            Err(error) => {
                failed_commands += 1;
                match error {
                    "missing MeshLib rewrite target contour edge" => {
                        failed_missing_target_edges += 1;
                    }
                    "target contour edge must not have a left face" => {
                        failed_closed_target_edges += 1;
                    }
                    "missing MeshLib rewrite source contour edge" => {
                        failed_missing_source_edges += 1;
                    }
                    _ => {
                        failed_other_commands += 1;
                    }
                }
                (
                    ExactMeshlibRecordRewriteApplyStatus::Failed,
                    Some(error),
                    None,
                )
            }
        };
        entries.push(ExactMeshlibRecordRewriteApplyEntry {
            command: *command,
            status,
            error,
            target_diagnostic,
        });
    }

    if let Some(prepared) = prepared_copied_edges {
        match output_topology
            .as_mut()
            .map_err(|error| *error)
            .and_then(|topology| topology.finalize_meshlib_copied_edges(prepared))
        {
            Ok(_) => {}
            Err(_) => {
                failed_copied_edge_records = 1;
            }
        }
    }

    for command in near_stitch_updates {
        let result = output_topology
            .as_mut()
            .map_err(|error| *error)
            .and_then(|topology| topology.apply_meshlib_near_stitch_edge_update_command(command));
        let (status, error) = match result {
            Ok(()) => {
                applied_near_stitch_updates += 1;
                (ExactMeshlibNearStitchUpdateStatus::Applied, None)
            }
            Err(error) => {
                failed_near_stitch_updates += 1;
                match command.endpoint {
                    Some(ExactMeshlibNearStitchEndpoint::Start) => {
                        failed_near_stitch_start_updates += 1;
                    }
                    Some(ExactMeshlibNearStitchEndpoint::End) => {
                        failed_near_stitch_end_updates += 1;
                    }
                    None => {}
                }
                match error {
                    "missing MeshLib near stitch previous edge" => {
                        failed_missing_near_stitch_previous_edges += 1;
                    }
                    "missing MeshLib near stitch next edge" => {
                        failed_missing_near_stitch_next_edges += 1;
                    }
                    "near stitch edges must share origin" => {
                        failed_near_stitch_origin_mismatches += 1;
                    }
                    "previous near stitch edge must not have a left face" => {
                        failed_near_stitch_previous_left_faces += 1;
                    }
                    "next near stitch edge must not have a right face" => {
                        failed_near_stitch_next_right_faces += 1;
                    }
                    _ => {
                        failed_other_near_stitch_updates += 1;
                    }
                }
                (ExactMeshlibNearStitchUpdateStatus::Failed, Some(error))
            }
        };
        let candidate_diagnostics = match (status, output_topology.as_mut()) {
            (ExactMeshlibNearStitchUpdateStatus::Failed, Ok(topology)) => {
                topology.take_meshlib_near_stitch_candidate_diagnostics()
            }
            _ => None,
        };
        near_stitch_update_entries.push(ExactMeshlibNearStitchUpdateEntry {
            command: *command,
            status,
            error,
            candidate_diagnostics,
        });
    }

    let prepared_synthetic_target_edges = output_topology
        .as_ref()
        .map(|topology| topology.meshlib_synthetic_target_edges)
        .unwrap_or(0);
    let translated_face_records = output_topology
        .as_ref()
        .map(|topology| topology.meshlib_translated_face_records)
        .unwrap_or(0);
    let translated_copied_edge_records = output_topology
        .as_ref()
        .map(|topology| topology.meshlib_copied_edge_translation.translated_records)
        .unwrap_or(0);
    let translated_copied_face_records = output_topology
        .as_ref()
        .map(|topology| {
            topology
                .meshlib_copied_edge_translation
                .translated_face_records
        })
        .unwrap_or(0);
    let mapped_source_record_replays = output_topology
        .as_ref()
        .map(|topology| topology.meshlib_prepared_mapped_source_record_replays)
        .unwrap_or(0);
    let mapped_source_record_replays_on_near_stitch_targets = output_topology
        .as_ref()
        .map(|topology| {
            topology.meshlib_prepared_mapped_source_record_replays_on_near_stitch_targets
        })
        .unwrap_or(0);
    let mapped_source_record_replay_details = output_topology
        .as_ref()
        .map(|topology| {
            topology
                .meshlib_prepared_mapped_source_record_replay_details
                .clone()
        })
        .unwrap_or_default();
    let mapped_source_record_replay_attempts = mapped_source_record_replay_details.len();
    let mapped_source_record_replay_attempts_on_near_stitch_targets =
        mapped_source_record_replay_details
            .iter()
            .filter(|detail| detail.target_was_near_stitch_target)
            .count();
    let skipped_mapped_source_record_replays = mapped_source_record_replay_details
        .iter()
        .filter(|detail| !detail.applied)
        .count();
    failed_copied_edge_records += output_topology
        .as_ref()
        .map(|topology| topology.meshlib_copied_edge_translation.failed_records)
        .unwrap_or(0);
    let refreshed_face_records = output_topology
        .as_mut()
        .map(|topology| topology.refresh_meshlib_face_records())
        .unwrap_or(0);
    let export_results = output_topology
        .as_ref()
        .map(OutputFaceTopology::export_face_results)
        .unwrap_or_default();
    let export_non_triangular_faces = export_results
        .iter()
        .filter(|result| matches!(result, Err("exported face is not triangular")))
        .count();
    let export_left_ring_not_closed_faces = export_results
        .iter()
        .filter(|result| matches!(result, Err("left ring did not close")))
        .count();
    let export_missing_origin_faces = export_results
        .iter()
        .filter(|result| matches!(result, Err("left ring edge missing origin")))
        .count();
    let export_face_record_left_mismatch_faces = export_results
        .iter()
        .filter(|result| {
            matches!(
                result,
                Err("MeshLib face record edge must have face on left")
            )
        })
        .count();
    let export_face_left_ring_mismatch_faces = export_results
        .iter()
        .filter(|result| matches!(result, Err("MeshLib face left ring must keep same face")))
        .count();
    let export_failed_face_indices = export_results
        .iter()
        .enumerate()
        .filter_map(|(index, result)| result.is_err().then_some(index))
        .collect::<Vec<_>>();
    let export_failed_face_details = output_topology
        .as_ref()
        .map(OutputFaceTopology::export_face_failure_details)
        .unwrap_or_default();
    let export_other_failed_faces = export_failed_face_indices.len()
        - export_non_triangular_faces
        - export_left_ring_not_closed_faces
        - export_missing_origin_faces
        - export_face_record_left_mismatch_faces
        - export_face_left_ring_mismatch_faces;
    let exported_face_indices = export_results
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    let exported_faces = exported_face_indices.len();
    let export_failed_faces = if output_topology.is_ok() {
        export_failed_face_indices.len()
    } else {
        faces.len()
    };
    let topology_edges_after_rewrite = output_topology
        .as_ref()
        .map(OutputFaceTopology::not_lone_undirected_edge_count)
        .unwrap_or(0);
    let export_changed_faces = export_failed_faces == 0 && exported_face_indices != faces;

    ExactMeshlibRecordRewriteApplyPlan {
        entries,
        near_stitch_update_entries,
        commands: commands.len(),
        applied_commands,
        failed_commands,
        failed_missing_target_edges,
        failed_closed_target_edges,
        failed_missing_source_edges,
        failed_other_commands,
        near_stitch_update_commands: near_stitch_updates.len(),
        applied_near_stitch_updates,
        failed_near_stitch_updates,
        failed_near_stitch_start_updates,
        failed_near_stitch_end_updates,
        failed_missing_near_stitch_previous_edges,
        failed_missing_near_stitch_next_edges,
        failed_near_stitch_origin_mismatches,
        failed_near_stitch_previous_left_faces,
        failed_near_stitch_next_right_faces,
        failed_other_near_stitch_updates,
        prepared_synthetic_target_edges,
        translated_face_records,
        translated_copied_edge_records,
        translated_copied_face_records,
        mapped_source_record_replays,
        mapped_source_record_replays_on_near_stitch_targets,
        mapped_source_record_replay_attempts,
        mapped_source_record_replay_attempts_on_near_stitch_targets,
        skipped_mapped_source_record_replays,
        mapped_source_record_replay_details,
        failed_copied_edge_records,
        refreshed_face_records,
        synthetic_side_edges,
        exported_faces,
        export_failed_faces,
        export_non_triangular_faces,
        export_left_ring_not_closed_faces,
        export_missing_origin_faces,
        export_face_record_left_mismatch_faces,
        export_face_left_ring_mismatch_faces,
        export_other_failed_faces,
        export_failed_face_indices,
        export_failed_face_details,
        exported_face_indices,
        topology_edges_before_rewrite,
        topology_edges_after_rewrite,
        export_changed_faces,
        ready_for_export: failed_commands == 0
            && failed_copied_edge_records == 0
            && failed_near_stitch_updates == 0
            && export_failed_faces == 0,
    }
}

fn prepare_meshlib_record_rewrite_maps(
    output_topology: &mut Result<OutputFaceTopology, &'static str>,
    commands: &[ExactMeshlibRecordRewriteCommand],
) {
    for command in commands {
        let prepare_result = if command.from_side_synthetic {
            output_topology
                .as_mut()
                .map_err(|error| *error)
                .and_then(|topology| topology.prepare_meshlib_record_rewrite_target_map(command))
        } else {
            output_topology
                .as_mut()
                .map_err(|error| *error)
                .and_then(|topology| topology.prepare_meshlib_record_rewrite_command_map(command))
        };
        if prepare_result.is_err() {
            let _ = output_topology
                .as_mut()
                .map_err(|error| *error)
                .and_then(|topology| topology.prepare_meshlib_record_rewrite_target_map(command));
        }
    }
}
