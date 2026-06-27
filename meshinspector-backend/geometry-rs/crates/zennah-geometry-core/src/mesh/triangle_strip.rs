use std::collections::{BTreeMap, BTreeSet};

use crate::math::{add, distance_sq, dot, scale, sub};
use crate::types::GeometryError;

use super::base::{edge_face_map, validate_faces};
use super::geodesic_strip::mesh_planar_triangle_strip_path;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshTriangleStripUnfoldedPath {
    pub start_face_index: usize,
    pub end_face_index: usize,
    pub strip_face_indices: Vec<usize>,
    pub crossed_edges: Vec<[usize; 2]>,
    pub oriented_edges: Vec<[usize; 2]>,
    pub crossing_positions: Vec<f64>,
    pub crossing_points: Vec<[f64; 3]>,
    pub points: Vec<[f64; 3]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub planar_length_mm: f64,
    pub meshlib_reference: &'static str,
}

pub fn mesh_triangle_strip_unfolded_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_face_index: usize,
    crossed_edges_i64: &[[i64; 2]],
    end_face_index: usize,
    start_point: [f64; 3],
    end_point: [f64; 3],
) -> Result<MeshTriangleStripUnfoldedPath, GeometryError> {
    validate_point3("start_point", start_point)?;
    validate_point3("end_point", end_point)?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    validate_face_id("start_face_index", start_face_index, faces.len())?;
    validate_face_id("end_face_index", end_face_index, faces.len())?;
    let edge_faces = edge_face_map(&faces);
    let valid_edges = edge_faces.keys().copied().collect::<BTreeSet<_>>();
    let crossed_edges = crossed_edges_i64
        .iter()
        .map(|edge| validate_edge(vertices.len(), &valid_edges, *edge))
        .collect::<Result<Vec<_>, _>>()?;

    if crossed_edges.is_empty() {
        if start_face_index != end_face_index {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "crossed_edges",
                value: "empty_strip_requires_same_start_and_end_face".to_string(),
            });
        }
        let segment_length = distance_sq(start_point, end_point).sqrt();
        return Ok(MeshTriangleStripUnfoldedPath {
            start_face_index,
            end_face_index,
            strip_face_indices: vec![start_face_index],
            crossed_edges,
            oriented_edges: Vec::new(),
            crossing_positions: Vec::new(),
            crossing_points: Vec::new(),
            points: vec![start_point, end_point],
            segment_lengths: vec![segment_length],
            length_mm: segment_length,
            planar_length_mm: segment_length,
            meshlib_reference: "MR::TriangleStripUnfolder / MR::reducePath",
        });
    }

    let strip_face_indices = strip_faces_for_crossed_edges(
        &faces,
        &edge_faces,
        start_face_index,
        end_face_index,
        &crossed_edges,
    )?;
    let oriented_edges = orient_strip_edges(&crossed_edges, faces[start_face_index])?;
    let mut unfolded_vertices = first_face_coordinates(vertices, faces[start_face_index])?;

    for (edge_index, edge) in crossed_edges.iter().enumerate() {
        let current_face_index = strip_face_indices[edge_index];
        let next_face_index = strip_face_indices[edge_index + 1];
        unfold_next_face_vertex(
            vertices,
            &faces,
            current_face_index,
            next_face_index,
            *edge,
            &mut unfolded_vertices,
        )?;
    }

    let portals = oriented_edges
        .iter()
        .map(|edge| {
            Ok([
                *unfolded_vertices
                    .get(&edge[0])
                    .ok_or_else(|| invalid_strip("portal_left_not_unfolded"))?,
                *unfolded_vertices
                    .get(&edge[1])
                    .ok_or_else(|| invalid_strip("portal_right_not_unfolded"))?,
            ])
        })
        .collect::<Result<Vec<_>, GeometryError>>()?;
    let start_2d = triangle_point_to_unfolded(
        vertices,
        faces[start_face_index],
        &unfolded_vertices,
        start_point,
    )?;
    let end_2d = triangle_point_to_unfolded(
        vertices,
        faces[end_face_index],
        &unfolded_vertices,
        end_point,
    )?;
    let planar = mesh_planar_triangle_strip_path(start_2d, &portals, end_2d)?;
    let crossing_points = planar
        .crossing_positions
        .iter()
        .zip(oriented_edges.iter())
        .map(|(position, edge)| edge_point(vertices, *edge, *position))
        .collect::<Vec<_>>();
    let mut points = Vec::with_capacity(crossing_points.len() + 2);
    points.push(start_point);
    points.extend(crossing_points.iter().copied());
    points.push(end_point);
    let segment_lengths = points
        .windows(2)
        .map(|window| distance_sq(window[0], window[1]).sqrt())
        .collect::<Vec<_>>();
    let length_mm = segment_lengths.iter().sum();

    Ok(MeshTriangleStripUnfoldedPath {
        start_face_index,
        end_face_index,
        strip_face_indices,
        crossed_edges,
        oriented_edges,
        crossing_positions: planar.crossing_positions,
        crossing_points,
        points,
        segment_lengths,
        length_mm,
        planar_length_mm: planar.length_mm,
        meshlib_reference: "MR::TriangleStripUnfolder / MR::reducePath",
    })
}

