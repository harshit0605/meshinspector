use crate::math::{dot, norm, sub};
use crate::mesh::{edge_face_map, validate_faces};
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ShortEdgeEntry {
    pub edge: [i64; 2],
    pub length_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShortEdgeDiagnostics {
    pub critical_length_mm: f64,
    pub edge_count: usize,
    pub short_edge_count: usize,
    pub min_short_edge_length_mm: Option<f64>,
    pub max_short_edge_length_mm: Option<f64>,
    pub edges: Vec<ShortEdgeEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DegenerateFaceEntry {
    pub face_index: usize,
    pub face: [i64; 3],
    pub aspect_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DegenerateFaceDiagnostics {
    pub critical_aspect_ratio: f64,
    pub face_count: usize,
    pub degenerate_face_count: usize,
    pub min_degenerate_aspect_ratio: Option<f64>,
    pub max_degenerate_aspect_ratio: Option<f64>,
    pub faces: Vec<DegenerateFaceEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultipleEdgeEntry {
    pub vertex_pair: [i64; 2],
    pub topology_edge_count: usize,
    pub face_edge_occurrences: usize,
    pub forward_occurrences: usize,
    pub reverse_occurrences: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultipleEdgeDiagnostics {
    pub edge_count: usize,
    pub multiple_edge_count: usize,
    pub edges: Vec<MultipleEdgeEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultipleEdgeRepairReport {
    pub input_edge_count: usize,
    pub output_edge_count: usize,
    pub input_multiple_edge_count: usize,
    pub output_multiple_edge_count: usize,
    pub split_edge_count: usize,
    pub split_face_count: usize,
    pub added_vertex_count: usize,
    pub input_face_count: usize,
    pub output_face_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultipleEdgeRepairResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: MultipleEdgeRepairReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateMultiHoleVerticesReport {
    pub input_multi_hole_vertex_count: usize,
    pub output_multi_hole_vertex_count: usize,
    pub duplicated_vertex_count: usize,
    pub input_vertex_count: usize,
    pub output_vertex_count: usize,
    pub input_face_count: usize,
    pub output_face_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateMultiHoleVerticesResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: DuplicateMultiHoleVerticesReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NonManifoldEdgeRepairReport {
    pub input_nonmanifold_edge_count: usize,
    pub output_nonmanifold_edge_count: usize,
    pub removed_face_count: usize,
    pub input_vertex_count: usize,
    pub output_vertex_count: usize,
    pub input_face_count: usize,
    pub output_face_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NonManifoldEdgeRepairResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: NonManifoldEdgeRepairReport,
}

#[derive(Default)]
struct DirectedEdgeCounts {
    forward: usize,
    reverse: usize,
    total: usize,
}

#[derive(Clone, Debug)]
struct FaceEdgeOccurrence {
    face_index: usize,
    edge_slot: usize,
    forward: bool,
}

#[derive(Clone, Debug)]
struct SplitOperation {
    edge: (usize, usize),
    occurrences: Vec<FaceEdgeOccurrence>,
}

#[derive(Clone, Debug)]
struct MultiHoleVertexComponents {
    vertex: usize,
    components: Vec<Vec<usize>>,
}

pub fn short_edge_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    critical_length_mm: f64,
) -> Result<ShortEdgeDiagnostics, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let critical_length = critical_length_mm.abs();
    let critical_length_sq = critical_length * critical_length;
    let mut mesh_edges = BTreeSet::new();
    for face in &faces {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            if a != b {
                mesh_edges.insert(ordered_edge(a, b));
            }
        }
    }

    let mut edges = Vec::new();
    for (a, b) in &mesh_edges {
        let delta = sub(vertices[*a], vertices[*b]);
        let length_sq = dot(delta, delta);
        if length_sq <= critical_length_sq {
            edges.push(ShortEdgeEntry {
                edge: [*a as i64, *b as i64],
                length_mm: length_sq.sqrt(),
            });
        }
    }
    let min_short_edge_length_mm = edges.iter().map(|edge| edge.length_mm).reduce(f64::min);
    let max_short_edge_length_mm = edges.iter().map(|edge| edge.length_mm).reduce(f64::max);

    Ok(ShortEdgeDiagnostics {
        critical_length_mm: critical_length,
        edge_count: mesh_edges.len(),
        short_edge_count: edges.len(),
        min_short_edge_length_mm,
        max_short_edge_length_mm,
        edges,
    })
}

pub fn select_short_edges(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_edge_length_mm: f64,
) -> Result<Vec<[i64; 2]>, GeometryError> {
    Ok(
        short_edge_diagnostics(vertices, faces_i64, max_edge_length_mm)?
            .edges
            .into_iter()
            .map(|entry| entry.edge)
            .collect(),
    )
}

pub fn degenerate_face_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    critical_aspect_ratio: f64,
) -> Result<DegenerateFaceDiagnostics, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut degenerate_faces = Vec::new();

    for (face_index, face) in faces.iter().enumerate() {
        let aspect_ratio = meshlib_triangle_aspect_ratio(vertices, *face);
        if aspect_ratio >= critical_aspect_ratio {
            degenerate_faces.push(DegenerateFaceEntry {
                face_index,
                face: faces_i64[face_index],
                aspect_ratio,
            });
        }
    }

    let min_degenerate_aspect_ratio = degenerate_faces
        .iter()
        .map(|face| face.aspect_ratio)
        .reduce(f64::min);
    let max_degenerate_aspect_ratio = degenerate_faces
        .iter()
        .map(|face| face.aspect_ratio)
        .reduce(f64::max);

    Ok(DegenerateFaceDiagnostics {
        critical_aspect_ratio,
        face_count: faces.len(),
        degenerate_face_count: degenerate_faces.len(),
        min_degenerate_aspect_ratio,
        max_degenerate_aspect_ratio,
        faces: degenerate_faces,
    })
}

pub fn select_degenerate_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    min_aspect_ratio: f64,
    boundary_only: bool,
) -> Result<Vec<i64>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let boundary_faces = if boundary_only {
        let mut face_ids = BTreeSet::<usize>::new();
        for face_ids_for_edge in edge_face_map(&faces).values() {
            if face_ids_for_edge.len() == 1 {
                face_ids.insert(face_ids_for_edge[0]);
            }
        }
        Some(face_ids)
    } else {
        None
    };

    let mut selected = Vec::new();
    for (face_index, face) in faces.iter().enumerate() {
        if boundary_faces
            .as_ref()
            .is_some_and(|face_ids| !face_ids.contains(&face_index))
        {
            continue;
        }
        if meshlib_triangle_aspect_ratio(vertices, *face) >= min_aspect_ratio {
            selected.push(face_index as i64);
        }
    }
    Ok(selected)
}

pub fn multiple_edge_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<MultipleEdgeDiagnostics, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut edge_counts: BTreeMap<(usize, usize), DirectedEdgeCounts> = BTreeMap::new();

    for face in &faces {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            if a == b {
                continue;
            }
            let (edge, forward) = ordered_directed_edge(a, b);
            let counts = edge_counts.entry(edge).or_default();
            counts.total += 1;
            if forward {
                counts.forward += 1;
            } else {
                counts.reverse += 1;
            }
        }
    }

    let mut edges = Vec::new();
    for ((a, b), counts) in &edge_counts {
        let topology_edge_count = meshlib_like_topology_edge_count(counts);
        if topology_edge_count > 1 {
            edges.push(MultipleEdgeEntry {
                vertex_pair: [*a as i64, *b as i64],
                topology_edge_count,
                face_edge_occurrences: counts.total,
                forward_occurrences: counts.forward,
                reverse_occurrences: counts.reverse,
            });
        }
    }

    Ok(MultipleEdgeDiagnostics {
        edge_count: edge_counts.len(),
        multiple_edge_count: edges.len(),
        edges,
    })
}

