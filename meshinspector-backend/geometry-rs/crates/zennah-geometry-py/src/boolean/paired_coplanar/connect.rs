use pyo3::prelude::*;
use pyo3::types::PyDict;

pub(super) fn set_paired_prepared_connect_summary(
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

    set_diag!(paired_coplanar_candidate_meshlib_base_faces);
    set_diag!(paired_coplanar_candidate_meshlib_incoming_faces);
    set_diag!(paired_coplanar_candidate_meshlib_base_vertices);
    set_diag!(paired_coplanar_candidate_meshlib_incoming_vertices);
    set_diag!(paired_coplanar_candidate_meshlib_unstitched_faces);
    set_diag!(paired_coplanar_candidate_meshlib_unstitched_vertices);
    set_diag!(paired_coplanar_candidate_meshlib_path_pairs);
    set_diag!(paired_coplanar_candidate_meshlib_path_count_mismatch);
    set_diag!(paired_coplanar_candidate_meshlib_path_length_mismatches);
    set_diag!(paired_coplanar_candidate_meshlib_path_closed_mismatches);
    set_diag!(paired_coplanar_candidate_meshlib_path_coordinate_mismatches);
    set_diag!(paired_coplanar_candidate_meshlib_path_same_direction_edges);
    set_diag!(paired_coplanar_candidate_meshlib_path_reversed_edges);
    set_diag!(paired_coplanar_candidate_meshlib_base_mapped_cut_path_edges);
    set_diag!(paired_coplanar_candidate_meshlib_incoming_mapped_cut_path_edges);
    set_diag!(paired_coplanar_candidate_meshlib_base_missing_cut_path_edges);
    set_diag!(paired_coplanar_candidate_meshlib_incoming_missing_cut_path_edges);
    set_diag!(paired_coplanar_candidate_meshlib_base_cut_paths_complete);
    set_diag!(paired_coplanar_candidate_meshlib_incoming_cut_paths_complete);
    Ok(())
}
