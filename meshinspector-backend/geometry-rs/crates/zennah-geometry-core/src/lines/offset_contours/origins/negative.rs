use super::{OffsetContourIndex, OffsetContoursOrigin};

#[derive(Clone, Copy)]
pub(super) struct SourceEdge {
    pub(super) org: usize,
    pub(super) dest: usize,
}

pub(super) fn negative_intersection_origin(
    contour_id: usize,
    index: usize,
    points: &[[f64; 3]],
    intersection: [f64; 3],
) -> OffsetContoursOrigin {
    let previous = if index == 0 {
        points.len() - 1
    } else {
        index - 1
    };
    let next = (index + 1) % points.len();
    let incoming = canonical_intersection_edge(previous, index, points);
    let outgoing = canonical_intersection_edge(index, next, points);
    let (lower, upper) =
        if source_edge_angle(incoming, points) >= source_edge_angle(outgoing, points) {
            (incoming, outgoing)
        } else {
            (outgoing, incoming)
        };
    OffsetContoursOrigin {
        l_org: source_index(contour_id, lower.org),
        l_dest: source_index(contour_id, lower.dest),
        u_org: source_index(contour_id, upper.org),
        u_dest: source_index(contour_id, upper.dest),
        l_ratio: source_edge_ratio(lower, points, intersection),
        u_ratio: source_edge_ratio(upper, points, intersection),
    }
}

fn canonical_intersection_edge(org: usize, dest: usize, points: &[[f64; 3]]) -> SourceEdge {
    let org_point = points[org];
    let dest_point = points[dest];
    if dest_point[0] > org_point[0] {
        return SourceEdge { org, dest };
    }
    SourceEdge {
        org: dest,
        dest: org,
    }
}

pub(super) fn canonical_ascending_edge(org: usize, dest: usize, points: &[[f64; 3]]) -> SourceEdge {
    let org_point = points[org];
    let dest_point = points[dest];
    if dest_point[0] > org_point[0] + 1e-12
        || ((dest_point[0] - org_point[0]).abs() <= 1e-12 && dest_point[1] >= org_point[1])
    {
        return SourceEdge { org, dest };
    }
    SourceEdge {
        org: dest,
        dest: org,
    }
}

pub(super) fn negative_variable_intersection_origin(
    contour_id: usize,
    index: usize,
    points: &[[f64; 3]],
    intersection: [f64; 3],
) -> OffsetContoursOrigin {
    let previous = if index == 0 {
        points.len() - 1
    } else {
        index - 1
    };
    let next = (index + 1) % points.len();
    let incoming = canonical_ascending_edge(previous, index, points);
    let outgoing = canonical_ascending_edge(index, next, points);
    let (lower, upper) =
        if source_edge_angle(incoming, points) >= source_edge_angle(outgoing, points) {
            (incoming, outgoing)
        } else {
            (outgoing, incoming)
        };
    OffsetContoursOrigin {
        l_org: source_index(contour_id, lower.org),
        l_dest: source_index(contour_id, lower.dest),
        u_org: source_index(contour_id, upper.org),
        u_dest: source_index(contour_id, upper.dest),
        l_ratio: source_edge_ratio(lower, points, intersection),
        u_ratio: source_edge_ratio(upper, points, intersection),
    }
}

pub(super) fn source_edge_angle(edge: SourceEdge, points: &[[f64; 3]]) -> f64 {
    let org = points[edge.org];
    let dest = points[edge.dest];
    (dest[1] - org[1]).atan2(dest[0] - org[0])
}

pub(super) fn source_edge_ratio(edge: SourceEdge, points: &[[f64; 3]], point: [f64; 3]) -> f64 {
    let org = points[edge.org];
    let dest = points[edge.dest];
    let segment = [dest[0] - org[0], dest[1] - org[1]];
    let denominator = segment[0] * segment[0] + segment[1] * segment[1];
    if denominator <= 1e-24 {
        return 0.0;
    }
    (((point[0] - org[0]) * segment[0] + (point[1] - org[1]) * segment[1]) / denominator)
        .clamp(0.0, 1.0)
}

pub(super) fn source_index(contour_id: usize, vert_id: usize) -> OffsetContourIndex {
    OffsetContourIndex {
        contour_id: contour_id as i32,
        vert_id: vert_id as i32,
    }
}
