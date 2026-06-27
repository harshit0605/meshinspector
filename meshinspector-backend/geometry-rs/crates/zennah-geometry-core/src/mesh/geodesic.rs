use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use crate::math::distance_sq;
use crate::types::GeometryError;

use super::base::{edge_face_map, validate_faces, vertex_neighbor_list, vertex_normals_from_faces};

#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeodesicPath {
    pub vertex_indices: Vec<usize>,
    pub points: Vec<[f64; 3]>,
    pub point_normals: Vec<[f64; 3]>,
    pub edge_lengths: Vec<f64>,
    pub length_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeodesicPolylinePath {
    pub control_vertex_indices: Vec<usize>,
    pub control_vertex_offsets: Vec<usize>,
    pub vertex_indices: Vec<usize>,
    pub points: Vec<[f64; 3]>,
    pub point_normals: Vec<[f64; 3]>,
    pub edge_lengths: Vec<f64>,
    pub leg_lengths: Vec<f64>,
    pub leg_vertex_offsets: Vec<usize>,
    pub length_mm: f64,
    pub line_segments: usize,
    pub closed_path: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshCutMeasureIntersection {
    pub primitive_type: &'static str,
    pub primitive_id: usize,
    pub coordinate: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshCutMeasureContour {
    pub closed: bool,
    pub intersections: Vec<MeshCutMeasureIntersection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshCutMeasureContours {
    pub path: MeshGeodesicPolylinePath,
    pub contours: Vec<MeshCutMeasureContour>,
    pub pivot_indices: Vec<usize>,
    pub result_cut_vertex_indices: Vec<Vec<usize>>,
    pub bad_face_indices: Vec<usize>,
    pub closed_path: bool,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshCutMeasureTopologyCut {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub source_path_vertex_indices: Vec<usize>,
    pub result_cut_vertex_indices: Vec<Vec<usize>>,
    pub duplicate_vertex_map: Vec<[usize; 2]>,
    pub cut_edge_pairs: Vec<[usize; 2]>,
    pub result_cut_edge_pairs: Vec<[usize; 2]>,
    pub bad_face_indices: Vec<usize>,
    pub closed_path: bool,
    pub length_mm: f64,
    pub meshlib_reference: &'static str,
}

#[derive(Copy, Clone, Debug)]
struct QueueState {
    cost: f64,
    vertex: usize,
}

impl PartialEq for QueueState {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.vertex == other.vertex
    }
}

impl Eq for QueueState {}

impl Ord for QueueState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| self.vertex.cmp(&other.vertex))
    }
}

impl PartialOrd for QueueState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn mesh_geodesic_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_vertex: usize,
    end_vertex: usize,
    max_path_len_mm: f64,
) -> Result<MeshGeodesicPath, GeometryError> {
    validate_vertex_id("start_vertex", start_vertex, vertices.len())?;
    validate_vertex_id("end_vertex", end_vertex, vertices.len())?;
    validate_max_distance(max_path_len_mm)?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    let normals = vertex_normals_from_faces(vertices, &faces);
    if start_vertex == end_vertex {
        return Ok(MeshGeodesicPath {
            vertex_indices: vec![start_vertex],
            points: vec![vertices[start_vertex]],
            point_normals: vec![normals[start_vertex]],
            edge_lengths: Vec::new(),
            length_mm: 0.0,
        });
    }

    let (distances, previous) =
        dijkstra_distance_field(vertices, &faces, &[start_vertex], max_path_len_mm);

    if !distances[end_vertex].is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "path",
            value: "start_end_not_connected".to_string(),
        });
    }

    let vertex_indices = reconstruct_path(&previous, start_vertex, end_vertex)?;
    let points = vertex_indices
        .iter()
        .map(|index| vertices[*index])
        .collect::<Vec<_>>();
    let point_normals = vertex_indices
        .iter()
        .map(|index| normals[*index])
        .collect::<Vec<_>>();
    let edge_lengths = points
        .windows(2)
        .map(|window| distance_sq(window[0], window[1]).sqrt())
        .collect::<Vec<_>>();
    let length_mm = edge_lengths.iter().sum();
    Ok(MeshGeodesicPath {
        vertex_indices,
        points,
        point_normals,
        edge_lengths,
        length_mm,
    })
}

