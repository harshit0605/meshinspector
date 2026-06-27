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
    from: usize,
    to: usize,
    left_face: Option<usize>,
    right_face: Option<usize>,
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
                let from = face[local_edge];
                let to = face[(local_edge + 1) % 3];
                directed_edges
                    .entry((from, to))
                    .or_insert(DirectedEdge {
                        from,
                        to,
                        left_face: None,
                        right_face: None,
                    })
                    .left_face = Some(face_index);
                directed_edges
                    .entry((to, from))
                    .or_insert(DirectedEdge {
                        from: to,
                        to: from,
                        left_face: None,
                        right_face: None,
                    })
                    .right_face = Some(face_index);
            }
        }
        Self {
            faces,
            directed_edges,
        }
    }

    fn face_edge(&self, face: usize, local_edge: usize) -> DirectedEdge {
        let vertices = self.faces[face];
        self.find_directed([vertices[local_edge], vertices[(local_edge + 1) % 3]])
            .expect("face edge should be present in topology")
    }

    fn edge_vertices(&self, edge: DirectedEdge) -> [usize; 2] {
        [edge.from, edge.to]
    }

    fn find_directed(&self, edge: [usize; 2]) -> Option<DirectedEdge> {
        self.directed_edges.get(&(edge[0], edge[1])).copied()
    }

    fn next(&self, edge: DirectedEdge) -> Option<DirectedEdge> {
        let face = self.faces[edge.left_face?];
        let third = third_vertex(face, edge.from, edge.to)?;
        self.find_directed([edge.from, third])
    }

    fn prev(&self, edge: DirectedEdge) -> Option<DirectedEdge> {
        let face = self.faces[edge.right_face?];
        let third = third_vertex(face, edge.from, edge.to)?;
        self.find_directed([edge.from, third])
    }

    fn sym(&self, edge: DirectedEdge) -> Option<DirectedEdge> {
        self.find_directed([edge.to, edge.from])
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
                edge_left_face: directed.left_face,
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
                edge_left_face: directed.left_face,
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
    let stored_edge = edge_topology.find_directed(record.edge)?;
    let current_edge = match record.edge_owner {
        ExactTriangleOwner::First => stored_edge,
        ExactTriangleOwner::Second => edge_topology.sym(stored_edge).unwrap_or(stored_edge),
    };
    let left_face = current_edge.left_face?;

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

    for edge in triangle_successor_edges(tri_topology, record.triangle_face) {
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
    let mut candidates = Vec::with_capacity(2);
    if let Some(next) = topology.next(current_edge) {
        candidates.push(next);
    }
    if let Some(prev_sym) = topology
        .sym(current_edge)
        .and_then(|sym| topology.prev(sym))
    {
        candidates.push(prev_sym);
    }
    candidates
}

fn triangle_successor_edges(topology: &MeshTopologyLite, face: usize) -> Vec<DirectedEdge> {
    let edge = topology.face_edge(face, 0);
    let mut candidates = Vec::with_capacity(3);
    candidates.push(edge);
    if let Some(next) = topology.next(edge) {
        candidates.push(next);
    }
    if let Some(prev_sym) = topology.sym(edge).and_then(|sym| topology.prev(sym)) {
        candidates.push(prev_sym);
    }
    candidates
}

fn third_vertex(face: [usize; 3], first: usize, second: usize) -> Option<usize> {
    face.into_iter()
        .find(|vertex| *vertex != first && *vertex != second)
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

    #[test]
    fn triangle_successor_edges_include_adjacent_symmetric_face_candidate() {
        let topology = MeshTopologyLite::new(vec![[0, 1, 2], [1, 0, 3]]);

        let candidates = triangle_successor_edges(&topology, 0)
            .into_iter()
            .map(|edge| topology.edge_vertices(edge))
            .collect::<Vec<_>>();

        assert_eq!(candidates, vec![[0, 1], [0, 2], [1, 2]]);
    }

    #[test]
    fn shifted_cube_contours_form_meshlib_style_closed_ring() {
        let first_vertices = vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        let first_faces = vec![
            [0, 3, 2],
            [0, 2, 1],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let second_vertices = first_vertices
            .iter()
            .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
            .collect::<Vec<_>>();
        let one_mesh = super::super::exact_one_mesh::exact_one_mesh_intersection_contours(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &first_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert_eq!(one_mesh.first.len(), 1);
        let contour = &one_mesh.first[0];
        assert!(contour.closed);
        assert_eq!(contour.intersections.len(), 16);
        let primitives = contour
            .intersections
            .iter()
            .map(|intersection| intersection.primitive.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            primitives,
            vec![
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([0, 2]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([1, 2]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(6),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(6),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([1, 6]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(7),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([1, 5]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([0, 5]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(5),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(5),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(5),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([5, 0]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([1, 0]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([2, 0]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(0),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(0),
            ]
        );
        let coordinates = contour
            .intersections
            .iter()
            .map(|intersection| quantized_unit_point(intersection.coordinate))
            .collect::<Vec<_>>();
        assert_eq!(
            coordinates,
            vec![
                [1, 1, -1],
                [1, 1, -1],
                [1, 1, 0],
                [1, 1, 1],
                [1, 1, 1],
                [1, 0, 1],
                [1, -1, 1],
                [1, -1, 1],
                [0, -1, 1],
                [0, -1, 1],
                [0, -1, 1],
                [0, -1, 0],
                [0, -1, -1],
                [0, 0, -1],
                [0, 1, -1],
                [0, 1, -1],
            ]
        );
        let second_primitives = one_mesh.second[0]
            .intersections
            .iter()
            .map(|intersection| intersection.primitive.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            second_primitives,
            vec![
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(8),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(8),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([2, 7]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([6, 7]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(3),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([6, 4]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(2),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(2),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([4, 6]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([4, 7]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([4, 3]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(10),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(10),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Face(10),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([3, 4]),
                super::super::exact_one_mesh::ExactOneMeshPrimitive::Edge([3, 7]),
            ]
        );
    }

    fn quantized_unit_point(point: [f64; 3]) -> [i32; 3] {
        [
            point[0].round() as i32,
            point[1].round() as i32,
            point[2].round() as i32,
        ]
    }
}
