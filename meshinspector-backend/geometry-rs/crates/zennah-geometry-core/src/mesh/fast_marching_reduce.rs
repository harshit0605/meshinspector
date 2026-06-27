use std::collections::VecDeque;

use crate::math::{add, distance_sq, dot, norm, scale, sub};

use super::fast_marching_prune::collapse_and_prune_crossing_locations;
use super::triangle_strip::mesh_triangle_strip_unfolded_path;

const VERTEX_EPSILON: f64 = 1e-9;

pub(super) fn reduce_single_crossing(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    start_face_index: usize,
    start_point: [f64; 3],
    end_face_index: usize,
    end_point: [f64; 3],
    edge: [usize; 2],
) -> Option<(f64, [f64; 3])> {
    let start_face = faces.get(start_face_index)?;
    let end_face = faces.get(end_face_index)?;
    if !start_face.contains(&edge[0])
        || !start_face.contains(&edge[1])
        || !end_face.contains(&edge[0])
        || !end_face.contains(&edge[1])
    {
        return None;
    }
    let start_unfolded = unfold_point_against_edge(vertices, edge, start_point)?;
    let end_unfolded = unfold_point_against_edge(vertices, edge, end_point)?;
    let denominator = start_unfolded[1] + end_unfolded[1];
    if denominator <= VERTEX_EPSILON {
        return None;
    }
    let position = (start_unfolded[0]
        + start_unfolded[1] / denominator * (end_unfolded[0] - start_unfolded[0]))
        .clamp(0.0, 1.0);
    Some((
        position,
        edge_point(vertices[edge[0]], vertices[edge[1]], position),
    ))
}

pub(super) fn reduce_adjacent_face_crossing(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    start_face_index: usize,
    start_point: [f64; 3],
    end_face_index: usize,
    end_point: [f64; 3],
) -> Option<([usize; 2], f64, [f64; 3])> {
    if start_face_index == end_face_index {
        return None;
    }
    let start_face = *faces.get(start_face_index)?;
    let end_face = *faces.get(end_face_index)?;
    let common = start_face
        .into_iter()
        .filter(|vertex| end_face.contains(vertex))
        .collect::<Vec<_>>();
    if common.len() != 2 {
        return None;
    }
    let edge = if common[0] <= common[1] {
        [common[0], common[1]]
    } else {
        [common[1], common[0]]
    };
    let (position, point) = reduce_single_crossing(
        vertices,
        faces,
        start_face_index,
        start_point,
        end_face_index,
        end_point,
        edge,
    )?;
    Some((edge, position, point))
}

pub(super) fn reduce_best_vertex_fan_crossing(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    start_face_index: usize,
    start_point: [f64; 3],
    end_face_index: usize,
    end_point: [f64; 3],
    approximate_edges: &[[usize; 2]],
    approximate_positions: &[f64],
    current_points: &[[f64; 3]],
) -> Option<(Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>)> {
    let current_length = path_length(start_point, current_points, end_point);
    let mut best: Option<(f64, Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>)> = None;
    for (edge, position) in approximate_edges.iter().zip(approximate_positions.iter()) {
        let Some(vertex) = edge_position_vertex(*edge, *position) else {
            continue;
        };
        let Some((candidate_edges, candidate_positions, candidate_points)) =
            reduce_vertex_fan_crossing(
                vertices,
                faces,
                start_face_index,
                start_point,
                end_face_index,
                end_point,
                vertex,
            )
        else {
            continue;
        };
        let candidate_length = path_length(start_point, &candidate_points, end_point);
        if candidate_length <= current_length + VERTEX_EPSILON
            && best
                .as_ref()
                .is_none_or(|(best_length, _, _, _)| candidate_length < *best_length)
        {
            best = Some((
                candidate_length,
                candidate_edges,
                candidate_positions,
                candidate_points,
            ));
        }
    }
    best.map(|(_, edges, positions, points)| {
        collapse_and_prune_crossing_locations(
            faces,
            start_face_index,
            end_face_index,
            edges,
            positions,
            points,
        )
    })
}

