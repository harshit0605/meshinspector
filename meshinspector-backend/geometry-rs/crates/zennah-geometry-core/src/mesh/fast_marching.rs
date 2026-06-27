use crate::math::{add, distance_sq, scale};
use crate::types::GeometryError;

use super::base::validate_faces;
use super::fast_marching_prune::collapse_repeated_crossing_locations;
use super::fast_marching_reduce::{
    reduce_adjacent_face_crossing, reduce_best_vertex_fan_crossing,
    reduce_repeated_location_strip_path, reduce_single_crossing,
};
use super::geodesic_descent::{
    mesh_steepest_descent_edge_step, mesh_steepest_descent_triangle_step,
};
use super::geodesic_descent_path::{mesh_steepest_descent_path, MeshSteepestDescentPath};
use super::geodesic_descent_vertex::mesh_steepest_descent_vertex_step;
use super::surface_distance::{surface_distance_field, surface_distance_field_from_tri_point};
use super::triangle_strip::mesh_triangle_strip_unfolded_path;

const VERTEX_EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshFastMarchingSurfacePath {
    pub start_vertex: usize,
    pub end_vertex: usize,
    pub start_face_index: usize,
    pub start_barycentric: [f64; 3],
    pub surface_distances_mm: Vec<f64>,
    pub surface_predecessor_vertices: Vec<Option<usize>>,
    pub edges: Vec<[usize; 2]>,
    pub positions: Vec<f64>,
    pub points: Vec<[f64; 3]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub reached_vertex: Option<usize>,
    pub stopped_reason: &'static str,
    pub steps: usize,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshFastMarchingTriPointSurfacePath {
    pub start_face_index: usize,
    pub start_barycentric: [f64; 3],
    pub start_point: [f64; 3],
    pub end_face_index: usize,
    pub end_barycentric: [f64; 3],
    pub end_point: [f64; 3],
    pub surface_distances_mm: Vec<f64>,
    pub surface_predecessor_vertices: Vec<Option<usize>>,
    pub edges: Vec<[usize; 2]>,
    pub positions: Vec<f64>,
    pub points: Vec<[f64; 3]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub reached_face_index: Option<usize>,
    pub stopped_reason: &'static str,
    pub steps: usize,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSurfaceTriPointPath {
    pub start_face_index: usize,
    pub start_barycentric: [f64; 3],
    pub start_point: [f64; 3],
    pub end_face_index: usize,
    pub end_barycentric: [f64; 3],
    pub end_point: [f64; 3],
    pub surface_distances_mm: Vec<f64>,
    pub surface_predecessor_vertices: Vec<Option<usize>>,
    pub approximate_edges: Vec<[usize; 2]>,
    pub approximate_positions: Vec<f64>,
    pub approximate_points: Vec<[f64; 3]>,
    pub edges: Vec<[usize; 2]>,
    pub positions: Vec<f64>,
    pub points: Vec<[f64; 3]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub reached_face_index: Option<usize>,
    pub stopped_reason: &'static str,
    pub reduce_iterations: usize,
    pub steps: usize,
    pub meshlib_reference: &'static str,
}

pub fn mesh_fast_marching_surface_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_vertex: usize,
    end_vertex: usize,
    max_steps: usize,
) -> Result<MeshFastMarchingSurfacePath, GeometryError> {
    if max_steps == 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_steps",
            value: "must_be_positive".to_string(),
        });
    }
    validate_vertex("start_vertex", start_vertex, vertices.len())?;
    validate_vertex("end_vertex", end_vertex, vertices.len())?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "faces",
            value: "mesh_must_have_faces".to_string(),
        });
    }

    let (surface_distances_mm, surface_predecessor_vertices) =
        surface_distance_field(vertices, &faces, &[end_vertex], f64::INFINITY);
    if !surface_distances_mm[start_vertex].is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "path",
            value: "start_end_not_connected".to_string(),
        });
    }

    let (start_face_index, start_barycentric) =
        start_face_and_barycentric(&faces, start_vertex, end_vertex)?;
    if start_face_contains_end(&faces[start_face_index], end_vertex) {
        return Ok(empty_path(
            start_vertex,
            end_vertex,
            start_face_index,
            start_barycentric,
            surface_distances_mm,
            surface_predecessor_vertices,
        ));
    }

    let scalars = finite_distance_scalars(&surface_distances_mm);
    let descent = mesh_steepest_descent_path(
        vertices,
        faces_i64,
        &scalars,
        start_face_index,
        start_barycentric,
        max_steps,
    )?;
    Ok(surface_path_from_descent(
        start_vertex,
        end_vertex,
        surface_distances_mm,
        surface_predecessor_vertices,
        descent,
    ))
}