pub fn mesh_geodesic_polyline_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    control_vertices: &[usize],
    max_path_len_mm: f64,
) -> Result<MeshGeodesicPolylinePath, GeometryError> {
    mesh_geodesic_polyline_path_with_close(
        vertices,
        faces_i64,
        control_vertices,
        false,
        max_path_len_mm,
    )
}

pub fn mesh_geodesic_polyline_path_with_close(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    control_vertices: &[usize],
    close_path: bool,
    max_path_len_mm: f64,
) -> Result<MeshGeodesicPolylinePath, GeometryError> {
    validate_max_distance(max_path_len_mm)?;
    if control_vertices.len() < 2 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "control_vertices",
            value: "requires_at_least_two_vertices".to_string(),
        });
    }
    for control_vertex in control_vertices {
        validate_vertex_id("control_vertices", *control_vertex, vertices.len())?;
    }
    let mut traversal_controls = control_vertices.to_vec();
    let closed_path = close_path && traversal_controls.first() != traversal_controls.last();
    if closed_path {
        traversal_controls.push(traversal_controls[0]);
    }

    let mut vertex_indices = Vec::new();
    let mut points = Vec::new();
    let mut point_normals = Vec::new();
    let mut edge_lengths = Vec::new();
    let mut leg_lengths = Vec::new();
    let mut leg_vertex_offsets = Vec::new();
    let mut control_vertex_offsets = Vec::with_capacity(traversal_controls.len());
    let mut length_mm = 0.0;

    for (leg_index, leg) in traversal_controls.windows(2).enumerate() {
        let leg_start_offset = if vertex_indices.is_empty() {
            0
        } else {
            vertex_indices.len() - 1
        };
        leg_vertex_offsets.push(leg_start_offset);
        if leg_index == 0 {
            control_vertex_offsets.push(0);
        }
        let remaining_path_len_mm = if max_path_len_mm.is_infinite() {
            max_path_len_mm
        } else {
            max_path_len_mm - length_mm
        };
        if remaining_path_len_mm <= 0.0 {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "path",
                value: "max_path_len_exceeded".to_string(),
            });
        }
        let segment =
            mesh_geodesic_path(vertices, faces_i64, leg[0], leg[1], remaining_path_len_mm)?;
        length_mm += segment.length_mm;
        if !max_path_len_mm.is_infinite() && length_mm > max_path_len_mm {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "path",
                value: "max_path_len_exceeded".to_string(),
            });
        }
        leg_lengths.push(segment.length_mm);
        edge_lengths.extend(segment.edge_lengths);
        let skip_count = usize::from(leg_index > 0);
        vertex_indices.extend(segment.vertex_indices.into_iter().skip(skip_count));
        points.extend(segment.points.into_iter().skip(skip_count));
        point_normals.extend(segment.point_normals.into_iter().skip(skip_count));
        control_vertex_offsets.push(vertex_indices.len() - 1);
    }

    let line_segments = edge_lengths.len();
    Ok(MeshGeodesicPolylinePath {
        control_vertex_indices: traversal_controls,
        control_vertex_offsets,
        vertex_indices,
        points,
        point_normals,
        edge_lengths,
        leg_lengths,
        leg_vertex_offsets,
        length_mm,
        line_segments,
        closed_path,
    })
}