pub(super) fn reduce_repeated_location_strip_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    faces: &[[usize; 3]],
    start_face_index: usize,
    start_point: [f64; 3],
    end_face_index: usize,
    end_point: [f64; 3],
    approximate_edges: &[[usize; 2]],
    approximate_positions: &[f64],
    current_points: &[[f64; 3]],
) -> Option<(Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>)> {
    let repeated_strip_edges = reduce_repeated_location_strip_edges(
        faces,
        start_face_index,
        end_face_index,
        approximate_edges,
        approximate_positions,
    )?;
    let crossed_edges = repeated_strip_edges
        .iter()
        .map(|edge| [edge[0] as i64, edge[1] as i64])
        .collect::<Vec<_>>();
    let unfolded = mesh_triangle_strip_unfolded_path(
        vertices,
        faces_i64,
        start_face_index,
        &crossed_edges,
        end_face_index,
        start_point,
        end_point,
    )
    .ok()?;
    let current_length = path_length(start_point, current_points, end_point);
    if unfolded.length_mm > current_length + VERTEX_EPSILON {
        return None;
    }
    Some(collapse_and_prune_crossing_locations(
        faces,
        start_face_index,
        end_face_index,
        unfolded.oriented_edges,
        unfolded.crossing_positions,
        unfolded.crossing_points,
    ))
}

fn reduce_repeated_location_strip_edges(
    faces: &[[usize; 3]],
    start_face_index: usize,
    end_face_index: usize,
    approximate_edges: &[[usize; 2]],
    approximate_positions: &[f64],
) -> Option<Vec<[usize; 2]>> {
    if approximate_edges.len() < 2
        || !has_repeated_location_signal(faces, approximate_edges, approximate_positions)
    {
        return None;
    }
    let max_crossings = approximate_edges.len().saturating_sub(1);
    let face_path = shortest_face_path(faces, start_face_index, end_face_index, max_crossings)?;
    let crossed_edges = face_path
        .windows(2)
        .map(|pair| shared_edge(faces[pair[0]], faces[pair[1]]))
        .collect::<Option<Vec<_>>>()?;
    (crossed_edges.len() < approximate_edges.len()).then_some(crossed_edges)
}

fn reduce_vertex_fan_crossing(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    start_face_index: usize,
    start_point: [f64; 3],
    end_face_index: usize,
    end_point: [f64; 3],
    vertex: usize,
) -> Option<(Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>)> {
    if start_face_index == end_face_index {
        return None;
    }
    if !faces.get(start_face_index)?.contains(&vertex)
        || !faces.get(end_face_index)?.contains(&vertex)
    {
        return None;
    }

    let paths = vertex_face_paths(faces, vertex, start_face_index, end_face_index);
    let mut best: Option<(f64, Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>)> = None;
    for path in paths {
        let Some(candidate) =
            reduce_vertex_fan_path(vertices, faces, &path, start_point, end_point, vertex)
        else {
            continue;
        };
        let length = path_length(start_point, &candidate.2, end_point);
        if best
            .as_ref()
            .is_none_or(|(best_length, _, _, _)| length < *best_length)
        {
            best = Some((length, candidate.0, candidate.1, candidate.2));
        }
    }
    best.map(|(_, edges, positions, points)| (edges, positions, points))
}

fn unfold_point_against_edge(
    vertices: &[[f64; 3]],
    edge: [usize; 2],
    point: [f64; 3],
) -> Option<[f64; 2]> {
    let origin = vertices[edge[0]];
    let dest = vertices[edge[1]];
    let edge_vec = sub(dest, origin);
    let edge_len_sq = distance_sq(origin, dest);
    if edge_len_sq <= VERTEX_EPSILON {
        return None;
    }
    let position = dot(sub(point, origin), edge_vec) / edge_len_sq;
    let projection = edge_point(origin, dest, position);
    let height = distance_sq(point, projection).sqrt();
    Some([position, height])
}

