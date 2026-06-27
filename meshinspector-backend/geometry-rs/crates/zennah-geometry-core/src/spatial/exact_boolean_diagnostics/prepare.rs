use super::super::exact_boolean::ExactBooleanOperation;
use super::super::exact_boolean_assembly::{
    assemble_classified_boolean_with_stitch_at_tolerance,
    exact_assemble_difference_with_coplanar_sampling,
    exact_assemble_intersection_with_coplanar_sampling,
    exact_assemble_union_with_coplanar_first_wins,
};
use super::super::exact_boolean_paths::mapped_prepare_result_cut_paths;
use super::super::exact_classify::{
    exact_classify_components_with_cut_paths_and_barriers,
    exact_classify_components_with_cut_paths_and_barriers_without_orientation_normalization,
    exact_cut_path_side_component_details, exact_cut_path_side_component_details_with_barriers,
    ExactCutPathClassificationInput,
};
use super::super::exact_fill_apply::ExactCutHoleFillResult;
use super::super::exact_stitch::exact_stitch_plan_from_cut_meshes;
use super::meshlib::{
    meshlib_rewrite_diagnostics, MeshlibPreparedBaseRecordRewriteDiagnostics,
    MeshlibRewriteDiagnosticsInput,
};
use crate::GeometryError;

mod helpers;
pub(in crate::spatial::exact_boolean_diagnostics) use helpers::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct PairedReplacementPrepareDiagnostics {
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
    pub(super) first_cut_path_component_faces: Vec<Vec<usize>>,
    pub(super) second_cut_path_component_faces: Vec<Vec<usize>>,
    pub(super) first_cut_path_left_component_indices: Vec<usize>,
    pub(super) second_cut_path_left_component_indices: Vec<usize>,
    pub(super) first_cut_path_right_component_indices: Vec<usize>,
    pub(super) second_cut_path_right_component_indices: Vec<usize>,
    pub(super) first_cut_path_overlap_component_indices: Vec<usize>,
    pub(super) second_cut_path_overlap_component_indices: Vec<usize>,
    pub(super) first_cut_path_left_component_faces: Vec<Vec<usize>>,
    pub(super) second_cut_path_left_component_faces: Vec<Vec<usize>>,
    pub(super) first_cut_path_right_component_faces: Vec<Vec<usize>>,
    pub(super) second_cut_path_right_component_faces: Vec<Vec<usize>>,
    pub(super) first_cut_path_overlap_component_faces: Vec<Vec<usize>>,
    pub(super) second_cut_path_overlap_component_faces: Vec<Vec<usize>>,
    pub(super) synthetic_contact_edges: [usize; 2],
    pub(super) barriered_first_prepare_part_dividable: bool,
    pub(super) barriered_second_prepare_part_dividable: bool,
    pub(super) barriered_prepare_first_face_indices: Vec<usize>,
    pub(super) barriered_prepare_second_face_indices: Vec<usize>,
    pub(super) barriered_first_cut_path_overlap_components: usize,
    pub(super) barriered_second_cut_path_overlap_components: usize,
    pub(super) barriered_first_cut_path_overlap_component_indices: Vec<usize>,
    pub(super) barriered_second_cut_path_overlap_component_indices: Vec<usize>,
    pub(super) barriered_first_cut_path_overlap_component_faces: Vec<Vec<usize>>,
    pub(super) barriered_second_cut_path_overlap_component_faces: Vec<Vec<usize>>,
    pub(super) fixed_barriered_first_prepare_part_dividable: bool,
    pub(super) fixed_barriered_second_prepare_part_dividable: bool,
    pub(super) fixed_barriered_prepare_first_face_indices: Vec<usize>,
    pub(super) fixed_barriered_prepare_second_face_indices: Vec<usize>,
    pub(super) fixed_barriered_first_cut_path_overlap_components: usize,
    pub(super) fixed_barriered_second_cut_path_overlap_components: usize,
    pub(super) result_cut_paths_complete: bool,
    pub(super) prepare_result_cut_paths_complete: bool,
    pub(super) prepared_base_record_rewrite: Option<MeshlibPreparedBaseRecordRewriteDiagnostics>,
    pub(super) barriered_prepared_base_record_rewrite:
        Option<MeshlibPreparedBaseRecordRewriteDiagnostics>,
    pub(super) slot_projected_barriered_prepare_first_face_indices: Vec<usize>,
    pub(super) slot_projected_barriered_prepare_second_face_indices: Vec<usize>,
    pub(super) slot_projected_barriered_selected_first_face_indices: Vec<usize>,
    pub(super) slot_projected_barriered_selected_second_face_indices: Vec<usize>,
    pub(super) slot_projected_barriered_first_component_summaries: Vec<[usize; 4]>,
    pub(super) slot_projected_barriered_second_component_summaries: Vec<[usize; 4]>,
    pub(super) slot_projected_barriered_first_component_faces: Vec<Vec<usize>>,
    pub(super) slot_projected_barriered_second_component_faces: Vec<Vec<usize>>,
    pub(super) slot_projected_fixed_barriered_first_prepare_part_dividable: bool,
    pub(super) slot_projected_fixed_barriered_second_prepare_part_dividable: bool,
    pub(super) slot_projected_fixed_barriered_selected_first_face_indices: Vec<usize>,
    pub(super) slot_projected_fixed_barriered_selected_second_face_indices: Vec<usize>,
    pub(super) slot_projected_no_contact_barrier_first_prepare_part_dividable: bool,
    pub(super) slot_projected_no_contact_barrier_second_prepare_part_dividable: bool,
    pub(super) slot_projected_no_contact_barrier_selected_first_face_indices: Vec<usize>,
    pub(super) slot_projected_no_contact_barrier_selected_second_face_indices: Vec<usize>,
    pub(super) slot_projected_barriered_prepare_first_added_face_indices: Vec<usize>,
    pub(super) slot_projected_barriered_prepare_second_added_face_indices: Vec<usize>,
    pub(super) slot_projected_barriered_prepared_base_record_rewrite:
        Option<MeshlibPreparedBaseRecordRewriteDiagnostics>,
    pub(super) slot_projected_barriered_added_fill_prepared_base_record_rewrite:
        Option<MeshlibPreparedBaseRecordRewriteDiagnostics>,
}

