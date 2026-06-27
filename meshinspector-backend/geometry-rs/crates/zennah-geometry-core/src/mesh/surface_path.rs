use std::collections::BTreeSet;

use crate::math::{add, distance_sq, scale};
use crate::types::GeometryError;

use super::base::{edge_face_map, validate_faces};

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSurfaceEdgePointPath {
    pub edges: Vec<[usize; 2]>,
    pub positions: Vec<f64>,
    pub points: Vec<[f64; 3]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeodesicEdgePointPath {
    pub start_point: [f64; 3],
    pub end_point: [f64; 3],
    pub edges: Vec<[usize; 2]>,
    pub positions: Vec<f64>,
    pub mid_points: Vec<[f64; 3]>,
    pub points: Vec<[f64; 3]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub meshlib_reference: &'static str,
}

pub fn mesh_surface_edge_point_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    edge_points: &[[i64; 2]],
    positions: &[f64],
) -> Result<MeshSurfaceEdgePointPath, GeometryError> {
    if edge_points.len() != positions.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "edge_points",
            value: format!(
                "edge count {} does not match position count {}",
                edge_points.len(),
                positions.len()
            ),
        });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    let valid_edges = edge_face_map(&faces)
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();

    let mut edges = Vec::with_capacity(edge_points.len());
    let mut points = Vec::with_capacity(edge_points.len());
    for (edge, position) in edge_points.iter().zip(positions.iter()) {
        let edge = validate_edge(vertices.len(), &valid_edges, *edge)?;
        validate_position(*position)?;
        edges.push(edge);
        points.push(edge_point(vertices, edge, *position));
    }

    let segment_lengths = points
        .windows(2)
        .map(|window| distance_sq(window[0], window[1]).sqrt())
        .collect::<Vec<_>>();
    let length_mm = segment_lengths.iter().sum();
    Ok(MeshSurfaceEdgePointPath {
        edges,
        positions: positions.to_vec(),
        points,
        segment_lengths,
        length_mm,
        meshlib_reference: "MR::surfacePathLength / MR::surfacePathToContour3f",
    })
}

pub fn mesh_geodesic_edge_point_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_point: [f64; 3],
    edge_points: &[[i64; 2]],
    positions: &[f64],
    end_point: [f64; 3],
) -> Result<MeshGeodesicEdgePointPath, GeometryError> {
    validate_point3("start_point", start_point)?;
    validate_point3("end_point", end_point)?;
    let mids = mesh_surface_edge_point_path(vertices, faces_i64, edge_points, positions)?;
    let mut points = Vec::with_capacity(mids.points.len() + 2);
    points.push(start_point);
    points.extend(mids.points.iter().copied());
    points.push(end_point);
    let segment_lengths = points
        .windows(2)
        .map(|window| distance_sq(window[0], window[1]).sqrt())
        .collect::<Vec<_>>();
    let length_mm = segment_lengths.iter().sum();
    Ok(MeshGeodesicEdgePointPath {
        start_point,
        end_point,
        edges: mids.edges,
        positions: mids.positions,
        mid_points: mids.points,
        points,
        segment_lengths,
        length_mm,
        meshlib_reference: "MR::geodesicPathLength / MR::geodesicPathToContour3f",
    })
}

fn validate_edge(
    vertex_count: usize,
    valid_edges: &BTreeSet<(usize, usize)>,
    edge: [i64; 2],
) -> Result<[usize; 2], GeometryError> {
    if edge[0] < 0 || edge[1] < 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "edge_points",
            value: "edge_vertices_must_be_non_negative".to_string(),
        });
    }
    let output = [edge[0] as usize, edge[1] as usize];
    if output[0] >= vertex_count || output[1] >= vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "edge_points",
            value: format!(
                "edge {:?} is out of range for {vertex_count} vertices",
                edge
            ),
        });
    }
    if output[0] == output[1] {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "edge_points",
            value: "edge_vertices_must_be_distinct".to_string(),
        });
    }
    let key = if output[0] <= output[1] {
        (output[0], output[1])
    } else {
        (output[1], output[0])
    };
    if !valid_edges.contains(&key) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "edge_points",
            value: format!("edge {:?} is not a mesh edge", edge),
        });
    }
    Ok(output)
}

fn validate_position(position: f64) -> Result<(), GeometryError> {
    if position.is_finite() && (0.0..=1.0).contains(&position) {
        Ok(())
    } else {
        Err(GeometryError::InvalidSelectionParameter {
            field: "edge_point_positions",
            value: format!("{position}"),
        })
    }
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
