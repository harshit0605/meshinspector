use pyo3::prelude::*;
use pyo3::types::PyDict;

pub(super) fn set_meshlib_cut2origin_owner_remap(
    output: &Bound<'_, PyDict>,
    diagnostics: &zennah_geometry_core::ExactBooleanPipelineDiagnostics,
) -> PyResult<()> {
    macro_rules! set_diag {
        ($field:ident) => {
            output.set_item(stringify!($field), diagnostics.$field)?;
        };
    }

    macro_rules! set_diag_clone {
        ($field:ident) => {
            output.set_item(stringify!($field), diagnostics.$field.clone())?;
        };
    }

    set_diag!(paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_ready);
    set_diag!(paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_source_records);
    set_diag!(
        paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_matching_source_records
    );
    set_diag!(
        paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_mismatched_source_records
    );
    set_diag!(paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_missing_materialized_source_records);
    set_diag!(paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_extra_materialized_source_records);
    set_diag_clone!(
        paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_faces
    );
    set_diag_clone!(
        paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_faces
    );
    set_diag_clone!(
        paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_appended_source_faces
    );
    set_diag_clone!(paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_appended_source_faces);
    set_diag_clone!(
        paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_face_counts
    );
    set_diag_clone!(
        paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_face_counts
    );
    set_diag_clone!(
        paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_face_runs
    );
    set_diag_clone!(
        paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_face_runs
    );
    set_diag_clone!(paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_appended_source_face_runs);
    set_diag_clone!(paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_appended_source_face_runs);
    set_diag_clone!(
        paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_mismatch_details
    );
    set_diag_clone!(
        paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_mismatch_details
    );
    set_diag_clone!(paired_coplanar_candidate_first_meshlib_valid_cut_faces);
    set_diag_clone!(paired_coplanar_candidate_second_meshlib_valid_cut_faces);
    Ok(())
}