pub fn mesh_fast_marching_surface_path_tri_points(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_face_index: usize,
    start_barycentric: [f64; 3],
    end_face_index: usize,
    end_barycentric: [f64; 3],
    max_steps: usize,
) -> Result<MeshFastMarchingTriPointSurfacePath, GeometryError> {
    if max_steps == 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_steps",
            value: "must_be_positive".to_string(),
        });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    validate_face_index("start_face_index", start_face_index, faces.len())?;
    validate_face_index("end_face_index", end_face_index, faces.len())?;
    validate_barycentric("start_barycentric", start_barycentric)?;
    validate_barycentric("end_barycentric", end_barycentric)?;

    let start_point = triangle_point(vertices, faces[start_face_index], start_barycentric);
    let end_point = triangle_point(vertices, faces[end_face_index], end_barycentric);
    let (surface_distances_mm, surface_predecessor_vertices, connected) =
        surface_distance_field_from_tri_point(
            vertices,
            &faces,
            end_face_index,
            end_barycentric,
            Some(start_point),
            Some(faces[start_face_index]),
        );
    if !connected {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "path",
            value: "start_end_not_connected".to_string(),
        });
    }
    if start_face_index == end_face_index {
        return Ok(MeshFastMarchingTriPointSurfacePath {
            start_face_index,
            start_barycentric,
            start_point,
            end_face_index,
            end_barycentric,
            end_point,
            surface_distances_mm,
            surface_predecessor_vertices,
            edges: Vec::new(),
            positions: Vec::new(),
            points: Vec::new(),
            segment_lengths: vec![distance_sq(start_point, end_point).sqrt()],
            length_mm: distance_sq(start_point, end_point).sqrt(),
            reached_face_index: Some(end_face_index),
            stopped_reason: "same_triangle",
            steps: 0,
            meshlib_reference: "MR::computeFastMarchingPath",
        });
    }

    let scalars = finite_distance_scalars(&surface_distances_mm);
    let mut edges = Vec::new();
    let mut positions = Vec::new();
    let mut points = Vec::new();
    let mut stopped_reason = "local_minimum";
    let mut reached_face_index = None;
    let end_face = faces[end_face_index];

    let first = mesh_steepest_descent_triangle_step(
        vertices,
        faces_i64,
        &scalars,
        start_face_index,
        start_barycentric,
    )?;
    if let (Some(edge), Some(position), Some(point)) = (
        first.crossed_edge,
        first.edge_position,
        first.crossing_point,
    ) {
        let reaches_end_face = edge_point_reaches_face(edge, position, end_face);
        edges.push(edge);
        positions.push(position);
        points.push(point);
        if reaches_end_face {
            reached_face_index = Some(end_face_index);
            stopped_reason = "end_triangle_reached";
        }
    }

    while reached_face_index.is_none() && !edges.is_empty() && edges.len() < max_steps {
        let edge = *edges.last().expect("edges is non-empty");
        let position = *positions.last().expect("positions tracks edges");
        let next = if let Some(vertex) = edge_position_vertex(edge, position) {
            let step = mesh_steepest_descent_vertex_step(vertices, faces_i64, &scalars, vertex)?;
            (step.crossed_edge, step.edge_position, step.crossing_point)
        } else {
            let step = mesh_steepest_descent_edge_step(
                vertices,
                faces_i64,
                &scalars,
                [edge[0] as i64, edge[1] as i64],
                position,
            )?;
            (
                step.crossed_edge,
                step.crossing_edge_position,
                step.crossing_point,
            )
        };

        let (Some(next_edge), Some(next_position), Some(next_point)) = next else {
            break;
        };
        if edge == next_edge && (position - next_position).abs() <= VERTEX_EPSILON {
            stopped_reason = "cycle_guard";
            break;
        }
        let reaches_end_face = edge_point_reaches_face(next_edge, next_position, end_face);
        edges.push(next_edge);
        positions.push(next_position);
        points.push(next_point);
        if reaches_end_face {
            reached_face_index = Some(end_face_index);
            stopped_reason = "end_triangle_reached";
            break;
        }
    }

    if edges.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "path",
            value: "internal_error_empty_path_outside_end_triangle".to_string(),
        });
    }
    if edges.len() == max_steps && reached_face_index.is_none() {
        stopped_reason = "max_steps";
    }

    let include_end = reached_face_index == Some(end_face_index);
    let segment_lengths =
        path_segment_lengths(start_point, &points, include_end.then_some(end_point));
    let length_mm = segment_lengths.iter().sum();
    let steps = edges.len();
    Ok(MeshFastMarchingTriPointSurfacePath {
        start_face_index,
        start_barycentric,
        start_point,
        end_face_index,
        end_barycentric,
        end_point,
        surface_distances_mm,
        surface_predecessor_vertices,
        edges,
        positions,
        points,
        segment_lengths,
        length_mm,
        reached_face_index,
        stopped_reason,
        steps,
        meshlib_reference: "MR::computeFastMarchingPath",
    })
}

