use std::collections::{BTreeMap, BTreeSet};

use crate::types::GeometryError;

use super::base::validate_faces;
use super::geodesic::{unique_valid_vertices, validate_max_distance, validate_vertex_id};
use super::surface_distance::surface_distance_field;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeodesicDistanceField {
    pub seed_vertices: Vec<usize>,
    pub distances_mm: Vec<f64>,
    pub predecessor_vertices: Vec<Option<usize>>,
    pub reachable_vertex_count: usize,
    pub max_distance_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshClosestSurfacePathTargets {
    pub start_vertices: Vec<usize>,
    pub end_vertices: Vec<usize>,
    pub target_vertices: Vec<Option<usize>>,
    pub target_distances_mm: Vec<f64>,
    pub distances_mm: Vec<f64>,
    pub predecessor_vertices: Vec<Option<usize>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshSurfaceDistanceSeedVertices {
    pub seed_vertices: Vec<usize>,
    pub selected_edges: Vec<[usize; 2]>,
    pub selected_face_indices: Vec<usize>,
    pub selected_face_boundary_edges: Vec<[usize; 2]>,
    pub meshlib_reference: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeodesicIsoRegion {
    pub field: MeshGeodesicDistanceField,
    pub iso_value_mm: f64,
    pub selected_vertex_indices: Vec<usize>,
    pub selected_face_indices: Vec<usize>,
    pub crossing_face_indices: Vec<usize>,
    pub boundary_edges: Vec<[usize; 2]>,
    pub iso_segments: Vec<[[f64; 3]; 2]>,
    pub clipped_vertices: Vec<[f64; 3]>,
    pub clipped_faces: Vec<[i64; 3]>,
    pub clipped_source_face_indices: Vec<usize>,
    pub clipped_source_vertex_indices: Vec<Option<usize>>,
}

#[derive(Copy, Clone, Debug)]
struct IsoClipVertex {
    point: [f64; 3],
    source_vertex_index: Option<usize>,
}

pub fn mesh_geodesic_distance_field(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    seed_vertices: &[usize],
    max_distance_mm: f64,
) -> Result<MeshGeodesicDistanceField, GeometryError> {
    validate_max_distance(max_distance_mm)?;
    let seeds = unique_valid_vertices("seed_vertices", seed_vertices, vertices.len())?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    let (distances_mm, predecessor_vertices) =
        surface_distance_field(vertices, &faces, &seeds, max_distance_mm);
    let reachable_distances = distances_mm
        .iter()
        .copied()
        .filter(|distance| distance.is_finite())
        .collect::<Vec<_>>();
    let max_reached = reachable_distances.iter().copied().fold(0.0_f64, f64::max);
    Ok(MeshGeodesicDistanceField {
        seed_vertices: seeds,
        distances_mm,
        predecessor_vertices,
        reachable_vertex_count: reachable_distances.len(),
        max_distance_mm: max_reached,
    })
}

pub fn mesh_closest_surface_path_targets(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_vertices: &[usize],
    end_vertices: &[usize],
    max_distance_mm: f64,
) -> Result<MeshClosestSurfacePathTargets, GeometryError> {
    validate_max_distance(max_distance_mm)?;
    let starts = unique_valid_vertices("start_vertices", start_vertices, vertices.len())?;
    let ends = unique_valid_vertices("end_vertices", end_vertices, vertices.len())?;
    let end_set = ends.iter().copied().collect::<BTreeSet<_>>();
    let faces = validate_faces(faces_i64, vertices.len())?;
    let (distances_mm, predecessor_vertices) =
        surface_distance_field(vertices, &faces, &ends, max_distance_mm);

    let mut target_vertices = Vec::with_capacity(starts.len());
    let mut target_distances_mm = Vec::with_capacity(starts.len());
    for start in &starts {
        target_distances_mm.push(distances_mm[*start]);
        if !distances_mm[*start].is_finite() {
            target_vertices.push(None);
            continue;
        }
        let mut cursor = *start;
        let mut steps = 0usize;
        while !end_set.contains(&cursor) {
            steps += 1;
            if steps > vertices.len() {
                return Err(GeometryError::InvalidSelectionParameter {
                    field: "surface_path_target",
                    value: "descent_loop".to_string(),
                });
            }
            let Some(next) = predecessor_vertices[cursor] else {
                target_vertices.push(None);
                break;
            };
            cursor = next;
        }
        if end_set.contains(&cursor) {
            target_vertices.push(Some(cursor));
        }
    }

    Ok(MeshClosestSurfacePathTargets {
        start_vertices: starts,
        end_vertices: ends,
        target_vertices,
        target_distances_mm,
        distances_mm,
        predecessor_vertices,
    })
}

pub fn mesh_surface_distance_seed_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    seed_vertices: &[usize],
    seed_edges_i64: &[[i64; 2]],
    seed_face_ids: &[usize],
) -> Result<MeshSurfaceDistanceSeedVertices, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut seeds = BTreeSet::new();
    for seed in seed_vertices {
        validate_vertex_id("seed_vertices", *seed, vertices.len())?;
        seeds.insert(*seed);
    }

    let mut selected_edges = BTreeSet::new();
    for edge in seed_edges_i64 {
        if edge[0] < 0 || edge[1] < 0 {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "seed_edges",
                value: format!("{edge:?}"),
            });
        }
        let a = edge[0] as usize;
        let b = edge[1] as usize;
        validate_vertex_id("seed_edges", a, vertices.len())?;
        validate_vertex_id("seed_edges", b, vertices.len())?;
        if a == b {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "seed_edges",
                value: format!("{edge:?}"),
            });
        }
        selected_edges.insert(sorted_edge(a, b));
        seeds.insert(a);
        seeds.insert(b);
    }

    let mut selected_face_indices = BTreeSet::new();
    let mut selected_face_edge_counts = BTreeMap::<[usize; 2], usize>::new();
    for face_id in seed_face_ids {
        if *face_id >= faces.len() {
            return Err(GeometryError::FaceRegionIndexOutOfBounds {
                index: *face_id,
                face_count: faces.len(),
            });
        }
        if !selected_face_indices.insert(*face_id) {
            continue;
        }
        let face = faces[*face_id];
        for edge in [
            sorted_edge(face[0], face[1]),
            sorted_edge(face[1], face[2]),
            sorted_edge(face[2], face[0]),
        ] {
            *selected_face_edge_counts.entry(edge).or_default() += 1;
        }
    }
    let selected_face_boundary_edges = selected_face_edge_counts
        .into_iter()
        .filter_map(|(edge, count)| {
            if count == 1 {
                seeds.insert(edge[0]);
                seeds.insert(edge[1]);
                Some(edge)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if seeds.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "surface_distance_sources",
            value: "empty".to_string(),
        });
    }

    Ok(MeshSurfaceDistanceSeedVertices {
        seed_vertices: seeds.into_iter().collect(),
        selected_edges: selected_edges.into_iter().collect(),
        selected_face_indices: selected_face_indices.into_iter().collect(),
        selected_face_boundary_edges,
        meshlib_reference: "Surface Distance selected edges / selected triangles boundary",
    })
}

