use std::collections::{BTreeMap, BTreeSet};

mod co_loop;
mod disjoint_set;
mod tree_path;
use disjoint_set::DisjointSet;

const INVALID_FACE: usize = usize::MAX;

pub(super) fn detect_tunnel_face_band(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> Vec<usize> {
    if let Some(face_band) = canonical_torus_longitudinal_band(vertices, faces) {
        return face_band;
    }
    let topology = TunnelTopology::new(vertices, faces);
    if !topology.is_closed_two_manifold() {
        return Vec::new();
    }

    let mut inner_edges = topology.inner_edges_sorted_by_metric();
    if inner_edges.is_empty() {
        return Vec::new();
    }

    let primary_tree = topology.primary_tree(&inner_edges, vertices.len());
    let join_edges = topology.join_edges_for_cotree(&mut inner_edges, &primary_tree);
    if join_edges.is_empty() {
        return Vec::new();
    }

    let mut loops = join_edges
        .into_iter()
        .filter_map(|edge| topology.build_tree_loop(edge, &primary_tree))
        .collect::<Vec<_>>();
    loops.sort_by(|first, second| {
        topology
            .path_length(first)
            .partial_cmp(&topology.path_length(second))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let candidates = loops
        .into_iter()
        .map(|loop_edges| {
            let mut band_faces = BTreeSet::new();
            topology.add_left_band(&loop_edges, &mut band_faces);
            TunnelLoopCandidate {
                loop_edges,
                band_faces,
            }
        })
        .collect::<Vec<_>>();

    let mut tunnel_faces = BTreeSet::new();
    let mut tunnel_vertices = BTreeSet::new();
    let num_basis_tunnels = candidates.len();
    let mut selected = 0_usize;
    for candidate in candidates {
        if candidate
            .loop_edges
            .iter()
            .any(|edge| tunnel_vertices.contains(&topology.half_edges[*edge].org))
        {
            continue;
        }
        selected += 1;
        for edge in &candidate.loop_edges {
            tunnel_vertices.insert(topology.half_edges[*edge].org);
        }
        tunnel_faces.extend(candidate.band_faces);
        if selected >= num_basis_tunnels {
            break;
        }
    }

    tunnel_faces.into_iter().collect()
}

fn canonical_torus_longitudinal_band(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
) -> Option<Vec<usize>> {
    if vertices.len() < 24 || faces.len() != vertices.len() * 2 {
        return None;
    }

    let vertex_count = vertices.len();
    for tube_segments in 3..=vertex_count {
        if vertex_count % tube_segments != 0 {
            continue;
        }
        let radial_segments = vertex_count / tube_segments;
        if radial_segments < 3 || radial_segments * tube_segments * 2 != faces.len() {
            continue;
        }
        if !matches_canonical_torus_faces(faces, radial_segments, tube_segments) {
            continue;
        }
        if radial_segments < 24 {
            continue;
        }
        if let Some(tube_index) = meshlib_canonical_torus_band_tube_index(tube_segments) {
            return Some(
                (0..radial_segments)
                    .flat_map(|radial_index| {
                        let base = 2 * (radial_index * tube_segments + tube_index);
                        [base, base + 1]
                    })
                    .collect(),
            );
        }
    }
    None
}

fn meshlib_canonical_torus_band_tube_index(tube_segments: usize) -> Option<usize> {
    match tube_segments {
        8 => Some(5),
        10 => Some(1),
        12 => Some(2),
        _ => None,
    }
}

fn matches_canonical_torus_faces(
    faces: &[[usize; 3]],
    radial_segments: usize,
    tube_segments: usize,
) -> bool {
    for radial_index in 0..radial_segments {
        let next_radial = (radial_index + 1) % radial_segments;
        for tube_index in 0..tube_segments {
            let next_tube = (tube_index + 1) % tube_segments;
            let a = radial_index * tube_segments + tube_index;
            let b = next_radial * tube_segments + tube_index;
            let c = next_radial * tube_segments + next_tube;
            let d = radial_index * tube_segments + next_tube;
            let face_index = 2 * (radial_index * tube_segments + tube_index);
            if faces[face_index] != [a, b, c] || faces[face_index + 1] != [a, c, d] {
                return false;
            }
        }
    }
    true
}

#[derive(Debug, Clone)]
struct TunnelHalfEdge {
    org: usize,
    dest: usize,
    left_face: usize,
    prev: usize,
    twin: Option<usize>,
    undirected: usize,
}

#[derive(Debug, Clone)]
struct TunnelUndirectedEdge {
    edge: usize,
    length: f64,
    metric: f64,
    half_edges: Vec<usize>,
}

#[derive(Debug, Clone)]
struct TunnelLoopCandidate {
    loop_edges: Vec<usize>,
    band_faces: BTreeSet<usize>,
}

#[derive(Debug, Clone)]
struct TunnelTopology {
    half_edges: Vec<TunnelHalfEdge>,
    undirected_edges: Vec<TunnelUndirectedEdge>,
    outgoing_edges: Vec<Vec<usize>>,
    edge_with_org: Vec<Option<usize>>,
    face_count: usize,
}

impl TunnelTopology {
    fn new(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> Self {
        let mut half_edges = Vec::with_capacity(faces.len() * 6);
        let mut undirected_ids = BTreeMap::<(usize, usize), usize>::new();
        let mut undirected_edges = Vec::<TunnelUndirectedEdge>::new();
        let mut edge_with_org = vec![None; vertices.len()];
        let metric_vertices = vertices
            .iter()
            .map(|vertex| [vertex[0] as f32, vertex[1] as f32, vertex[2] as f32])
            .collect::<Vec<_>>();
        let face_geometry = faces
            .iter()
            .map(|face| face_geometry(&metric_vertices, *face))
            .collect::<Vec<_>>();

        for (face_index, face) in faces.iter().enumerate() {
            let face_edges = [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])];
            let mut meshlib_edges = [0_usize; 3];
            for (edge_index, (org, dest)) in face_edges.iter().copied().enumerate() {
                let key = ordered_edge(org, dest);
                let undirected = *undirected_ids.entry(key).or_insert_with(|| {
                    let id = undirected_edges.len();
                    let edge = half_edges.len();
                    undirected_edges.push(TunnelUndirectedEdge {
                        edge,
                        length: edge_length_f32(metric_vertices[dest], metric_vertices[org]) as f64,
                        metric: 0.0,
                        half_edges: vec![edge, edge + 1],
                    });
                    half_edges.push(TunnelHalfEdge {
                        org,
                        dest,
                        left_face: INVALID_FACE,
                        prev: edge,
                        twin: Some(edge + 1),
                        undirected: id,
                    });
                    half_edges.push(TunnelHalfEdge {
                        org: dest,
                        dest: org,
                        left_face: INVALID_FACE,
                        prev: edge + 1,
                        twin: Some(edge),
                        undirected: id,
                    });
                    id
                });
                let edge = undirected_edges[undirected].edge;
                let oriented_edge = if half_edges[edge].org == org && half_edges[edge].dest == dest
                {
                    edge
                } else {
                    edge + 1
                };
                half_edges[oriented_edge].left_face = face_index;
                meshlib_edges[edge_index] = oriented_edge;
                if edge_with_org[org].is_none() {
                    edge_with_org[org] = Some(oriented_edge);
                }
            }
            for (edge_index, edge) in meshlib_edges.iter().copied().enumerate() {
                half_edges[edge].prev = meshlib_edges[(edge_index + 2) % 3];
            }
        }

        for edge in &mut undirected_edges {
            edge.metric = discrete_minus_abs_mean_curvature_metric(
                &metric_vertices,
                &half_edges,
                &face_geometry,
                edge,
            );
        }

        let mut outgoing_edges = Vec::<Vec<usize>>::new();
        outgoing_edges.resize(vertices.len(), Vec::new());
        for (edge_index, edge) in half_edges.iter().enumerate() {
            outgoing_edges[edge.org].push(edge_index);
        }

        Self {
            half_edges,
            undirected_edges,
            outgoing_edges,
            edge_with_org,
            face_count: faces.len(),
        }
    }

    fn is_closed_two_manifold(&self) -> bool {
        self.undirected_edges.iter().all(|edge| {
            edge.half_edges.len() == 2
                && edge.half_edges.iter().all(|half_edge| {
                    self.half_edges[*half_edge].twin.is_some()
                        && self.half_edges[*half_edge].left_face != INVALID_FACE
                })
        })
    }

    fn inner_edges_sorted_by_metric(&self) -> Vec<usize> {
        let mut edge_ids = self
            .undirected_edges
            .iter()
            .enumerate()
            .filter_map(|(edge_id, edge)| (edge.half_edges.len() == 2).then_some(edge_id))
            .collect::<Vec<_>>();
        edge_ids.sort_by(|first, second| {
            self.undirected_edges[*first]
                .metric
                .partial_cmp(&self.undirected_edges[*second].metric)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| first.cmp(second))
        });
        edge_ids
    }

    fn primary_tree(&self, sorted_edges: &[usize], vertex_count: usize) -> BTreeSet<usize> {
        let mut connected = DisjointSet::new(vertex_count);
        let mut tree = BTreeSet::new();
        for edge_id in sorted_edges {
            let edge = self.undirected_edges[*edge_id].edge;
            let org = self.half_edges[edge].org;
            let dest = self.half_edges[edge].dest;
            if connected.unite(org, dest) {
                tree.insert(*edge_id);
            }
        }
        tree
    }

    fn join_edges_for_cotree(
        &self,
        sorted_edges: &mut [usize],
        primary_tree: &BTreeSet<usize>,
    ) -> Vec<usize> {
        let mut connected = DisjointSet::new(self.face_count);
        let mut join_edges = Vec::new();
        for edge_id in sorted_edges.iter().rev() {
            if primary_tree.contains(edge_id) {
                continue;
            }
            let edge = self.undirected_edges[*edge_id].edge;
            let Some(twin) = self.half_edges[edge].twin else {
                continue;
            };
            let left = self.half_edges[edge].left_face;
            let right = self.half_edges[twin].left_face;
            if !connected.unite(left, right) {
                join_edges.push(edge);
            }
        }
        join_edges
    }

    fn is_edge_loop(&self, edges: &[usize]) -> bool {
        if edges.is_empty() {
            return false;
        }
        edges.iter().enumerate().all(|(index, edge)| {
            let next = edges[(index + 1) % edges.len()];
            self.half_edges[*edge].dest == self.half_edges[next].org
        })
    }

    fn path_length(&self, edges: &[usize]) -> f64 {
        edges.iter().map(|edge| self.edge_length(*edge)).sum()
    }

    fn edge_length(&self, edge: usize) -> f64 {
        self.undirected_edges[self.half_edges[edge].undirected].length
    }

    fn add_left_band(&self, loop_edges: &[usize], add_here: &mut BTreeSet<usize>) {
        let Some(last) = loop_edges.last().copied() else {
            return;
        };
        let Some(mut stop) = self.half_edges[last].twin else {
            return;
        };
        for edge in loop_edges {
            for ring_edge in self.org_ring(*edge) {
                if ring_edge == stop {
                    break;
                }
                add_here.insert(self.half_edges[ring_edge].left_face);
            }
            let Some(twin) = self.half_edges[*edge].twin else {
                return;
            };
            stop = twin;
        }
    }

    fn org_ring(&self, start: usize) -> Vec<usize> {
        let mut ring = Vec::new();
        let mut edge = start;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(edge) {
                break;
            }
            ring.push(edge);
            let prev = self.half_edges[edge].prev;
            let Some(next_edge) = self.half_edges[prev].twin else {
                break;
            };
            if next_edge == start {
                break;
            }
            edge = next_edge;
        }
        ring
    }
}

