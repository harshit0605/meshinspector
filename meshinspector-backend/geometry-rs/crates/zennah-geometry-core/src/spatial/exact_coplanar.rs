mod merge;

use self::merge::{
    merged_contours_for_face_groups, paired_merged_contours_for_overlaps,
    push_face_overlap_polygon, FaceOverlapPolygons,
};
use super::exact_one_mesh::{
    ExactOneMeshContour, ExactOneMeshContours, ExactOneMeshIntersection, ExactOneMeshPrimitive,
};
use crate::math::{cross, dot, norm, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ExactCoplanarTriangleOverlap {
    pub first_face: usize,
    pub second_face: usize,
    pub polygon: Vec<[f64; 3]>,
    pub area: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ExactCoplanarOverlapContours {
    pub overlaps: Vec<ExactCoplanarTriangleOverlap>,
    pub contours: ExactOneMeshContours,
    pub merged_contours: ExactOneMeshContours,
    pub paired_merged_contours: ExactOneMeshContours,
}

pub(super) fn coplanar_overlap_contours(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    epsilon: f64,
) -> Result<ExactCoplanarOverlapContours, GeometryError> {
    let first_faces = validate_faces(first_faces_i64, first_vertices.len())?;
    let second_faces = validate_faces(second_faces_i64, second_vertices.len())?;
    let tolerance = effective_epsilon(epsilon);
    let mut overlaps = Vec::new();
    let mut first_contours = Vec::new();
    let mut second_contours = Vec::new();
    let mut coordinates = Vec::new();
    let mut first_groups = BTreeMap::<usize, FaceOverlapPolygons>::new();
    let mut second_groups = BTreeMap::<usize, FaceOverlapPolygons>::new();
    for (first_face_index, first_face) in first_faces.iter().copied().enumerate() {
        let first_triangle = triangle_points(first_vertices, first_face);
        let first_bounds = triangle_bounds(first_triangle);
        for (second_face_index, second_face) in second_faces.iter().copied().enumerate() {
            let second_triangle = triangle_points(second_vertices, second_face);
            if !bounds_overlap(first_bounds, triangle_bounds(second_triangle), tolerance) {
                continue;
            }
            if let Some(overlap) = coplanar_overlap_polygon(
                first_triangle,
                second_triangle,
                first_face_index,
                second_face_index,
                tolerance,
            ) {
                first_contours.push(contour_for_overlap_polygon(
                    &overlap.polygon,
                    first_vertices,
                    first_face,
                    first_face_index,
                    tolerance,
                ));
                second_contours.push(contour_for_overlap_polygon(
                    &overlap.polygon,
                    second_vertices,
                    second_face,
                    second_face_index,
                    tolerance,
                ));
                coordinates.push(overlap.polygon.clone());
                push_face_overlap_polygon(
                    &mut first_groups,
                    first_face_index,
                    first_face,
                    overlap.polygon.clone(),
                );
                push_face_overlap_polygon(
                    &mut second_groups,
                    second_face_index,
                    second_face,
                    overlap.polygon.clone(),
                );
                overlaps.push(overlap);
            }
        }
    }
    let merged_first = merged_contours_for_face_groups(&first_groups, first_vertices, tolerance);
    let merged_second = merged_contours_for_face_groups(&second_groups, second_vertices, tolerance);
    let paired_merged_contours = paired_merged_contours_for_overlaps(
        &overlaps,
        &first_faces,
        first_vertices,
        &second_faces,
        second_vertices,
        tolerance,
    );
    Ok(ExactCoplanarOverlapContours {
        overlaps,
        contours: ExactOneMeshContours {
            first: first_contours,
            second: second_contours,
            coordinates_in_first_space: coordinates,
        },
        merged_contours: ExactOneMeshContours {
            first: merged_first,
            second: merged_second,
            coordinates_in_first_space: Vec::new(),
        },
        paired_merged_contours,
    })
}

pub(super) fn same_oriented_coplanar_overlap_faces(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    epsilon: f64,
) -> Result<BTreeSet<usize>, GeometryError> {
    let first_faces = validate_faces(first_faces_i64, first_vertices.len())?;
    let second_faces = validate_faces(second_faces_i64, second_vertices.len())?;
    let tolerance = effective_epsilon(epsilon);
    let mut faces_with_overlap = BTreeSet::new();
    for (first_face_index, first_face) in first_faces.iter().copied().enumerate() {
        let first_triangle = triangle_points(first_vertices, first_face);
        let first_normal = triangle_normal(first_triangle);
        let first_normal_len = norm(first_normal);
        if first_normal_len <= tolerance {
            continue;
        }
        let first_bounds = triangle_bounds(first_triangle);
        for (second_face_index, second_face) in second_faces.iter().copied().enumerate() {
            let second_triangle = triangle_points(second_vertices, second_face);
            if !bounds_overlap(first_bounds, triangle_bounds(second_triangle), tolerance) {
                continue;
            }
            let second_normal = triangle_normal(second_triangle);
            let second_normal_len = norm(second_normal);
            if second_normal_len <= tolerance
                || dot(first_normal, second_normal)
                    <= tolerance * first_normal_len * second_normal_len
            {
                continue;
            }
            if coplanar_overlap_polygon(
                first_triangle,
                second_triangle,
                first_face_index,
                second_face_index,
                tolerance,
            )
            .is_some()
            {
                faces_with_overlap.insert(first_face_index);
                break;
            }
        }
    }
    Ok(faces_with_overlap)
}

fn coplanar_overlap_polygon(
    first: [[f64; 3]; 3],
    second: [[f64; 3]; 3],
    first_face: usize,
    second_face: usize,
    epsilon: f64,
) -> Option<ExactCoplanarTriangleOverlap> {
    let normal = cross(sub(first[1], first[0]), sub(first[2], first[0]));
    let normal_len = norm(normal);
    if normal_len <= epsilon {
        return None;
    }
    let other_normal = cross(sub(second[1], second[0]), sub(second[2], second[0]));
    let other_normal_len = norm(other_normal);
    if other_normal_len <= epsilon {
        return None;
    }
    if norm(cross(normal, other_normal)) > epsilon * normal_len * other_normal_len {
        return None;
    }
    if second
        .iter()
        .any(|point| dot(sub(*point, first[0]), normal).abs() > epsilon * normal_len)
    {
        return None;
    }

    let axis = projection_axis(normal);
    let subject = first
        .into_iter()
        .map(|point| project(point, axis))
        .collect::<Vec<_>>();
    let clip = second
        .into_iter()
        .map(|point| project(point, axis))
        .collect::<Vec<_>>();
    let polygon_2d = sanitize_polygon_2d(clip_polygon(subject, &clip, epsilon), epsilon);
    let projected_area = polygon_area(&polygon_2d).abs();
    if polygon_2d.len() < 3 || projected_area <= epsilon * epsilon {
        return None;
    }
    let axis_normal = normal[axis].abs();
    if axis_normal <= epsilon * normal_len {
        return None;
    }
    let area = projected_area * normal_len / axis_normal;
    let polygon = polygon_2d
        .into_iter()
        .map(|point| unproject(point, axis, first[0], normal))
        .collect::<Vec<_>>();
    Some(ExactCoplanarTriangleOverlap {
        first_face,
        second_face,
        polygon,
        area,
    })
}

fn triangle_normal(triangle: [[f64; 3]; 3]) -> [f64; 3] {
    cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0]))
}