pub fn mesh_cut_measure_contours(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    control_vertices: &[usize],
    close_path: bool,
    max_path_len_mm: f64,
) -> Result<MeshCutMeasureContours, GeometryError> {
    let path = mesh_geodesic_polyline_path_with_close(
        vertices,
        faces_i64,
        control_vertices,
        close_path,
        max_path_len_mm,
    )?;
    let intersections = path
        .vertex_indices
        .iter()
        .map(|vertex_index| MeshCutMeasureIntersection {
            primitive_type: "VertId",
            primitive_id: *vertex_index,
            coordinate: vertices[*vertex_index],
        })
        .collect::<Vec<_>>();
    let pivot_indices = path.control_vertex_offsets.clone();
    let result_cut_vertex_indices = vec![path.vertex_indices.clone()];
    let closed_path = path.closed_path;

    Ok(MeshCutMeasureContours {
        contours: vec![MeshCutMeasureContour {
            closed: closed_path,
            intersections,
        }],
        pivot_indices,
        result_cut_vertex_indices,
        bad_face_indices: Vec::new(),
        closed_path,
        path,
        meshlib_reference: "MR::convertSurfacePathsToMeshContours / MR::cutMesh",
    })
}

pub fn mesh_cut_measure_edge_path_topology_cut(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    control_vertices: &[usize],
    close_path: bool,
    max_path_len_mm: f64,
) -> Result<MeshCutMeasureTopologyCut, GeometryError> {
    let path = mesh_geodesic_polyline_path_with_close(
        vertices,
        faces_i64,
        control_vertices,
        close_path,
        max_path_len_mm,
    )?;
    if path.vertex_indices.len() < 2 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "control_vertices",
            value: "cut path must contain at least one edge".to_string(),
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    let edge_faces = edge_face_map(&faces);
    let mut output_vertices = vertices.to_vec();
    let mut output_faces = faces_i64.to_vec();
    let mut cut_edge_pairs = Vec::new();
    let mut result_cut_edge_pairs = Vec::new();
    let mut bad_faces = BTreeSet::new();
    let mut faces_to_duplicate = BTreeSet::new();

    for edge in path.vertex_indices.windows(2) {
        let a = edge[0];
        let b = edge[1];
        if a == b {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "control_vertices",
                value: "cut path contains a zero-length edge".to_string(),
            });
        }
        let key = ordered_edge(a, b);
        cut_edge_pairs.push([a, b]);
        let incident_faces =
            edge_faces
                .get(&key)
                .ok_or_else(|| GeometryError::InvalidSelectionParameter {
                    field: "control_vertices",
                    value: "cut path is not aligned to mesh edges".to_string(),
                })?;
        match incident_faces.len() {
            0 | 1 => {}
            2 => {
                faces_to_duplicate.insert(incident_faces[1]);
            }
            _ => {
                bad_faces.extend(incident_faces.iter().copied());
            }
        }
    }

    if !bad_faces.is_empty() {
        return Ok(MeshCutMeasureTopologyCut {
            vertices: output_vertices,
            faces: output_faces,
            source_path_vertex_indices: path.vertex_indices,
            result_cut_vertex_indices: Vec::new(),
            duplicate_vertex_map: Vec::new(),
            cut_edge_pairs,
            result_cut_edge_pairs,
            bad_face_indices: bad_faces.into_iter().collect(),
            closed_path: path.closed_path,
            length_mm: path.length_mm,
            meshlib_reference:
                "MR::convertSurfacePathsToMeshContours / MR::cutMesh edge-path seam subset",
        });
    }

    let cut_vertices = path.vertex_indices.iter().copied().collect::<BTreeSet<_>>();
    let mut duplicate_by_source = BTreeMap::new();
    for source_vertex in path.vertex_indices.iter().copied() {
        if duplicate_by_source.contains_key(&source_vertex) {
            continue;
        }
        let used_by_duplicated_face = faces_to_duplicate
            .iter()
            .any(|face_index| faces[*face_index].contains(&source_vertex));
        if !used_by_duplicated_face {
            continue;
        }
        let duplicate_vertex = output_vertices.len();
        output_vertices.push(vertices[source_vertex]);
        duplicate_by_source.insert(source_vertex, duplicate_vertex);
    }

    for face_index in faces_to_duplicate {
        let face = &mut output_faces[face_index];
        for vertex in face.iter_mut() {
            let source_vertex = *vertex as usize;
            if cut_vertices.contains(&source_vertex) {
                if let Some(duplicate_vertex) = duplicate_by_source.get(&source_vertex) {
                    *vertex = *duplicate_vertex as i64;
                }
            }
        }
    }

    let duplicate_path = path
        .vertex_indices
        .iter()
        .map(|vertex| duplicate_by_source.get(vertex).copied().unwrap_or(*vertex))
        .collect::<Vec<_>>();
    for edge in duplicate_path.windows(2) {
        result_cut_edge_pairs.push([edge[0], edge[1]]);
    }
    let duplicate_vertex_map = duplicate_by_source
        .into_iter()
        .map(|(source, duplicate)| [source, duplicate])
        .collect::<Vec<_>>();

    Ok(MeshCutMeasureTopologyCut {
        vertices: output_vertices,
        faces: output_faces,
        source_path_vertex_indices: path.vertex_indices,
        result_cut_vertex_indices: vec![duplicate_path],
        duplicate_vertex_map,
        cut_edge_pairs,
        result_cut_edge_pairs,
        bad_face_indices: Vec::new(),
        closed_path: path.closed_path,
        length_mm: path.length_mm,
        meshlib_reference:
            "MR::convertSurfacePathsToMeshContours / MR::cutMesh edge-path seam subset",
    })
}

