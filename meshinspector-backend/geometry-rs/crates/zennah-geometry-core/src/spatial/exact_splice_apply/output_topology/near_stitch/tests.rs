use super::*;
use crate::spatial::exact_boolean::ExactBooleanOperand;
use crate::spatial::exact_meshlib_near_stitch::ExactMeshlibSourceHalfedgeKey;

#[test]
fn near_stitch_candidates_report_best_meshlib_guard_progress() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let mismatched_previous = output.topology.make_edge(Some(1), Some(9));
    let guarded_previous = output.topology.make_edge(Some(7), Some(8));
    let next = output.topology.make_edge(Some(7), Some(10));
    output
        .topology
        .set_left_direct(guarded_previous, Some(42))
        .unwrap();

    let error = output
        .apply_meshlib_near_stitch_candidates(
            &[mismatched_previous, guarded_previous],
            &[next],
            "missing MeshLib near stitch edge",
        )
        .unwrap_err();

    assert_eq!(error, "previous near stitch edge must not have a left face");
    let diagnostics = output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .unwrap();
    assert_eq!(diagnostics.attempt, "test-candidates");
    assert_eq!(diagnostics.previous_candidates, 2);
    assert_eq!(diagnostics.next_candidates, 1);
    assert!(diagnostics.fallback_from.is_none());
    assert_eq!(diagnostics.failures.len(), 2);
    assert_eq!(diagnostics.failures[1].previous_edge_id, guarded_previous.0);
    assert_eq!(diagnostics.failures[1].previous_origin, Some(7));
    assert_eq!(diagnostics.failures[1].next_origin, Some(7));
    assert_eq!(diagnostics.failures[1].previous_left, Some(42));
    assert_eq!(diagnostics.failures[1].previous_right, None);
    assert_eq!(diagnostics.failures[1].next_left, None);
    assert_eq!(
        diagnostics.failures[1].previous_next_edge_id,
        guarded_previous.0
    );
    assert_eq!(diagnostics.failures[1].next_prev_edge_id, next.0);
    assert_eq!(
        diagnostics.failures[1].previous_left_ring.edge_ids.first(),
        Some(&guarded_previous.0)
    );
    assert_eq!(
        diagnostics.failures[1].previous_left_ring.edge_ids.len(),
        diagnostics.failures[1].previous_left_ring.origins.len()
    );
    assert_eq!(
        diagnostics.failures[1].next_right_ring.edge_ids.len(),
        diagnostics.failures[1].next_right_ring.left_faces.len()
    );
}

#[test]
fn near_stitch_missing_candidates_report_empty_side_counts() {
    let mut output = OutputFaceTopology::from_faces(&[[0, 1, 2]]).unwrap();

    let error = output
        .apply_meshlib_near_stitch_edge_update([1, 0], [7, 8])
        .unwrap_err();

    assert_eq!(error, "missing MeshLib near stitch next edge");
    let diagnostics = output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .unwrap();
    assert_eq!(diagnostics.attempt, "vertex-pair");
    assert_eq!(diagnostics.previous_candidates, 1);
    assert_eq!(diagnostics.next_candidates, 0);
    assert!(diagnostics.fallback_from.is_none());
    assert!(diagnostics.failures.is_empty());
}

#[test]
fn near_stitch_source_candidates_fall_back_to_guarded_output_edge() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let previous = output.topology.make_edge(Some(1), Some(9));
    let wrong_source_next = output.topology.make_edge(Some(99), Some(100));
    let fallback_next = output.topology.make_edge(Some(1), Some(10));
    output
        .meshlib_near_stitch_target_edges
        .insert((0, ExactMeshlibNearStitchEndpoint::Start), vec![previous]);
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(4),
        None,
        None,
        wrong_source_next,
    );
    output.register_meshlib_copied_edge(
        ExactBooleanOperand::Second,
        [1, 10],
        [1, 10],
        fallback_next,
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: Some(4),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: None,
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap();

    assert_eq!(output.topology.next(previous), fallback_next);
    assert!(output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .is_none());
}

#[test]
fn near_stitch_candidate_diagnostics_include_meshlib_source_edge_identity() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let previous = output.topology.make_edge(Some(1), Some(9));
    let wrong_source_next = output.topology.make_edge(Some(99), Some(100));
    output
        .meshlib_near_stitch_target_edges
        .insert((0, ExactMeshlibNearStitchEndpoint::Start), vec![previous]);
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(4),
        None,
        Some([2, 1]),
        wrong_source_next,
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: Some(4),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: Some([2, 3]),
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [2, 3],
    };

    let error = output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap_err();

    assert_eq!(error, "near stitch edges must share origin");
    let diagnostics = output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .unwrap();
    assert_eq!(diagnostics.failures.len(), 1);
    assert_eq!(diagnostics.attempt, "identity-target-source");
    assert_eq!(
        diagnostics.failures[0].next_candidate_source,
        "source-halfedge"
    );
    assert_eq!(
        diagnostics.failures[0].next_candidate_source_edge,
        Some([2, 1])
    );
    assert_eq!(diagnostics.failures[0].previous_candidate_source_edge, None);
}