fn clip_polygon(mut subject: Vec<[f64; 2]>, clip: &[[f64; 2]], epsilon: f64) -> Vec<[f64; 2]> {
    let clip_area = polygon_area(clip);
    if clip_area.abs() <= epsilon * epsilon {
        return Vec::new();
    }
    for edge_index in 0..clip.len() {
        if subject.is_empty() {
            break;
        }
        let start = clip[edge_index];
        let end = clip[(edge_index + 1) % clip.len()];
        subject = clip_against_edge(subject, start, end, clip_area, epsilon);
    }
    subject
}

fn clip_against_edge(
    subject: Vec<[f64; 2]>,
    start: [f64; 2],
    end: [f64; 2],
    clip_area: f64,
    epsilon: f64,
) -> Vec<[f64; 2]> {
    let mut output = Vec::new();
    let mut previous = *subject.last().expect("subject is non-empty");
    let mut previous_inside = is_inside(previous, start, end, clip_area, epsilon);
    for current in subject {
        let current_inside = is_inside(current, start, end, clip_area, epsilon);
        if current_inside != previous_inside {
            output.push(line_intersection(previous, current, start, end));
        }
        if current_inside {
            output.push(current);
        }
        previous = current;
        previous_inside = current_inside;
    }
    output
}

fn is_inside(point: [f64; 2], start: [f64; 2], end: [f64; 2], area: f64, epsilon: f64) -> bool {
    let edge = [end[0] - start[0], end[1] - start[1]];
    let relative = [point[0] - start[0], point[1] - start[1]];
    let side = edge[0] * relative[1] - edge[1] * relative[0];
    if area > 0.0 {
        side >= -epsilon
    } else {
        side <= epsilon
    }
}

