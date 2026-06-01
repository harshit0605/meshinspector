use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

pub(super) fn near_stitch_failure_details(
    py: Python<'_>,
    details: &[zennah_geometry_core::MeshlibNearStitchFailureDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("stitch_pair_index", detail.stitch_pair_index)?;
        item.set_item("endpoint", detail.endpoint)?;
        item.set_item("source_operand", detail.source_operand)?;
        item.set_item("previous_source_halfedge", detail.previous_source_halfedge)?;
        item.set_item("next_source_halfedge", detail.next_source_halfedge)?;
        item.set_item(
            "previous_source_halfedge_key_face",
            detail.previous_source_halfedge_key_face,
        )?;
        item.set_item(
            "previous_source_halfedge_key_edge",
            detail.previous_source_halfedge_key_edge,
        )?;
        item.set_item(
            "next_source_halfedge_key_face",
            detail.next_source_halfedge_key_face,
        )?;
        item.set_item(
            "next_source_halfedge_key_edge",
            detail.next_source_halfedge_key_edge,
        )?;
        item.set_item("previous_source_edge", detail.previous_source_edge)?;
        item.set_item("next_source_edge", detail.next_source_edge)?;
        item.set_item("previous_edge", detail.previous_edge)?;
        item.set_item("next_edge", detail.next_edge)?;
        item.set_item("strict_source_identity", detail.strict_source_identity)?;
        item.set_item("error", detail.error)?;
        if let Some(candidate_diagnostics) = &detail.candidate_diagnostics {
            let candidates = PyDict::new(py);
            candidates.set_item("attempt", candidate_diagnostics.attempt)?;
            candidates.set_item(
                "previous_candidates",
                candidate_diagnostics.previous_candidates,
            )?;
            candidates.set_item("next_candidates", candidate_diagnostics.next_candidates)?;
            let failures = PyList::empty(py);
            for failure in &candidate_diagnostics.failures {
                let failure_item = PyDict::new(py);
                failure_item.set_item("previous_edge_id", failure.previous_edge_id)?;
                failure_item.set_item("next_edge_id", failure.next_edge_id)?;
                failure_item.set_item(
                    "previous_candidate_source",
                    failure.previous_candidate_source,
                )?;
                failure_item.set_item("next_candidate_source", failure.next_candidate_source)?;
                failure_item.set_item("previous_candidate_key", failure.previous_candidate_key)?;
                failure_item.set_item("next_candidate_key", failure.next_candidate_key)?;
                failure_item.set_item(
                    "previous_candidate_source_edge",
                    failure.previous_candidate_source_edge,
                )?;
                failure_item.set_item(
                    "next_candidate_source_edge",
                    failure.next_candidate_source_edge,
                )?;
                failure_item.set_item("previous_origin", failure.previous_origin)?;
                failure_item.set_item("next_origin", failure.next_origin)?;
                failure_item.set_item("previous_left", failure.previous_left)?;
                failure_item.set_item("previous_right", failure.previous_right)?;
                failure_item.set_item("next_left", failure.next_left)?;
                failure_item.set_item("next_right", failure.next_right)?;
                failure_item.set_item("previous_next_edge_id", failure.previous_next_edge_id)?;
                failure_item.set_item("next_prev_edge_id", failure.next_prev_edge_id)?;
                set_linked_edge(
                    py,
                    &failure_item,
                    "previous_next_edge",
                    &failure.previous_next_edge,
                )?;
                set_linked_edge(py, &failure_item, "next_prev_edge", &failure.next_prev_edge)?;
                set_ring_diagnostic(
                    py,
                    &failure_item,
                    "previous_left_ring",
                    &failure.previous_left_ring,
                )?;
                set_ring_diagnostic(
                    py,
                    &failure_item,
                    "next_right_ring",
                    &failure.next_right_ring,
                )?;
                set_target_snapshot(
                    py,
                    &failure_item,
                    "previous_target_snapshot",
                    failure.previous_target_snapshot.as_ref(),
                )?;
                set_target_snapshot(
                    py,
                    &failure_item,
                    "next_target_snapshot",
                    failure.next_target_snapshot.as_ref(),
                )?;
                failure_item.set_item(
                    "captured_open_target_reopened_previous",
                    failure.captured_open_target_reopened_previous,
                )?;
                failure_item.set_item(
                    "captured_open_target_reopened_next",
                    failure.captured_open_target_reopened_next,
                )?;
                failure_item.set_item(
                    "captured_open_target_retry_error",
                    failure.captured_open_target_retry_error,
                )?;
                failure_item.set_item("error", failure.error)?;
                failures.append(failure_item)?;
            }
            candidates.set_item("failures", failures)?;
            if let Some(fallback_from) = candidate_diagnostics.fallback_from {
                let fallback = PyDict::new(py);
                fallback.set_item("attempt", fallback_from.attempt)?;
                fallback.set_item("error", fallback_from.error)?;
                fallback.set_item("previous_candidates", fallback_from.previous_candidates)?;
                fallback.set_item("next_candidates", fallback_from.next_candidates)?;
                fallback.set_item("failure_count", fallback_from.failure_count)?;
                candidates.set_item("fallback_from", fallback)?;
            } else {
                candidates.set_item("fallback_from", py.None())?;
            }
            set_source_lookup(
                py,
                &candidates,
                "previous_source_lookup",
                candidate_diagnostics.previous_source_lookup.as_ref(),
            )?;
            set_source_lookup(
                py,
                &candidates,
                "next_source_lookup",
                candidate_diagnostics.next_source_lookup.as_ref(),
            )?;
            item.set_item("candidate_diagnostics", candidates)?;
        } else {
            item.set_item("candidate_diagnostics", py.None())?;
        }
        output.append(item)?;
    }
    Ok(output.unbind())
}