#[test]
fn near_stitch_fallback_diagnostics_preserve_identity_attempt_counts() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let source_next = output.topology.make_edge(Some(1), Some(10));
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(4),
        None,
        Some([1, 10]),
        source_next,
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: Some(4),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: Some([1, 10]),
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    let error = output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap_err();

    assert_eq!(error, "missing MeshLib near stitch previous edge");
    let diagnostics = output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .unwrap();
    assert_eq!(diagnostics.attempt, "vertex-pair-fallback");
    assert_eq!(diagnostics.previous_candidates, 0);
    assert_eq!(diagnostics.next_candidates, 0);
    assert!(diagnostics.failures.is_empty());
    let source_lookup = diagnostics.next_source_lookup.unwrap();
    assert_eq!(source_lookup.requested_halfedge, Some(4));
    assert_eq!(source_lookup.requested_source_edge, Some([1, 10]));
    assert_eq!(source_lookup.exact_key_candidates, 0);
    assert_eq!(source_lookup.same_edge_key_candidates, 0);
    assert_eq!(source_lookup.halfedge_candidates, 1);
    assert_eq!(source_lookup.source_edge_candidates, 0);
    assert_eq!(source_lookup.topology_candidates, 0);
    assert_eq!(source_lookup.total_candidates, 1);
    let identity_attempt = diagnostics.fallback_from.unwrap();
    assert_eq!(identity_attempt.attempt, "identity-target-source");
    assert_eq!(
        identity_attempt.error,
        "missing MeshLib near stitch target edge"
    );
    assert_eq!(identity_attempt.previous_candidates, 0);
    assert_eq!(identity_attempt.next_candidates, 1);
    assert_eq!(identity_attempt.failure_count, 0);
}

#[test]
fn near_stitch_source_lookup_reports_not_prepared_source_edges() {
    let output = OutputFaceTopology::from_faces(&[]).unwrap();

    let lookup = output.meshlib_near_stitch_source_candidates(
        Some(ExactBooleanOperand::Second),
        None,
        None,
        Some([1, 10]),
        [1, 10],
    );

    let copied_source_edge = lookup.diagnostics.copied_source_edge.unwrap();
    assert_eq!(
        copied_source_edge.status.label(),
        "not-prepared-source-edge"
    );
    assert_eq!(copied_source_edge.matched_source_edge, None);
    assert_eq!(copied_source_edge.matching_statuses, 0);
    assert_eq!(copied_source_edge.source_halfedge, None);
    assert_eq!(copied_source_edge.output_edge_id, None);
    assert_eq!(copied_source_edge.output_origin, None);
    assert_eq!(copied_source_edge.output_left, None);
    assert_eq!(copied_source_edge.output_right, None);
    assert_eq!(copied_source_edge.output_next_edge_id, None);
    assert_eq!(copied_source_edge.output_prev_edge_id, None);
}

#[test]
fn near_stitch_diagnostics_report_captured_target_retry_guard() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let previous = output.topology.make_edge(Some(1), Some(9));
    let next = output.topology.make_edge(Some(1), Some(10));
    output.register_meshlib_near_stitch_target_edges(0, ExactHalfEdgeTopology::sym(previous));
    output.topology.set_left_direct(previous, Some(42)).unwrap();
    output
        .topology
        .set_left_direct(ExactHalfEdgeTopology::sym(next), Some(50))
        .unwrap();
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(4),
        None,
        Some([1, 10]),
        next,
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: Some(4),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: Some([1, 10]),
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    let error = output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap_err();

    assert_eq!(error, "previous near stitch edge must not have a left face");
    let diagnostics = output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .unwrap();
    assert_eq!(diagnostics.failures.len(), 1);
    let failure = &diagnostics.failures[0];
    assert_eq!(failure.previous_edge_id, previous.0);
    assert_eq!(failure.next_edge_id, next.0);
    assert!(failure.captured_open_target_reopened_previous);
    assert!(!failure.captured_open_target_reopened_next);
    assert_eq!(
        failure.captured_open_target_retry_error,
        Some("next near stitch edge must not have a right face")
    );
    assert_eq!(output.topology.left(previous), Some(42));
}

#[test]
fn near_stitch_source_candidates_prefer_stable_source_halfedge_key() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let previous = output.topology.make_edge(Some(1), Some(9));
    let wrong_local_next = output.topology.make_edge(Some(99), Some(100));
    let keyed_next = output.topology.make_edge(Some(1), Some(10));
    let source_key = ExactMeshlibSourceHalfedgeKey {
        face: 12,
        edge: [2, 3],
    };
    output
        .meshlib_near_stitch_target_edges
        .insert((0, ExactMeshlibNearStitchEndpoint::Start), vec![previous]);
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(4),
        None,
        Some([99, 100]),
        wrong_local_next,
    );
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(42),
        Some(source_key),
        Some([2, 3]),
        keyed_next,
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: Some(4),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: Some(source_key),
        previous_source_edge: None,
        next_source_edge: Some([2, 3]),
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap();

    assert_eq!(output.topology.next(previous), keyed_next);
    assert!(output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .is_none());
}

