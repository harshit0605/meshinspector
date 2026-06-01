use super::super::exact_boolean::ExactBooleanOperation;
use super::super::exact_fill_apply::ExactCutHoleFillResult;
use super::requires_topology_splice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::spatial) struct MeshlibResultCutPathSummary {
    pub(in crate::spatial) paths: usize,
    pub(in crate::spatial) path_edges: usize,
    pub(in crate::spatial) closed_paths: usize,
}

pub(in crate::spatial) fn meshlib_result_cut_path_summary(
    operation: ExactBooleanOperation,
    first_cut: &ExactCutHoleFillResult,
    second_cut: &ExactCutHoleFillResult,
) -> MeshlibResultCutPathSummary {
    let (paths, closed) = meshlib_result_cut_path_source(operation, first_cut, second_cut);
    MeshlibResultCutPathSummary {
        paths: paths.len(),
        path_edges: paths.iter().map(Vec::len).sum(),
        closed_paths: closed.iter().filter(|&&is_closed| is_closed).count(),
    }
}

fn meshlib_result_cut_path_source<'a>(
    operation: ExactBooleanOperation,
    first_cut: &'a ExactCutHoleFillResult,
    second_cut: &'a ExactCutHoleFillResult,
) -> (&'a [Vec<[usize; 2]>], &'a [bool]) {
    let source = if requires_topology_splice(operation) {
        if operation == ExactBooleanOperation::Intersection {
            &second_cut.mesh
        } else {
            &first_cut.mesh
        }
    } else if matches!(
        operation,
        ExactBooleanOperation::InsideA | ExactBooleanOperation::OutsideA
    ) {
        &first_cut.mesh
    } else {
        &second_cut.mesh
    };
    (&source.cut_edge_paths, &source.cut_edge_path_closed)
}

pub(super) fn cut_path_length_mismatches(
    first_paths: &[Vec<[usize; 2]>],
    second_paths: &[Vec<[usize; 2]>],
) -> usize {
    let shared_mismatches = first_paths
        .iter()
        .zip(second_paths)
        .filter(|(first, second)| first.len() != second.len())
        .count();
    shared_mismatches + first_paths.len().abs_diff(second_paths.len())
}
