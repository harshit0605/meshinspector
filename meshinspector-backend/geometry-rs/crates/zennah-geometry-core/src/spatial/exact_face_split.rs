use crate::math::{cross, dot, norm, sub};

pub(super) fn split_triangle_with_interior_spokes(
    face: [usize; 3],
    interior_vertex: usize,
    boundary_points: &[(usize, f64)],
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> Option<Vec<[usize; 3]>> {
    if boundary_points.is_empty() {
        return None;
    }

    let boundary_loop = boundary_loop_with_inserted_points(face, boundary_points, epsilon)?;
    let mut output = Vec::with_capacity(boundary_loop.len());
    for index in 0..boundary_loop.len() {
        let next = (index + 1) % boundary_loop.len();
        push_if_area(
            &mut output,
            [interior_vertex, boundary_loop[index], boundary_loop[next]],
            vertices,
            epsilon,
        );
    }
    if boundary_points.iter().any(|(vertex, _)| {
        output
            .iter()
            .filter(|triangle| triangle_has_edge(**triangle, [interior_vertex, *vertex]))
            .count()
            < 2
    }) {
        return None;
    }
    Some(output)
}

pub(super) fn split_triangle_with_interior_segment(
    face: [usize; 3],
    first_interior_vertex: usize,
    second_interior_vertex: usize,
    second_coordinate: [f64; 3],
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> Option<Vec<[usize; 3]>> {
    if first_interior_vertex == second_interior_vertex {
        return None;
    }

    let first_fan = [
        [first_interior_vertex, face[0], face[1]],
        [first_interior_vertex, face[1], face[2]],
        [first_interior_vertex, face[2], face[0]],
    ];
    let split_index = first_fan
        .iter()
        .position(|triangle| point_in_triangle(second_coordinate, *triangle, vertices, epsilon))?;

    let mut output = Vec::with_capacity(5);
    for (index, triangle) in first_fan.into_iter().enumerate() {
        if index == split_index {
            for split_face in [
                [second_interior_vertex, triangle[0], triangle[1]],
                [second_interior_vertex, triangle[1], triangle[2]],
                [second_interior_vertex, triangle[2], triangle[0]],
            ] {
                push_if_area(&mut output, split_face, vertices, epsilon);
            }
        } else {
            push_if_area(&mut output, triangle, vertices, epsilon);
        }
    }

    let cut_edge_faces = output
        .iter()
        .filter(|triangle| {
            triangle_has_edge(**triangle, [first_interior_vertex, second_interior_vertex])
        })
        .count();
    if cut_edge_faces < 2 {
        return None;
    }
    Some(output)
}

pub(super) fn split_triangle_with_interior_cycle(
    face: [usize; 3],
    cycle_vertices: &[usize],
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> Option<Vec<[usize; 3]>> {
    if cycle_vertices.len() != 3 {
        return None;
    }

    let mut by_corner = [None; 3];
    for vertex in cycle_vertices {
        let corner = nearest_face_corner(face, *vertex, vertices);
        if by_corner[corner].replace(*vertex).is_some() {
            return None;
        }
    }
    let [Some(q0), Some(q1), Some(q2)] = by_corner else {
        return None;
    };

    let mut output = Vec::with_capacity(7);
    push_if_area(&mut output, [q0, q1, q2], vertices, epsilon);
    for (outer_a, outer_b, inner_a, inner_b) in [
        (face[0], face[1], q0, q1),
        (face[1], face[2], q1, q2),
        (face[2], face[0], q2, q0),
    ] {
        push_if_area(&mut output, [outer_a, outer_b, inner_b], vertices, epsilon);
        push_if_area(&mut output, [outer_a, inner_b, inner_a], vertices, epsilon);
    }

    for edge in [[q0, q1], [q1, q2], [q2, q0]] {
        if output
            .iter()
            .filter(|triangle| triangle_has_edge(**triangle, edge))
            .count()
            < 2
        {
            return None;
        }
    }
    Some(output)
}

pub(super) fn split_triangle_with_boundary_segments(
    face: [usize; 3],
    boundary_points: &[(usize, f64)],
    segments: &[[usize; 2]],
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> Option<Vec<[usize; 3]>> {
    if segments.is_empty() {
        return None;
    }

    let boundary_loop = boundary_loop_with_inserted_points(face, boundary_points, epsilon)?;
    let mut polygons = vec![boundary_loop];
    let mut boundary_piece_segments = Vec::new();
    for segment in segments {
        if segment[0] == segment[1] {
            return None;
        }
        match split_polygon_by_segment(&mut polygons, *segment)? {
            SegmentSplitKind::BoundaryPiece => boundary_piece_segments.push(*segment),
            SegmentSplitKind::Chord => {}
        }
    }

    let mut output = Vec::new();
    for polygon in polygons {
        if polygon.len() < 3 {
            return None;
        }
        let polygon_faces = triangulate_polygon_best_fan(&polygon, vertices, epsilon);
        if polygon_faces.is_empty() {
            return None;
        }
        output.extend(polygon_faces);
    }
    for segment in segments {
        let required_faces = if boundary_piece_segments.contains(segment) {
            1
        } else {
            2
        };
        if output
            .iter()
            .filter(|triangle| triangle_has_edge(**triangle, *segment))
            .count()
            < required_faces
        {
            return None;
        }
    }
    (!output.is_empty()).then_some(output)
}

enum SegmentSplitKind {
    BoundaryPiece,
    Chord,
}

fn split_polygon_by_segment(
    polygons: &mut Vec<Vec<usize>>,
    segment: [usize; 2],
) -> Option<SegmentSplitKind> {
    for polygon_index in 0..polygons.len() {
        let polygon = &polygons[polygon_index];
        let first = polygon.iter().position(|vertex| *vertex == segment[0]);
        let second = polygon.iter().position(|vertex| *vertex == segment[1]);
        let (Some(first), Some(second)) = (first, second) else {
            continue;
        };
        if cyclic_adjacent(first, second, polygon.len()) {
            return Some(SegmentSplitKind::BoundaryPiece);
        }

        let left = cyclic_path(polygon, first, second);
        let right = cyclic_path(polygon, second, first);
        if left.len() < 3 || right.len() < 3 {
            return None;
        }
        polygons.remove(polygon_index);
        polygons.push(left);
        polygons.push(right);
        return Some(SegmentSplitKind::Chord);
    }
    None
}

fn cyclic_adjacent(first: usize, second: usize, len: usize) -> bool {
    first.abs_diff(second) == 1 || first.abs_diff(second) + 1 == len
}

fn cyclic_path(polygon: &[usize], from: usize, to: usize) -> Vec<usize> {
    let mut output = Vec::new();
    let mut index = from;
    loop {
        output.push(polygon[index]);
        if index == to {
            break;
        }
        index = (index + 1) % polygon.len();
    }
    output
}

fn triangulate_polygon_best_fan(
    polygon: &[usize],
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> Vec<[usize; 3]> {
    let mut best_faces = Vec::new();
    for anchor_index in 0..polygon.len() {
        let mut candidate = Vec::with_capacity(polygon.len().saturating_sub(2));
        for offset in 1..polygon.len() - 1 {
            push_if_area(
                &mut candidate,
                [
                    polygon[anchor_index],
                    polygon[(anchor_index + offset) % polygon.len()],
                    polygon[(anchor_index + offset + 1) % polygon.len()],
                ],
                vertices,
                epsilon,
            );
        }
        if candidate.len() > best_faces.len() {
            best_faces = candidate;
        }
    }
    best_faces
}

fn boundary_loop_with_inserted_points(
    face: [usize; 3],
    boundary_points: &[(usize, f64)],
    epsilon: f64,
) -> Option<Vec<usize>> {
    let mut positions = vec![(0.0, face[0]), (1.0, face[1]), (2.0, face[2])];
    for (vertex, position) in boundary_points {
        let normalized = normalize_boundary_position(*position);
        positions.push((normalized, *vertex));
    }
    positions.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });

    let mut loop_vertices = Vec::with_capacity(positions.len());
    for (position, vertex) in positions {
        let duplicate = loop_vertices
            .iter()
            .any(|(existing_position, existing_vertex)| {
                *existing_vertex == vertex || nearly_equal(*existing_position, position, epsilon)
            });
        if !duplicate {
            loop_vertices.push((position, vertex));
        }
    }
    (loop_vertices.len() >= 3).then(|| {
        loop_vertices
            .into_iter()
            .map(|(_, vertex)| vertex)
            .collect()
    })
}

fn normalize_boundary_position(position: f64) -> f64 {
    let normalized = position.rem_euclid(3.0);
    if nearly_equal(normalized, 3.0, f64::EPSILON) {
        0.0
    } else {
        normalized
    }
}

fn push_if_area(
    output: &mut Vec<[usize; 3]>,
    face: [usize; 3],
    vertices: &[[f64; 3]],
    epsilon: f64,
) {
    if triangle_area(face, vertices) > epsilon * epsilon {
        output.push(face);
    }
}

fn point_in_triangle(
    point: [f64; 3],
    triangle: [usize; 3],
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> bool {
    let a = vertices[triangle[0]];
    let b = vertices[triangle[1]];
    let c = vertices[triangle[2]];
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(point, a);
    let d00 = dot(ab, ab);
    let d01 = dot(ab, ac);
    let d11 = dot(ac, ac);
    let d20 = dot(ap, ab);
    let d21 = dot(ap, ac);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() <= f64::EPSILON {
        return false;
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    let tol = barycentric_tolerance([a, b, c], epsilon);
    u >= -tol && v >= -tol && w >= -tol
}

fn nearest_face_corner(face: [usize; 3], vertex: usize, vertices: &[[f64; 3]]) -> usize {
    let point = vertices[vertex];
    (0..3)
        .min_by(|left, right| {
            let left_distance = dot(
                sub(point, vertices[face[*left]]),
                sub(point, vertices[face[*left]]),
            );
            let right_distance = dot(
                sub(point, vertices[face[*right]]),
                sub(point, vertices[face[*right]]),
            );
            left_distance.total_cmp(&right_distance)
        })
        .unwrap_or(0)
}

fn barycentric_tolerance(triangle: [[f64; 3]; 3], epsilon: f64) -> f64 {
    let max_edge = norm(sub(triangle[1], triangle[0]))
        .max(norm(sub(triangle[2], triangle[1])))
        .max(norm(sub(triangle[0], triangle[2])));
    if max_edge <= f64::EPSILON {
        return 0.0;
    }
    (epsilon / max_edge).clamp(1e-12, 1e-6)
}

fn triangle_area(face: [usize; 3], vertices: &[[f64; 3]]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    0.5 * norm(cross(sub(b, a), sub(c, a)))
}

fn triangle_has_edge(triangle: [usize; 3], edge: [usize; 2]) -> bool {
    (0..3).any(|index| {
        let candidate = [triangle[index], triangle[(index + 1) % 3]];
        candidate == edge || candidate == [edge[1], edge[0]]
    })
}

fn nearly_equal(left: f64, right: f64, epsilon: f64) -> bool {
    (left - right).abs() <= epsilon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_triangle_with_interior_segment_preserves_cut_edge() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.4, 0.4, 0.0],
            [0.9, 0.5, 0.0],
        ];

        let result =
            split_triangle_with_interior_segment([0, 1, 2], 3, 4, vertices[4], &vertices, 1e-9)
                .unwrap();

        assert_eq!(result.len(), 5);
        let cut_edge_count = result
            .iter()
            .filter(|triangle| triangle_has_edge(**triangle, [3, 4]))
            .count();
        assert_eq!(cut_edge_count, 2);
    }

    #[test]
    fn split_triangle_with_interior_spokes_preserves_all_cut_edges() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 0.5, 0.0],
            [1.0, 1.0, 0.0],
        ];

        let result = split_triangle_with_interior_spokes(
            [0, 1, 2],
            4,
            &[(3, 0.5), (5, 1.5)],
            &vertices,
            1e-9,
        )
        .unwrap();

        assert_eq!(result.len(), 5);
        assert_eq!(
            result
                .iter()
                .filter(|triangle| triangle_has_edge(**triangle, [3, 4]))
                .count(),
            2
        );
        assert_eq!(
            result
                .iter()
                .filter(|triangle| triangle_has_edge(**triangle, [4, 5]))
                .count(),
            2
        );
    }

    #[test]
    fn split_triangle_with_interior_cycle_preserves_closed_cut_edges() {
        let vertices = vec![
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 2.0],
            [1.0, 0.5, 0.5],
            [0.5, 1.0, 0.5],
            [0.5, 0.5, 1.0],
        ];

        let result =
            split_triangle_with_interior_cycle([0, 1, 2], &[3, 4, 5], &vertices, 1e-9).unwrap();

        assert_eq!(result.len(), 7);
        for edge in [[3, 4], [4, 5], [5, 3]] {
            assert_eq!(
                result
                    .iter()
                    .filter(|triangle| triangle_has_edge(**triangle, edge))
                    .count(),
                2
            );
        }
    }

    #[test]
    fn split_triangle_with_boundary_segments_preserves_two_chords() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [0.0, 4.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
        ];

        let result = split_triangle_with_boundary_segments(
            [0, 1, 2],
            &[(3, 0.25), (4, 1.5), (5, 0.5), (6, 2.5)],
            &[[3, 6], [5, 4]],
            &vertices,
            1e-9,
        )
        .unwrap();

        assert_eq!(result.len(), 5);
        for edge in [[3, 6], [5, 4]] {
            assert_eq!(
                result
                    .iter()
                    .filter(|triangle| triangle_has_edge(**triangle, edge))
                    .count(),
                2
            );
        }
    }

    #[test]
    fn split_triangle_with_boundary_segments_keeps_boundary_pieces_with_chords() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [0.0, 4.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
        ];

        let result = split_triangle_with_boundary_segments(
            [0, 1, 2],
            &[(3, 0.25), (4, 0.5), (5, 1.5)],
            &[[3, 4], [4, 5]],
            &vertices,
            1e-9,
        )
        .unwrap();

        assert!(result
            .iter()
            .any(|triangle| triangle_has_edge(*triangle, [3, 4])));
        assert_eq!(
            result
                .iter()
                .filter(|triangle| triangle_has_edge(**triangle, [4, 5]))
                .count(),
            2
        );
    }

    #[test]
    fn split_triangle_with_boundary_segments_accepts_disconnected_boundary_pieces() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
        ];

        let result = split_triangle_with_boundary_segments(
            [0, 1, 2],
            &[(1, 1.0), (3, 0.5), (2, 2.0), (4, 1.5)],
            &[[1, 3], [3, 0], [2, 4], [4, 1]],
            &vertices,
            1e-9,
        )
        .unwrap();

        for edge in [[1, 3], [3, 0], [2, 4], [4, 1]] {
            assert!(result
                .iter()
                .any(|triangle| triangle_has_edge(*triangle, edge)));
        }
    }
}
