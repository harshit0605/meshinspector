use super::super::exact_one_mesh::{
    ExactOneMeshContour, ExactOneMeshContours, ExactOneMeshIntersection, ExactOneMeshPrimitive,
};
use super::contour_for_overlap_polygon;
use super::ExactCoplanarTriangleOverlap;
use crate::math::{cross, dot, norm, sub};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct FaceOverlapPolygons {
    pub face: [usize; 3],
    pub polygons: Vec<Vec<[f64; 3]>>,
}

pub(super) fn push_face_overlap_polygon(
    groups: &mut BTreeMap<usize, FaceOverlapPolygons>,
    face_index: usize,
    face: [usize; 3],
    polygon: Vec<[f64; 3]>,
) {
    groups
        .entry(face_index)
        .or_insert_with(|| FaceOverlapPolygons {
            face,
            polygons: Vec::new(),
        })
        .polygons
        .push(polygon);
}

pub(super) fn merged_contours_for_face_groups(
    groups: &BTreeMap<usize, FaceOverlapPolygons>,
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> Vec<ExactOneMeshContour> {
    let mut contours = Vec::new();
    for (face_index, group) in groups {
        let loops = boundary_loops_for_polygons(&group.polygons, epsilon);
        if loops.is_empty() {
            contours.extend(group.polygons.iter().map(|polygon| {
                contour_for_overlap_polygon(polygon, vertices, group.face, *face_index, epsilon)
            }));
        } else {
            contours.extend(loops.iter().map(|polygon| {
                contour_for_overlap_polygon(polygon, vertices, group.face, *face_index, epsilon)
            }));
        }
    }
    contours
}

pub(super) fn paired_merged_contours_for_overlaps(
    overlaps: &[ExactCoplanarTriangleOverlap],
    first_faces: &[[usize; 3]],
    first_vertices: &[[f64; 3]],
    second_faces: &[[usize; 3]],
    second_vertices: &[[f64; 3]],
    epsilon: f64,
) -> ExactOneMeshContours {
    let polygons = overlaps
        .iter()
        .map(|overlap| overlap.polygon.clone())
        .collect::<Vec<_>>();
    let loops = boundary_loops_for_polygons(&polygons, epsilon);
    if loops.is_empty() {
        return fallback_raw_overlap_contours(
            overlaps,
            first_faces,
            first_vertices,
            second_faces,
            second_vertices,
            epsilon,
        );
    }

    let first_candidate_faces = overlaps
        .iter()
        .map(|overlap| overlap.first_face)
        .collect::<BTreeSet<_>>();
    let second_candidate_faces = overlaps
        .iter()
        .map(|overlap| overlap.second_face)
        .collect::<BTreeSet<_>>();
    let first = loops
        .iter()
        .map(|polygon| {
            contour_for_boundary_loop(
                polygon,
                first_vertices,
                first_faces,
                &first_candidate_faces,
                epsilon,
            )
        })
        .collect::<Vec<_>>();
    let second = loops
        .iter()
        .map(|polygon| {
            contour_for_boundary_loop(
                polygon,
                second_vertices,
                second_faces,
                &second_candidate_faces,
                epsilon,
            )
        })
        .collect::<Vec<_>>();
    ExactOneMeshContours {
        first,
        second,
        coordinates_in_first_space: loops,
    }
}

fn fallback_raw_overlap_contours(
    overlaps: &[ExactCoplanarTriangleOverlap],
    first_faces: &[[usize; 3]],
    first_vertices: &[[f64; 3]],
    second_faces: &[[usize; 3]],
    second_vertices: &[[f64; 3]],
    epsilon: f64,
) -> ExactOneMeshContours {
    let mut first = Vec::new();
    let mut second = Vec::new();
    let mut coordinates = Vec::new();
    for overlap in overlaps {
        let Some(first_face) = first_faces.get(overlap.first_face).copied() else {
            continue;
        };
        let Some(second_face) = second_faces.get(overlap.second_face).copied() else {
            continue;
        };
        first.push(contour_for_overlap_polygon(
            &overlap.polygon,
            first_vertices,
            first_face,
            overlap.first_face,
            epsilon,
        ));
        second.push(contour_for_overlap_polygon(
            &overlap.polygon,
            second_vertices,
            second_face,
            overlap.second_face,
            epsilon,
        ));
        coordinates.push(overlap.polygon.clone());
    }
    ExactOneMeshContours {
        first,
        second,
        coordinates_in_first_space: coordinates,
    }
}

fn contour_for_boundary_loop(
    polygon: &[[f64; 3]],
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    candidate_faces: &BTreeSet<usize>,
    epsilon: f64,
) -> ExactOneMeshContour {
    ExactOneMeshContour {
        intersections: polygon
            .iter()
            .map(|point| ExactOneMeshIntersection {
                primitive: primitive_for_boundary_point(
                    *point,
                    vertices,
                    faces,
                    candidate_faces,
                    epsilon,
                ),
                coordinate: *point,
            })
            .collect(),
        closed: true,
    }
}

fn primitive_for_boundary_point(
    point: [f64; 3],
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    candidate_faces: &BTreeSet<usize>,
    epsilon: f64,
) -> ExactOneMeshPrimitive {
    for face_index in candidate_faces {
        let Some(face) = faces.get(*face_index).copied() else {
            continue;
        };
        for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
            if point_lies_on_segment(point, vertices[edge[0]], vertices[edge[1]], epsilon) {
                return ExactOneMeshPrimitive::Edge(edge);
            }
        }
    }
    for face_index in candidate_faces {
        let Some(face) = faces.get(*face_index).copied() else {
            continue;
        };
        if point_lies_in_triangle(point, vertices, face, epsilon) {
            return ExactOneMeshPrimitive::Face(*face_index);
        }
    }
    ExactOneMeshPrimitive::Face(candidate_faces.first().copied().unwrap_or_default())
}

