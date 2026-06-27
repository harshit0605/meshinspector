use crate::math::{dot, norm, sub};
use crate::mesh::validate_faces;
use crate::{ordered_boundary_loops, GeometryError};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct HoleFillPlanEntry {
    pub hole_index: usize,
    pub representative_edge: [i64; 2],
    pub boundary_vertex_indices: Vec<i64>,
    pub boundary_edge_count: usize,
    pub planned_triangles: usize,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleFillPlanDiagnostics {
    pub input_holes: usize,
    pub planned_holes: usize,
    pub skipped_holes: usize,
    pub total_boundary_edges: usize,
    pub total_planned_triangles: usize,
    pub max_edges: Option<usize>,
    pub plans: Vec<HoleFillPlanEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatedHoleBoundaryVertexEntry {
    pub vertex_index: i64,
    pub hole_indices: Vec<usize>,
    pub occurrences: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatedHoleBoundaryVerticesDiagnostics {
    pub input_holes: usize,
    pub repeated_vertex_count: usize,
    pub vertices: Vec<RepeatedHoleBoundaryVertexEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleComplicatingFaceEntry {
    pub repeated_vertex_index: i64,
    pub face_index: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleComplicatingFacesDiagnostics {
    pub input_repeated_vertex_count: usize,
    pub complicating_face_count: usize,
    pub faces: Vec<HoleComplicatingFaceEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveHoleComplicatingFacesReport {
    pub input_face_count: usize,
    pub output_face_count: usize,
    pub removed_face_count: usize,
    pub input_repeated_vertex_count: usize,
    pub output_repeated_vertex_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveHoleComplicatingFacesResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: RemoveHoleComplicatingFacesReport,
}

pub fn hole_fill_plan_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_edges: Option<usize>,
) -> Result<HoleFillPlanDiagnostics, GeometryError> {
    let loops = ordered_boundary_loops(vertices, faces_i64)?;
    let mut planned_holes = 0_usize;
    let mut skipped_holes = 0_usize;
    let mut total_boundary_edges = 0_usize;
    let mut total_planned_triangles = 0_usize;
    let mut plans = Vec::with_capacity(loops.len());

    for (hole_index, boundary_loop) in loops.iter().enumerate() {
        let boundary_edge_count = boundary_loop.len();
        total_boundary_edges += boundary_edge_count;
        let (skipped, skip_reason) = skip_hole_reason(boundary_edge_count, max_edges);
        let planned_triangles = if skipped {
            skipped_holes += 1;
            0
        } else {
            planned_holes += 1;
            boundary_edge_count.saturating_sub(2)
        };
        total_planned_triangles += planned_triangles;

        plans.push(HoleFillPlanEntry {
            hole_index,
            representative_edge: representative_edge(boundary_loop),
            boundary_vertex_indices: boundary_loop.iter().map(|vertex| *vertex as i64).collect(),
            boundary_edge_count,
            planned_triangles,
            skipped,
            skip_reason,
        });
    }

    Ok(HoleFillPlanDiagnostics {
        input_holes: loops.len(),
        planned_holes,
        skipped_holes,
        total_boundary_edges,
        total_planned_triangles,
        max_edges,
        plans,
    })
}

pub fn repeated_hole_boundary_vertices_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<RepeatedHoleBoundaryVerticesDiagnostics, GeometryError> {
    let loops = ordered_boundary_loops(vertices, faces_i64)?;
    let mut repeated_vertices: BTreeMap<usize, RepeatedHoleBoundaryVertexEntry> = BTreeMap::new();

    for (hole_index, boundary_loop) in loops.iter().enumerate() {
        let mut occurrences_by_vertex: BTreeMap<usize, usize> = BTreeMap::new();
        for vertex_index in boundary_loop {
            *occurrences_by_vertex.entry(*vertex_index).or_default() += 1;
        }

        for (vertex_index, occurrences) in occurrences_by_vertex {
            if occurrences < 2 {
                continue;
            }
            let entry = repeated_vertices.entry(vertex_index).or_insert_with(|| {
                RepeatedHoleBoundaryVertexEntry {
                    vertex_index: vertex_index as i64,
                    hole_indices: Vec::new(),
                    occurrences: 0,
                }
            });
            entry.hole_indices.push(hole_index);
            entry.occurrences += occurrences;
        }
    }

    let vertices = repeated_vertices.into_values().collect::<Vec<_>>();
    Ok(RepeatedHoleBoundaryVerticesDiagnostics {
        input_holes: loops.len(),
        repeated_vertex_count: vertices.len(),
        vertices,
    })
}

pub fn hole_complicating_faces_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<HoleComplicatingFacesDiagnostics, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let repeated = repeated_hole_boundary_vertices_diagnostics(vertices, faces_i64)?;
    let topology = LocalDirectedTopology::new(&faces);
    let mut entries: BTreeMap<usize, HoleComplicatingFaceEntry> = BTreeMap::new();

    for repeated_vertex in &repeated.vertices {
        let vertex_index = repeated_vertex.vertex_index as usize;
        let wedges = topology.boundary_wedges(vertices, vertex_index);
        let Some(keep_wedge_index) = largest_wedge_index(&wedges) else {
            continue;
        };
        for (wedge_index, wedge) in wedges.iter().enumerate() {
            if wedge_index == keep_wedge_index {
                continue;
            }
            for face_index in &wedge.face_indices {
                entries
                    .entry(*face_index)
                    .or_insert(HoleComplicatingFaceEntry {
                        repeated_vertex_index: repeated_vertex.vertex_index,
                        face_index: *face_index as i64,
                    });
            }
        }
    }

    let faces = entries.into_values().collect::<Vec<_>>();
    Ok(HoleComplicatingFacesDiagnostics {
        input_repeated_vertex_count: repeated.repeated_vertex_count,
        complicating_face_count: faces.len(),
        faces,
    })
}

pub fn remove_hole_complicating_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<RemoveHoleComplicatingFacesResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let diagnostics = hole_complicating_faces_diagnostics(vertices, faces_i64)?;
    let remove_faces = diagnostics
        .faces
        .iter()
        .map(|entry| entry.face_index as usize)
        .collect::<BTreeSet<_>>();
    let output_faces = faces_i64
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| (!remove_faces.contains(&face_index)).then_some(*face))
        .collect::<Vec<_>>();
    let output_repeated =
        repeated_hole_boundary_vertices_diagnostics(vertices, &output_faces)?.repeated_vertex_count;

    Ok(RemoveHoleComplicatingFacesResult {
        vertices: vertices.to_vec(),
        faces: output_faces,
        report: RemoveHoleComplicatingFacesReport {
            input_face_count: faces_i64.len(),
            output_face_count: faces_i64.len().saturating_sub(remove_faces.len()),
            removed_face_count: remove_faces.len(),
            input_repeated_vertex_count: diagnostics.input_repeated_vertex_count,
            output_repeated_vertex_count: output_repeated,
        },
    })
}

fn skip_hole_reason(
    boundary_edge_count: usize,
    max_edges: Option<usize>,
) -> (bool, Option<String>) {
    if boundary_edge_count < 3 {
        return (true, Some("invalid_boundary".to_string()));
    }
    if max_edges.is_some_and(|limit| boundary_edge_count > limit) {
        return (true, Some("max_edges_exceeded".to_string()));
    }
    (false, None)
}

fn representative_edge(boundary_loop: &[usize]) -> [i64; 2] {
    if boundary_loop.len() < 2 {
        return [-1, -1];
    }
    [boundary_loop[0] as i64, boundary_loop[1] as i64]
}

#[derive(Debug, Clone)]
struct HoleWedge {
    angle_radians: f64,
    face_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct LocalDirectedTopology {
    directed_edges: BTreeSet<(usize, usize)>,
    left_faces: BTreeMap<(usize, usize), usize>,
    next_in_left_face: BTreeMap<(usize, usize), (usize, usize)>,
}

impl LocalDirectedTopology {
    fn new(faces: &[[usize; 3]]) -> Self {
        let mut directed_edges = BTreeSet::new();
        let mut left_faces = BTreeMap::new();
        let mut next_in_left_face = BTreeMap::new();
        for (face_index, face) in faces.iter().enumerate() {
            let face_edges = [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])];
            for (edge_index, edge) in face_edges.iter().enumerate() {
                directed_edges.insert(*edge);
                directed_edges.insert((edge.1, edge.0));
                left_faces.entry(*edge).or_insert(face_index);
                next_in_left_face
                    .entry(*edge)
                    .or_insert(face_edges[(edge_index + 1) % face_edges.len()]);
            }
        }
        Self {
            directed_edges,
            left_faces,
            next_in_left_face,
        }
    }

    fn left(&self, edge: (usize, usize)) -> Option<usize> {
        self.left_faces.get(&edge).copied()
    }

    fn right(&self, edge: (usize, usize)) -> Option<usize> {
        self.left((edge.1, edge.0))
    }

    fn prev_origin(&self, edge: (usize, usize)) -> Option<(usize, usize)> {
        self.next_in_left_face.get(&(edge.1, edge.0)).copied()
    }

    fn boundary_wedges(&self, vertices: &[[f64; 3]], vertex_index: usize) -> Vec<HoleWedge> {
        let mut wedges = Vec::new();
        for edge in self
            .directed_edges
            .iter()
            .copied()
            .filter(|edge| edge.0 == vertex_index && self.left(*edge).is_none())
        {
            if self.right(edge).is_none() {
                continue;
            }
            wedges.push(self.boundary_wedge(vertices, vertex_index, edge));
        }
        wedges
    }

    fn boundary_wedge(
        &self,
        vertices: &[[f64; 3]],
        vertex_index: usize,
        start_edge: (usize, usize),
    ) -> HoleWedge {
        let mut current = start_edge;
        let mut visited = BTreeSet::new();
        let mut angle_radians = 0.0;
        let mut face_indices = Vec::new();

        while let Some(face_index) = self.right(current) {
            if !visited.insert(current) {
                break;
            }
            let Some(next_edge) = self.prev_origin(current) else {
                break;
            };
            if next_edge.0 != vertex_index {
                break;
            }
            angle_radians += angle_between(
                edge_vector(vertices, next_edge),
                edge_vector(vertices, current),
            );
            face_indices.push(face_index);
            current = next_edge;
        }

        HoleWedge {
            angle_radians,
            face_indices,
        }
    }
}

fn largest_wedge_index(wedges: &[HoleWedge]) -> Option<usize> {
    let mut best = None;
    let mut best_angle = -1.0;
    for (index, wedge) in wedges.iter().enumerate() {
        if wedge.angle_radians > best_angle {
            best = Some(index);
            best_angle = wedge.angle_radians;
        }
    }
    best
}

fn edge_vector(vertices: &[[f64; 3]], edge: (usize, usize)) -> [f64; 3] {
    sub(vertices[edge.1], vertices[edge.0])
}

fn angle_between(first: [f64; 3], second: [f64; 3]) -> f64 {
    let denominator = norm(first) * norm(second);
    if denominator <= 1e-12 {
        return 0.0;
    }
    (dot(first, second) / denominator).clamp(-1.0, 1.0).acos()
}