fn strip_faces_for_crossed_edges(
    faces: &[[usize; 3]],
    edge_faces: &rustc_hash::FxHashMap<(usize, usize), Vec<usize>>,
    start_face_index: usize,
    end_face_index: usize,
    crossed_edges: &[[usize; 2]],
) -> Result<Vec<usize>, GeometryError> {
    let mut current_face_index = start_face_index;
    let mut strip_face_indices = vec![current_face_index];
    for edge in crossed_edges {
        if !face_contains_edge(faces[current_face_index], *edge) {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "crossed_edges",
                value: "edge_not_in_current_strip_face".to_string(),
            });
        }
        let key = sorted_edge(*edge);
        let incident_faces = edge_faces
            .get(&key)
            .ok_or_else(|| invalid_strip("edge_has_no_incident_faces"))?;
        let next_face = incident_faces
            .iter()
            .copied()
            .find(|face_index| *face_index != current_face_index)
            .ok_or_else(|| invalid_strip("edge_does_not_cross_to_next_face"))?;
        current_face_index = next_face;
        strip_face_indices.push(current_face_index);
    }
    if current_face_index != end_face_index {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "end_face_index",
            value: format!("strip ends at face {current_face_index}, expected {end_face_index}"),
        });
    }
    Ok(strip_face_indices)
}

fn orient_strip_edges(
    crossed_edges: &[[usize; 2]],
    start_face: [usize; 3],
) -> Result<Vec<[usize; 2]>, GeometryError> {
    let mut oriented_edges = Vec::with_capacity(crossed_edges.len());
    oriented_edges.push(first_portal_orientation(crossed_edges[0], start_face)?);
    for edge in crossed_edges.iter().skip(1) {
        let previous = *oriented_edges
            .last()
            .ok_or_else(|| invalid_strip("empty_oriented_edges"))?;
        let shared = previous
            .into_iter()
            .filter(|vertex| edge.contains(vertex))
            .collect::<Vec<_>>();
        if shared.len() != 1 {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "crossed_edges",
                value: "consecutive_edges_must_share_exactly_one_vertex".to_string(),
            });
        }
        let shared_vertex = shared[0];
        let other = if edge[0] == shared_vertex {
            edge[1]
        } else {
            edge[0]
        };
        if previous[0] == shared_vertex {
            oriented_edges.push([shared_vertex, other]);
        } else {
            oriented_edges.push([other, shared_vertex]);
        }
    }
    Ok(oriented_edges)
}

fn first_portal_orientation(
    edge: [usize; 2],
    start_face: [usize; 3],
) -> Result<[usize; 2], GeometryError> {
    if face_has_directed_edge(start_face, edge[0], edge[1]) {
        return Ok([edge[1], edge[0]]);
    }
    if face_has_directed_edge(start_face, edge[1], edge[0]) {
        return Ok([edge[0], edge[1]]);
    }
    Err(GeometryError::InvalidSelectionParameter {
        field: "crossed_edges",
        value: "first_edge_is_not_in_start_face".to_string(),
    })
}

