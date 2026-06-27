use super::super::exact_cut_apply::ExactCutFaceSourceEventKind;
use super::super::exact_fill_apply::ExactCutHoleFillResult;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeshlibPrefillRepairKind {
    OpenEdgeRestoration,
    OrphanRepair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MeshlibPrefillRepairRecord {
    pub kind: MeshlibPrefillRepairKind,
    pub path_index: usize,
    pub source_face: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct MeshlibCut2OriginShadowInventory {
    pub source_records: usize,
    pub unique_source_faces: usize,
    pub duplicate_source_records: usize,
    pub split_source_records: usize,
    pub cut_event_source_records: usize,
    pub split_event_source_records: usize,
    pub prefill_repair_source_records: usize,
    pub open_edge_restoration_source_records: usize,
    pub orphan_repair_source_records: usize,
    pub fill_source_records: usize,
    pub source_face_counts: Vec<[usize; 2]>,
    pub source_face_runs: Vec<[usize; 2]>,
    pub appended_source_face_runs: Vec<[usize; 2]>,
    pub prefill_repair_record_details: Vec<[usize; 2]>,
    pub open_edge_restoration_record_details: Vec<[usize; 2]>,
    pub orphan_repair_record_details: Vec<[usize; 2]>,
    pub source_face_sequence: Vec<usize>,
    pub appended_source_face_sequence: Vec<usize>,
    pub split_appended_source_face_sequence: Vec<usize>,
    pub cut_event_source_face_sequence: Vec<usize>,
    pub split_event_source_face_sequence: Vec<usize>,
    pub prefill_repair_source_face_sequence: Vec<usize>,
    pub open_edge_restoration_source_face_sequence: Vec<usize>,
    pub orphan_repair_source_face_sequence: Vec<usize>,
    pub fill_appended_source_face_sequence: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct MeshlibCut2OriginOwnerRemapDiagnostics {
    pub ready: bool,
    pub source_records: usize,
    pub matching_source_records: usize,
    pub mismatched_source_records: usize,
    pub missing_materialized_source_records: usize,
    pub extra_materialized_source_records: usize,
    pub mismatch_details: Vec<[usize; 3]>,
    pub source_face_sequence: Vec<usize>,
    pub appended_source_face_sequence: Vec<usize>,
    pub source_face_counts: Vec<[usize; 2]>,
    pub source_face_runs: Vec<[usize; 2]>,
    pub appended_source_face_runs: Vec<[usize; 2]>,
}

pub(super) fn meshlib_cut2origin_shadow_inventory(
    cut: &ExactCutHoleFillResult,
) -> MeshlibCut2OriginShadowInventory {
    let original_source_faces = inferred_original_source_face_count(cut);
    let prepared_source_face_sequence = (0..original_source_faces).collect::<Vec<_>>();
    let cut_event_source_face_sequence = cut_event_source_face_sequence(cut);
    let split_event_source_face_sequence = split_event_source_face_sequence(cut);
    let prefill_repair_records = meshlib_prefill_repair_records(cut);
    let materialized_prefill_fill_plans =
        materialized_prefill_fill_plan_indices(cut, &prefill_repair_records);
    let (split_appended_source_face_sequence, fill_appended_source_face_sequence) =
        appended_stage_source_face_sequences(
            cut,
            original_source_faces,
            &materialized_prefill_fill_plans,
        );
    let prefill_repair_record_details = prefill_repair_records
        .iter()
        .map(|record| [record.path_index, record.source_face])
        .collect::<Vec<_>>();
    let open_edge_restoration_record_details = prefill_repair_records
        .iter()
        .filter(|record| record.kind == MeshlibPrefillRepairKind::OpenEdgeRestoration)
        .map(|record| [record.path_index, record.source_face])
        .collect::<Vec<_>>();
    let orphan_repair_record_details = prefill_repair_records
        .iter()
        .filter(|record| record.kind == MeshlibPrefillRepairKind::OrphanRepair)
        .map(|record| [record.path_index, record.source_face])
        .collect::<Vec<_>>();
    let prefill_repair_source_face_sequence = prefill_repair_records
        .iter()
        .map(|record| record.source_face)
        .collect::<Vec<_>>();
    let open_edge_restoration_source_face_sequence = prefill_repair_records
        .iter()
        .filter(|record| record.kind == MeshlibPrefillRepairKind::OpenEdgeRestoration)
        .map(|record| record.source_face)
        .collect::<Vec<_>>();
    let orphan_repair_source_face_sequence = prefill_repair_records
        .iter()
        .filter(|record| record.kind == MeshlibPrefillRepairKind::OrphanRepair)
        .map(|record| record.source_face)
        .collect::<Vec<_>>();
    let mut appended_source_face_sequence = Vec::with_capacity(
        split_appended_source_face_sequence.len()
            + prefill_repair_source_face_sequence.len()
            + fill_appended_source_face_sequence.len(),
    );
    appended_source_face_sequence.extend(split_appended_source_face_sequence.iter().copied());
    appended_source_face_sequence.extend(prefill_repair_source_face_sequence.iter().copied());
    appended_source_face_sequence.extend(fill_appended_source_face_sequence.iter().copied());
    let mut source_face_sequence = Vec::with_capacity(
        prepared_source_face_sequence.len() + appended_source_face_sequence.len(),
    );
    source_face_sequence.extend(prepared_source_face_sequence);
    source_face_sequence.extend(appended_source_face_sequence.iter().copied());
    let source_face_counts = source_face_counts(&source_face_sequence);
    let full_source_face_runs = source_face_runs(&source_face_sequence);
    let appended_source_face_runs = source_face_runs(&appended_source_face_sequence);
    let source_records = source_face_sequence.len();

    MeshlibCut2OriginShadowInventory {
        source_records,
        unique_source_faces: source_face_counts.len(),
        duplicate_source_records: source_records.saturating_sub(source_face_counts.len()),
        split_source_records: split_appended_source_face_sequence.len(),
        cut_event_source_records: cut_event_source_face_sequence.len(),
        split_event_source_records: split_event_source_face_sequence.len(),
        prefill_repair_source_records: prefill_repair_source_face_sequence.len(),
        open_edge_restoration_source_records: open_edge_restoration_source_face_sequence.len(),
        orphan_repair_source_records: orphan_repair_source_face_sequence.len(),
        fill_source_records: fill_appended_source_face_sequence.len(),
        source_face_counts,
        source_face_runs: full_source_face_runs,
        appended_source_face_runs,
        prefill_repair_record_details,
        open_edge_restoration_record_details,
        orphan_repair_record_details,
        source_face_sequence,
        appended_source_face_sequence,
        split_appended_source_face_sequence,
        cut_event_source_face_sequence,
        split_event_source_face_sequence,
        prefill_repair_source_face_sequence,
        open_edge_restoration_source_face_sequence,
        orphan_repair_source_face_sequence,
        fill_appended_source_face_sequence,
    }
}

pub(super) fn meshlib_cut2origin_owner_remap_diagnostics(
    materialized: &MeshlibCut2OriginShadowInventory,
    source_preserving_paths: &[Vec<usize>],
) -> MeshlibCut2OriginOwnerRemapDiagnostics {
    let Some(source_preserving_source_faces) = source_preserving_paths.first() else {
        return MeshlibCut2OriginOwnerRemapDiagnostics::default();
    };
    let prepared_source_records = materialized
        .source_face_sequence
        .len()
        .saturating_sub(materialized.appended_source_face_sequence.len());
    let matching_source_records = materialized
        .source_face_sequence
        .iter()
        .zip(source_preserving_source_faces)
        .filter(|(materialized, source_preserving)| materialized == source_preserving)
        .count();
    let mismatch_details = materialized
        .source_face_sequence
        .iter()
        .zip(source_preserving_source_faces)
        .enumerate()
        .filter_map(|(index, (materialized, source_preserving))| {
            (materialized != source_preserving).then_some([
                index,
                *materialized,
                *source_preserving,
            ])
        })
        .collect::<Vec<_>>();
    let appended_source_face_sequence = source_preserving_source_faces
        .get(prepared_source_records..)
        .unwrap_or_default()
        .to_vec();
    let source_records = source_preserving_source_faces.len();
    let missing_materialized_source_records =
        source_records.saturating_sub(materialized.source_face_sequence.len());
    let extra_materialized_source_records = materialized
        .source_face_sequence
        .len()
        .saturating_sub(source_records);

    MeshlibCut2OriginOwnerRemapDiagnostics {
        ready: source_records == materialized.source_face_sequence.len() && source_records > 0,
        source_records,
        matching_source_records,
        mismatched_source_records: mismatch_details.len(),
        missing_materialized_source_records,
        extra_materialized_source_records,
        mismatch_details,
        source_face_sequence: source_preserving_source_faces.clone(),
        appended_source_face_sequence: appended_source_face_sequence.clone(),
        source_face_counts: source_face_counts(source_preserving_source_faces),
        source_face_runs: source_face_runs(source_preserving_source_faces),
        appended_source_face_runs: source_face_runs(&appended_source_face_sequence),
    }
}

pub(super) fn meshlib_cut2origin_owner_remapped_cut(
    cut: &ExactCutHoleFillResult,
    source_preserving_paths: &[Vec<usize>],
) -> Option<ExactCutHoleFillResult> {
    let target_source_faces = source_preserving_paths.first()?;
    if target_source_faces.len() != cut.mesh.source_face_for_faces.len() {
        return None;
    }

    let mut remapped = cut.clone();
    remapped.mesh.source_face_for_faces = target_source_faces.clone();
    for (fill_plan_index, fill_plan) in remapped.fill_plans.iter_mut().enumerate() {
        let [start, end] = *remapped.added_face_ranges.get(fill_plan_index)?;
        let source_face_for_faces = target_source_faces.get(start..end)?.to_vec();
        fill_plan.source_face = source_face_for_faces
            .first()
            .copied()
            .unwrap_or(fill_plan.source_face);
        fill_plan.source_face_for_faces = source_face_for_faces;
    }
    Some(remapped)
}

fn inferred_original_source_face_count(cut: &ExactCutHoleFillResult) -> usize {
    cut.mesh
        .source_face_for_faces
        .iter()
        .copied()
        .chain(
            cut.mesh
                .cut_edge_path_source_faces
                .iter()
                .flat_map(|path| path.iter().copied().flatten()),
        )
        .chain(
            cut.mesh
                .collapsed_cut_segment_path_source_faces
                .iter()
                .flat_map(|path| path.iter().copied().flatten()),
        )
        .chain(cut.fill_plans.iter().map(|plan| plan.source_face))
        .chain(
            cut.fill_plans
                .iter()
                .flat_map(|plan| plan.source_face_for_faces.iter().copied()),
        )
        .max()
        .map_or(0, |source_face| source_face + 1)
}

fn appended_stage_source_face_sequences(
    cut: &ExactCutHoleFillResult,
    original_source_faces: usize,
    materialized_prefill_fill_plans: &BTreeSet<usize>,
) -> (Vec<usize>, Vec<usize>) {
    let split_appended_source_face_sequence =
        split_appended_source_face_sequence(cut, original_source_faces);
    let fill_appended_source_face_sequence =
        fill_appended_source_face_sequence(cut, materialized_prefill_fill_plans);
    (
        split_appended_source_face_sequence,
        fill_appended_source_face_sequence,
    )
}

fn split_appended_source_face_sequence(
    cut: &ExactCutHoleFillResult,
    original_source_faces: usize,
) -> Vec<usize> {
    let cut_event_source_face_sequence = cut_event_source_face_sequence(cut);
    if !cut_event_source_face_sequence.is_empty() {
        return duplicate_source_records_after_prepared(
            cut_event_source_face_sequence,
            original_source_faces,
        );
    }

    let cut_face_source_sequence = cut
        .mesh
        .source_face_for_faces
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(face_index, source_face)| {
            (!added_face_index(face_index, &cut.added_face_ranges)).then_some(source_face)
        });
    duplicate_source_records_after_prepared(cut_face_source_sequence, original_source_faces)
}

fn duplicate_source_records_after_prepared(
    source_faces: impl IntoIterator<Item = usize>,
    original_source_faces: usize,
) -> Vec<usize> {
    let mut prepared_record_consumed = vec![false; original_source_faces];
    let mut split_appended_source_face_sequence = Vec::new();
    for source_face in source_faces {
        if source_face < original_source_faces && !prepared_record_consumed[source_face] {
            prepared_record_consumed[source_face] = true;
        } else {
            split_appended_source_face_sequence.push(source_face);
        }
    }
    split_appended_source_face_sequence
}

fn fill_appended_source_face_sequence(
    cut: &ExactCutHoleFillResult,
    materialized_prefill_fill_plans: &BTreeSet<usize>,
) -> Vec<usize> {
    cut.mesh
        .source_face_for_faces
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(face_index, source_face)| {
            let fill_plan_index = added_fill_plan_index(face_index, &cut.added_face_ranges)?;
            (!materialized_prefill_fill_plans.contains(&fill_plan_index)).then_some(source_face)
        })
        .collect()
}

fn cut_event_source_face_sequence(cut: &ExactCutHoleFillResult) -> Vec<usize> {
    cut.mesh
        .cut_face_source_events
        .iter()
        .map(|event| event.source_face)
        .collect()
}

fn split_event_source_face_sequence(cut: &ExactCutHoleFillResult) -> Vec<usize> {
    cut.mesh
        .cut_face_source_events
        .iter()
        .filter(|event| event.kind == ExactCutFaceSourceEventKind::Split)
        .map(|event| event.source_face)
        .collect()
}

fn added_face_index(face_index: usize, added_face_ranges: &[[usize; 2]]) -> bool {
    added_fill_plan_index(face_index, added_face_ranges).is_some()
}

fn added_fill_plan_index(face_index: usize, added_face_ranges: &[[usize; 2]]) -> Option<usize> {
    added_face_ranges
        .iter()
        .position(|[start, end]| (*start..*end).contains(&face_index))
}

pub(super) fn meshlib_prefill_repair_records(
    cut: &ExactCutHoleFillResult,
) -> Vec<MeshlibPrefillRepairRecord> {
    cut.mesh
        .cut_edge_paths
        .iter()
        .zip(cut.mesh.cut_edge_path_closed.iter().copied())
        .zip(cut.mesh.cut_edge_path_source_faces.iter())
        .enumerate()
        .filter_map(|(path_index, ((path, closed), source_faces))| {
            if closed || path.is_empty() || source_faces.len() != 1 {
                return None;
            }
            source_faces[0].map(|source_face| MeshlibPrefillRepairRecord {
                kind: MeshlibPrefillRepairKind::OpenEdgeRestoration,
                path_index,
                source_face,
            })
        })
        .collect()
}

fn materialized_prefill_fill_plan_indices(
    cut: &ExactCutHoleFillResult,
    records: &[MeshlibPrefillRepairRecord],
) -> BTreeSet<usize> {
    records
        .iter()
        .filter_map(|record| {
            cut.fill_plans
                .iter()
                .position(|plan| fill_plan_matches_prefill_repair_record(cut, plan, record))
        })
        .collect()
}

fn fill_plan_matches_prefill_repair_record(
    cut: &ExactCutHoleFillResult,
    plan: &super::super::exact_fill_apply::ExactCutHoleFillPlan,
    record: &MeshlibPrefillRepairRecord,
) -> bool {
    let Some(path) = cut.mesh.cut_edge_paths.get(record.path_index) else {
        return false;
    };
    path.len() == 1
        && plan.source_face == record.source_face
        && plan.representative_edge == path[0]
        && plan.boundary_edges == *path
        && plan.fill_plan.num_tris == 1
}

fn source_face_counts(source_faces: &[usize]) -> Vec<[usize; 2]> {
    let mut counts = BTreeMap::<usize, usize>::new();
    for source_face in source_faces {
        *counts.entry(*source_face).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(source_face, count)| [source_face, count])
        .collect()
}

fn source_face_runs(source_faces: &[usize]) -> Vec<[usize; 2]> {
    let mut runs = Vec::<[usize; 2]>::new();
    for source_face in source_faces {
        match runs.last_mut() {
            Some([run_source_face, count]) if run_source_face == source_face => *count += 1,
            _ => runs.push([*source_face, 1]),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::exact_cut_apply::{
        ExactCutFaceSourceEvent, ExactCutFaceSourceEventKind, ExactCutMeshResult,
    };
    use crate::spatial::exact_fill_apply::ExactCutHoleFillPlan;
    use crate::spatial::exact_fill_plan::ExactPlanarHoleFillPlan;

    #[test]
    fn meshlib_prefill_repair_records_track_open_edge_restoration_paths() {
        let cut = cut_fixture();
        let records = meshlib_prefill_repair_records(&cut);

        assert_eq!(
            records,
            vec![MeshlibPrefillRepairRecord {
                kind: MeshlibPrefillRepairKind::OpenEdgeRestoration,
                path_index: 0,
                source_face: 1,
            }]
        );
    }

    #[test]
    fn meshlib_cut2origin_shadow_places_prefill_repair_before_final_fill() {
        let inventory = meshlib_cut2origin_shadow_inventory(&cut_fixture());

        assert_eq!(inventory.split_appended_source_face_sequence, vec![0]);
        assert_eq!(inventory.cut_event_source_face_sequence, vec![0, 0]);
        assert_eq!(inventory.split_event_source_face_sequence, vec![0]);
        assert_eq!(inventory.prefill_repair_source_face_sequence, vec![1]);
        assert_eq!(inventory.prefill_repair_record_details, vec![[0, 1]]);
        assert_eq!(inventory.open_edge_restoration_record_details, vec![[0, 1]]);
        assert!(inventory.orphan_repair_record_details.is_empty());
        assert_eq!(
            inventory.open_edge_restoration_source_face_sequence,
            vec![1]
        );
        assert!(inventory.orphan_repair_source_face_sequence.is_empty());
        assert_eq!(inventory.fill_appended_source_face_sequence, vec![4, 4]);
        assert_eq!(inventory.appended_source_face_sequence, vec![0, 1, 4, 4]);
        assert_eq!(
            inventory.appended_source_face_runs,
            vec![[0, 1], [1, 1], [4, 2]]
        );
        assert_eq!(
            inventory.source_face_sequence,
            vec![0, 1, 2, 3, 4, 0, 1, 4, 4]
        );
        assert_eq!(
            inventory.source_face_runs,
            vec![
                [0, 1],
                [1, 1],
                [2, 1],
                [3, 1],
                [4, 1],
                [0, 1],
                [1, 1],
                [4, 2],
            ]
        );
        assert_eq!(inventory.source_records, 9);
        assert_eq!(inventory.cut_event_source_records, 2);
        assert_eq!(inventory.split_event_source_records, 1);
        assert_eq!(inventory.prefill_repair_source_records, 1);
        assert_eq!(inventory.open_edge_restoration_source_records, 1);
        assert_eq!(inventory.orphan_repair_source_records, 0);
    }

    #[test]
    fn meshlib_cut2origin_shadow_counts_collapsed_metadata_as_original_source_inventory() {
        let mut cut = cut_fixture();
        cut.mesh.collapsed_cut_segment_path_source_faces = vec![vec![Some(6)]];

        let inventory = meshlib_cut2origin_shadow_inventory(&cut);

        assert_eq!(inventory.unique_source_faces, 7);
        assert_eq!(inventory.source_records, 11);
        assert_eq!(
            inventory.source_face_sequence,
            vec![0, 1, 2, 3, 4, 5, 6, 0, 1, 4, 4]
        );
    }

    #[test]
    fn meshlib_cut2origin_owner_remap_reports_target_lifecycle_mismatches() {
        let inventory = meshlib_cut2origin_shadow_inventory(&cut_fixture());

        let remap = meshlib_cut2origin_owner_remap_diagnostics(
            &inventory,
            &[vec![0, 1, 2, 3, 4, 4, 4, 1, 0]],
        );

        assert!(remap.ready);
        assert_eq!(remap.source_records, 9);
        assert_eq!(remap.matching_source_records, 5);
        assert_eq!(remap.mismatched_source_records, 4);
        assert_eq!(remap.missing_materialized_source_records, 0);
        assert_eq!(remap.extra_materialized_source_records, 0);
        assert_eq!(
            remap.mismatch_details,
            vec![[5, 0, 4], [6, 1, 4], [7, 4, 1], [8, 4, 0]]
        );
        assert_eq!(remap.appended_source_face_sequence, vec![4, 4, 1, 0]);
        assert_eq!(
            remap.source_face_counts,
            vec![[0, 2], [1, 2], [2, 1], [3, 1], [4, 3]]
        );
        assert_eq!(
            remap.appended_source_face_runs,
            vec![[4, 2], [1, 1], [0, 1]]
        );
    }

    #[test]
    fn meshlib_cut2origin_owner_remapped_cut_updates_face_and_fill_plan_sources() {
        let cut = cut_fixture();

        let remapped = meshlib_cut2origin_owner_remapped_cut(&cut, &[vec![10, 11, 12, 13]])
            .expect("target source records match cut face records");

        assert_eq!(remapped.mesh.source_face_for_faces, vec![10, 11, 12, 13]);
        assert_eq!(remapped.fill_plans[0].source_face, 12);
        assert_eq!(remapped.fill_plans[0].source_face_for_faces, vec![12, 13]);
    }

    fn cut_fixture() -> ExactCutHoleFillResult {
        ExactCutHoleFillResult {
            mesh: ExactCutMeshResult {
                vertices: Vec::new(),
                faces: vec![[0, 1, 2], [0, 2, 3], [0, 1, 4], [1, 2, 4]],
                cut_edges: Vec::new(),
                cut_edge_paths: vec![vec![[0, 1]], vec![[1, 2], [2, 1]], vec![[2, 3]]],
                cut_edge_path_closed: vec![false, true, false],
                cut_edge_path_source_faces: vec![vec![Some(1)], vec![Some(3), Some(4)], vec![None]],
                collapsed_cut_segment_paths: Vec::new(),
                collapsed_cut_segment_path_source_faces: Vec::new(),
                source_face_for_faces: vec![0, 0, 4, 4],
                cut_face_source_events: vec![
                    ExactCutFaceSourceEvent {
                        kind: ExactCutFaceSourceEventKind::Original,
                        source_face: 0,
                    },
                    ExactCutFaceSourceEvent {
                        kind: ExactCutFaceSourceEventKind::Split,
                        source_face: 0,
                    },
                ],
                skipped_source_faces: Vec::new(),
            },
            fill_plans: vec![ExactCutHoleFillPlan {
                representative_edge: [0, 1],
                boundary_loop: vec![0, 1, 2, 3],
                boundary_edges: vec![[0, 1], [1, 2], [2, 3], [3, 0]],
                source_face: 4,
                source_face_for_faces: vec![4, 4],
                fill_plan: ExactPlanarHoleFillPlan {
                    boundary_loop: vec![0, 1, 2, 3],
                    triangles: vec![[0, 1, 2], [0, 2, 3]],
                    num_tris: 2,
                },
            }],
            added_face_ranges: vec![[2, 4]],
        }
    }
}
