mod copied_edges;
mod export;
pub(super) mod meshlib;
#[rustfmt::skip]
pub use meshlib::{MeshlibCopiedFaceRecordCandidateDiagnostic, MeshlibCopiedFaceRecordDiagnostic, MeshlibCopiedPrevNextEdgeUpdateDiagnostic, MeshlibFaceExportFailureDiagnostic, MeshlibNearStitchFailureDiagnostic, MeshlibNearStitchLinkedEdgeDiagnostic, MeshlibNearStitchRingDiagnostic, MeshlibNearStitchSourceLookupDiagnostic, MeshlibNearStitchTargetSnapshotDiagnostic, MeshlibPreparedBaseRecordRewriteDiagnostics, MeshlibPreparedSourceRecordReplayDiagnostic, MeshlibRecordRewriteFailedCommandDiagnostic, MeshlibRecordRewriteTargetDiagnostic};
mod result_cut;
#[cfg(test)]
pub(super) use result_cut::meshlib_result_cut_path_summary;
#[cfg(test)]
pub(super) use result_cut::MeshlibResultCutPathSummary;
mod output;
mod source;
mod source_cut2origin;
mod topology;
use topology::requires_topology_splice;
mod types;
pub use types::ExactBooleanPipelineDiagnostics;
const BOOLEAN_DIAGNOSTIC_RAY_DIRECTION: [f64; 3] = [1.0, 0.371, 0.219];
const EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET: usize = 20_000;
#[macro_use]
mod pipeline_fields;
mod pipeline;
mod prepare;
pub(super) use pipeline::{
    exact_boolean_pipeline_diagnostics, ExactBooleanPipelineDiagnosticInputs,
};