pub(super) fn paired_replacement_prepare_diagnostics(
    replacement_cuts: Option<&(ExactCutHoleFillResult, ExactCutHoleFillResult)>,
    prepare_cuts: Option<&(ExactCutHoleFillResult, ExactCutHoleFillResult)>,
    valid_first_cut_faces: Option<&[usize]>,
    valid_second_cut_faces: Option<&[usize]>,
    projected_first_cut_edge_paths: Option<&[Vec<[usize; 2]>]>,
    projected_second_cut_edge_paths: Option<&[Vec<[usize; 2]>]>,
    operation: ExactBooleanOperation,
    epsilon: f64,
    original_first_vertices: &[[f64; 3]],
    original_first_faces: &[[i64; 3]],
    original_second_vertices: &[[f64; 3]],
    original_second_faces: &[[i64; 3]],
) -> Result<PairedReplacementPrepareDiagnostics, GeometryError> {
    let Some((first_cut, second_cut)) = replacement_cuts else {
        return Ok(PairedReplacementPrepareDiagnostics::default());
    };
    let (prepare_first_cut, prepare_second_cut) = prepare_cuts
        .map(|(first, second)| (first, second))
        .unwrap_or((first_cut, second_cut));
    let stitch_plan = exact_stitch_plan_from_cut_meshes(
        &prepare_first_cut.mesh,
        &prepare_second_cut.mesh,
        epsilon,
    );
    let assembly = match operation {
        ExactBooleanOperation::Union => exact_assemble_union_with_coplanar_first_wins(
            &prepare_first_cut.mesh,
            &prepare_second_cut.mesh,
            Some(&stitch_plan),
            epsilon,
        )?,
        ExactBooleanOperation::Intersection => exact_assemble_intersection_with_coplanar_sampling(
            &prepare_first_cut.mesh,
            &prepare_second_cut.mesh,
            Some(&stitch_plan),
            epsilon,
        )?,
        ExactBooleanOperation::DifferenceAB | ExactBooleanOperation::DifferenceBA => {
            exact_assemble_difference_with_coplanar_sampling(
                &prepare_first_cut.mesh,
                &prepare_second_cut.mesh,
                Some(&stitch_plan),
                operation,
                epsilon,
            )?
        }
        _ => return Ok(PairedReplacementPrepareDiagnostics::default()),
    };
    let prepare_result_cut = mapped_prepare_result_cut_paths(
        operation,
        &prepare_first_cut.mesh,
        &prepare_second_cut.mesh,
        &assembly.prepare_first_faces,
        &assembly.prepare_second_faces,
    );
    let first_side_details = exact_cut_path_side_component_details(
        prepare_first_cut.mesh.vertices.len(),
        &prepare_first_cut.mesh.faces,
        &prepare_first_cut.mesh.cut_edge_paths,
    )?;
    let second_side_details = exact_cut_path_side_component_details(
        prepare_second_cut.mesh.vertices.len(),
        &prepare_second_cut.mesh.faces,
        &prepare_second_cut.mesh.cut_edge_paths,
    )?;
    let first_synthetic_contact_edges = synthetic_contact_edges(prepare_first_cut);
    let second_synthetic_contact_edges = synthetic_contact_edges(prepare_second_cut);
    let first_barriered_classification = exact_classify_components_with_cut_paths_and_barriers(
        ExactCutPathClassificationInput {
            vertices: &prepare_first_cut.mesh.vertices,
            faces_i64: &prepare_first_cut.mesh.faces,
            other_vertices: original_second_vertices,
            other_faces_i64: original_second_faces,
            cut_edges: &prepare_first_cut.mesh.cut_edges,
            cut_edge_paths: &prepare_first_cut.mesh.cut_edge_paths,
            need_inside: replacement_need_inside(operation, true),
            origin_is_first: true,
            epsilon,
        },
        &first_synthetic_contact_edges,
    )?;
    let second_barriered_classification = exact_classify_components_with_cut_paths_and_barriers(
        ExactCutPathClassificationInput {
            vertices: &prepare_second_cut.mesh.vertices,
            faces_i64: &prepare_second_cut.mesh.faces,
            other_vertices: original_first_vertices,
            other_faces_i64: original_first_faces,
            cut_edges: &prepare_second_cut.mesh.cut_edges,
            cut_edge_paths: &prepare_second_cut.mesh.cut_edge_paths,
            need_inside: replacement_need_inside(operation, false),
            origin_is_first: false,
            epsilon,
        },
        &second_synthetic_contact_edges,
    )?;
    let prepared_base_record_rewrite =
        meshlib_rewrite_diagnostics(MeshlibRewriteDiagnosticsInput {
            first_cut: &first_cut.mesh,
            second_cut: &second_cut.mesh,
            first_added_face_ranges: &first_cut.added_face_ranges,
            second_added_face_ranges: &second_cut.added_face_ranges,
            stitch_plan: Some(&stitch_plan),
            assembly: &assembly,
            operation,
            epsilon,
        })?
        .prepared_base_record_rewrite;
    let barriered_assembly = assemble_classified_boolean_with_stitch_at_tolerance(
        &prepare_first_cut.mesh,
        &prepare_second_cut.mesh,
        Some(&first_barriered_classification),
        Some(&second_barriered_classification),
        Some(&stitch_plan),
        operation,
        epsilon,
    );
    let barriered_prepared_base_record_rewrite =
        meshlib_rewrite_diagnostics(MeshlibRewriteDiagnosticsInput {
            first_cut: &first_cut.mesh,
            second_cut: &second_cut.mesh,
            first_added_face_ranges: &first_cut.added_face_ranges,
            second_added_face_ranges: &second_cut.added_face_ranges,
            stitch_plan: Some(&stitch_plan),
            assembly: &barriered_assembly,
            operation,
            epsilon,
        })?
        .prepared_base_record_rewrite;
    let (
        slot_projected_barriered_prepare_first_faces,
        slot_projected_barriered_prepare_second_faces,
        slot_projected_barriered_selected_first_faces,
        slot_projected_barriered_selected_second_faces,
        slot_projected_barriered_first_component_summaries,
        slot_projected_barriered_second_component_summaries,
        slot_projected_barriered_first_component_faces,
        slot_projected_barriered_second_component_faces,
        slot_projected_fixed_barriered_first_prepare_part_dividable,
        slot_projected_fixed_barriered_second_prepare_part_dividable,
        slot_projected_fixed_barriered_selected_first_faces,
        slot_projected_fixed_barriered_selected_second_faces,
        slot_projected_no_contact_barrier_first_prepare_part_dividable,
        slot_projected_no_contact_barrier_second_prepare_part_dividable,
        slot_projected_no_contact_barrier_selected_first_faces,
        slot_projected_no_contact_barrier_selected_second_faces,
        slot_projected_barriered_prepare_first_added_faces,
        slot_projected_barriered_prepare_second_added_faces,
        slot_projected_barriered_prepared_base_record_rewrite,
        slot_projected_barriered_added_fill_prepared_base_record_rewrite,
    ) = if let (Some(valid_first_cut_faces), Some(valid_second_cut_faces)) =
        (valid_first_cut_faces, valid_second_cut_faces)
    {
        let first_slot_projected_barriered_classification =
            exact_classify_projected_valid_cut_faces_with_barriers(
                &prepare_first_cut.mesh,
                original_second_vertices,
                original_second_faces,
                valid_first_cut_faces,
                projected_first_cut_edge_paths,
                &first_synthetic_contact_edges,
                replacement_need_inside(operation, true),
                true,
                epsilon,
                true,
            )?;
        let second_slot_projected_barriered_classification =
            exact_classify_projected_valid_cut_faces_with_barriers(
                &prepare_second_cut.mesh,
                original_first_vertices,
                original_first_faces,
                valid_second_cut_faces,
                projected_second_cut_edge_paths,
                &second_synthetic_contact_edges,
                replacement_need_inside(operation, false),
                false,
                epsilon,
                true,
            )?;
        let first_slot_projected_fixed_barriered_classification =
            exact_classify_projected_valid_cut_faces_with_barriers(
                &prepare_first_cut.mesh,
                original_second_vertices,
                original_second_faces,
                valid_first_cut_faces,
                projected_first_cut_edge_paths,
                &first_synthetic_contact_edges,
                replacement_need_inside(operation, true),
                true,
                epsilon,
                false,
            )?;
        let second_slot_projected_fixed_barriered_classification =
            exact_classify_projected_valid_cut_faces_with_barriers(
                &prepare_second_cut.mesh,
                original_first_vertices,
                original_first_faces,
                valid_second_cut_faces,
                projected_second_cut_edge_paths,
                &second_synthetic_contact_edges,
                replacement_need_inside(operation, false),
                false,
                epsilon,
                false,
            )?;
        let no_contact_barrier_edges: &[[usize; 2]] = &[];
        let first_slot_projected_no_contact_barrier_classification =
            exact_classify_projected_valid_cut_faces_with_barriers(
                &prepare_first_cut.mesh,
                original_second_vertices,
                original_second_faces,
                valid_first_cut_faces,
                projected_first_cut_edge_paths,
                no_contact_barrier_edges,
                replacement_need_inside(operation, true),
                true,
                epsilon,
                true,
            )?;
        let second_slot_projected_no_contact_barrier_classification =
            exact_classify_projected_valid_cut_faces_with_barriers(
                &prepare_second_cut.mesh,
                original_first_vertices,
                original_first_faces,
                valid_second_cut_faces,
                projected_second_cut_edge_paths,
                no_contact_barrier_edges,
                replacement_need_inside(operation, false),
                false,
                epsilon,
                true,
            )?;
        let mut slot_projected_barriered_assembly =
            assemble_classified_boolean_with_stitch_at_tolerance(
                &prepare_first_cut.mesh,
                &prepare_second_cut.mesh,
                Some(&first_slot_projected_barriered_classification),
                Some(&second_slot_projected_barriered_classification),
                Some(&stitch_plan),
                operation,
                epsilon,
            );
        let slot_projected_barriered_selected_first_faces = slot_projected_barriered_assembly
            .selected_first_faces
            .clone();
        let slot_projected_barriered_selected_second_faces = slot_projected_barriered_assembly
            .selected_second_faces
            .clone();
        let slot_projected_barriered_first_component_summaries =
            classification_component_summaries(&first_slot_projected_barriered_classification);
        let slot_projected_barriered_second_component_summaries =
            classification_component_summaries(&second_slot_projected_barriered_classification);
        let slot_projected_barriered_first_component_faces =
            classification_component_faces(&first_slot_projected_barriered_classification);
        let slot_projected_barriered_second_component_faces =
            classification_component_faces(&second_slot_projected_barriered_classification);
        let slot_projected_fixed_barriered_first_prepare_part_dividable =
            first_slot_projected_fixed_barriered_classification.cut_paths_consistent;
        let slot_projected_fixed_barriered_second_prepare_part_dividable =
            second_slot_projected_fixed_barriered_classification.cut_paths_consistent;
        let slot_projected_fixed_barriered_selected_first_faces =
            first_slot_projected_fixed_barriered_classification.selected_faces;
        let slot_projected_fixed_barriered_selected_second_faces =
            second_slot_projected_fixed_barriered_classification.selected_faces;
        let slot_projected_no_contact_barrier_first_prepare_part_dividable =
            first_slot_projected_no_contact_barrier_classification.cut_paths_consistent;
        let slot_projected_no_contact_barrier_second_prepare_part_dividable =
            second_slot_projected_no_contact_barrier_classification.cut_paths_consistent;
        let slot_projected_no_contact_barrier_selected_first_faces =
            first_slot_projected_no_contact_barrier_classification.selected_faces;
        let slot_projected_no_contact_barrier_selected_second_faces =
            second_slot_projected_no_contact_barrier_classification.selected_faces;
        preserve_projected_coplanar_difference_prepare_masks(
            &mut slot_projected_barriered_assembly,
            &assembly,
            valid_first_cut_faces,
            valid_second_cut_faces,
            operation,
        );
        let slot_projected_barriered_prepared_base_record_rewrite =
            meshlib_rewrite_diagnostics(MeshlibRewriteDiagnosticsInput {
                first_cut: &first_cut.mesh,
                second_cut: &second_cut.mesh,
                first_added_face_ranges: &first_cut.added_face_ranges,
                second_added_face_ranges: &second_cut.added_face_ranges,
                stitch_plan: Some(&stitch_plan),
                assembly: &slot_projected_barriered_assembly,
                operation,
                epsilon,
            })?
            .prepared_base_record_rewrite;
        let empty_added_face_ranges: &[[usize; 2]] = &[];
        let slot_projected_barriered_added_fill_prepared_base_record_rewrite =
            meshlib_rewrite_diagnostics(MeshlibRewriteDiagnosticsInput {
                first_cut: &first_cut.mesh,
                second_cut: &second_cut.mesh,
                first_added_face_ranges: empty_added_face_ranges,
                second_added_face_ranges: empty_added_face_ranges,
                stitch_plan: Some(&stitch_plan),
                assembly: &slot_projected_barriered_assembly,
                operation,
                epsilon,
            })?
            .prepared_base_record_rewrite;
        let slot_projected_barriered_prepare_first_faces =
            slot_projected_barriered_assembly.prepare_first_faces;
        let slot_projected_barriered_prepare_second_faces =
            slot_projected_barriered_assembly.prepare_second_faces;
        let slot_projected_barriered_prepare_first_added_faces = faces_in_added_ranges(
            &slot_projected_barriered_prepare_first_faces,
            &first_cut.added_face_ranges,
        );
        let slot_projected_barriered_prepare_second_added_faces = faces_in_added_ranges(
            &slot_projected_barriered_prepare_second_faces,
            &second_cut.added_face_ranges,
        );
        (
            slot_projected_barriered_prepare_first_faces,
            slot_projected_barriered_prepare_second_faces,
            slot_projected_barriered_selected_first_faces,
            slot_projected_barriered_selected_second_faces,
            slot_projected_barriered_first_component_summaries,
            slot_projected_barriered_second_component_summaries,
            slot_projected_barriered_first_component_faces,
            slot_projected_barriered_second_component_faces,
            slot_projected_fixed_barriered_first_prepare_part_dividable,
            slot_projected_fixed_barriered_second_prepare_part_dividable,
            slot_projected_fixed_barriered_selected_first_faces,
            slot_projected_fixed_barriered_selected_second_faces,
            slot_projected_no_contact_barrier_first_prepare_part_dividable,
            slot_projected_no_contact_barrier_second_prepare_part_dividable,
            slot_projected_no_contact_barrier_selected_first_faces,
            slot_projected_no_contact_barrier_selected_second_faces,
            slot_projected_barriered_prepare_first_added_faces,
            slot_projected_barriered_prepare_second_added_faces,
            Some(slot_projected_barriered_prepared_base_record_rewrite),
            Some(slot_projected_barriered_added_fill_prepared_base_record_rewrite),
        )
    } else {
        (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
        )
    };
    let first_barriered_prepare_faces = barriered_prepare_faces(&first_barriered_classification);
    let second_barriered_prepare_faces = barriered_prepare_faces(&second_barriered_classification);
    let first_fixed_barriered_classification =
        exact_classify_components_with_cut_paths_and_barriers_without_orientation_normalization(
            ExactCutPathClassificationInput {
                vertices: &prepare_first_cut.mesh.vertices,
                faces_i64: &prepare_first_cut.mesh.faces,
                other_vertices: original_second_vertices,
                other_faces_i64: original_second_faces,
                cut_edges: &prepare_first_cut.mesh.cut_edges,
                cut_edge_paths: &prepare_first_cut.mesh.cut_edge_paths,
                need_inside: replacement_need_inside(operation, true),
                origin_is_first: true,
                epsilon,
            },
            &first_synthetic_contact_edges,
        )?;
    let second_fixed_barriered_classification =
        exact_classify_components_with_cut_paths_and_barriers_without_orientation_normalization(
            ExactCutPathClassificationInput {
                vertices: &prepare_second_cut.mesh.vertices,
                faces_i64: &prepare_second_cut.mesh.faces,
                other_vertices: original_first_vertices,
                other_faces_i64: original_first_faces,
                cut_edges: &prepare_second_cut.mesh.cut_edges,
                cut_edge_paths: &prepare_second_cut.mesh.cut_edge_paths,
                need_inside: replacement_need_inside(operation, false),
                origin_is_first: false,
                epsilon,
            },
            &second_synthetic_contact_edges,
        )?;
    let first_fixed_barriered_prepare_faces =
        barriered_prepare_faces(&first_fixed_barriered_classification);
    let second_fixed_barriered_prepare_faces =
        barriered_prepare_faces(&second_fixed_barriered_classification);
    let first_barriered_side_details = exact_cut_path_side_component_details_with_barriers(
        prepare_first_cut.mesh.vertices.len(),
        &prepare_first_cut.mesh.faces,
        &prepare_first_cut.mesh.cut_edge_paths,
        &first_synthetic_contact_edges,
    )?;
    let second_barriered_side_details = exact_cut_path_side_component_details_with_barriers(
        prepare_second_cut.mesh.vertices.len(),
        &prepare_second_cut.mesh.faces,
        &prepare_second_cut.mesh.cut_edge_paths,
        &second_synthetic_contact_edges,
    )?;

    Ok(PairedReplacementPrepareDiagnostics {
        first_prepare_part_dividable: assembly.first_cut_paths_consistent,
        second_prepare_part_dividable: assembly.second_cut_paths_consistent,
        prepare_first_face_indices: assembly.prepare_first_faces,
        prepare_second_face_indices: assembly.prepare_second_faces,
        selected_first_face_indices: assembly.selected_first_faces,
        selected_second_face_indices: assembly.selected_second_faces,
        first_cut_path_side_components: assembly.first_cut_path_side_components,
        second_cut_path_side_components: assembly.second_cut_path_side_components,
        first_cut_path_overlap_components: assembly.first_cut_path_overlap_components,
        second_cut_path_overlap_components: assembly.second_cut_path_overlap_components,
        first_cut_path_component_faces: first_side_details.component_faces,
        second_cut_path_component_faces: second_side_details.component_faces,
        first_cut_path_left_component_indices: first_side_details.left_component_indices,
        second_cut_path_left_component_indices: second_side_details.left_component_indices,
        first_cut_path_right_component_indices: first_side_details.right_component_indices,
        second_cut_path_right_component_indices: second_side_details.right_component_indices,
        first_cut_path_overlap_component_indices: first_side_details.overlap_component_indices,
        second_cut_path_overlap_component_indices: second_side_details.overlap_component_indices,
        first_cut_path_left_component_faces: first_side_details.left_component_faces,
        second_cut_path_left_component_faces: second_side_details.left_component_faces,
        first_cut_path_right_component_faces: first_side_details.right_component_faces,
        second_cut_path_right_component_faces: second_side_details.right_component_faces,
        first_cut_path_overlap_component_faces: first_side_details.overlap_component_faces,
        second_cut_path_overlap_component_faces: second_side_details.overlap_component_faces,
        synthetic_contact_edges: [
            first_synthetic_contact_edges.len(),
            second_synthetic_contact_edges.len(),
        ],
        barriered_first_prepare_part_dividable: first_barriered_classification.cut_paths_consistent,
        barriered_second_prepare_part_dividable: second_barriered_classification
            .cut_paths_consistent,
        barriered_prepare_first_face_indices: first_barriered_prepare_faces,
        barriered_prepare_second_face_indices: second_barriered_prepare_faces,
        barriered_first_cut_path_overlap_components: first_barriered_classification
            .cut_path_overlap_components,
        barriered_second_cut_path_overlap_components: second_barriered_classification
            .cut_path_overlap_components,
        barriered_first_cut_path_overlap_component_indices: first_barriered_side_details
            .overlap_component_indices,
        barriered_second_cut_path_overlap_component_indices: second_barriered_side_details
            .overlap_component_indices,
        barriered_first_cut_path_overlap_component_faces: first_barriered_side_details
            .overlap_component_faces,
        barriered_second_cut_path_overlap_component_faces: second_barriered_side_details
            .overlap_component_faces,
        fixed_barriered_first_prepare_part_dividable: first_fixed_barriered_classification
            .cut_paths_consistent,
        fixed_barriered_second_prepare_part_dividable: second_fixed_barriered_classification
            .cut_paths_consistent,
        fixed_barriered_prepare_first_face_indices: first_fixed_barriered_prepare_faces,
        fixed_barriered_prepare_second_face_indices: second_fixed_barriered_prepare_faces,
        fixed_barriered_first_cut_path_overlap_components: first_fixed_barriered_classification
            .cut_path_overlap_components,
        fixed_barriered_second_cut_path_overlap_components: second_fixed_barriered_classification
            .cut_path_overlap_components,
        result_cut_paths_complete: assembly.result_cut_paths_complete,
        prepare_result_cut_paths_complete: prepare_result_cut.complete,
        prepared_base_record_rewrite: Some(prepared_base_record_rewrite),
        barriered_prepared_base_record_rewrite: Some(barriered_prepared_base_record_rewrite),
        slot_projected_barriered_prepare_first_face_indices:
            slot_projected_barriered_prepare_first_faces,
        slot_projected_barriered_prepare_second_face_indices:
            slot_projected_barriered_prepare_second_faces,
        slot_projected_barriered_selected_first_face_indices:
            slot_projected_barriered_selected_first_faces,
        slot_projected_barriered_selected_second_face_indices:
            slot_projected_barriered_selected_second_faces,
        slot_projected_barriered_first_component_summaries:
            slot_projected_barriered_first_component_summaries,
        slot_projected_barriered_second_component_summaries:
            slot_projected_barriered_second_component_summaries,
        slot_projected_barriered_first_component_faces:
            slot_projected_barriered_first_component_faces,
        slot_projected_barriered_second_component_faces:
            slot_projected_barriered_second_component_faces,
        slot_projected_fixed_barriered_first_prepare_part_dividable:
            slot_projected_fixed_barriered_first_prepare_part_dividable,
        slot_projected_fixed_barriered_second_prepare_part_dividable:
            slot_projected_fixed_barriered_second_prepare_part_dividable,
        slot_projected_fixed_barriered_selected_first_face_indices:
            slot_projected_fixed_barriered_selected_first_faces,
        slot_projected_fixed_barriered_selected_second_face_indices:
            slot_projected_fixed_barriered_selected_second_faces,
        slot_projected_no_contact_barrier_first_prepare_part_dividable:
            slot_projected_no_contact_barrier_first_prepare_part_dividable,
        slot_projected_no_contact_barrier_second_prepare_part_dividable:
            slot_projected_no_contact_barrier_second_prepare_part_dividable,
        slot_projected_no_contact_barrier_selected_first_face_indices:
            slot_projected_no_contact_barrier_selected_first_faces,
        slot_projected_no_contact_barrier_selected_second_face_indices:
            slot_projected_no_contact_barrier_selected_second_faces,
        slot_projected_barriered_prepare_first_added_face_indices:
            slot_projected_barriered_prepare_first_added_faces,
        slot_projected_barriered_prepare_second_added_face_indices:
            slot_projected_barriered_prepare_second_added_faces,
        slot_projected_barriered_prepared_base_record_rewrite:
            slot_projected_barriered_prepared_base_record_rewrite,
        slot_projected_barriered_added_fill_prepared_base_record_rewrite:
            slot_projected_barriered_added_fill_prepared_base_record_rewrite,
    })
}