pub fn repair_multiple_edges(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<MultipleEdgeRepairResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let input_diagnostics = multiple_edge_diagnostics(vertices, faces_i64)?;
    let mut output_vertices = vertices.to_vec();
    let mut output_faces = faces_i64.to_vec();
    let mut split_edge_count = 0_usize;
    let mut split_face_count = 0_usize;
    let max_iterations = faces_i64.len().saturating_mul(3).saturating_add(1);

    for _ in 0..max_iterations {
        let faces = validate_faces(&output_faces, output_vertices.len())?;
        let operations = multiple_edge_split_operations(&faces);
        if operations.is_empty() {
            break;
        }

        let mut used_faces = HashSet::new();
        let mut split_map: HashMap<usize, (usize, i64)> = HashMap::new();
        for operation in operations {
            if operation
                .occurrences
                .iter()
                .any(|occurrence| used_faces.contains(&occurrence.face_index))
            {
                continue;
            }
            let midpoint = edge_midpoint(&output_vertices, operation.edge);
            let midpoint_index = output_vertices.len() as i64;
            output_vertices.push(midpoint);
            split_edge_count += 1;
            split_face_count += operation.occurrences.len();
            for occurrence in operation.occurrences {
                used_faces.insert(occurrence.face_index);
                split_map.insert(
                    occurrence.face_index,
                    (occurrence.edge_slot, midpoint_index),
                );
            }
        }

        if split_map.is_empty() {
            break;
        }
        output_faces = split_marked_faces(&output_faces, &split_map);
    }

    let output_diagnostics = multiple_edge_diagnostics(&output_vertices, &output_faces)?;
    let output_face_count = output_faces.len();
    Ok(MultipleEdgeRepairResult {
        vertices: output_vertices,
        faces: output_faces,
        report: MultipleEdgeRepairReport {
            input_edge_count: input_diagnostics.edge_count,
            output_edge_count: output_diagnostics.edge_count,
            input_multiple_edge_count: input_diagnostics.multiple_edge_count,
            output_multiple_edge_count: output_diagnostics.multiple_edge_count,
            split_edge_count,
            split_face_count,
            added_vertex_count: split_edge_count,
            input_face_count: faces_i64.len(),
            output_face_count,
        },
    })
}