fn first_face_coordinates(
    vertices: &[[f64; 3]],
    face: [usize; 3],
) -> Result<BTreeMap<usize, [f64; 2]>, GeometryError> {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let ab = distance_sq(a, b).sqrt();
    let ac = distance_sq(a, c).sqrt();
    let bc = distance_sq(b, c).sqrt();
    if ab <= 1e-12 {
        return Err(invalid_strip("degenerate_first_face_edge"));
    }
    let x = (ac * ac + ab * ab - bc * bc) / (2.0 * ab);
    let y_sq = (ac * ac - x * x).max(0.0);
    let mut coords = BTreeMap::new();
    coords.insert(face[0], [0.0, 0.0]);
    coords.insert(face[1], [ab, 0.0]);
    coords.insert(face[2], [x, y_sq.sqrt()]);
    Ok(coords)
}

fn unfold_next_face_vertex(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    current_face_index: usize,
    next_face_index: usize,
    edge: [usize; 2],
    unfolded_vertices: &mut BTreeMap<usize, [f64; 2]>,
) -> Result<(), GeometryError> {
    let current_third =
        third_vertex_across_edge(faces[current_face_index], edge).ok_or_else(|| {
            GeometryError::InvalidSelectionParameter {
                field: "crossed_edges",
                value: "current_face_does_not_contain_edge".to_string(),
            }
        })?;
    let next_third = third_vertex_across_edge(faces[next_face_index], edge).ok_or_else(|| {
        GeometryError::InvalidSelectionParameter {
            field: "crossed_edges",
            value: "next_face_does_not_contain_edge".to_string(),
        }
    })?;
    if unfolded_vertices.contains_key(&next_third) {
        return Ok(());
    }
    let p = *unfolded_vertices
        .get(&edge[0])
        .ok_or_else(|| invalid_strip("edge_origin_not_unfolded"))?;
    let q = *unfolded_vertices
        .get(&edge[1])
        .ok_or_else(|| invalid_strip("edge_dest_not_unfolded"))?;
    let current_third_2d = *unfolded_vertices
        .get(&current_third)
        .ok_or_else(|| invalid_strip("current_third_not_unfolded"))?;
    let next_2d = unfold_vertex_across_edge(
        p,
        q,
        current_third_2d,
        distance_sq(vertices[next_third], vertices[edge[0]]).sqrt(),
        distance_sq(vertices[next_third], vertices[edge[1]]).sqrt(),
    )?;
    unfolded_vertices.insert(next_third, next_2d);
    Ok(())
}

fn unfold_vertex_across_edge(
    p: [f64; 2],
    q: [f64; 2],
    current_third: [f64; 2],
    distance_to_p: f64,
    distance_to_q: f64,
) -> Result<[f64; 2], GeometryError> {
    let edge = sub2(q, p);
    let edge_length = norm2(edge);
    if edge_length <= 1e-12 {
        return Err(invalid_strip("degenerate_unfold_edge"));
    }
    let along = (distance_to_p * distance_to_p - distance_to_q * distance_to_q
        + edge_length * edge_length)
        / (2.0 * edge_length);
    let height_sq = (distance_to_p * distance_to_p - along * along).max(0.0);
    let unit = [edge[0] / edge_length, edge[1] / edge_length];
    let base = [p[0] + unit[0] * along, p[1] + unit[1] * along];
    let perp = [-unit[1], unit[0]];
    let height = height_sq.sqrt();
    let first = [base[0] + perp[0] * height, base[1] + perp[1] * height];
    let second = [base[0] - perp[0] * height, base[1] - perp[1] * height];
    let side = cross2(edge, sub2(current_third, p));
    let first_side = cross2(edge, sub2(first, p));
    if side > 0.0 {
        Ok(if first_side < 0.0 { first } else { second })
    } else if side < 0.0 {
        Ok(if first_side > 0.0 { first } else { second })
    } else {
        Ok(second)
    }
}