fn line_intersection(
    left_start: [f64; 2],
    left_end: [f64; 2],
    right_start: [f64; 2],
    right_end: [f64; 2],
) -> [f64; 2] {
    let left_dir = [left_end[0] - left_start[0], left_end[1] - left_start[1]];
    let right_dir = [right_end[0] - right_start[0], right_end[1] - right_start[1]];
    let denominator = left_dir[0] * right_dir[1] - left_dir[1] * right_dir[0];
    if denominator.abs() <= f64::EPSILON {
        return left_end;
    }
    let delta = [
        right_start[0] - left_start[0],
        right_start[1] - left_start[1],
    ];
    let t = (delta[0] * right_dir[1] - delta[1] * right_dir[0]) / denominator;
    [
        left_start[0] + t * left_dir[0],
        left_start[1] + t * left_dir[1],
    ]
}

fn polygon_area(points: &[[f64; 2]]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let left = points[index];
        let right = points[(index + 1) % points.len()];
        area += left[0] * right[1] - left[1] * right[0];
    }
    0.5 * area
}

fn sanitize_polygon_2d(points: Vec<[f64; 2]>, epsilon: f64) -> Vec<[f64; 2]> {
    let mut output = Vec::with_capacity(points.len());
    for point in points {
        if output
            .last()
            .is_none_or(|previous| distance_sq_2d(*previous, point) > epsilon * epsilon)
        {
            output.push(point);
        }
    }
    if output.len() > 1
        && distance_sq_2d(output[0], *output.last().expect("output is non-empty"))
            <= epsilon * epsilon
    {
        output.pop();
    }
    output
}

fn distance_sq_2d(left: [f64; 2], right: [f64; 2]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    dx * dx + dy * dy
}

fn projection_axis(normal: [f64; 3]) -> usize {
    let abs = [normal[0].abs(), normal[1].abs(), normal[2].abs()];
    if abs[0] >= abs[1] && abs[0] >= abs[2] {
        0
    } else if abs[1] >= abs[2] {
        1
    } else {
        2
    }
}

fn project(point: [f64; 3], axis: usize) -> [f64; 2] {
    match axis {
        0 => [point[1], point[2]],
        1 => [point[0], point[2]],
        _ => [point[0], point[1]],
    }
}

fn unproject(point: [f64; 2], axis: usize, plane_point: [f64; 3], normal: [f64; 3]) -> [f64; 3] {
    let plane_dot = dot(normal, plane_point);
    match axis {
        0 => {
            let y = point[0];
            let z = point[1];
            [
                (plane_dot - normal[1] * y - normal[2] * z) / normal[0],
                y,
                z,
            ]
        }
        1 => {
            let x = point[0];
            let z = point[1];
            [
                x,
                (plane_dot - normal[0] * x - normal[2] * z) / normal[1],
                z,
            ]
        }
        _ => {
            let x = point[0];
            let y = point[1];
            [
                x,
                y,
                (plane_dot - normal[0] * x - normal[1] * y) / normal[2],
            ]
        }
    }
}

pub(super) fn contour_for_overlap_polygon(
    polygon: &[[f64; 3]],
    vertices: &[[f64; 3]],
    face: [usize; 3],
    face_index: usize,
    epsilon: f64,
) -> ExactOneMeshContour {
    ExactOneMeshContour {
        intersections: polygon
            .iter()
            .map(|point| ExactOneMeshIntersection {
                primitive: primitive_for_face_point(*point, vertices, face, face_index, epsilon),
                coordinate: *point,
            })
            .collect(),
        closed: true,
    }
}

fn primitive_for_face_point(
    point: [f64; 3],
    vertices: &[[f64; 3]],
    face: [usize; 3],
    face_index: usize,
    epsilon: f64,
) -> ExactOneMeshPrimitive {
    for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
        if point_lies_on_segment(point, vertices[edge[0]], vertices[edge[1]], epsilon) {
            return ExactOneMeshPrimitive::Edge(edge);
        }
    }
    ExactOneMeshPrimitive::Face(face_index)
}

fn point_lies_on_segment(point: [f64; 3], start: [f64; 3], end: [f64; 3], epsilon: f64) -> bool {
    let segment = sub(end, start);
    let length_sq = dot(segment, segment);
    if length_sq <= epsilon * epsilon {
        return norm(sub(point, start)) <= epsilon;
    }
    let t = dot(sub(point, start), segment) / length_sq;
    if t < -epsilon || t > 1.0 + epsilon {
        return false;
    }
    let closest = [
        start[0] + t * segment[0],
        start[1] + t * segment[1],
        start[2] + t * segment[2],
    ];
    norm(sub(point, closest)) <= epsilon
}

fn triangle_points(vertices: &[[f64; 3]], face: [usize; 3]) -> [[f64; 3]; 3] {
    [vertices[face[0]], vertices[face[1]], vertices[face[2]]]
}