fn set_source_lookup(
    py: Python<'_>,
    item: &Bound<'_, PyDict>,
    key: &str,
    lookup: Option<&zennah_geometry_core::MeshlibNearStitchSourceLookupDiagnostic>,
) -> PyResult<()> {
    let Some(lookup) = lookup else {
        return item.set_item(key, py.None());
    };
    let output = PyDict::new(py);
    output.set_item("requested_halfedge", lookup.requested_halfedge)?;
    output.set_item("requested_key_face", lookup.requested_key_face)?;
    output.set_item("requested_key_edge", lookup.requested_key_edge)?;
    output.set_item("requested_source_edge", lookup.requested_source_edge)?;
    output.set_item("fallback_edge", lookup.fallback_edge)?;
    output.set_item("exact_key_candidates", lookup.exact_key_candidates)?;
    output.set_item("same_edge_key_candidates", lookup.same_edge_key_candidates)?;
    output.set_item("halfedge_candidates", lookup.halfedge_candidates)?;
    output.set_item("source_edge_candidates", lookup.source_edge_candidates)?;
    output.set_item("topology_candidates", lookup.topology_candidates)?;
    output.set_item("total_candidates", lookup.total_candidates)?;
    if let Some(copied_source_edge) = &lookup.copied_source_edge {
        let copied = PyDict::new(py);
        copied.set_item("status", copied_source_edge.status)?;
        copied.set_item(
            "matched_source_edge",
            copied_source_edge.matched_source_edge,
        )?;
        copied.set_item("source_halfedge", copied_source_edge.source_halfedge)?;
        copied.set_item("source_origin", copied_source_edge.source_origin)?;
        copied.set_item("source_left", copied_source_edge.source_left)?;
        copied.set_item("source_right", copied_source_edge.source_right)?;
        copied.set_item(
            "source_left_mapped_face",
            copied_source_edge.source_left_mapped_face,
        )?;
        copied.set_item(
            "source_right_mapped_face",
            copied_source_edge.source_right_mapped_face,
        )?;
        copied.set_item(
            "source_next_halfedge",
            copied_source_edge.source_next_halfedge,
        )?;
        copied.set_item(
            "source_prev_halfedge",
            copied_source_edge.source_prev_halfedge,
        )?;
        copied.set_item("output_edge_id", copied_source_edge.output_edge_id)?;
        copied.set_item("output_origin", copied_source_edge.output_origin)?;
        copied.set_item("output_left", copied_source_edge.output_left)?;
        copied.set_item("output_right", copied_source_edge.output_right)?;
        copied.set_item(
            "output_next_edge_id",
            copied_source_edge.output_next_edge_id,
        )?;
        copied.set_item(
            "output_prev_edge_id",
            copied_source_edge.output_prev_edge_id,
        )?;
        copied.set_item("matching_statuses", copied_source_edge.matching_statuses)?;
        output.set_item("copied_source_edge", copied)?;
    } else {
        output.set_item("copied_source_edge", py.None())?;
    }
    item.set_item(key, output)
}

fn set_linked_edge(
    py: Python<'_>,
    item: &Bound<'_, PyDict>,
    key: &str,
    detail: &zennah_geometry_core::MeshlibNearStitchLinkedEdgeDiagnostic,
) -> PyResult<()> {
    let edge = PyDict::new(py);
    edge.set_item("edge_id", detail.edge_id)?;
    edge.set_item("origin", detail.origin)?;
    edge.set_item("left", detail.left)?;
    edge.set_item("right", detail.right)?;
    item.set_item(key, edge)
}

fn set_ring_diagnostic(
    py: Python<'_>,
    item: &Bound<'_, PyDict>,
    key: &str,
    detail: &zennah_geometry_core::MeshlibNearStitchRingDiagnostic,
) -> PyResult<()> {
    let ring = PyDict::new(py);
    ring.set_item("edge_ids", &detail.edge_ids)?;
    ring.set_item("origins", &detail.origins)?;
    ring.set_item("left_faces", &detail.left_faces)?;
    ring.set_item("error", detail.error)?;
    item.set_item(key, ring)
}

fn set_target_snapshot(
    py: Python<'_>,
    item: &Bound<'_, PyDict>,
    key: &str,
    snapshot: Option<&zennah_geometry_core::MeshlibNearStitchTargetSnapshotDiagnostic>,
) -> PyResult<()> {
    let Some(snapshot) = snapshot else {
        return item.set_item(key, py.None());
    };
    let output = PyDict::new(py);
    output.set_item("edge_id", snapshot.edge_id)?;
    output.set_item("origin", snapshot.origin)?;
    output.set_item("left", snapshot.left)?;
    output.set_item("right", snapshot.right)?;
    output.set_item("next_edge_id", snapshot.next_edge_id)?;
    output.set_item("prev_edge_id", snapshot.prev_edge_id)?;
    item.set_item(key, output)
}
