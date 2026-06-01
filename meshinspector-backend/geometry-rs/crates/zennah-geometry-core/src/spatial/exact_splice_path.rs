use super::exact_halfedge::ExactHalfEdgeTopology;
use super::exact_splice::{
    ExactTopologySpliceEntry, ExactTopologySplicePlan, ExactTopologySpliceStatus,
};
use super::exact_stitch::ExactStitchPath;

#[derive(Default)]
pub(super) struct StitchPathVerificationSummary {
    pub stitched_paths: usize,
    pub verified_boundary_paths: usize,
    pub blocked_paths: usize,
    pub failed_paths: usize,
}

pub(super) fn verify_stitch_paths(
    splice_plan: &ExactTopologySplicePlan,
    stitch_paths: &[ExactStitchPath],
) -> StitchPathVerificationSummary {
    let mut summary = StitchPathVerificationSummary {
        stitched_paths: stitch_paths.len(),
        ..StitchPathVerificationSummary::default()
    };
    for path in stitch_paths {
        let entries = path
            .pair_indices
            .iter()
            .filter_map(|index| splice_plan.entries.get(*index))
            .collect::<Vec<_>>();
        if entries.len() != path.pair_indices.len() || path_blocked(&entries) {
            summary.blocked_paths += 1;
        } else if entries
            .iter()
            .all(|entry| entry.status == ExactTopologySpliceStatus::Manifold)
            || verify_boundary_stitch_path(path, &entries).is_ok()
        {
            summary.verified_boundary_paths += 1;
        } else {
            summary.failed_paths += 1;
        }
    }
    summary
}

fn path_blocked(entries: &[&ExactTopologySpliceEntry]) -> bool {
    entries.is_empty()
        || entries.iter().any(|entry| {
            matches!(
                entry.status,
                ExactTopologySpliceStatus::Missing | ExactTopologySpliceStatus::NonManifold
            )
        })
        || entries
            .iter()
            .any(|entry| entry.first_stitch_edge.is_none() || entry.second_stitch_edge.is_none())
}

fn verify_boundary_stitch_path(
    path: &ExactStitchPath,
    entries: &[&ExactTopologySpliceEntry],
) -> Result<(), &'static str> {
    let first_path = entries
        .iter()
        .map(|entry| {
            entry
                .first_stitch_edge
                .ok_or("missing first stitch path edge")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let second_path = entries
        .iter()
        .map(|entry| {
            entry
                .second_stitch_edge
                .ok_or("missing second stitch path edge")
        })
        .collect::<Result<Vec<_>, _>>()?;
    require_path_continuity(&first_path)?;
    require_counter_path_continuity(&second_path)?;
    if path.closed {
        require_closed_path_continuity(&first_path)?;
        require_closed_counter_path_continuity(&second_path)?;
    }

    for (first_edge, second_edge) in first_path.into_iter().zip(second_path) {
        verify_boundary_stitch_pair(first_edge, second_edge)?;
    }
    Ok(())
}

fn require_path_continuity(edges: &[[usize; 2]]) -> Result<(), &'static str> {
    for window in edges.windows(2) {
        if window[0][1] != window[1][0] {
            return Err("stitch path edges are not contiguous");
        }
    }
    Ok(())
}

fn require_counter_path_continuity(edges: &[[usize; 2]]) -> Result<(), &'static str> {
    for window in edges.windows(2) {
        if window[0][0] != window[1][1] {
            return Err("counter stitch path edges are not contiguous");
        }
    }
    Ok(())
}

fn require_closed_path_continuity(edges: &[[usize; 2]]) -> Result<(), &'static str> {
    if edges.last().map(|edge| edge[1]) != edges.first().map(|edge| edge[0]) {
        return Err("closed stitch path does not return to the first vertex");
    }
    Ok(())
}

fn require_closed_counter_path_continuity(edges: &[[usize; 2]]) -> Result<(), &'static str> {
    if edges.last().map(|edge| edge[0]) != edges.first().map(|edge| edge[1]) {
        return Err("closed counter stitch path does not return to the first vertex");
    }
    Ok(())
}

fn verify_boundary_stitch_pair(
    first_edge: [usize; 2],
    second_edge: [usize; 2],
) -> Result<(), &'static str> {
    let mut topology = ExactHalfEdgeTopology::new();
    let first = topology.make_edge(Some(first_edge[0]), Some(first_edge[1]));
    let second = topology.make_edge(Some(second_edge[0]), Some(second_edge[1]));
    topology.stitch_contours(&[first], &[second])
}

#[cfg(test)]
mod tests {
    use super::super::exact_boolean::ExactBooleanStitchedEdgeSource;
    use super::super::exact_splice::exact_topology_splice_plan;
    use super::*;

    fn stitched_edge(output_edge: [usize; 2]) -> ExactBooleanStitchedEdgeSource {
        ExactBooleanStitchedEdgeSource {
            output_edge: ordered_edge(output_edge),
            first_output_edge: Some(output_edge),
            second_output_edge: Some([output_edge[1], output_edge[0]]),
            first_stitch_edge: Some(output_edge),
            second_stitch_edge: Some([output_edge[1], output_edge[0]]),
            first_stitch_edge_synthetic: false,
            second_stitch_edge_synthetic: false,
            first_edge_index: 0,
            second_edge_index: 0,
            first_cut_edge: output_edge,
            second_cut_edge: output_edge,
        }
    }

    fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
        if edge[0] <= edge[1] {
            edge
        } else {
            [edge[1], edge[0]]
        }
    }

    #[test]
    fn verify_stitch_paths_blocks_missing_pair_indices() {
        let faces = [[0, 1, 2]];
        let splice_plan = exact_topology_splice_plan(&faces, &[stitched_edge([1, 2])]);
        let paths = [ExactStitchPath {
            pair_indices: vec![0, 10],
            closed: false,
        }];

        let summary = verify_stitch_paths(&splice_plan, &paths);

        assert_eq!(summary.stitched_paths, 1);
        assert_eq!(summary.blocked_paths, 1);
        assert_eq!(summary.verified_boundary_paths, 0);
        assert_eq!(summary.failed_paths, 0);
    }

    #[test]
    fn verify_stitch_paths_accepts_closed_boundary_stitch_path() {
        let faces = [[0, 1, 3], [1, 2, 3], [2, 0, 3]];
        let splice_plan = exact_topology_splice_plan(
            &faces,
            &[
                stitched_edge([0, 1]),
                stitched_edge([1, 2]),
                stitched_edge([2, 0]),
            ],
        );
        let paths = [ExactStitchPath {
            pair_indices: vec![0, 1, 2],
            closed: true,
        }];

        let summary = verify_stitch_paths(&splice_plan, &paths);

        assert_eq!(summary.stitched_paths, 1);
        assert_eq!(summary.verified_boundary_paths, 1);
        assert_eq!(summary.blocked_paths, 0);
        assert_eq!(summary.failed_paths, 0);
    }

    #[test]
    fn verify_stitch_paths_rejects_open_edges_marked_closed() {
        let faces = [[0, 1, 3], [1, 2, 3]];
        let splice_plan =
            exact_topology_splice_plan(&faces, &[stitched_edge([0, 1]), stitched_edge([1, 2])]);
        let paths = [ExactStitchPath {
            pair_indices: vec![0, 1],
            closed: true,
        }];

        let summary = verify_stitch_paths(&splice_plan, &paths);

        assert_eq!(summary.stitched_paths, 1);
        assert_eq!(summary.verified_boundary_paths, 0);
        assert_eq!(summary.blocked_paths, 0);
        assert_eq!(summary.failed_paths, 1);
    }
}