fn triangle_point_to_unfolded(
    vertices: &[[f64; 3]],
    face: [usize; 3],
    unfolded_vertices: &BTreeMap<usize, [f64; 2]>,
    point: [f64; 3],
) -> Result<[f64; 2], GeometryError> {
    let weights = barycentric_coordinates(
        vertices[face[0]],
        vertices[face[1]],
        vertices[face[2]],
        point,
    )?;
    let a = *unfolded_vertices
        .get(&face[0])
        .ok_or_else(|| invalid_strip("face_vertex_a_not_unfolded"))?;
    let b = *unfolded_vertices
        .get(&face[1])
        .ok_or_else(|| invalid_strip("face_vertex_b_not_unfolded"))?;
    let c = *unfolded_vertices
        .get(&face[2])
        .ok_or_else(|| invalid_strip("face_vertex_c_not_unfolded"))?;
    Ok([
        a[0] * weights[0] + b[0] * weights[1] + c[0] * weights[2],
        a[1] * weights[0] + b[1] * weights[1] + c[1] * weights[2],
    ])
}

fn barycentric_coordinates(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    point: [f64; 3],
) -> Result<[f64; 3], GeometryError> {
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let v2 = sub(point, a);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denominator = d00 * d11 - d01 * d01;
    if denominator.abs() <= 1e-18 {
        return Err(invalid_strip("degenerate_barycentric_triangle"));
    }
    let v = (d11 * d20 - d01 * d21) / denominator;
    let w = (d00 * d21 - d01 * d20) / denominator;
    Ok([1.0 - v - w, v, w])
}

fn validate_face_id(
    field: &'static str,
    face_index: usize,
    face_count: usize,
) -> Result<(), GeometryError> {
    if face_index < face_count {
        Ok(())
    } else {
        Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("{face_index} for {face_count} faces"),
        })
    }
}

fn validate_edge(
    vertex_count: usize,
    valid_edges: &BTreeSet<(usize, usize)>,
    edge: [i64; 2],
) -> Result<[usize; 2], GeometryError> {
    if edge[0] < 0 || edge[1] < 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "crossed_edges",
            value: "edge_vertices_must_be_non_negative".to_string(),
        });
    }
    let output = [edge[0] as usize, edge[1] as usize];
    if output[0] >= vertex_count || output[1] >= vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "crossed_edges",
            value: format!(
                "edge {:?} is out of range for {vertex_count} vertices",
                edge
            ),
        });
    }
    if output[0] == output[1] {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "crossed_edges",
            value: "edge_vertices_must_be_distinct".to_string(),
        });
    }
    if !valid_edges.contains(&sorted_edge(output)) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "crossed_edges",
            value: format!("edge {:?} is not a mesh edge", edge),
        });
    }
    Ok(output)
}

fn validate_point3(field: &'static str, point: [f64; 3]) -> Result<(), GeometryError> {
    if point.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(GeometryError::InvalidSelectionParameter {
            field,
            value: "coordinates_must_be_finite".to_string(),
        })
    }
}

fn edge_point(vertices: &[[f64; 3]], edge: [usize; 2], position: f64) -> [f64; 3] {
    add(
        scale(vertices[edge[0]], 1.0 - position),
        scale(vertices[edge[1]], position),
    )
}

fn face_contains_edge(face: [usize; 3], edge: [usize; 2]) -> bool {
    face.contains(&edge[0]) && face.contains(&edge[1])
}

fn face_has_directed_edge(face: [usize; 3], from: usize, to: usize) -> bool {
    (face[0] == from && face[1] == to)
        || (face[1] == from && face[2] == to)
        || (face[2] == from && face[0] == to)
}

fn third_vertex_across_edge(face: [usize; 3], edge: [usize; 2]) -> Option<usize> {
    if !face_contains_edge(face, edge) {
        return None;
    }
    face.into_iter()
        .find(|vertex| *vertex != edge[0] && *vertex != edge[1])
}

fn sorted_edge(edge: [usize; 2]) -> (usize, usize) {
    if edge[0] <= edge[1] {
        (edge[0], edge[1])
    } else {
        (edge[1], edge[0])
    }
}

fn invalid_strip(value: &str) -> GeometryError {
    GeometryError::InvalidSelectionParameter {
        field: "triangle_strip",
        value: value.to_string(),
    }
}

fn sub2(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] - right[0], left[1] - right[1]]
}

fn norm2(value: [f64; 2]) -> f64 {
    (value[0] * value[0] + value[1] * value[1]).sqrt()
}

fn cross2(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[1] - left[1] * right[0]
}
