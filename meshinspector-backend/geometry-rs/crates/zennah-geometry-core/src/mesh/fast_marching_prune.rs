const VERTEX_EPSILON: f64 = 1e-9;

pub(super) fn collapse_repeated_crossing_locations(
    edges: Vec<[usize; 2]>,
    positions: Vec<f64>,
    points: Vec<[f64; 3]>,
) -> (Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>, bool) {
    let mut collapsed_edges = Vec::with_capacity(edges.len());
    let mut collapsed_positions = Vec::with_capacity(positions.len());
    let mut collapsed_points = Vec::with_capacity(points.len());
    let mut changed = false;

    for ((edge, position), point) in edges.into_iter().zip(positions).zip(points) {
        let previous_location = collapsed_edges.last().zip(collapsed_positions.last());
        if previous_location.is_some_and(|(previous_edge, previous_position)| {
            same_crossing_location(*previous_edge, *previous_position, edge, position)
        }) {
            *collapsed_edges
                .last_mut()
                .expect("previous crossing location exists") = edge;
            *collapsed_positions
                .last_mut()
                .expect("previous crossing position exists") = position;
            *collapsed_points
                .last_mut()
                .expect("previous crossing point exists") = point;
            changed = true;
        } else {
            collapsed_edges.push(edge);
            collapsed_positions.push(position);
            collapsed_points.push(point);
        }
    }

    (
        collapsed_edges,
        collapsed_positions,
        collapsed_points,
        changed,
    )
}

pub(super) fn collapse_and_prune_crossing_locations(
    faces: &[[usize; 3]],
    start_face_index: usize,
    end_face_index: usize,
    edges: Vec<[usize; 2]>,
    positions: Vec<f64>,
    points: Vec<[f64; 3]>,
) -> (Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>) {
    let collapsed = collapse_repeated_crossing_locations(edges, positions, points);
    prune_same_triangle_nonvertex_detours(
        faces,
        start_face_index,
        end_face_index,
        collapsed.0,
        collapsed.1,
        collapsed.2,
    )
}

fn same_crossing_location(
    previous_edge: [usize; 2],
    previous_position: f64,
    edge: [usize; 2],
    position: f64,
) -> bool {
    let previous_vertex = edge_position_vertex(previous_edge, previous_position);
    let vertex = edge_position_vertex(edge, position);
    if previous_vertex.is_some() || vertex.is_some() {
        return previous_vertex.is_some() && previous_vertex == vertex;
    }
    if sorted_edge(previous_edge) != sorted_edge(edge) {
        return false;
    }
    (normalized_edge_position(previous_edge, previous_position)
        - normalized_edge_position(edge, position))
    .abs()
        <= VERTEX_EPSILON
}

fn normalized_edge_position(edge: [usize; 2], position: f64) -> f64 {
    if edge[0] <= edge[1] {
        position
    } else {
        1.0 - position
    }
}

fn prune_same_triangle_nonvertex_detours(
    faces: &[[usize; 3]],
    start_face_index: usize,
    end_face_index: usize,
    edges: Vec<[usize; 2]>,
    positions: Vec<f64>,
    points: Vec<[f64; 3]>,
) -> (Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>) {
    let mut out_edges = Vec::with_capacity(edges.len());
    let mut out_positions = Vec::with_capacity(positions.len());
    let mut out_points = Vec::with_capacity(points.len());

    for ((edge, position), point) in edges.into_iter().zip(positions).zip(points) {
        if edge_position_vertex(edge, position).is_none() {
            while out_edges.len() >= 2 {
                let previous_kept = out_edges.len() - 2;
                if edge_position_vertex(out_edges[previous_kept], out_positions[previous_kept])
                    .is_some()
                    || shared_face(faces, edge, out_edges[previous_kept]).is_none()
                {
                    break;
                }
                out_edges.pop();
                out_positions.pop();
                out_points.pop();
            }
            if out_edges.len() == 1 && edge_in_face(faces[start_face_index], edge) {
                out_edges.pop();
                out_positions.pop();
                out_points.pop();
            }
        }
        out_edges.push(edge);
        out_positions.push(position);
        out_points.push(point);
    }
    while out_edges.len() >= 2
        && edge_in_face(faces[end_face_index], out_edges[out_edges.len() - 2])
    {
        out_edges.pop();
        out_positions.pop();
        out_points.pop();
    }

    (out_edges, out_positions, out_points)
}

fn edge_position_vertex(edge: [usize; 2], position: f64) -> Option<usize> {
    if position <= VERTEX_EPSILON {
        Some(edge[0])
    } else if position >= 1.0 - VERTEX_EPSILON {
        Some(edge[1])
    } else {
        None
    }
}

fn shared_face(faces: &[[usize; 3]], left: [usize; 2], right: [usize; 2]) -> Option<usize> {
    faces
        .iter()
        .position(|face| edge_in_face(*face, left) && edge_in_face(*face, right))
}

fn edge_in_face(face: [usize; 3], edge: [usize; 2]) -> bool {
    face.contains(&edge[0]) && face.contains(&edge[1])
}

fn sorted_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}