pub fn duplicate_multi_hole_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<DuplicateMultiHoleVerticesResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let input_multi_hole_vertex_count = multi_hole_vertex_count(vertices.len(), faces_i64)?;
    let mut output_vertices = vertices.to_vec();
    let mut output_faces = faces_i64.to_vec();
    let mut duplicated_vertex_count = 0_usize;

    loop {
        let faces = validate_faces(&output_faces, output_vertices.len())?;
        let components = multi_hole_vertex_components(output_vertices.len(), &faces);
        if components.is_empty() {
            break;
        }

        for entry in components {
            for component in entry.components.iter().skip(1) {
                let duplicate = output_vertices.len() as i64;
                output_vertices.push(output_vertices[entry.vertex]);
                duplicated_vertex_count += 1;
                for face_index in component {
                    for corner in &mut output_faces[*face_index] {
                        if *corner == entry.vertex as i64 {
                            *corner = duplicate;
                        }
                    }
                }
            }
        }
    }

    let output_multi_hole_vertex_count =
        multi_hole_vertex_count(output_vertices.len(), &output_faces)?;
    let output_vertex_count = output_vertices.len();
    let output_face_count = output_faces.len();
    Ok(DuplicateMultiHoleVerticesResult {
        vertices: output_vertices,
        faces: output_faces,
        report: DuplicateMultiHoleVerticesReport {
            input_multi_hole_vertex_count,
            output_multi_hole_vertex_count,
            duplicated_vertex_count,
            input_vertex_count: vertices.len(),
            output_vertex_count,
            input_face_count: faces_i64.len(),
            output_face_count,
        },
    })
}

pub fn repair_nonmanifold_edges(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<NonManifoldEdgeRepairResult, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let edge_map = edge_face_map(&faces);
    let input_nonmanifold_edge_count = edge_map
        .values()
        .filter(|face_ids| face_ids.len() > 2)
        .count();
    let mut nonmanifold_edges = edge_map
        .into_iter()
        .filter(|(_, face_ids)| face_ids.len() > 2)
        .collect::<Vec<_>>();
    nonmanifold_edges.sort_by_key(|((a, b), _)| (*a, *b));

    let mut faces_to_remove = BTreeSet::new();
    for (_, mut face_ids) in nonmanifold_edges {
        face_ids.sort_unstable();
        for face_id in face_ids.into_iter().skip(2) {
            faces_to_remove.insert(face_id);
        }
    }

    let output_faces = faces_i64
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(face_index, face)| (!faces_to_remove.contains(&face_index)).then_some(face))
        .collect::<Vec<_>>();
    let output_faces_usize = validate_faces(&output_faces, vertices.len())?;
    let output_nonmanifold_edge_count = edge_face_map(&output_faces_usize)
        .values()
        .filter(|face_ids| face_ids.len() > 2)
        .count();

    Ok(NonManifoldEdgeRepairResult {
        vertices: vertices.to_vec(),
        faces: output_faces,
        report: NonManifoldEdgeRepairReport {
            input_nonmanifold_edge_count,
            output_nonmanifold_edge_count,
            removed_face_count: faces_to_remove.len(),
            input_vertex_count: vertices.len(),
            output_vertex_count: vertices.len(),
            input_face_count: faces_i64.len(),
            output_face_count: output_faces_usize.len(),
        },
    })
}

