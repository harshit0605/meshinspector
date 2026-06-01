use super::super::exact_cut::{ExactCutPathSegment, ExactCutPreplan, ExactCutPrimitive};
use std::collections::BTreeSet;

pub(super) fn cut_edge_paths_from_preplan(
    preplan: &ExactCutPreplan,
    cut_edges: &[[usize; 2]],
) -> Vec<Vec<[usize; 2]>> {
    let cut_edge_set = cut_edges.iter().copied().collect::<BTreeSet<_>>();
    let mut paths = vec![Vec::new(); preplan.contour_points.len()];
    for segment in &preplan.path_segments {
        let from = preplan.cut_points[segment.from_point].vertex_index;
        let to = preplan.cut_points[segment.to_point].vertex_index;
        if from == to {
            continue;
        }
        let edge = [from, to];
        if cut_edge_set.contains(&ordered_edge(edge)) {
            let path = &mut paths[segment.contour_index];
            if path.last().copied() != Some(edge) {
                path.push(edge);
            }
        }
    }
    paths
}

pub(super) fn directed_path_is_closed(path: &[[usize; 2]]) -> bool {
    path.len() > 1
        && path.windows(2).all(|window| window[0][1] == window[1][0])
        && path.first().map(|edge| edge[0]) == path.last().map(|edge| edge[1])
}

pub(super) fn segment_lies_on_shared_boundary_edge(
    segment: &ExactCutPathSegment,
    preplan: &ExactCutPreplan,
) -> bool {
    if segment.source_faces.len() <= 1 || segment.source_faces.len() > 2 {
        return false;
    }
    let from = preplan.cut_points[segment.from_point].primitive;
    let to = preplan.cut_points[segment.to_point].primitive;
    cut_primitives_share_boundary_edge(from, to)
}

fn cut_primitives_share_boundary_edge(from: ExactCutPrimitive, to: ExactCutPrimitive) -> bool {
    match (from, to) {
        (ExactCutPrimitive::Edge(left), ExactCutPrimitive::Edge(right)) => {
            ordered_edge(left) == ordered_edge(right)
        }
        (ExactCutPrimitive::Vertex(vertex), ExactCutPrimitive::Edge(edge))
        | (ExactCutPrimitive::Edge(edge), ExactCutPrimitive::Vertex(vertex)) => {
            edge.contains(&vertex)
        }
        (ExactCutPrimitive::Vertex(_), ExactCutPrimitive::Vertex(_)) => true,
        _ => false,
    }
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}