#[test]
fn near_stitch_source_candidates_fall_back_to_same_source_edge_key() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let previous = output.topology.make_edge(Some(1), Some(9));
    let keyed_next = output.topology.make_edge(Some(1), Some(10));
    let stored_key = ExactMeshlibSourceHalfedgeKey {
        face: 12,
        edge: [2, 3],
    };
    let requested_key = ExactMeshlibSourceHalfedgeKey {
        face: 99,
        edge: [2, 3],
    };
    output
        .meshlib_near_stitch_target_edges
        .insert((0, ExactMeshlibNearStitchEndpoint::Start), vec![previous]);
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(42),
        Some(stored_key),
        Some([2, 3]),
        keyed_next,
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: None,
        previous_source_halfedge_key: None,
        next_source_halfedge_key: Some(requested_key),
        previous_source_edge: None,
        next_source_edge: Some([2, 3]),
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap();

    assert_eq!(output.topology.next(previous), keyed_next);
    assert!(output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .is_none());
}

#[test]
fn near_stitch_source_candidates_fall_back_to_reversed_source_edge_key() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let previous = output.topology.make_edge(Some(1), Some(9));
    let reversed_key_next = output.topology.make_edge(Some(10), Some(1));
    let stored_key = ExactMeshlibSourceHalfedgeKey {
        face: 12,
        edge: [10, 1],
    };
    let requested_key = ExactMeshlibSourceHalfedgeKey {
        face: 99,
        edge: [1, 10],
    };
    output
        .meshlib_near_stitch_target_edges
        .insert((0, ExactMeshlibNearStitchEndpoint::Start), vec![previous]);
    output.register_meshlib_source_halfedge(
        ExactBooleanOperand::Second,
        ExactHalfEdgeId(42),
        Some(stored_key),
        Some([10, 1]),
        reversed_key_next,
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: None,
        previous_source_halfedge_key: None,
        next_source_halfedge_key: Some(requested_key),
        previous_source_edge: None,
        next_source_edge: Some([1, 10]),
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap();

    assert_eq!(
        output.topology.next(previous),
        ExactHalfEdgeTopology::sym(reversed_key_next)
    );
    assert!(output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .is_none());
}

#[test]
fn near_stitch_target_candidates_fall_back_to_guarded_face_edge() {
    let mut output = OutputFaceTopology::from_faces(&[[9, 1, 3], [1, 10, 4]]).unwrap();
    let wrong_target_previous = output.topology.make_edge(Some(99), Some(100));
    output.meshlib_near_stitch_target_edges.insert(
        (0, ExactMeshlibNearStitchEndpoint::Start),
        vec![wrong_target_previous],
    );
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: None,
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: None,
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap();

    let previous = output
        .topology_face_edge_candidates_for_directed_edge([1, 9])
        .into_iter()
        .find(|edge| output.topology.left(*edge).is_none())
        .unwrap();
    let next = output
        .topology_face_edge_candidates_for_directed_edge([1, 10])
        .into_iter()
        .find(|edge| output.topology.right(*edge).is_none())
        .unwrap();
    assert_eq!(output.topology.next(previous), next);
    assert!(output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .is_none());
}

#[test]
fn near_stitch_target_candidates_include_materialized_topology_edges() {
    let mut output = OutputFaceTopology::from_faces(&[]).unwrap();
    let previous = output.topology.make_edge(Some(1), Some(9));
    let next = output.topology.make_edge(Some(1), Some(10));
    output.register_meshlib_copied_edge(ExactBooleanOperand::First, [1, 9], [1, 9], previous);
    output.register_meshlib_copied_edge(ExactBooleanOperand::Second, [1, 10], [1, 10], next);
    let command = ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(0),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(ExactBooleanOperand::Second),
        previous_source_halfedge: None,
        next_source_halfedge: None,
        previous_source_halfedge_key: None,
        next_source_halfedge_key: None,
        previous_source_edge: None,
        next_source_edge: None,
        strict_source_identity: true,
        previous_edge: [1, 9],
        next_edge: [1, 10],
    };

    output
        .apply_meshlib_near_stitch_edge_update_command(&command)
        .unwrap();

    assert_eq!(output.topology.next(previous), next);
    assert!(output
        .take_meshlib_near_stitch_candidate_diagnostics()
        .is_none());
}

#[test]
fn near_stitch_target_registration_retains_duplicate_contour_candidates() {
    let mut output = OutputFaceTopology::from_faces(&[[0, 1, 2], [0, 1, 3]]).unwrap();
    let first_target = ExactHalfEdgeTopology::sym(output.directed_face_edge(0, [0, 1]).unwrap());
    let second_target = ExactHalfEdgeTopology::sym(output.directed_face_edge(1, [0, 1]).unwrap());

    output.register_meshlib_near_stitch_target_edge_candidates(
        7,
        [first_target, second_target, first_target],
    );

    assert_eq!(
        output.meshlib_near_stitch_target_edge_count(7, ExactMeshlibNearStitchEndpoint::Start),
        2
    );
    assert_eq!(
        output.meshlib_near_stitch_target_edge_count(7, ExactMeshlibNearStitchEndpoint::End),
        2
    );
}