fn boundary_loops_for_polygons(polygons: &[Vec<[f64; 3]>], epsilon: f64) -> Vec<Vec<[f64; 3]>> {
    let mut occurrences = BTreeMap::<([i64; 3], [i64; 3]), BoundaryEdgeOccurrence>::new();
    let mut coordinates = BTreeMap::<[i64; 3], [f64; 3]>::new();
    for polygon in polygons {
        if polygon.len() < 3 {
            continue;
        }
        for index in 0..polygon.len() {
            let from = polygon[index];
            let to = polygon[(index + 1) % polygon.len()];
            if norm(sub(to, from)) <= epsilon {
                continue;
            }
            let from_key = point_key(from, epsilon);
            let to_key = point_key(to, epsilon);
            if from_key == to_key {
                continue;
            }
            coordinates.entry(from_key).or_insert(from);
            coordinates.entry(to_key).or_insert(to);
            occurrences
                .entry(ordered_point_edge(from_key, to_key))
                .and_modify(|occurrence| occurrence.count += 1)
                .or_insert(BoundaryEdgeOccurrence {
                    count: 1,
                    from: from_key,
                    to: to_key,
                });
        }
    }

    let mut adjacency = BTreeMap::<[i64; 3], Vec<[i64; 3]>>::new();
    let mut remaining_edges = BTreeSet::<([i64; 3], [i64; 3])>::new();
    for occurrence in occurrences
        .values()
        .filter(|occurrence| occurrence.count == 1)
    {
        adjacency
            .entry(occurrence.from)
            .or_default()
            .push(occurrence.to);
        adjacency
            .entry(occurrence.to)
            .or_default()
            .push(occurrence.from);
        remaining_edges.insert(ordered_point_edge(occurrence.from, occurrence.to));
    }

    let mut loops = Vec::new();
    while let Some((start, next)) = remaining_edges.iter().next().copied() {
        remaining_edges.remove(&(start, next));
        let mut loop_keys = vec![start, next];
        let mut previous = start;
        let mut current = next;
        let mut closed = false;
        while current != start {
            let Some(neighbors) = adjacency.get(&current) else {
                break;
            };
            let candidate = neighbors
                .iter()
                .copied()
                .find(|neighbor| {
                    *neighbor != previous
                        && remaining_edges.contains(&ordered_point_edge(current, *neighbor))
                })
                .or_else(|| {
                    neighbors.iter().copied().find(|neighbor| {
                        remaining_edges.contains(&ordered_point_edge(current, *neighbor))
                    })
                });
            let Some(next_key) = candidate else {
                break;
            };
            remaining_edges.remove(&ordered_point_edge(current, next_key));
            if next_key == start {
                closed = true;
                break;
            }
            loop_keys.push(next_key);
            previous = current;
            current = next_key;
        }
        if closed && loop_keys.len() >= 3 {
            let polygon = loop_keys
                .into_iter()
                .filter_map(|key| coordinates.get(&key).copied())
                .collect::<Vec<_>>();
            if polygon.len() >= 3 && polygon_area_3d(&polygon) > epsilon * epsilon {
                loops.push(polygon);
            }
        }
    }
    loops
}

struct BoundaryEdgeOccurrence {
    count: usize,
    from: [i64; 3],
    to: [i64; 3],
}

fn point_key(point: [f64; 3], epsilon: f64) -> [i64; 3] {
    let scale = 1.0 / effective_epsilon(epsilon);
    [
        (point[0] * scale).round() as i64,
        (point[1] * scale).round() as i64,
        (point[2] * scale).round() as i64,
    ]
}

fn ordered_point_edge(left: [i64; 3], right: [i64; 3]) -> ([i64; 3], [i64; 3]) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn polygon_area_3d(points: &[[f64; 3]]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = [0.0, 0.0, 0.0];
    for index in 0..points.len() {
        let left = points[index];
        let right = points[(index + 1) % points.len()];
        area[0] += left[1] * right[2] - left[2] * right[1];
        area[1] += left[2] * right[0] - left[0] * right[2];
        area[2] += left[0] * right[1] - left[1] * right[0];
    }
    0.5 * norm(area)
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

fn point_lies_in_triangle(
    point: [f64; 3],
    vertices: &[[f64; 3]],
    face: [usize; 3],
    epsilon: f64,
) -> bool {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let normal = cross(sub(b, a), sub(c, a));
    let area2 = norm(normal);
    if area2 <= epsilon {
        return false;
    }
    if dot(sub(point, a), normal).abs() > epsilon * area2 {
        return false;
    }
    let area_sum = norm(cross(sub(b, point), sub(c, point)))
        + norm(cross(sub(c, point), sub(a, point)))
        + norm(cross(sub(a, point), sub(b, point)));
    (area_sum - area2).abs() <= epsilon * area2.max(1.0)
}

fn effective_epsilon(epsilon: f64) -> f64 {
    if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    }
}
