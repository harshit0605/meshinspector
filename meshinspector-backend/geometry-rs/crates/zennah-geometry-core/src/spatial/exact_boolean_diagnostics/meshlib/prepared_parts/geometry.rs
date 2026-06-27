#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EdgeCoordinateRelation {
    SameDirection,
    Reversed,
    Mismatch,
}

pub(super) fn valid_face_vertices(face: [i64; 3], vertex_count: usize) -> Option<[usize; 3]> {
    let vertices = [
        usize::try_from(face[0]).ok()?,
        usize::try_from(face[1]).ok()?,
        usize::try_from(face[2]).ok()?,
    ];
    vertices
        .iter()
        .all(|vertex| *vertex < vertex_count)
        .then_some(vertices)
}

pub(super) fn directed_face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
}

#[cfg(test)]
pub(super) fn edge_points(vertices: &[[f64; 3]], edge: [usize; 2]) -> Option<[[f64; 3]; 2]> {
    Some([*vertices.get(edge[0])?, *vertices.get(edge[1])?])
}

pub(super) fn edge_coordinate_relation(
    first_vertices: &[[f64; 3]],
    second_vertices: &[[f64; 3]],
    first_edge: [usize; 2],
    second_edge: [usize; 2],
    tolerance_sq: f64,
) -> EdgeCoordinateRelation {
    let Some(first_from) = first_vertices.get(first_edge[0]).copied() else {
        return EdgeCoordinateRelation::Mismatch;
    };
    let Some(first_to) = first_vertices.get(first_edge[1]).copied() else {
        return EdgeCoordinateRelation::Mismatch;
    };
    let Some(second_from) = second_vertices.get(second_edge[0]).copied() else {
        return EdgeCoordinateRelation::Mismatch;
    };
    let Some(second_to) = second_vertices.get(second_edge[1]).copied() else {
        return EdgeCoordinateRelation::Mismatch;
    };
    if points_close(first_from, second_from, tolerance_sq)
        && points_close(first_to, second_to, tolerance_sq)
    {
        return EdgeCoordinateRelation::SameDirection;
    }
    if points_close(first_from, second_to, tolerance_sq)
        && points_close(first_to, second_from, tolerance_sq)
    {
        return EdgeCoordinateRelation::Reversed;
    }
    EdgeCoordinateRelation::Mismatch
}

fn points_close(first: [f64; 3], second: [f64; 3], tolerance_sq: f64) -> bool {
    squared_distance(first, second) <= tolerance_sq
}

fn squared_distance(first: [f64; 3], second: [f64; 3]) -> f64 {
    let dx = first[0] - second[0];
    let dy = first[1] - second[1];
    let dz = first[2] - second[2];
    dx * dx + dy * dy + dz * dz
}

pub(super) fn effective_epsilon(epsilon: f64) -> f64 {
    let base = if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    };
    base * 4.0
}

pub(super) fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

pub(super) fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}