fn edge_point(a: [f64; 3], b: [f64; 3], position: f64) -> [f64; 3] {
    add(scale(a, 1.0 - position), scale(b, position))
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

fn has_repeated_location_signal(
    faces: &[[usize; 3]],
    edges: &[[usize; 2]],
    positions: &[f64],
) -> bool {
    for index in 1..edges.len() {
        if sorted_edge(edges[index - 1]) == sorted_edge(edges[index])
            && (edge_position_vertex(edges[index - 1], positions[index - 1]).is_some()
                || edge_position_vertex(edges[index], positions[index]).is_some())
        {
            return true;
        }
    }
    for index in 2..edges.len() {
        if edge_position_vertex(edges[index], positions[index]).is_none()
            && shared_face(faces, edges[index - 2], edges[index]).is_some()
        {
            return true;
        }
    }
    false
}

fn shortest_face_path(
    faces: &[[usize; 3]],
    start_face_index: usize,
    end_face_index: usize,
    max_crossings: usize,
) -> Option<Vec<usize>> {
    if start_face_index == end_face_index {
        return Some(vec![start_face_index]);
    }
    let mut queue = VecDeque::from([vec![start_face_index]]);
    let mut visited = vec![false; faces.len()];
    visited[start_face_index] = true;
    while let Some(path) = queue.pop_front() {
        if path.len().saturating_sub(1) >= max_crossings {
            continue;
        }
        let current = *path.last()?;
        let mut neighbors = faces
            .iter()
            .enumerate()
            .filter_map(|(candidate, face)| {
                (!visited[candidate] && shared_edge(faces[current], *face).is_some())
                    .then_some(candidate)
            })
            .collect::<Vec<_>>();
        neighbors.sort_unstable();
        for neighbor in neighbors {
            let mut next_path = path.clone();
            next_path.push(neighbor);
            if neighbor == end_face_index {
                return Some(next_path);
            }
            visited[neighbor] = true;
            queue.push_back(next_path);
        }
    }
    None
}

fn shared_edge(left: [usize; 3], right: [usize; 3]) -> Option<[usize; 2]> {
    let common = left
        .into_iter()
        .filter(|vertex| right.contains(vertex))
        .collect::<Vec<_>>();
    if common.len() != 2 {
        return None;
    }
    Some(sorted_edge([common[0], common[1]]))
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

fn reduce_vertex_fan_path(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    face_path: &[usize],
    start_point: [f64; 3],
    end_point: [f64; 3],
    vertex: usize,
) -> Option<(Vec<[usize; 2]>, Vec<f64>, Vec<[f64; 3]>)> {
    if face_path.len() < 2 {
        return None;
    }

    let mut crossed = Vec::with_capacity(face_path.len() - 1);
    for pair in face_path.windows(2) {
        crossed.push(common_outer_vertex(faces[pair[0]], faces[pair[1]], vertex)?);
    }

    let start_outer = other_face_vertex(faces[*face_path.first()?], vertex, crossed[0])?;
    let end_outer = other_face_vertex(faces[*face_path.last()?], vertex, *crossed.last()?)?;
    let mut boundary = Vec::with_capacity(crossed.len() + 2);
    boundary.push(start_outer);
    boundary.extend(crossed.iter().copied());
    boundary.push(end_outer);

    let boundary_points = unfold_radial_boundary(vertices, vertex, &boundary)?;
    let start_barycentric = barycentric_in_triangle(
        vertices[vertex],
        vertices[boundary[0]],
        vertices[boundary[1]],
        start_point,
    )?;
    let end_index = boundary.len() - 1;
    let end_barycentric = barycentric_in_triangle(
        vertices[vertex],
        vertices[boundary[end_index - 1]],
        vertices[boundary[end_index]],
        end_point,
    )?;
    let start_2d = barycentric_point_2d(
        [0.0, 0.0],
        boundary_points[0],
        boundary_points[1],
        start_barycentric,
    );
    let end_2d = barycentric_point_2d(
        [0.0, 0.0],
        boundary_points[end_index - 1],
        boundary_points[end_index],
        end_barycentric,
    );

    let mut edges = Vec::with_capacity(crossed.len());
    let mut positions = Vec::with_capacity(crossed.len());
    let mut points = Vec::with_capacity(crossed.len());
    for (index, outer_vertex) in crossed.into_iter().enumerate() {
        let radial = boundary_points[index + 1];
        let position = line_intersection_on_radial(radial, start_2d, end_2d)?.clamp(0.0, 1.0);
        if position <= VERTEX_EPSILON {
            return None;
        }
        edges.push([vertex, outer_vertex]);
        positions.push(position);
        points.push(edge_point(
            vertices[vertex],
            vertices[outer_vertex],
            position,
        ));
    }
    Some((edges, positions, points))
}

fn vertex_face_paths(
    faces: &[[usize; 3]],
    vertex: usize,
    start_face_index: usize,
    end_face_index: usize,
) -> Vec<Vec<usize>> {
    let vertex_faces = faces
        .iter()
        .enumerate()
        .filter_map(|(index, face)| face.contains(&vertex).then_some(index))
        .collect::<Vec<_>>();
    let mut paths = Vec::new();
    let mut path = vec![start_face_index];
    collect_vertex_face_paths(
        faces,
        vertex,
        end_face_index,
        vertex_faces.len().max(1),
        &mut path,
        &mut paths,
    );
    paths
}

fn collect_vertex_face_paths(
    faces: &[[usize; 3]],
    vertex: usize,
    end_face_index: usize,
    max_depth: usize,
    path: &mut Vec<usize>,
    paths: &mut Vec<Vec<usize>>,
) {
    if paths.len() >= 64 {
        return;
    }
    let current = *path.last().expect("path always has a start face");
    if current == end_face_index {
        paths.push(path.clone());
        return;
    }
    if path.len() > max_depth {
        return;
    }
    let mut neighbors = faces
        .iter()
        .enumerate()
        .filter_map(|(candidate, face)| {
            (candidate != current
                && !path.contains(&candidate)
                && common_outer_vertex(faces[current], *face, vertex).is_some())
            .then_some(candidate)
        })
        .collect::<Vec<_>>();
    neighbors.sort_unstable();
    for neighbor in neighbors {
        path.push(neighbor);
        collect_vertex_face_paths(faces, vertex, end_face_index, max_depth, path, paths);
        path.pop();
    }
}

fn common_outer_vertex(left: [usize; 3], right: [usize; 3], vertex: usize) -> Option<usize> {
    left.into_iter()
        .find(|candidate| *candidate != vertex && right.contains(candidate))
}

fn other_face_vertex(face: [usize; 3], vertex: usize, known_outer: usize) -> Option<usize> {
    face.into_iter()
        .find(|candidate| *candidate != vertex && *candidate != known_outer)
}

fn unfold_radial_boundary(
    vertices: &[[f64; 3]],
    center: usize,
    boundary: &[usize],
) -> Option<Vec<[f64; 2]>> {
    let center_point = vertices[center];
    let first = sub(vertices[*boundary.first()?], center_point);
    let first_length = norm(first);
    if first_length <= VERTEX_EPSILON {
        return None;
    }
    let mut angle = 0.0;
    let mut points = vec![[first_length, 0.0]];
    for pair in boundary.windows(2) {
        let previous = sub(vertices[pair[0]], center_point);
        let next = sub(vertices[pair[1]], center_point);
        let previous_length = norm(previous);
        let next_length = norm(next);
        if previous_length <= VERTEX_EPSILON || next_length <= VERTEX_EPSILON {
            return None;
        }
        let cosine = (dot(previous, next) / (previous_length * next_length)).clamp(-1.0, 1.0);
        angle += cosine.acos();
        points.push([next_length * angle.cos(), next_length * angle.sin()]);
    }
    Some(points)
}

fn barycentric_in_triangle(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    point: [f64; 3],
) -> Option<[f64; 3]> {
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let v2 = sub(point, a);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denominator = d00 * d11 - d01 * d01;
    if denominator.abs() <= VERTEX_EPSILON {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / denominator;
    let w = (d00 * d21 - d01 * d20) / denominator;
    Some([1.0 - v - w, v, w])
}

fn barycentric_point_2d(a: [f64; 2], b: [f64; 2], c: [f64; 2], barycentric: [f64; 3]) -> [f64; 2] {
    [
        barycentric[0] * a[0] + barycentric[1] * b[0] + barycentric[2] * c[0],
        barycentric[0] * a[1] + barycentric[1] * b[1] + barycentric[2] * c[1],
    ]
}

fn line_intersection_on_radial(radial: [f64; 2], start: [f64; 2], end: [f64; 2]) -> Option<f64> {
    let c1 = cross_2d(end, start);
    let c2 = cross_2d(sub_2d(start, radial), sub_2d(end, radial));
    let denominator = c1 + c2;
    if denominator.abs() <= VERTEX_EPSILON {
        return None;
    }
    Some(c1 / denominator)
}

fn path_length(start: [f64; 3], points: &[[f64; 3]], end: [f64; 3]) -> f64 {
    let mut length = 0.0;
    let mut previous = start;
    for point in points {
        length += distance_sq(previous, *point).sqrt();
        previous = *point;
    }
    length + distance_sq(previous, end).sqrt()
}

fn sub_2d(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] - right[0], left[1] - right[1]]
}

fn cross_2d(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[1] - left[1] * right[0]
}