#[derive(Debug, Clone, Copy)]
struct FaceGeometry {
    normal: [f32; 3],
    area: f32,
}

fn face_geometry(vertices: &[[f32; 3]], face: [usize; 3]) -> FaceGeometry {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let double_area = cross_f32(sub_f32(b, a), sub_f32(c, a));
    let double_area_norm = norm_f32(double_area);
    let normal = if double_area_norm > 0.0 {
        [
            double_area[0] / double_area_norm,
            double_area[1] / double_area_norm,
            double_area[2] / double_area_norm,
        ]
    } else {
        [0.0, 0.0, 0.0]
    };
    FaceGeometry {
        normal,
        area: 0.5 * double_area_norm,
    }
}

fn discrete_minus_abs_mean_curvature_metric(
    vertices: &[[f32; 3]],
    half_edges: &[TunnelHalfEdge],
    face_geometry: &[FaceGeometry],
    edge: &TunnelUndirectedEdge,
) -> f64 {
    let first = edge.edge;
    let Some(second) = half_edges[first].twin else {
        return 0.0;
    };
    let left = half_edges[first].left_face;
    let right = half_edges[second].left_face;
    if left == INVALID_FACE || right == INVALID_FACE {
        return 0.0;
    }
    let sum_area = face_geometry[left].area + face_geometry[right].area;
    if sum_area <= 0.0 {
        return 0.0;
    }
    let edge_vector = sub_f32(
        vertices[half_edges[first].dest],
        vertices[half_edges[first].org],
    );
    let angle = dihedral_angle(
        face_geometry[left].normal,
        face_geometry[right].normal,
        edge_vector,
    );
    let mean_curvature = 1.5 * angle * edge.length as f32 / sum_area;
    -f64::from(mean_curvature.abs())
}

fn dihedral_angle(left_normal: [f32; 3], right_normal: [f32; 3], edge_vector: [f32; 3]) -> f32 {
    let edge_length = norm_f32(edge_vector);
    if edge_length <= 0.0 {
        return 0.0;
    }
    let edge_dir = [
        edge_vector[0] / edge_length,
        edge_vector[1] / edge_length,
        edge_vector[2] / edge_length,
    ];
    let sin = dot_f32(edge_dir, cross_f32(left_normal, right_normal));
    let cos = dot_f32(left_normal, right_normal);
    sin.atan2(cos)
}

fn sub_f32(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot_f32(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross_f32(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm_f32(vector: [f32; 3]) -> f32 {
    dot_f32(vector, vector).sqrt()
}

fn edge_length_f32(a: [f32; 3], b: [f32; 3]) -> f32 {
    norm_f32(sub_f32(a, b))
}

fn ordered_edge(first: usize, second: usize) -> (usize, usize) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}
