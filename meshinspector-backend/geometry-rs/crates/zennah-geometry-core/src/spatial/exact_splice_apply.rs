use super::exact_halfedge::ExactHalfEdgeTopology;
use super::exact_splice::{ExactTopologySplicePlan, ExactTopologySpliceStatus};
use super::exact_splice_path::verify_stitch_paths;
use super::exact_stitch::ExactStitchPath;
use std::collections::BTreeMap;

mod copied_edges;
mod output_topology;
mod prepared_base;
mod prepared_base_contours;
mod source_records;
pub(super) use copied_edges::{
    meshlib_copied_vertex_map_for_input, ExactMeshlibCopiedEdgeTranslationInput,
    ExactMeshlibCopiedSourceEdgeLookupDiagnostic,
};
pub(super) use output_topology::OutputFaceTopology;
pub(crate) use output_topology::{
    ExactMeshlibCopiedFaceRecordCandidateDiagnostic, ExactMeshlibCopiedFaceRecordDiagnostic,
    ExactMeshlibCopiedPrevNextEdgeUpdateDiagnostic, ExactMeshlibFaceExportFailureDiagnostic,
    ExactMeshlibNearStitchCandidateDiagnostics, ExactMeshlibNearStitchLinkedEdgeDiagnostic,
    ExactMeshlibNearStitchRingDiagnostic, ExactMeshlibNearStitchSourceLookupDiagnostics,
    ExactMeshlibNearStitchTargetSnapshot, ExactMeshlibPreparedSourceRecordReplayDiagnostic,
    ExactMeshlibRecordRewriteTargetDiagnostic,
};
pub(super) use prepared_base::{
    output_topology_from_prepared_base, ExactMeshlibPreparedBaseTopologyInput,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactTopologySpliceApplyStatus {
    AlreadyManifold,
    VerifiedBoundaryStitch,
    BlockedMissing,
    BlockedMissingSide,
    BlockedNonManifold,
    FailedBoundaryStitch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactTopologySpliceApplyEntry {
    pub stitched_edge_index: usize,
    pub output_edge: [usize; 2],
    pub first_output_edge: Option<[usize; 2]>,
    pub second_output_edge: Option<[usize; 2]>,
    pub first_stitch_edge: Option<[usize; 2]>,
    pub second_stitch_edge: Option<[usize; 2]>,
    pub first_stitch_edge_synthetic: bool,
    pub second_stitch_edge_synthetic: bool,
    pub incident_faces: Vec<usize>,
    pub directed_face_edge: Option<[usize; 2]>,
    pub status: ExactTopologySpliceApplyStatus,
    pub error: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactTopologySpliceApplyPlan {
    pub entries: Vec<ExactTopologySpliceApplyEntry>,
    pub already_manifold_edges: usize,
    pub verified_boundary_edges: usize,
    pub blocked_edges: usize,
    pub failed_edges: usize,
    pub synthetic_side_edges: usize,
    pub materialized_boundary_edges: usize,
    pub materialization_failed_edges: usize,
    pub stitched_paths: usize,
    pub verified_boundary_paths: usize,
    pub blocked_paths: usize,
    pub failed_paths: usize,
    pub exported_faces: usize,
    pub export_failed_faces: usize,
    pub exported_face_indices: Vec<[i64; 3]>,
    pub exported_manifold_edges: usize,
    pub exported_boundary_edges: usize,
    pub exported_non_manifold_edges: usize,
    pub topology_edges_before_materialization: usize,
    pub topology_edges_after_materialization: usize,
    pub deleted_synthetic_stitch_edges: usize,
    pub duplicated_output_topology_edges: usize,
    pub ready_for_mutation: bool,
}

pub fn exact_topology_splice_apply_plan(
    faces: &[[i64; 3]],
    splice_plan: &ExactTopologySplicePlan,
    stitch_paths: &[ExactStitchPath],
) -> ExactTopologySpliceApplyPlan {
    let mut entries = Vec::with_capacity(splice_plan.entries.len());
    let mut already_manifold_edges = 0;
    let mut verified_boundary_edges = 0;
    let mut blocked_edges = 0;
    let mut failed_edges = 0;
    let mut synthetic_side_edges = 0;
    let mut materialized_boundary_edges = 0;
    let mut materialization_failed_edges = 0;
    let path_summary = verify_stitch_paths(splice_plan, stitch_paths);
    let mut output_topology = OutputFaceTopology::from_faces(faces);
    let path_materialization =
        materialize_stitch_paths(&mut output_topology, faces, splice_plan, stitch_paths);
    let topology_edges_before_materialization = output_topology
        .as_ref()
        .map(OutputFaceTopology::not_lone_undirected_edge_count)
        .unwrap_or(0);

    for entry in &splice_plan.entries {
        let directed_face_edge =
            directed_edge_for_entry(faces, &entry.incident_faces, entry.output_edge);
        let (status, error) = match entry.status {
            ExactTopologySpliceStatus::Missing => {
                blocked_edges += 1;
                (ExactTopologySpliceApplyStatus::BlockedMissing, None)
            }
            ExactTopologySpliceStatus::NonManifold => {
                blocked_edges += 1;
                (ExactTopologySpliceApplyStatus::BlockedNonManifold, None)
            }
            ExactTopologySpliceStatus::Manifold => {
                already_manifold_edges += 1;
                (ExactTopologySpliceApplyStatus::AlreadyManifold, None)
            }
            ExactTopologySpliceStatus::BoundaryNeedsSplice => {
                if entry.first_stitch_edge_synthetic {
                    synthetic_side_edges += 1;
                }
                if entry.second_stitch_edge_synthetic {
                    synthetic_side_edges += 1;
                }
                if entry.first_stitch_edge.is_none() || entry.second_stitch_edge.is_none() {
                    blocked_edges += 1;
                    (
                        ExactTopologySpliceApplyStatus::BlockedMissingSide,
                        Some("missing directed seam side"),
                    )
                } else if let Some(result) = path_materialization.get(&entry.stitched_edge_index) {
                    match *result {
                        Ok(()) => {
                            verified_boundary_edges += 1;
                            materialized_boundary_edges += 1;
                            (ExactTopologySpliceApplyStatus::VerifiedBoundaryStitch, None)
                        }
                        Err(error) => {
                            failed_edges += 1;
                            materialization_failed_edges += 1;
                            (
                                ExactTopologySpliceApplyStatus::FailedBoundaryStitch,
                                Some(error),
                            )
                        }
                    }
                } else {
                    match verify_boundary_stitch(
                        entry.output_edge,
                        entry.first_stitch_edge,
                        entry.second_stitch_edge,
                    ) {
                        Ok(()) => match materialize_boundary_stitch(
                            &mut output_topology,
                            entry,
                            directed_face_edge,
                        ) {
                            Ok(()) => {
                                verified_boundary_edges += 1;
                                materialized_boundary_edges += 1;
                                (ExactTopologySpliceApplyStatus::VerifiedBoundaryStitch, None)
                            }
                            Err(error) => {
                                failed_edges += 1;
                                materialization_failed_edges += 1;
                                (
                                    ExactTopologySpliceApplyStatus::FailedBoundaryStitch,
                                    Some(error),
                                )
                            }
                        },
                        Err(error) => {
                            failed_edges += 1;
                            (
                                ExactTopologySpliceApplyStatus::FailedBoundaryStitch,
                                Some(error),
                            )
                        }
                    }
                }
            }
        };
        entries.push(ExactTopologySpliceApplyEntry {
            stitched_edge_index: entry.stitched_edge_index,
            output_edge: entry.output_edge,
            first_output_edge: entry.first_output_edge,
            second_output_edge: entry.second_output_edge,
            first_stitch_edge: entry.first_stitch_edge,
            second_stitch_edge: entry.second_stitch_edge,
            first_stitch_edge_synthetic: entry.first_stitch_edge_synthetic,
            second_stitch_edge_synthetic: entry.second_stitch_edge_synthetic,
            incident_faces: entry.incident_faces.clone(),
            directed_face_edge,
            status,
            error,
        });
    }
    let exported_face_indices = output_topology
        .as_ref()
        .map(|topology| topology.export_faces())
        .unwrap_or_else(|_| Err("output topology build failed"))
        .unwrap_or_default();
    let exported_faces = exported_face_indices.len();
    let export_failed_faces = faces.len().abs_diff(exported_faces);
    let (exported_manifold_edges, exported_boundary_edges, exported_non_manifold_edges) =
        exported_edge_counts(&exported_face_indices);
    let topology_edges_after_materialization = output_topology
        .as_ref()
        .map(OutputFaceTopology::not_lone_undirected_edge_count)
        .unwrap_or(0);
    let deleted_synthetic_stitch_edges = output_topology
        .as_ref()
        .map(OutputFaceTopology::deleted_synthetic_stitch_edges)
        .unwrap_or(0);
    let duplicated_output_topology_edges = output_topology
        .as_ref()
        .map(|topology| topology.duplicated_directed_edges)
        .unwrap_or(0);

    ExactTopologySpliceApplyPlan {
        entries,
        already_manifold_edges,
        verified_boundary_edges,
        blocked_edges,
        failed_edges,
        synthetic_side_edges,
        materialized_boundary_edges,
        materialization_failed_edges,
        stitched_paths: path_summary.stitched_paths,
        verified_boundary_paths: path_summary.verified_boundary_paths,
        blocked_paths: path_summary.blocked_paths,
        failed_paths: path_summary.failed_paths,
        exported_faces,
        export_failed_faces,
        exported_face_indices,
        exported_manifold_edges,
        exported_boundary_edges,
        exported_non_manifold_edges,
        topology_edges_before_materialization,
        topology_edges_after_materialization,
        deleted_synthetic_stitch_edges,
        duplicated_output_topology_edges,
        ready_for_mutation: blocked_edges == 0
            && failed_edges == 0
            && path_summary.blocked_paths == 0
            && path_summary.failed_paths == 0
            && export_failed_faces == 0,
    }
}

fn verify_boundary_stitch(
    output_edge: [usize; 2],
    first_output_edge: Option<[usize; 2]>,
    second_output_edge: Option<[usize; 2]>,
) -> Result<(), &'static str> {
    let first_output_edge = first_output_edge.ok_or("missing first directed seam edge")?;
    let second_output_edge = second_output_edge.ok_or("missing second directed seam edge")?;
    if ordered_edge(first_output_edge) != output_edge {
        return Err("first directed seam edge does not match stitched output edge");
    }
    if ordered_edge(second_output_edge) != output_edge {
        return Err("second directed seam edge does not match stitched output edge");
    }
    let mut topology = ExactHalfEdgeTopology::new();
    let first = topology.make_edge(Some(first_output_edge[0]), Some(first_output_edge[1]));
    let second = topology.make_edge(Some(second_output_edge[0]), Some(second_output_edge[1]));
    topology.stitch_contours(&[first], &[second])
}

fn materialize_boundary_stitch(
    output_topology: &mut Result<OutputFaceTopology, &'static str>,
    entry: &super::exact_splice::ExactTopologySpliceEntry,
    directed_face_edge: Option<[usize; 2]>,
) -> Result<(), &'static str> {
    let output_topology = output_topology.as_mut().map_err(|error| *error)?;
    let face_index = *entry
        .incident_faces
        .first()
        .ok_or("missing incident face for boundary stitch")?;
    let directed_face_edge = directed_face_edge.ok_or("missing incident face direction")?;
    if ordered_edge(directed_face_edge) != entry.output_edge {
        return Err("incident face edge does not match stitched output edge");
    }

    let open_face_edge = reverse_edge(directed_face_edge);
    if !stitch_edges_contain(entry, open_face_edge) {
        return Err("stitch metadata does not contain output open side");
    }
    if !stitch_edges_contain(entry, directed_face_edge) {
        return Err("stitch metadata does not contain opposite stitch side");
    }

    let face_edge = output_topology
        .directed_face_edge(face_index, directed_face_edge)
        .ok_or("missing output topology face edge")?;
    let open_face_edge_id = ExactHalfEdgeTopology::sym(face_edge);
    let synthetic_edge = output_topology.add_synthetic_stitch_edge(directed_face_edge);
    output_topology
        .topology
        .stitch_contours(&[open_face_edge_id], &[synthetic_edge])
}

fn materialize_stitch_paths(
    output_topology: &mut Result<OutputFaceTopology, &'static str>,
    faces: &[[i64; 3]],
    splice_plan: &ExactTopologySplicePlan,
    stitch_paths: &[ExactStitchPath],
) -> BTreeMap<usize, Result<(), &'static str>> {
    let mut results = BTreeMap::new();
    for path in stitch_paths {
        let entries = path
            .pair_indices
            .iter()
            .filter_map(|index| splice_plan.entries.get(*index))
            .collect::<Vec<_>>();
        if entries.len() != path.pair_indices.len()
            || entries.is_empty()
            || !entries
                .iter()
                .all(|entry| entry.status == ExactTopologySpliceStatus::BoundaryNeedsSplice)
        {
            continue;
        }
        let result = materialize_boundary_stitch_path(output_topology, faces, path, &entries);
        for entry in entries {
            results.insert(entry.stitched_edge_index, result);
        }
    }
    results
}