fn triangle_bounds(triangle: [[f64; 3]; 3]) -> ([f64; 3], [f64; 3]) {
    let mut min = triangle[0];
    let mut max = triangle[0];
    for point in triangle.into_iter().skip(1) {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    (min, max)
}

fn bounds_overlap(left: ([f64; 3], [f64; 3]), right: ([f64; 3], [f64; 3]), epsilon: f64) -> bool {
    (0..3).all(|axis| {
        left.0[axis] <= right.1[axis] + epsilon && right.0[axis] <= left.1[axis] + epsilon
    })
}

fn effective_epsilon(epsilon: f64) -> f64 {
    if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coplanar_triangle_overlap_pairs_detects_area_overlap() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let second_vertices = vec![[0.5, 0.5, 0.0], [2.5, 0.5, 0.0], [0.5, 2.5, 0.0]];

        let count = coplanar_overlap_contours(
            &first_vertices,
            &[[0, 1, 2]],
            &second_vertices,
            &[[0, 1, 2]],
            1e-9,
        )
        .unwrap()
        .overlaps
        .len();

        assert_eq!(count, 1);
    }

    #[test]
    fn coplanar_triangle_overlaps_return_meshlib_style_overlap_region() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let second_vertices = vec![[0.5, 0.5, 0.0], [2.5, 0.5, 0.0], [0.5, 2.5, 0.0]];

        let overlaps = coplanar_overlap_contours(
            &first_vertices,
            &[[0, 1, 2]],
            &second_vertices,
            &[[0, 1, 2]],
            1e-9,
        )
        .unwrap()
        .overlaps;

        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].first_face, 0);
        assert_eq!(overlaps[0].second_face, 0);
        assert_eq!(overlaps[0].polygon.len(), 3);
        assert!((overlaps[0].area - 0.5).abs() < 1e-9);
        assert!(overlaps[0]
            .polygon
            .iter()
            .all(|point| point[2].abs() < 1e-9));
    }

    #[test]
    fn coplanar_overlap_contours_emit_closed_cut_contours_for_both_operands() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let second_vertices = vec![[0.5, 0.5, 0.0], [2.5, 0.5, 0.0], [0.5, 2.5, 0.0]];

        let result = coplanar_overlap_contours(
            &first_vertices,
            &[[0, 1, 2]],
            &second_vertices,
            &[[0, 1, 2]],
            1e-9,
        )
        .unwrap();

        assert_eq!(result.overlaps.len(), 1);
        assert_eq!(result.contours.first.len(), 1);
        assert_eq!(result.contours.second.len(), 1);
        assert_eq!(result.contours.coordinates_in_first_space.len(), 1);
        assert!(result.contours.first[0].closed);
        assert!(result.contours.second[0].closed);
        assert_eq!(
            result.contours.first[0].intersections.len(),
            result.overlaps[0].polygon.len()
        );
        assert!(result.contours.first[0]
            .intersections
            .iter()
            .any(|intersection| matches!(intersection.primitive, ExactOneMeshPrimitive::Edge(_))));
        assert!(result.contours.first[0]
            .intersections
            .iter()
            .any(|intersection| matches!(intersection.primitive, ExactOneMeshPrimitive::Face(0))));
    }

    #[test]
    fn coplanar_overlap_merged_contours_drop_internal_overlap_edges() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let second_vertices = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [1.0, 0.0, 0.0],
        ];

        let result = coplanar_overlap_contours(
            &first_vertices,
            &[[0, 1, 2]],
            &second_vertices,
            &[[0, 3, 2], [3, 1, 2]],
            1e-9,
        )
        .unwrap();

        assert_eq!(result.overlaps.len(), 2);
        assert_eq!(result.contours.first.len(), 2);
        assert_eq!(result.merged_contours.first.len(), 1);
        assert!(result.merged_contours.first[0].intersections.len() < 6);
        assert_eq!(
            result.paired_merged_contours.first.len(),
            result.paired_merged_contours.second.len()
        );
        assert_eq!(
            result.paired_merged_contours.first[0].intersections.len(),
            result.paired_merged_contours.second[0].intersections.len()
        );
        assert_eq!(
            result
                .paired_merged_contours
                .coordinates_in_first_space
                .len(),
            1
        );
    }

    #[test]
    fn coplanar_triangle_overlap_pairs_ignores_edge_touch() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let second_vertices = vec![[1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 1.0, 0.0]];

        let count = coplanar_overlap_contours(
            &first_vertices,
            &[[0, 1, 2]],
            &second_vertices,
            &[[0, 1, 2]],
            1e-9,
        )
        .unwrap()
        .overlaps
        .len();

        assert_eq!(count, 0);
    }
}
