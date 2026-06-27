use crate::math::{cross, dot, norm, sub};
use crate::mesh::{edge_face_map, validate_faces};
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct CreaseEdgeEntry {
    pub edge: [i64; 2],
    pub face_indices: [usize; 2],
    pub dihedral_cosine: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreaseEdgeDiagnostics {
    pub angle_from_planar_radians: f64,
    pub min_component_length_mm: Option<f64>,
    pub min_branch_length_mm: Option<f64>,
    pub edge_count: usize,
    pub raw_crease_edge_count: usize,
    pub crease_edge_count: usize,
    pub edges: Vec<CreaseEdgeEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreaseRepairPlanRegion {
    pub crease_edge: [i64; 2],
    pub selected_origin_vertex: i64,
    pub selected_face_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreaseRepairPlanDiagnostics {
    pub angle_from_planar_radians: f64,
    pub critical_tri_aspect_ratio: f64,
    pub crease_edge_count: usize,
    pub planned_region_count: usize,
    pub planned_face_count: usize,
    pub regions: Vec<CreaseRepairPlanRegion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CreaseEdgeFilterOptions {
    pub min_component_length_mm: Option<f64>,
    pub min_branch_length_mm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotSmoothFaceEntry {
    pub face_index: usize,
    pub face: [i64; 3],
    pub angle_delta_radians: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotSmoothFaceDiagnostics {
    pub min_angle_radians: f64,
    pub face_count: usize,
    pub not_smooth_face_count: usize,
    pub faces: Vec<NotSmoothFaceEntry>,
}

pub fn not_smooth_face_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    min_angle_radians: f64,
) -> Result<NotSmoothFaceDiagnostics, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let edge_faces = edge_face_map(&faces);
    let normals = faces
        .iter()
        .map(|face| face_normal(vertices, *face))
        .collect::<Vec<_>>();
    let mut not_smooth_faces = Vec::new();

    for (face_index, face) in faces.iter().copied().enumerate() {
        let Some(current_normal) = normals[face_index] else {
            continue;
        };
        let mut neighbor_normals = Vec::with_capacity(3);
        let mut has_boundary_edge = false;
        for edge in face_edges(face) {
            let Some(face_ids) = edge_faces.get(&ordered_edge(edge.0, edge.1)) else {
                has_boundary_edge = true;
                break;
            };
            if face_ids.len() != 2 {
                has_boundary_edge = true;
                break;
            }
            let neighbor = if face_ids[0] == face_index {
                face_ids[1]
            } else {
                face_ids[0]
            };
            let Some(neighbor_normal) = normals[neighbor] else {
                has_boundary_edge = true;
                break;
            };
            neighbor_normals.push(neighbor_normal);
        }
        if has_boundary_edge || neighbor_normals.len() != 3 {
            continue;
        }

        let current_to_neighbors: f64 = neighbor_normals
            .iter()
            .map(|normal| angle_between(current_normal, *normal))
            .sum();
        let neighbor_to_neighbor = angle_between(neighbor_normals[0], neighbor_normals[1])
            + angle_between(neighbor_normals[1], neighbor_normals[2])
            + angle_between(neighbor_normals[2], neighbor_normals[0]);
        let angle_delta = current_to_neighbors - neighbor_to_neighbor;
        if angle_delta > min_angle_radians {
            not_smooth_faces.push(NotSmoothFaceEntry {
                face_index,
                face: faces_i64[face_index],
                angle_delta_radians: angle_delta,
            });
        }
    }

    Ok(NotSmoothFaceDiagnostics {
        min_angle_radians,
        face_count: faces.len(),
        not_smooth_face_count: not_smooth_faces.len(),
        faces: not_smooth_faces,
    })
}

pub fn select_not_smooth_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    min_angle_radians: f64,
) -> Result<Vec<i64>, GeometryError> {
    Ok(
        not_smooth_face_diagnostics(vertices, faces_i64, min_angle_radians)?
            .faces
            .into_iter()
            .map(|face| face.face_index as i64)
            .collect(),
    )
}

pub fn crease_edge_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    angle_from_planar_radians: f64,
) -> Result<CreaseEdgeDiagnostics, GeometryError> {
    crease_edge_diagnostics_with_filter(
        vertices,
        faces_i64,
        angle_from_planar_radians,
        CreaseEdgeFilterOptions::default(),
    )
}

pub fn crease_edge_diagnostics_with_filter(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    angle_from_planar_radians: f64,
    filter_options: CreaseEdgeFilterOptions,
) -> Result<CreaseEdgeDiagnostics, GeometryError> {
    if !angle_from_planar_radians.is_finite()
        || angle_from_planar_radians <= 0.0
        || angle_from_planar_radians >= PI
    {
        return Err(GeometryError::InvalidThicknessInput {
            field: "angle_from_planar_radians",
            value: angle_from_planar_radians,
        });
    }
    if let Some(min_component_length_mm) = filter_options.min_component_length_mm {
        if !min_component_length_mm.is_finite() || min_component_length_mm < 0.0 {
            return Err(GeometryError::InvalidThicknessInput {
                field: "min_component_length_mm",
                value: min_component_length_mm,
            });
        }
    }
    if let Some(min_branch_length_mm) = filter_options.min_branch_length_mm {
        if !min_branch_length_mm.is_finite() || min_branch_length_mm < 0.0 {
            return Err(GeometryError::InvalidThicknessInput {
                field: "min_branch_length_mm",
                value: min_branch_length_mm,
            });
        }
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    let edge_faces = edge_face_map(&faces);
    let normals = faces
        .iter()
        .map(|face| face_normal(vertices, *face))
        .collect::<Vec<_>>();
    let critical_cosine = angle_from_planar_radians.cos();
    let mut crease_edges = Vec::new();

    for (edge, face_indices) in &edge_faces {
        if face_indices.len() != 2 {
            continue;
        }
        let Some(left_normal) = normals[face_indices[0]] else {
            continue;
        };
        let Some(right_normal) = normals[face_indices[1]] else {
            continue;
        };
        let dihedral_cosine = dot(left_normal, right_normal).clamp(-1.0, 1.0);
        if dihedral_cosine <= critical_cosine {
            crease_edges.push(CreaseEdgeEntry {
                edge: [edge.0 as i64, edge.1 as i64],
                face_indices: [face_indices[0], face_indices[1]],
                dihedral_cosine,
            });
        }
    }
    crease_edges.sort_by_key(|edge| edge.edge);
    let raw_crease_edge_count = crease_edges.len();
    if let Some(min_component_length_mm) = filter_options.min_component_length_mm {
        filter_crease_components_by_length(vertices, &mut crease_edges, min_component_length_mm);
    }
    if let Some(min_branch_length_mm) = filter_options.min_branch_length_mm {
        filter_crease_branches_by_length(vertices, &mut crease_edges, min_branch_length_mm);
    }

    Ok(CreaseEdgeDiagnostics {
        angle_from_planar_radians,
        min_component_length_mm: filter_options.min_component_length_mm,
        min_branch_length_mm: filter_options.min_branch_length_mm,
        edge_count: edge_faces.len(),
        raw_crease_edge_count,
        crease_edge_count: crease_edges.len(),
        edges: crease_edges,
    })
}

pub fn crease_repair_plan_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    angle_from_planar_radians: f64,
    critical_tri_aspect_ratio: f64,
) -> Result<CreaseRepairPlanDiagnostics, GeometryError> {
    if !critical_tri_aspect_ratio.is_finite() || critical_tri_aspect_ratio <= 0.0 {
        return Err(GeometryError::InvalidThicknessInput {
            field: "critical_tri_aspect_ratio",
            value: critical_tri_aspect_ratio,
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    let crease_report = crease_edge_diagnostics(vertices, faces_i64, angle_from_planar_radians)?;
    let crease_edges = crease_report
        .edges
        .iter()
        .map(|entry| (entry.edge[0] as usize, entry.edge[1] as usize))
        .collect::<BTreeSet<_>>();
    let topology = CreaseDirectedTopology::new(&faces);
    let normals = faces
        .iter()
        .map(|face| face_normal(vertices, *face))
        .collect::<Vec<_>>();
    let planar_angle_cosine = (PI - angle_from_planar_radians).abs().cos();
    let mut regions = Vec::new();

    for crease in &crease_report.edges {
        let mut repair_edge = (crease.edge[0] as usize, crease.edge[1] as usize);
        let left_incident_creases = incident_crease_count(&crease_edges, repair_edge.0);
        let right_incident_creases = incident_crease_count(&crease_edges, repair_edge.1);
        if right_incident_creases > left_incident_creases
            || (right_incident_creases == left_incident_creases
                && topology.origin_degree(repair_edge.0) < topology.origin_degree(repair_edge.1))
        {
            repair_edge = (repair_edge.1, repair_edge.0);
        }

        let mut selected_faces = BTreeSet::new();
        collect_crease_repair_faces(
            vertices,
            &faces,
            &normals,
            &topology,
            &crease_edges,
            repair_edge,
            true,
            critical_tri_aspect_ratio,
            planar_angle_cosine,
            &mut selected_faces,
        );
        collect_crease_repair_faces(
            vertices,
            &faces,
            &normals,
            &topology,
            &crease_edges,
            repair_edge,
            false,
            critical_tri_aspect_ratio,
            planar_angle_cosine,
            &mut selected_faces,
        );
        if selected_faces.is_empty() {
            continue;
        }
        regions.push(CreaseRepairPlanRegion {
            crease_edge: crease.edge,
            selected_origin_vertex: repair_edge.0 as i64,
            selected_face_indices: selected_faces.into_iter().collect(),
        });
    }

    let planned_face_count = regions
        .iter()
        .map(|region| region.selected_face_indices.len())
        .sum();
    Ok(CreaseRepairPlanDiagnostics {
        angle_from_planar_radians,
        critical_tri_aspect_ratio,
        crease_edge_count: crease_report.crease_edge_count,
        planned_region_count: regions.len(),
        planned_face_count,
        regions,
    })
}

fn filter_crease_components_by_length(
    vertices: &[[f64; 3]],
    crease_edges: &mut Vec<CreaseEdgeEntry>,
    min_component_length_mm: f64,
) {
    if crease_edges.is_empty() || min_component_length_mm <= 0.0 {
        return;
    }

    let mut incident_edges: HashMap<i64, Vec<usize>> = HashMap::new();
    for (edge_index, edge) in crease_edges.iter().enumerate() {
        incident_edges
            .entry(edge.edge[0])
            .or_default()
            .push(edge_index);
        incident_edges
            .entry(edge.edge[1])
            .or_default()
            .push(edge_index);
    }

    let mut visited = vec![false; crease_edges.len()];
    let mut keep = vec![false; crease_edges.len()];
    for edge_index in 0..crease_edges.len() {
        if visited[edge_index] {
            continue;
        }
        let mut stack = vec![edge_index];
        let mut component = Vec::new();
        visited[edge_index] = true;
        while let Some(current) = stack.pop() {
            component.push(current);
            for vertex in crease_edges[current].edge {
                if let Some(neighbors) = incident_edges.get(&vertex) {
                    for &neighbor in neighbors {
                        if !visited[neighbor] {
                            visited[neighbor] = true;
                            stack.push(neighbor);
                        }
                    }
                }
            }
        }

        let component_length = component
            .iter()
            .map(|&index| edge_length(vertices, crease_edges[index].edge))
            .sum::<f64>();
        if component_length >= min_component_length_mm {
            for index in component {
                keep[index] = true;
            }
        }
    }

    let mut index = 0;
    crease_edges.retain(|_| {
        let retain = keep[index];
        index += 1;
        retain
    });
}

fn filter_crease_branches_by_length(
    vertices: &[[f64; 3]],
    crease_edges: &mut Vec<CreaseEdgeEntry>,
    min_branch_length_mm: f64,
) {
    if crease_edges.is_empty() || min_branch_length_mm <= 0.0 {
        return;
    }

    let adjacency = crease_adjacency(crease_edges);
    let mut remove = vec![false; crease_edges.len()];

    for (&start_vertex, connections) in &adjacency {
        if connections.len() != 1 {
            continue;
        }

        let mut current_edge = connections[0].0;
        let mut next_vertex = connections[0].1;
        let mut branch_length = 0.0;
        let mut branch = Vec::new();

        loop {
            branch.push(current_edge);
            branch_length += edge_length(vertices, crease_edges[current_edge].edge);
            if branch_length > min_branch_length_mm {
                break;
            }

            let Some(next_connections) = adjacency.get(&next_vertex) else {
                break;
            };
            if next_connections.len() == 1 || next_connections.len() > 2 {
                break;
            }

            let next_connection = if next_connections[0].0 == current_edge {
                next_connections[1]
            } else {
                next_connections[0]
            };
            current_edge = next_connection.0;
            next_vertex = next_connection.1;
            if next_vertex == start_vertex {
                break;
            }
        }

        if branch_length < min_branch_length_mm {
            for edge_index in branch {
                remove[edge_index] = true;
            }
        }
    }

    let mut index = 0;
    crease_edges.retain(|_| {
        let retain = !remove[index];
        index += 1;
        retain
    });
}

fn crease_adjacency(crease_edges: &[CreaseEdgeEntry]) -> HashMap<i64, Vec<(usize, i64)>> {
    let mut adjacency: HashMap<i64, Vec<(usize, i64)>> = HashMap::new();
    for (edge_index, edge) in crease_edges.iter().enumerate() {
        adjacency
            .entry(edge.edge[0])
            .or_default()
            .push((edge_index, edge.edge[1]));
        adjacency
            .entry(edge.edge[1])
            .or_default()
            .push((edge_index, edge.edge[0]));
    }
    adjacency
}

fn edge_length(vertices: &[[f64; 3]], edge: [i64; 2]) -> f64 {
    let a = vertices[edge[0] as usize];
    let b = vertices[edge[1] as usize];
    norm(sub(a, b))
}

fn face_normal(vertices: &[[f64; 3]], face: [usize; 3]) -> Option<[f64; 3]> {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let normal = cross(sub(b, a), sub(c, a));
    let magnitude = norm(normal);
    if magnitude <= 1e-12 {
        return None;
    }
    Some([
        normal[0] / magnitude,
        normal[1] / magnitude,
        normal[2] / magnitude,
    ])
}

fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(a, b).clamp(-1.0, 1.0).acos()
}

fn face_edges(face: [usize; 3]) -> [(usize, usize); 3] {
    [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
}

fn ordered_edge(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[derive(Debug, Clone)]
struct CreaseDirectedTopology {
    left_faces: BTreeMap<(usize, usize), usize>,
    next_origin: BTreeMap<(usize, usize), (usize, usize)>,
    prev_origin: BTreeMap<(usize, usize), (usize, usize)>,
    outgoing_vertices: BTreeMap<usize, BTreeSet<usize>>,
}

impl CreaseDirectedTopology {
    fn new(faces: &[[usize; 3]]) -> Self {
        let mut left_faces = BTreeMap::new();
        let mut prev_origin = BTreeMap::new();
        let mut outgoing_vertices: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();

        for (face_index, face) in faces.iter().enumerate() {
            let face_edges = [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])];
            for (edge_index, edge) in face_edges.iter().copied().enumerate() {
                let next_face_edge = face_edges[(edge_index + 1) % face_edges.len()];
                left_faces.entry(edge).or_insert(face_index);
                outgoing_vertices.entry(edge.0).or_default().insert(edge.1);
                outgoing_vertices.entry(edge.1).or_default().insert(edge.0);
                prev_origin
                    .entry((edge.1, edge.0))
                    .or_insert(next_face_edge);
            }
        }

        let mut next_origin = BTreeMap::new();
        for (edge, previous) in &prev_origin {
            next_origin.entry(*previous).or_insert(*edge);
        }

        Self {
            left_faces,
            next_origin,
            prev_origin,
            outgoing_vertices,
        }
    }

    fn left(&self, edge: (usize, usize)) -> Option<usize> {
        self.left_faces.get(&edge).copied()
    }

    fn right(&self, edge: (usize, usize)) -> Option<usize> {
        self.left((edge.1, edge.0))
    }

    fn next(&self, edge: (usize, usize)) -> Option<(usize, usize)> {
        self.next_origin.get(&edge).copied()
    }

    fn prev(&self, edge: (usize, usize)) -> Option<(usize, usize)> {
        self.prev_origin.get(&edge).copied()
    }

    fn origin_degree(&self, vertex: usize) -> usize {
        self.outgoing_vertices.get(&vertex).map_or(0, BTreeSet::len)
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_crease_repair_faces(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    normals: &[Option<[f64; 3]>],
    topology: &CreaseDirectedTopology,
    crease_edges: &BTreeSet<(usize, usize)>,
    crease_edge: (usize, usize),
    left_side: bool,
    critical_tri_aspect_ratio: f64,
    planar_angle_cosine: f64,
    selected_faces: &mut BTreeSet<usize>,
) {
    let mut current = crease_edge;
    let mut visited = BTreeSet::new();

    loop {
        if !visited.insert(current) {
            return;
        }
        let Some(face_index) = (if left_side {
            topology.left(current)
        } else {
            topology.right(current)
        }) else {
            return;
        };
        selected_faces.insert(face_index);

        let Some(next_edge) = (if left_side {
            topology.next(current)
        } else {
            topology.prev(current)
        }) else {
            return;
        };
        current = next_edge;
        if current == crease_edge {
            return;
        }

        let Some(next_face_index) = (if left_side {
            topology.left(current)
        } else {
            topology.right(current)
        }) else {
            return;
        };
        if crease_edges.contains(&ordered_edge(current.0, current.1)) {
            continue;
        }
        if meshlib_triangle_aspect_ratio(vertices, faces[face_index]) > critical_tri_aspect_ratio
            || meshlib_triangle_aspect_ratio(vertices, faces[next_face_index])
                > critical_tri_aspect_ratio
        {
            continue;
        }
        let Some(dihedral_cosine) = dihedral_cosine(topology, normals, current) else {
            return;
        };
        if dihedral_cosine < planar_angle_cosine {
            return;
        }
    }
}

fn incident_crease_count(crease_edges: &BTreeSet<(usize, usize)>, vertex: usize) -> usize {
    crease_edges
        .iter()
        .filter(|edge| edge.0 == vertex || edge.1 == vertex)
        .count()
}

fn dihedral_cosine(
    topology: &CreaseDirectedTopology,
    normals: &[Option<[f64; 3]>],
    edge: (usize, usize),
) -> Option<f64> {
    let left_normal = normals[topology.left(edge)?]?;
    let right_normal = normals[topology.right(edge)?]?;
    Some(dot(left_normal, right_normal).clamp(-1.0, 1.0))
}

fn meshlib_triangle_aspect_ratio(vertices: &[[f64; 3]], face: [usize; 3]) -> f64 {
    let bc = norm(sub(vertices[face[2]], vertices[face[1]]));
    let ca = norm(sub(vertices[face[0]], vertices[face[2]]));
    let ab = norm(sub(vertices[face[1]], vertices[face[0]]));
    let half_perimeter = (bc + ca + ab) / 2.0;
    let denominator = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if denominator <= 0.0 {
        return f64::MAX;
    }
    bc * ca * ab / denominator
}
