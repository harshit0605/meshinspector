use pyo3::prelude::*;
use pyo3::types::PyDict;

mod connect;
mod cut_source;
mod record_rewrite;

use connect::set_paired_prepared_connect_summary;
use cut_source::set_paired_cut_source_inventory;
use record_rewrite::set_paired_prepared_base_record_rewrite;

pub(super) fn set_paired_coplanar_candidate_diagnostics(
    py: Python<'_>,
    output: &Bound<'_, PyDict>,
    diagnostics: &zennah_geometry_core::ExactBooleanPipelineDiagnostics,
) -> PyResult<()> {
    macro_rules! set_diag {
        ($field:ident) => {
            output.set_item(stringify!($field), diagnostics.$field)?;
        };
        ($key:literal => $field:ident) => {
            output.set_item($key, diagnostics.$field)?;
        };
    }

    macro_rules! set_diag_clone {
        ($field:ident) => {
            output.set_item(stringify!($field), diagnostics.$field.clone())?;
        };
        ($key:literal => $field:ident) => {
            output.set_item($key, diagnostics.$field.clone())?;
        };
    }

    set_diag!(paired_coplanar_candidate_stitch_compatible);
    set_diag!(paired_coplanar_candidate_first_prepare_part_dividable);
    set_diag!(paired_coplanar_candidate_second_prepare_part_dividable);
    set_diag_clone!(paired_coplanar_candidate_prepare_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_prepare_second_face_indices);
    set_diag_clone!(paired_coplanar_candidate_selected_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_selected_second_face_indices);
    set_diag!(paired_coplanar_candidate_replacement_first_prepare_part_dividable);
    set_diag!(paired_coplanar_candidate_replacement_second_prepare_part_dividable);
    set_diag_clone!(paired_coplanar_candidate_replacement_prepare_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_replacement_prepare_second_face_indices);
    set_diag_clone!(paired_coplanar_candidate_replacement_selected_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_replacement_selected_second_face_indices);
    set_diag!(paired_coplanar_candidate_replacement_first_cut_path_side_components);
    set_diag!(paired_coplanar_candidate_replacement_second_cut_path_side_components);
    set_diag!(paired_coplanar_candidate_replacement_first_cut_path_overlap_components);
    set_diag!(paired_coplanar_candidate_replacement_second_cut_path_overlap_components);
    set_diag_clone!(paired_coplanar_candidate_replacement_first_cut_path_component_faces);
    set_diag_clone!(paired_coplanar_candidate_replacement_second_cut_path_component_faces);
    set_diag_clone!(paired_coplanar_candidate_replacement_first_cut_path_left_component_indices);
    set_diag_clone!(paired_coplanar_candidate_replacement_second_cut_path_left_component_indices);
    set_diag_clone!(paired_coplanar_candidate_replacement_first_cut_path_right_component_indices);
    set_diag_clone!(paired_coplanar_candidate_replacement_second_cut_path_right_component_indices);
    set_diag_clone!(paired_coplanar_candidate_replacement_first_cut_path_overlap_component_indices);
    set_diag_clone!(
        paired_coplanar_candidate_replacement_second_cut_path_overlap_component_indices
    );
    set_diag_clone!(paired_coplanar_candidate_replacement_first_cut_path_left_component_faces);
    set_diag_clone!(paired_coplanar_candidate_replacement_second_cut_path_left_component_faces);
    set_diag_clone!(paired_coplanar_candidate_replacement_first_cut_path_right_component_faces);
    set_diag_clone!(paired_coplanar_candidate_replacement_second_cut_path_right_component_faces);
    set_diag_clone!(paired_coplanar_candidate_replacement_first_cut_path_overlap_component_faces);
    set_diag_clone!(paired_coplanar_candidate_replacement_second_cut_path_overlap_component_faces);
    set_diag!(paired_coplanar_candidate_replacement_result_cut_paths_complete);
    set_diag!("paired_coplanar_candidate_replacement_prepare_result_cut_paths_complete" => paired_coplanar_candidate_replacement_prepare_cut_complete);
    set_diag!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_first_prepare_part_dividable);
    set_diag!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_second_prepare_part_dividable);
    set_diag!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_result_cut_paths_complete);
    set_diag!("paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_result_cut_paths_complete" => paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_cut_complete);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_second_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_selected_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_selected_second_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_second_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_second_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_first_component_summaries);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_second_component_summaries);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_first_component_faces);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_second_component_faces);
    set_diag!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_first_prepare_part_dividable);
    set_diag!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_second_prepare_part_dividable);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_selected_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_selected_second_face_indices);
    set_diag!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_first_prepare_part_dividable);
    set_diag!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_second_prepare_part_dividable);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_selected_first_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_selected_second_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_first_added_face_indices);
    set_diag_clone!(paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_second_added_face_indices);
    set_diag_clone!(paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_coverage);
    set_diag_clone!(paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_coverage);
    set_diag_clone!(paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_coverage);
    set_diag_clone!(paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_coverage);
    set_diag_clone!(paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_coverage);
    set_diag_clone!(paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_coverage);
    set_diag_clone!(paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_slots);
    set_diag_clone!(paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_slots);
    set_diag_clone!(paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_slots);
    set_diag_clone!(paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_slots);
    set_diag_clone!(paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots);
    set_diag_clone!(paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots);
    set_diag!(paired_coplanar_candidate_first_cut_path_side_components);
    set_diag!(paired_coplanar_candidate_second_cut_path_side_components);
    set_diag!(paired_coplanar_candidate_first_cut_path_overlap_components);
    set_diag!(paired_coplanar_candidate_second_cut_path_overlap_components);
    set_diag!(paired_coplanar_candidate_result_cut_paths_complete);
    set_diag!("paired_coplanar_candidate_prepare_result_cut_paths_complete" => paired_coplanar_candidate_prepare_cut_complete);
    set_diag!(paired_coplanar_candidate_output_faces);
    set_diag!(paired_coplanar_candidate_output_area);
    set_diag!(paired_coplanar_candidate_output_volume);
    set_diag!(paired_coplanar_candidate_self_intersections);
    set_diag!(paired_coplanar_candidate_self_intersections_available);
    set_diag!(paired_coplanar_candidate_active_volume_delta);
    set_diag!(paired_coplanar_candidate_preserves_active_volume);
    set_diag!(paired_coplanar_candidate_boundary_edges);
    set_diag!(paired_coplanar_candidate_nonmanifold_edges);
    set_diag!(paired_coplanar_candidate_duplicate_output_faces);
    set_paired_prepared_connect_summary(output, diagnostics)?;
    set_paired_cut_source_inventory(output, diagnostics)?;
    set_paired_prepared_base_record_rewrite(py, output, diagnostics)?;
    Ok(())
}