pub fn mesh_surface_path_tri_points(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_face_index: usize,
    start_barycentric: [f64; 3],
    end_face_index: usize,
    end_barycentric: [f64; 3],
    max_geodesic_iters: usize,
) -> Result<MeshSurfaceTriPointPath, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let max_fast_marching_steps = faces.len().saturating_add(vertices.len()).saturating_add(1);
    let approximate = mesh_fast_marching_surface_path_tri_points(
        vertices,
        faces_i64,
        start_face_index,
        start_barycentric,
        end_face_index,
        end_barycentric,
        max_fast_marching_steps.max(1),
    )?;
    let mut edges = approximate.edges.clone();
    let mut positions = approximate.positions.clone();
    let mut points = approximate.points.clone();
    let mut reduce_iterations = 0;

    if max_geodesic_iters > 0 && approximate.reached_face_index == Some(end_face_index) {
        if let Some((edge, position, point)) = reduce_adjacent_face_crossing(
            vertices,
            &faces,
            start_face_index,
            approximate.start_point,
            end_face_index,
            approximate.end_point,
        ) {
            let candidate_length = distance_sq(approximate.start_point, point).sqrt()
                + distance_sq(point, approximate.end_point).sqrt();
            if candidate_length <= approximate.length_mm + VERTEX_EPSILON {
                edges = vec![edge];
                positions = vec![position];
                points = vec![point];
                let same_edge = approximate.edges.as_slice() == [edge];
                reduce_iterations = 1 + usize::from(!same_edge && max_geodesic_iters > 1);
            }
        }
    }
    if reduce_iterations == 0
        && max_geodesic_iters > 0
        && approximate.reached_face_index == Some(end_face_index)
    {
        if let Some((candidate_edges, candidate_positions, candidate_points)) =
            reduce_best_vertex_fan_crossing(
                vertices,
                &faces,
                start_face_index,
                approximate.start_point,
                end_face_index,
                approximate.end_point,
                &approximate.edges,
                &approximate.positions,
                &points,
            )
        {
            edges = candidate_edges;
            positions = candidate_positions;
            points = candidate_points;
            reduce_iterations = max_geodesic_iters.min(2);
        }
    }
    if reduce_iterations == 0
        && max_geodesic_iters > 0
        && approximate.reached_face_index == Some(end_face_index)
    {
        if let Some((candidate_edges, candidate_positions, candidate_points)) =
            reduce_repeated_location_strip_path(
                vertices,
                faces_i64,
                &faces,
                start_face_index,
                approximate.start_point,
                end_face_index,
                approximate.end_point,
                &approximate.edges,
                &approximate.positions,
                &points,
            )
        {
            edges = candidate_edges;
            positions = candidate_positions;
            points = candidate_points;
            reduce_iterations = max_geodesic_iters.min(2);
        }
    }
    if reduce_iterations == 0
        && max_geodesic_iters > 0
        && edges.len() == 1
        && approximate.reached_face_index == Some(end_face_index)
    {
        if let Some((position, point)) = reduce_single_crossing(
            vertices,
            &faces,
            start_face_index,
            approximate.start_point,
            end_face_index,
            approximate.end_point,
            edges[0],
        ) {
            positions[0] = position;
            points[0] = point;
            reduce_iterations = 1;
        }
    } else if reduce_iterations == 0
        && max_geodesic_iters > 0
        && edges.len() > 1
        && approximate.reached_face_index == Some(end_face_index)
    {
        let crossed_edges = edges
            .iter()
            .map(|edge| [edge[0] as i64, edge[1] as i64])
            .collect::<Vec<_>>();
        if let Ok(unfolded) = mesh_triangle_strip_unfolded_path(
            vertices,
            faces_i64,
            start_face_index,
            &crossed_edges,
            end_face_index,
            approximate.start_point,
            approximate.end_point,
        ) {
            if unfolded.length_mm <= approximate.length_mm + VERTEX_EPSILON {
                if max_geodesic_iters > 1 {
                    let collapsed = collapse_repeated_crossing_locations(
                        unfolded.oriented_edges,
                        unfolded.crossing_positions,
                        unfolded.crossing_points,
                    );
                    edges = collapsed.0;
                    positions = collapsed.1;
                    points = collapsed.2;
                    reduce_iterations = 1 + usize::from(collapsed.3);
                } else {
                    edges = unfolded.oriented_edges;
                    positions = unfolded.crossing_positions;
                    points = unfolded.crossing_points;
                    reduce_iterations = 1;
                }
            }
        }
    }
    let include_end = approximate.reached_face_index == Some(end_face_index);
    let segment_lengths = path_segment_lengths(
        approximate.start_point,
        &points,
        include_end.then_some(approximate.end_point),
    );
    let length_mm = segment_lengths.iter().sum();
    let steps = edges.len();
    Ok(MeshSurfaceTriPointPath {
        start_face_index: approximate.start_face_index,
        start_barycentric: approximate.start_barycentric,
        start_point: approximate.start_point,
        end_face_index: approximate.end_face_index,
        end_barycentric: approximate.end_barycentric,
        end_point: approximate.end_point,
        surface_distances_mm: approximate.surface_distances_mm,
        surface_predecessor_vertices: approximate.surface_predecessor_vertices,
        approximate_edges: approximate.edges,
        approximate_positions: approximate.positions,
        approximate_points: approximate.points,
        edges,
        positions,
        points,
        segment_lengths,
        length_mm,
        reached_face_index: approximate.reached_face_index,
        stopped_reason: approximate.stopped_reason,
        reduce_iterations,
        steps,
        meshlib_reference: "MR::computeSurfacePath / MR::reducePath",
    })
}

