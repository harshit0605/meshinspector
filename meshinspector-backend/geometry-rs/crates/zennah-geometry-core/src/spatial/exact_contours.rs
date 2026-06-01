use super::exact_intersections::exact_mesh_intersections;
use super::exact_kernel::{ExactTriangleOwner, TriangleEdgeIntersection};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactContourIntersection {
    pub edge_owner: ExactTriangleOwner,
    pub edge: [usize; 2],
    pub edge_left_face: Option<usize>,
    pub triangle_owner: ExactTriangleOwner,
    pub triangle_face: usize,
    pub triangle: [usize; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactIntersectionContour {
    pub intersections: Vec<ExactContourIntersection>,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectedEdge {
    face: usize,
    local_edge: usize,
}

#[derive(Debug, Clone)]
struct MeshTopologyLite {
    faces: Vec<[usize; 3]>,
    directed_edges: HashMap<(usize, usize), DirectedEdge>,
}

impl MeshTopologyLite {
    fn new(faces: Vec<[usize; 3]>) -> Self {
        let mut directed_edges = HashMap::with_capacity(faces.len() * 3);
        for (face_index, face) in faces.iter().enumerate() {
            for local_edge in 0..3 {
                let edge = [face[local_edge], face[(local_edge + 1) % 3]];
                directed_edges.insert(
                    (edge[0], edge[1]),
                    DirectedEdge {
                        face: face_index,
                        local_edge,
                    },
                );
            }
        }
        Self {
            faces,
            directed_edges,
        }
    }

    fn face_edge(&self, face: usize, local_edge: usize) -> DirectedEdge {
        DirectedEdge { face, local_edge }
    }

    fn edge_vertices(&self, edge: DirectedEdge) -> [usize; 2] {
        let face = self.faces[edge.face];
        [face[edge.local_edge], face[(edge.local_edge + 1) % 3]]
    }

    fn find_directed(&self, edge: [usize; 2]) -> Option<DirectedEdge> {
        self.directed_edges.get(&(edge[0], edge[1])).copied()
    }

    fn next(&self, edge: DirectedEdge) -> DirectedEdge {
        DirectedEdge {
            face: edge.face,
            local_edge: (edge.local_edge + 1) % 3,
        }
    }

    fn prev(&self, edge: DirectedEdge) -> DirectedEdge {
        DirectedEdge {
            face: edge.face,
            local_edge: (edge.local_edge + 2) % 3,
        }
    }

    fn sym(&self, edge: DirectedEdge) -> Option<DirectedEdge> {
        let vertices = self.edge_vertices(edge);
        self.find_directed([vertices[1], vertices[0]])
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct RecordKey {
    edge_owner: ExactTriangleOwner,
    edge: [usize; 2],
    triangle_face: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct NeighborLinks {
    prev: Option<usize>,
    next: Option<usize>,
}

pub fn exact_intersection_contours(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<Vec<ExactIntersectionContour>, GeometryError> {
    let first_faces = validate_faces(first_faces_i64, first_vertices.len())?;
    let second_faces = validate_faces(second_faces_i64, second_vertices.len())?;
    let intersections = exact_mesh_intersections(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    if intersections.is_empty() {
        return Ok(Vec::new());
    }

    let first_topology = MeshTopologyLite::new(first_faces.clone());
    let second_topology = MeshTopologyLite::new(second_faces.clone());
    let records = flatten_intersections(
        &intersections,
        &first_faces,
        &second_faces,
        &first_topology,
        &second_topology,
    );
    Ok(order_records_into_contours(
        records,
        &first_topology,
        &second_topology,
    ))
}

fn flatten_intersections(
    intersections: &[super::exact_intersections::ExactMeshIntersection],
    first_faces: &[[usize; 3]],
    second_faces: &[[usize; 3]],
    first_topology: &MeshTopologyLite,
    second_topology: &MeshTopologyLite,
) -> Vec<ExactContourIntersection> {
    let mut records = Vec::new();
    let mut seen = HashMap::<RecordKey, usize>::new();
    for pair in intersections {
        for intersection in &pair.intersections {
            let record = make_record(
                pair.first_face,
                pair.second_face,
                *intersection,
                first_faces,
                second_faces,
                first_topology,
                second_topology,
            );
            let key = record_key(&record);
            seen.entry(key).or_insert_with(|| {
                records.push(record);
                records.len() - 1
            });
        }
    }
    records
}

fn make_record(
    first_face: usize,
    second_face: usize,
    intersection: TriangleEdgeIntersection,
    first_faces: &[[usize; 3]],
    second_faces: &[[usize; 3]],
    first_topology: &MeshTopologyLite,
    second_topology: &MeshTopologyLite,
) -> ExactContourIntersection {
    match intersection.edge_owner {
        ExactTriangleOwner::First => {
            let directed = oriented_edge(
                first_topology,
                first_face,
                intersection.edge[0],
                intersection.d_is_left_from_triangle,
            );
            ExactContourIntersection {
                edge_owner: ExactTriangleOwner::First,
                edge: first_topology.edge_vertices(directed),
                edge_left_face: Some(directed.face),
                triangle_owner: ExactTriangleOwner::Second,
                triangle_face: second_face,
                triangle: second_faces[second_face],
            }
        }
        ExactTriangleOwner::Second => {
            let directed = oriented_edge(
                second_topology,
                second_face,
                intersection.edge[0],
                intersection.d_is_left_from_triangle,
            );
            ExactContourIntersection {
                edge_owner: ExactTriangleOwner::Second,
                edge: second_topology.edge_vertices(directed),
                edge_left_face: Some(directed.face),
                triangle_owner: ExactTriangleOwner::First,
                triangle_face: first_face,
                triangle: first_faces[first_face],
            }
        }
    }
}

fn oriented_edge(
    topology: &MeshTopologyLite,
    face: usize,
    local_edge: usize,
    d_is_left_from_triangle: bool,
) -> DirectedEdge {
    let edge = topology.face_edge(face, local_edge);
    if d_is_left_from_triangle {
        return edge;
    }
    topology.sym(edge).unwrap_or(edge)
}

fn order_records_into_contours(
    records: Vec<ExactContourIntersection>,
    first_topology: &MeshTopologyLite,
    second_topology: &MeshTopologyLite,
) -> Vec<ExactIntersectionContour> {
    let lookup = build_lookup(&records);
    let links = build_links(&records, &lookup, first_topology, second_topology);
    build_contours(records, links)
}

fn build_lookup(records: &[ExactContourIntersection]) -> HashMap<RecordKey, usize> {
    let mut lookup = HashMap::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        lookup.entry(record_key(record)).or_insert(index);
    }
    lookup
}

fn build_links(
    records: &[ExactContourIntersection],
    lookup: &HashMap<RecordKey, usize>,
    first_topology: &MeshTopologyLite,
    second_topology: &MeshTopologyLite,
) -> Vec<NeighborLinks> {
    let mut links = vec![NeighborLinks::default(); records.len()];
    for (index, record) in records.iter().enumerate() {
        let Some(next) = find_next(record, lookup, first_topology, second_topology) else {
            continue;
        };
        links[index].next = Some(next);
        if links[next].prev.is_none() {
            links[next].prev = Some(index);
        }
    }
    links
}

fn find_next(
    record: &ExactContourIntersection,
    lookup: &HashMap<RecordKey, usize>,
    first_topology: &MeshTopologyLite,
    second_topology: &MeshTopologyLite,
) -> Option<usize> {
    let edge_topology = topology_for(record.edge_owner, first_topology, second_topology);
    let tri_topology = topology_for(record.triangle_owner, first_topology, second_topology);
    let current_edge = edge_topology.find_directed(record.edge)?;
    let left_face = record.edge_left_face?;

    let same_owner_candidates = same_owner_successor_edges(edge_topology, current_edge);
    for edge in same_owner_candidates {
        let Some(index) = lookup_directed(
            lookup,
            RecordKey {
                edge_owner: record.edge_owner,
                edge: edge_topology.edge_vertices(edge),
                triangle_face: record.triangle_face,
            },
        ) else {
            continue;
        };
        return Some(index);
    }

    for edge in triangle_face_edges(tri_topology, record.triangle_face) {
        let Some(index) = lookup_directed(
            lookup,
            RecordKey {
                edge_owner: record.triangle_owner,
                edge: tri_topology.edge_vertices(edge),
                triangle_face: left_face,
            },
        ) else {
            continue;
        };
        return Some(index);
    }

    None
}

fn lookup_directed(lookup: &HashMap<RecordKey, usize>, key: RecordKey) -> Option<usize> {
    lookup.get(&key).copied().or_else(|| {
        lookup
            .get(&RecordKey {
                edge_owner: key.edge_owner,
                edge: [key.edge[1], key.edge[0]],
                triangle_face: key.triangle_face,
            })
            .copied()
    })
}

fn same_owner_successor_edges(
    topology: &MeshTopologyLite,
    current_edge: DirectedEdge,
) -> Vec<DirectedEdge> {
    let mut candidates = vec![topology.next(current_edge)];
    if let Some(sym) = topology.sym(current_edge) {
        candidates.push(topology.prev(sym));
    }
    candidates
}

fn triangle_face_edges(topology: &MeshTopologyLite, face: usize) -> [DirectedEdge; 3] {
    [
        topology.face_edge(face, 0),
        topology.face_edge(face, 1),
        topology.face_edge(face, 2),
    ]
}

fn build_contours(
    records: Vec<ExactContourIntersection>,
    links: Vec<NeighborLinks>,
) -> Vec<ExactIntersectionContour> {
    let mut queued = vec![true; records.len()];
    let mut contours = Vec::new();
    for seed in 0..records.len() {
        if !queued[seed] {
            continue;
        }
        let start = open_contour_start(seed, &links);
        let mut indices = Vec::new();
        let mut current = start;
        let mut closed = false;
        loop {
            if !queued[current] {
                closed = current == start;
                break;
            }
            queued[current] = false;
            indices.push(current);
            let Some(next) = links[current].next else {
                break;
            };
            current = next;
            if current == start {
                closed = true;
                break;
            }
        }
        contours.push(ExactIntersectionContour {
            intersections: indices
                .into_iter()
                .map(|index| records[index].clone())
                .collect(),
            closed,
        });
    }
    contours
}

fn open_contour_start(seed: usize, links: &[NeighborLinks]) -> usize {
    let mut current = seed;
    while let Some(prev) = links[current].prev {
        if prev == seed {
            return seed;
        }
        current = prev;
    }
    current
}

fn topology_for<'a>(
    owner: ExactTriangleOwner,
    first_topology: &'a MeshTopologyLite,
    second_topology: &'a MeshTopologyLite,
) -> &'a MeshTopologyLite {
    match owner {
        ExactTriangleOwner::First => first_topology,
        ExactTriangleOwner::Second => second_topology,
    }
}

fn record_key(record: &ExactContourIntersection) -> RecordKey {
    RecordKey {
        edge_owner: record.edge_owner,
        edge: record.edge,
        triangle_face: record.triangle_face,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_intersection_contours_group_crossing_open_planes() {
        let first_vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let first_faces = vec![[0, 1, 2], [0, 2, 3]];
        let second_vertices = vec![
            [0.5, -0.25, -1.0],
            [0.5, 1.25, -1.0],
            [0.5, 1.25, 1.0],
            [0.5, -0.25, 1.0],
        ];
        let second_faces = vec![[0, 1, 2], [0, 2, 3]];

        let contours = exact_intersection_contours(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert!(!contours.is_empty());
        assert!(contours
            .iter()
            .any(|contour| contour.intersections.len() >= 2));
    }

    #[test]
    fn exact_intersection_contours_skip_separated_meshes() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let first_faces = vec![[0, 1, 2]];
        let second_vertices = vec![[0.0, 0.0, 3.0], [2.0, 0.0, 3.0], [0.0, 2.0, 3.0]];
        let second_faces = vec![[0, 1, 2]];

        let contours = exact_intersection_contours(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert!(contours.is_empty());
    }
}