fn materialize_boundary_stitch_path(
    output_topology: &mut Result<OutputFaceTopology, &'static str>,
    faces: &[[i64; 3]],
    path: &ExactStitchPath,
    entries: &[&super::exact_splice::ExactTopologySpliceEntry],
) -> Result<(), &'static str> {
    let output_topology = output_topology.as_mut().map_err(|error| *error)?;
    let mut open_edges = Vec::with_capacity(entries.len());
    let mut synthetic_edges = Vec::with_capacity(entries.len());
    let mut directed_edges = Vec::with_capacity(entries.len());
    for entry in entries {
        let face_index = *entry
            .incident_faces
            .first()
            .ok_or("missing incident face for boundary stitch path")?;
        let directed = directed_edge_for_entry(faces, &entry.incident_faces, entry.output_edge)
            .ok_or("missing incident face direction")?;
        if !stitch_edges_contain(entry, reverse_edge(directed))
            || !stitch_edges_contain(entry, directed)
        {
            return Err("stitch path metadata does not contain both output sides");
        }
        let face_edge = output_topology
            .directed_face_edge(face_index, directed)
            .ok_or("missing output topology face edge")?;
        directed_edges.push(directed);
        open_edges.push(ExactHalfEdgeTopology::sym(face_edge));
        synthetic_edges.push(output_topology.add_synthetic_stitch_edge(directed));
    }
    validate_materialized_stitch_path(path, &directed_edges)?;
    output_topology
        .topology
        .stitch_contours(&open_edges, &synthetic_edges)
}