pub(super) fn validate_vertex_id(
    field: &'static str,
    vertex: usize,
    vertex_count: usize,
) -> Result<(), GeometryError> {
    if vertex >= vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("{vertex} for {vertex_count} vertices"),
        });
    }
    Ok(())
}

pub(super) fn unique_valid_vertices(
    field: &'static str,
    vertices: &[usize],
    vertex_count: usize,
) -> Result<Vec<usize>, GeometryError> {
    if vertices.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: "empty".to_string(),
        });
    }
    let mut unique = BTreeSet::new();
    for vertex in vertices {
        validate_vertex_id(field, *vertex, vertex_count)?;
        unique.insert(*vertex);
    }
    Ok(unique.into_iter().collect())
}

pub(super) fn validate_max_distance(max_distance_mm: f64) -> Result<(), GeometryError> {
    if max_distance_mm.is_nan() || max_distance_mm <= 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_distance_mm",
            value: max_distance_mm.to_string(),
        });
    }
    Ok(())
}

fn dijkstra_distance_field(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    seed_vertices: &[usize],
    max_distance_mm: f64,
) -> (Vec<f64>, Vec<Option<usize>>) {
    let neighbors = vertex_neighbor_list(vertices.len(), faces);
    let mut distances = vec![f64::INFINITY; vertices.len()];
    let mut previous: Vec<Option<usize>> = vec![None; vertices.len()];
    let mut heap = BinaryHeap::new();
    for seed in seed_vertices {
        distances[*seed] = 0.0;
        heap.push(QueueState {
            cost: 0.0,
            vertex: *seed,
        });
    }

    while let Some(QueueState { cost, vertex }) = heap.pop() {
        if cost > distances[vertex] {
            continue;
        }
        if cost > max_distance_mm {
            break;
        }
        for neighbor in &neighbors[vertex] {
            let edge_length = distance_sq(vertices[vertex], vertices[*neighbor]).sqrt();
            let next_cost = cost + edge_length;
            if next_cost < distances[*neighbor] && next_cost <= max_distance_mm {
                distances[*neighbor] = next_cost;
                previous[*neighbor] = Some(vertex);
                heap.push(QueueState {
                    cost: next_cost,
                    vertex: *neighbor,
                });
            }
        }
    }

    (distances, previous)
}

fn ordered_edge(a: usize, b: usize) -> (usize, usize) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn reconstruct_path(
    previous: &[Option<usize>],
    start_vertex: usize,
    end_vertex: usize,
) -> Result<Vec<usize>, GeometryError> {
    let mut path = Vec::new();
    let mut cursor = end_vertex;
    path.push(cursor);
    while cursor != start_vertex {
        cursor = previous[cursor].ok_or_else(|| GeometryError::InvalidSelectionParameter {
            field: "path",
            value: "start_end_not_connected".to_string(),
        })?;
        path.push(cursor);
    }
    path.reverse();
    Ok(path)
}