fn surface_path_from_descent(
    start_vertex: usize,
    end_vertex: usize,
    surface_distances_mm: Vec<f64>,
    surface_predecessor_vertices: Vec<Option<usize>>,
    descent: MeshSteepestDescentPath,
) -> MeshFastMarchingSurfacePath {
    let stopped_reason = if descent.reached_vertex == Some(end_vertex) {
        "end_reached"
    } else {
        descent.stopped_reason
    };
    MeshFastMarchingSurfacePath {
        start_vertex,
        end_vertex,
        start_face_index: descent.start_face_index,
        start_barycentric: descent.start_barycentric,
        surface_distances_mm,
        surface_predecessor_vertices,
        edges: descent.edges,
        positions: descent.positions,
        points: descent.points,
        segment_lengths: descent.segment_lengths,
        length_mm: descent.length_mm,
        reached_vertex: descent.reached_vertex,
        stopped_reason,
        steps: descent.steps,
        meshlib_reference: "MR::computeFastMarchingPath",
    }
}

fn empty_path(
    start_vertex: usize,
    end_vertex: usize,
    start_face_index: usize,
    start_barycentric: [f64; 3],
    surface_distances_mm: Vec<f64>,
    surface_predecessor_vertices: Vec<Option<usize>>,
) -> MeshFastMarchingSurfacePath {
    MeshFastMarchingSurfacePath {
        start_vertex,
        end_vertex,
        start_face_index,
        start_barycentric,
        surface_distances_mm,
        surface_predecessor_vertices,
        edges: Vec::new(),
        positions: Vec::new(),
        points: Vec::new(),
        segment_lengths: Vec::new(),
        length_mm: 0.0,
        reached_vertex: Some(end_vertex),
        stopped_reason: "same_triangle",
        steps: 0,
        meshlib_reference: "MR::computeFastMarchingPath",
    }
}