fn validate_materialized_stitch_path(
    path: &ExactStitchPath,
    directed_edges: &[[usize; 2]],
) -> Result<(), &'static str> {
    for window in directed_edges.windows(2) {
        if window[0][1] != window[1][0] {
            return Err("materialized stitch path edges are not contiguous");
        }
    }
    if path.closed
        && directed_edges.last().map(|edge| edge[1]) != directed_edges.first().map(|edge| edge[0])
    {
        return Err("materialized closed stitch path does not return to the first vertex");
    }
    Ok(())
}

fn stitch_edges_contain(
    entry: &super::exact_splice::ExactTopologySpliceEntry,
    edge: [usize; 2],
) -> bool {
    entry.first_stitch_edge == Some(edge) || entry.second_stitch_edge == Some(edge)
}

fn directed_edge_for_entry(
    faces: &[[i64; 3]],
    incident_faces: &[usize],
    output_edge: [usize; 2],
) -> Option<[usize; 2]> {
    let face_index = *incident_faces.first()?;
    let face = faces.get(face_index)?;
    let face = [face[0] as usize, face[1] as usize, face[2] as usize];
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
        .into_iter()
        .find(|&edge| ordered_edge(edge) == output_edge)
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

fn exported_edge_counts(faces: &[[i64; 3]]) -> (usize, usize, usize) {
    let mut edge_counts = BTreeMap::<[i64; 2], usize>::new();
    for face in faces {
        for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
            *edge_counts.entry(ordered_edge_i64(edge)).or_default() += 1;
        }
    }
    let manifold = edge_counts.values().filter(|&&count| count == 2).count();
    let boundary = edge_counts.values().filter(|&&count| count == 1).count();
    let non_manifold = edge_counts.values().filter(|&&count| count > 2).count();
    (manifold, boundary, non_manifold)
}

fn ordered_edge_i64(edge: [i64; 2]) -> [i64; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

#[cfg(test)]
mod tests;
