use super::super::super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOperand, ExactBooleanOperation,
};
use super::super::super::exact_classify::{
    exact_classify_components_with_cut_paths_and_barriers,
    exact_classify_components_with_cut_paths_and_barriers_without_orientation_normalization,
    ExactCutPathClassificationInput, ExactMeshPartClassification,
};
use super::super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::super::exact_cut_pair::ExactCutShadowRepairPath;
use super::super::super::exact_fill_apply::ExactCutHoleFillResult;
use super::super::meshlib::MeshlibPreparedBaseRecordRewriteDiagnostics;
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::spatial::exact_boolean_diagnostics) fn faces_in_added_ranges(
    prepared_faces: &[usize],
    added_face_ranges: &[[usize; 2]],
) -> Vec<usize> {
    prepared_faces
        .iter()
        .copied()
        .filter(|face| {
            added_face_ranges
                .iter()
                .any(|[start, end]| (*start..*end).contains(face))
        })
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn preserve_projected_coplanar_difference_prepare_masks(
    projected: &mut ExactBooleanAssemblyResult,
    raw: &ExactBooleanAssemblyResult,
    valid_first_cut_faces: &[usize],
    valid_second_cut_faces: &[usize],
    operation: ExactBooleanOperation,
) {
    match operation {
        ExactBooleanOperation::DifferenceAB => {
            projected.prepare_second_faces =
                faces_in_valid_cut_faces(&raw.prepare_second_faces, valid_second_cut_faces);
        }
        ExactBooleanOperation::DifferenceBA => {
            projected.prepare_first_faces =
                faces_in_valid_cut_faces(&raw.prepare_first_faces, valid_first_cut_faces);
        }
        _ => {}
    }
}

pub(in crate::spatial::exact_boolean_diagnostics) fn faces_in_valid_cut_faces(
    prepared_faces: &[usize],
    valid_cut_faces: &[usize],
) -> Vec<usize> {
    let valid = valid_cut_faces.iter().copied().collect::<BTreeSet<_>>();
    prepared_faces
        .iter()
        .copied()
        .filter(|face| valid.contains(face))
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn exact_classify_projected_valid_cut_faces_with_barriers(
    cut_mesh: &ExactCutMeshResult,
    other_vertices: &[[f64; 3]],
    other_faces_i64: &[[i64; 3]],
    valid_cut_faces: &[usize],
    cut_edge_paths: Option<&[Vec<[usize; 2]>]>,
    extra_barrier_edges: &[[usize; 2]],
    need_inside: bool,
    origin_is_first: bool,
    epsilon: f64,
    normalize_cut_path_orientation: bool,
) -> Result<ExactMeshPartClassification, GeometryError> {
    let mut seen = BTreeSet::new();
    let mut face_slots = Vec::new();
    let mut projected_faces = Vec::new();
    for face_slot in valid_cut_faces.iter().copied() {
        let Some(face) = cut_mesh.faces.get(face_slot).copied() else {
            continue;
        };
        if seen.insert(face_slot) {
            face_slots.push(face_slot);
            projected_faces.push(face);
        }
    }
    if projected_faces.is_empty() {
        return Ok(ExactMeshPartClassification {
            components: Vec::new(),
            selected_faces: Vec::new(),
            used_cut_path_sides: true,
            cut_paths_consistent: true,
            cut_left_components: 0,
            cut_right_components: 0,
            cut_path_overlap_components: 0,
        });
    }
    let classification_input = ExactCutPathClassificationInput {
        vertices: &cut_mesh.vertices,
        faces_i64: &projected_faces,
        other_vertices,
        other_faces_i64,
        cut_edges: &cut_mesh.cut_edges,
        cut_edge_paths: cut_edge_paths.unwrap_or(&cut_mesh.cut_edge_paths),
        need_inside,
        origin_is_first,
        epsilon,
    };
    let mut classification = if normalize_cut_path_orientation {
        exact_classify_components_with_cut_paths_and_barriers(
            classification_input,
            extra_barrier_edges,
        )?
    } else {
        exact_classify_components_with_cut_paths_and_barriers_without_orientation_normalization(
            classification_input,
            extra_barrier_edges,
        )?
    };
    for component in &mut classification.components {
        for face_index in &mut component.face_indices {
            if let Some(face_slot) = face_slots.get(*face_index).copied() {
                *face_index = face_slot;
            }
        }
    }
    classification.selected_faces = classification
        .selected_faces
        .iter()
        .filter_map(|face_index| face_slots.get(*face_index).copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(classification)
}

pub(in crate::spatial::exact_boolean_diagnostics) fn classification_component_summaries(
    classification: &ExactMeshPartClassification,
) -> Vec<[usize; 4]> {
    classification
        .components
        .iter()
        .map(|component| {
            [
                component.component_index,
                usize::from(component.selected),
                usize::from(component.inside_other),
                component.face_indices.len(),
            ]
        })
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn classification_component_faces(
    classification: &ExactMeshPartClassification,
) -> Vec<Vec<usize>> {
    classification
        .components
        .iter()
        .map(|component| component.face_indices.clone())
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn lifecycle_slot_face_coverage(
    lifecycle_slot_paths: &[Vec<[usize; 8]>],
    face_slots: &[usize],
) -> Vec<[usize; 9]> {
    let Some(first_path) = lifecycle_slot_paths.first() else {
        return Vec::new();
    };

    first_path
        .iter()
        .copied()
        .map(
            |[
                 path_index,
                 run_index,
                 source_face,
                 contour_hits,
                 collapsed_hits,
                 replacement_records,
                 start_slot,
                 end_slot,
             ]| {
                let face_count = face_slots
                    .iter()
                    .filter(|slot| (start_slot..end_slot).contains(slot))
                    .count();
                [
                    path_index,
                    run_index,
                    source_face,
                    contour_hits,
                    collapsed_hits,
                    replacement_records,
                    start_slot,
                    end_slot,
                    face_count,
                ]
            },
        )
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn lifecycle_slot_face_groups(
    lifecycle_slot_paths: &[Vec<[usize; 8]>],
    face_slots: &[usize],
) -> Vec<Vec<usize>> {
    let Some(first_path) = lifecycle_slot_paths.first() else {
        return Vec::new();
    };

    first_path
        .iter()
        .map(|run| {
            let start_slot = run[6];
            let end_slot = run[7];
            face_slots
                .iter()
                .copied()
                .filter(|slot| (start_slot..end_slot).contains(slot))
                .collect()
        })
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn lifecycle_slot_export_coverage(
    lifecycle_slot_paths: &[Vec<[usize; 8]>],
    rewrite: Option<&MeshlibPreparedBaseRecordRewriteDiagnostics>,
    operand: ExactBooleanOperand,
) -> Vec<[usize; 9]> {
    let Some(first_path) = lifecycle_slot_paths.first() else {
        return Vec::new();
    };
    let exported_slots = rewrite
        .map(|rewrite| {
            rewrite
                .exported_face_operands
                .iter()
                .zip(&rewrite.exported_face_cut_faces)
                .filter_map(|(exported_operand, cut_face)| {
                    (exported_operand == &Some(operand)).then_some(cut_face.as_ref())?
                })
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    first_path
        .iter()
        .copied()
        .map(
            |[
                 path_index,
                 run_index,
                 source_face,
                 contour_hits,
                 collapsed_hits,
                 replacement_records,
                 start_slot,
                 end_slot,
             ]| {
                let exported_count = exported_slots
                    .iter()
                    .filter(|slot| (start_slot..end_slot).contains(slot))
                    .count();
                [
                    path_index,
                    run_index,
                    source_face,
                    contour_hits,
                    collapsed_hits,
                    replacement_records,
                    start_slot,
                    end_slot,
                    exported_count,
                ]
            },
        )
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn lifecycle_slot_export_groups(
    lifecycle_slot_paths: &[Vec<[usize; 8]>],
    rewrite: Option<&MeshlibPreparedBaseRecordRewriteDiagnostics>,
    operand: ExactBooleanOperand,
) -> Vec<Vec<usize>> {
    let exported_slots = rewrite
        .map(|rewrite| {
            rewrite
                .exported_face_operands
                .iter()
                .zip(&rewrite.exported_face_cut_faces)
                .filter_map(|(exported_operand, cut_face)| {
                    (exported_operand == &Some(operand)).then_some(cut_face.as_ref())?
                })
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    lifecycle_slot_face_groups(lifecycle_slot_paths, &exported_slots)
}

pub(in crate::spatial::exact_boolean_diagnostics) fn first_path_source_face_count_deltas(
    materialized_counts: &[[usize; 2]],
    source_preserving_path_counts: &[Vec<[usize; 2]>],
) -> Vec<[i64; 2]> {
    source_preserving_path_counts
        .first()
        .map(|source_preserving_counts| {
            source_face_count_deltas(materialized_counts, source_preserving_counts)
        })
        .unwrap_or_default()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn source_face_count_deltas(
    materialized_counts: &[[usize; 2]],
    source_preserving_counts: &[[usize; 2]],
) -> Vec<[i64; 2]> {
    let mut deltas = BTreeMap::<usize, i64>::new();
    for &[source_face, count] in source_preserving_counts {
        *deltas.entry(source_face).or_default() -= count as i64;
    }
    for &[source_face, count] in materialized_counts {
        *deltas.entry(source_face).or_default() += count as i64;
    }
    deltas
        .into_iter()
        .filter_map(|(source_face, delta)| (delta != 0).then_some([source_face as i64, delta]))
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn meshlib_valid_cut_faces(
    original_faces: usize,
    cut2origin_source_faces: &[Vec<usize>],
    removed_face_owner_paths: &[Vec<usize>],
) -> Vec<usize> {
    let Some(cut2origin_source_faces) = cut2origin_source_faces.first() else {
        return Vec::new();
    };
    let invalid_original_faces = removed_face_owner_paths
        .first()
        .into_iter()
        .flat_map(|path| path.iter())
        .copied()
        .filter(|face| *face < original_faces)
        .collect::<BTreeSet<_>>();
    (0..original_faces)
        .filter(|face| !invalid_original_faces.contains(face))
        .chain(original_faces..cut2origin_source_faces.len())
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn projected_result_cut_edge_paths_without_shadow_repairs(
    projected_cut_edge_paths: &[Vec<[usize; 2]>],
    trailing_shadow_repair_paths: usize,
) -> Vec<Vec<[usize; 2]>> {
    let result_cut_path_count = projected_cut_edge_paths
        .len()
        .saturating_sub(trailing_shadow_repair_paths);
    projected_cut_edge_paths
        .iter()
        .take(result_cut_path_count)
        .cloned()
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn non_empty_projected_cut_edge_paths(
    paths: &[Vec<[usize; 2]>],
) -> Option<&[Vec<[usize; 2]>]> {
    (!paths.is_empty()).then_some(paths)
}

pub(in crate::spatial::exact_boolean_diagnostics) fn cut_mesh_with_shadow_repair_paths(
    cut: &ExactCutMeshResult,
    repair_paths: &[ExactCutShadowRepairPath],
) -> ExactCutMeshResult {
    let mut shadow = cut.clone();
    for repair in repair_paths {
        for edge in repair.path.iter().copied() {
            if !shadow.cut_edges.contains(&edge) {
                shadow.cut_edges.push(edge);
            }
        }
        shadow.cut_edge_paths.push(repair.path.clone());
        shadow.cut_edge_path_closed.push(false);
        shadow
            .cut_edge_path_source_faces
            .push(repair.source_faces.clone());
    }
    shadow
}

pub(in crate::spatial::exact_boolean_diagnostics) fn shadow_repair_path_source_faces(
    repair_paths: &[ExactCutShadowRepairPath],
) -> Vec<Vec<usize>> {
    repair_paths
        .iter()
        .map(|repair| repair.source_faces.iter().flatten().copied().collect())
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn cut_with_shadow_repair_paths(
    cut: &ExactCutHoleFillResult,
    repair_paths: &[ExactCutShadowRepairPath],
) -> ExactCutHoleFillResult {
    let mut shadow = cut.clone();
    for repair in repair_paths {
        for edge in repair.path.iter().copied() {
            if !shadow.mesh.cut_edges.contains(&edge) {
                shadow.mesh.cut_edges.push(edge);
            }
        }
        shadow.mesh.cut_edge_paths.push(repair.path.clone());
        shadow.mesh.cut_edge_path_closed.push(false);
        shadow
            .mesh
            .cut_edge_path_source_faces
            .push(repair.source_faces.clone());
    }
    shadow
}

pub(in crate::spatial::exact_boolean_diagnostics) fn cut_without_trailing_shadow_repair_paths(
    cut: &ExactCutHoleFillResult,
    repair_paths: &[ExactCutShadowRepairPath],
) -> ExactCutHoleFillResult {
    let mut stripped = cut.clone();
    for repair in repair_paths.iter().rev() {
        if stripped.mesh.cut_edge_paths.last() != Some(&repair.path) {
            break;
        }
        stripped.mesh.cut_edge_paths.pop();
        stripped.mesh.cut_edge_path_closed.pop();
        stripped.mesh.cut_edge_path_source_faces.pop();
    }
    stripped
}

pub(in crate::spatial::exact_boolean_diagnostics) fn replacement_need_inside(
    operation: ExactBooleanOperation,
    origin_is_first: bool,
) -> bool {
    match (operation, origin_is_first) {
        (ExactBooleanOperation::Union, _) => false,
        (ExactBooleanOperation::Intersection, _) => true,
        (ExactBooleanOperation::DifferenceAB, true) => false,
        (ExactBooleanOperation::DifferenceAB, false) => true,
        (ExactBooleanOperation::DifferenceBA, true) => true,
        (ExactBooleanOperation::DifferenceBA, false) => false,
        _ => false,
    }
}

pub(in crate::spatial::exact_boolean_diagnostics) fn barriered_prepare_faces(
    classification: &ExactMeshPartClassification,
) -> Vec<usize> {
    if classification.cut_paths_consistent {
        classification.selected_faces.clone()
    } else {
        Vec::new()
    }
}

pub(in crate::spatial::exact_boolean_diagnostics) fn synthetic_contact_edges(
    cut: &ExactCutHoleFillResult,
) -> Vec<[usize; 2]> {
    let mut added_faces = vec![false; cut.mesh.faces.len()];
    for [start, end] in &cut.added_face_ranges {
        for face in *start..(*end).min(added_faces.len()) {
            added_faces[face] = true;
        }
    }
    let mut edge_faces = BTreeMap::<[usize; 2], [bool; 2]>::new();
    for (face_index, face) in cut.mesh.faces.iter().enumerate() {
        let is_added = added_faces.get(face_index).copied().unwrap_or_default();
        for edge_index in 0..3 {
            let edge = ordered_edge([
                face[edge_index] as usize,
                face[(edge_index + 1) % 3] as usize,
            ]);
            let entry = edge_faces.entry(edge).or_default();
            if is_added {
                entry[0] = true;
            } else {
                entry[1] = true;
            }
        }
    }
    edge_faces
        .into_iter()
        .filter_map(|(edge, [has_added, has_original])| (has_added && has_original).then_some(edge))
        .collect()
}

pub(in crate::spatial::exact_boolean_diagnostics) fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}