fn start_face_and_barycentric(
    faces: &[[usize; 3]],
    start_vertex: usize,
    end_vertex: usize,
) -> Result<(usize, [f64; 3]), GeometryError> {
    let mut fallback = None;
    for (face_index, face) in faces.iter().enumerate() {
        if !face.contains(&start_vertex) {
            continue;
        }
        let barycentric = vertex_barycentric(face, start_vertex);
        if !face.contains(&end_vertex) {
            return Ok((face_index, barycentric));
        }
        fallback.get_or_insert((face_index, barycentric));
    }
    fallback.ok_or_else(|| GeometryError::InvalidSelectionParameter {
        field: "start_vertex",
        value: "start_vertex_is_not_incident_to_any_face".to_string(),
    })
}

fn start_face_contains_end(face: &[usize; 3], end_vertex: usize) -> bool {
    face.contains(&end_vertex)
}

fn vertex_barycentric(face: &[usize; 3], vertex: usize) -> [f64; 3] {
    let mut barycentric = [0.0; 3];
    for (corner, face_vertex) in face.iter().enumerate() {
        if *face_vertex == vertex {
            barycentric[corner] = 1.0;
            break;
        }
    }
    barycentric
}

fn finite_distance_scalars(distances: &[f64]) -> Vec<f64> {
    distances
        .iter()
        .map(|distance| {
            if distance.is_finite() {
                *distance
            } else {
                f64::MAX
            }
        })
        .collect()
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

fn edge_point_reaches_face(edge: [usize; 2], position: f64, face: [usize; 3]) -> bool {
    if position <= VERTEX_EPSILON {
        face.contains(&edge[0])
    } else if position >= 1.0 - VERTEX_EPSILON {
        face.contains(&edge[1])
    } else {
        face.contains(&edge[0]) && face.contains(&edge[1])
    }
}

fn path_segment_lengths(
    start_point: [f64; 3],
    points: &[[f64; 3]],
    end_point: Option<[f64; 3]>,
) -> Vec<f64> {
    let mut previous = start_point;
    let mut lengths = Vec::with_capacity(points.len() + usize::from(end_point.is_some()));
    for point in points {
        lengths.push(distance_sq(previous, *point).sqrt());
        previous = *point;
    }
    if let Some(end) = end_point {
        lengths.push(distance_sq(previous, end).sqrt());
    }
    lengths
}

fn triangle_point(vertices: &[[f64; 3]], face: [usize; 3], barycentric: [f64; 3]) -> [f64; 3] {
    add(
        add(
            scale(vertices[face[0]], barycentric[0]),
            scale(vertices[face[1]], barycentric[1]),
        ),
        scale(vertices[face[2]], barycentric[2]),
    )
}

fn validate_face_index(
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

fn validate_barycentric(field: &'static str, barycentric: [f64; 3]) -> Result<(), GeometryError> {
    if barycentric.iter().any(|value| !value.is_finite()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: "weights_must_be_finite".to_string(),
        });
    }
    if barycentric.iter().any(|value| *value < -1e-9) {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: "weights_must_be_non_negative".to_string(),
        });
    }
    let sum = barycentric[0] + barycentric[1] + barycentric[2];
    if (sum - 1.0).abs() > 1e-8 {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("weights_sum_must_be_1_got_{sum}"),
        });
    }
    Ok(())
}

fn validate_vertex(
    field: &'static str,
    vertex: usize,
    vertex_count: usize,
) -> Result<(), GeometryError> {
    if vertex < vertex_count {
        Ok(())
    } else {
        Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("{vertex} for {vertex_count} vertices"),
        })
    }
}