pub fn mesh_geodesic_iso_region(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    seed_vertices: &[usize],
    iso_value_mm: f64,
    max_distance_mm: f64,
) -> Result<MeshGeodesicIsoRegion, GeometryError> {
    if !iso_value_mm.is_finite() || iso_value_mm < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "iso_value_mm",
            value: iso_value_mm.to_string(),
        });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    let field = mesh_geodesic_distance_field(vertices, faces_i64, seed_vertices, max_distance_mm)?;
    let mut selected_vertex_indices = Vec::new();
    for (index, distance) in field.distances_mm.iter().enumerate() {
        if distance.is_finite() && *distance <= iso_value_mm {
            selected_vertex_indices.push(index);
        }
    }

    let mut selected_face_indices = Vec::new();
    let mut crossing_face_indices = Vec::new();
    let mut boundary_edges = BTreeSet::<[usize; 2]>::new();
    let mut iso_segments = Vec::new();
    let mut clipped_vertices = Vec::new();
    let mut clipped_faces = Vec::new();
    let mut clipped_source_face_indices = Vec::new();
    let mut clipped_source_vertex_indices = Vec::new();
    for (face_index, face) in faces.iter().enumerate() {
        let distances = [
            field.distances_mm[face[0]],
            field.distances_mm[face[1]],
            field.distances_mm[face[2]],
        ];
        let clipped_polygon = clipped_iso_polygon(vertices, face, distances, iso_value_mm);
        if clipped_polygon.len() >= 3 {
            let base_index = clipped_vertices.len();
            let vertex_count = clipped_polygon.len();
            for vertex in clipped_polygon {
                clipped_vertices.push(vertex.point);
                clipped_source_vertex_indices.push(vertex.source_vertex_index);
            }
            for index in 1..(vertex_count - 1) {
                clipped_faces.push([
                    base_index as i64,
                    (base_index + index) as i64,
                    (base_index + index + 1) as i64,
                ]);
                clipped_source_face_indices.push(face_index);
            }
        }
        if distances
            .iter()
            .all(|distance| distance.is_finite() && *distance <= iso_value_mm)
        {
            selected_face_indices.push(face_index);
            continue;
        }
        let mut crossings = Vec::new();
        for (a_corner, b_corner) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
            let a = face[a_corner];
            let b = face[b_corner];
            let da = distances[a_corner];
            let db = distances[b_corner];
            if !da.is_finite() || !db.is_finite() {
                continue;
            }
            let a_inside = da <= iso_value_mm;
            let b_inside = db <= iso_value_mm;
            if a_inside == b_inside {
                continue;
            }
            boundary_edges.insert(sorted_edge(a, b));
            crossings.push(interpolate_iso_point(
                vertices[a],
                vertices[b],
                da,
                db,
                iso_value_mm,
            ));
        }
        if crossings.len() == 2 {
            crossing_face_indices.push(face_index);
            iso_segments.push([crossings[0], crossings[1]]);
        }
    }

    Ok(MeshGeodesicIsoRegion {
        field,
        iso_value_mm,
        selected_vertex_indices,
        selected_face_indices,
        crossing_face_indices,
        boundary_edges: boundary_edges.into_iter().collect(),
        iso_segments,
        clipped_vertices,
        clipped_faces,
        clipped_source_face_indices,
        clipped_source_vertex_indices,
    })
}

fn sorted_edge(a: usize, b: usize) -> [usize; 2] {
    if a <= b {
        [a, b]
    } else {
        [b, a]
    }
}

fn interpolate_iso_point(
    a: [f64; 3],
    b: [f64; 3],
    distance_a: f64,
    distance_b: f64,
    iso_value_mm: f64,
) -> [f64; 3] {
    let denominator = distance_b - distance_a;
    let t = if denominator.abs() <= f64::EPSILON {
        0.0
    } else {
        ((iso_value_mm - distance_a) / denominator).clamp(0.0, 1.0)
    };
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn clipped_iso_polygon(
    vertices: &[[f64; 3]],
    face: &[usize; 3],
    distances: [f64; 3],
    iso_value_mm: f64,
) -> Vec<IsoClipVertex> {
    if distances.iter().any(|distance| !distance.is_finite()) {
        return Vec::new();
    }
    let mut polygon = Vec::new();
    for (current_corner, next_corner) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
        let current_vertex = face[current_corner];
        let next_vertex = face[next_corner];
        let current_inside = distances[current_corner] <= iso_value_mm;
        let next_inside = distances[next_corner] <= iso_value_mm;
        if current_inside {
            polygon.push(IsoClipVertex {
                point: vertices[current_vertex],
                source_vertex_index: Some(current_vertex),
            });
        }
        if current_inside != next_inside {
            polygon.push(IsoClipVertex {
                point: interpolate_iso_point(
                    vertices[current_vertex],
                    vertices[next_vertex],
                    distances[current_corner],
                    distances[next_corner],
                    iso_value_mm,
                ),
                source_vertex_index: None,
            });
        }
    }
    polygon
}
