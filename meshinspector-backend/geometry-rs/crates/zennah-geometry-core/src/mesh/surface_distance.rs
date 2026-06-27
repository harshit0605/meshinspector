use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::math::{distance_sq, dot, sub};

#[derive(Copy, Clone, Debug)]
struct QueueState {
    priority: f64,
    vertex: usize,
}

impl PartialEq for QueueState {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.vertex == other.vertex
    }
}

impl Eq for QueueState {}

impl Ord for QueueState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .total_cmp(&self.priority)
            .then_with(|| self.vertex.cmp(&other.vertex))
    }
}

impl PartialOrd for QueueState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct EdgeAdjacency {
    dest: usize,
    opposite_vertices: Vec<usize>,
}

pub(super) fn surface_distance_field_from_tri_point(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    seed_face_index: usize,
    seed_barycentric: [f64; 3],
    target_point: Option<[f64; 3]>,
    stop_vertices: Option<[usize; 3]>,
) -> (Vec<f64>, Vec<Option<usize>>, bool) {
    let adjacency = edge_adjacency(vertices.len(), faces);
    let mut distances = vec![f64::INFINITY; vertices.len()];
    let mut predecessors = vec![None; vertices.len()];
    let mut update_counts = vec![0_u8; vertices.len()];
    let mut heap = BinaryHeap::new();

    let seed_face = faces[seed_face_index];
    let seed_point = triangle_point(vertices, seed_face, seed_barycentric);
    for vertex in seed_face {
        suggest_vertex_distance(
            vertex,
            distance_sq(vertices[vertex], seed_point).sqrt(),
            None,
            vertices,
            target_point,
            &mut distances,
            &mut predecessors,
            &mut heap,
        );
    }

    let mut remaining = stop_vertices.map(|vertices| vertices.to_vec());
    while remaining
        .as_ref()
        .is_none_or(|vertices| !vertices.is_empty())
    {
        let Some(vertex) = grow_one(
            vertices,
            &adjacency,
            target_point,
            &mut distances,
            &mut predecessors,
            &mut update_counts,
            &mut heap,
        ) else {
            return (distances, predecessors, remaining.is_none());
        };
        if let Some(vertices) = &mut remaining {
            if let Some(index) = vertices.iter().position(|candidate| *candidate == vertex) {
                vertices.remove(index);
            }
        }
    }

    (distances, predecessors, true)
}

pub(super) fn surface_distance_field(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    seed_vertices: &[usize],
    max_distance_mm: f64,
) -> (Vec<f64>, Vec<Option<usize>>) {
    let adjacency = edge_adjacency(vertices.len(), faces);
    let mut distances = vec![f64::INFINITY; vertices.len()];
    let mut predecessors = vec![None; vertices.len()];
    let mut update_counts = vec![0_u8; vertices.len()];
    let mut heap = BinaryHeap::new();

    for seed in seed_vertices {
        if distances[*seed] > 0.0 {
            distances[*seed] = 0.0;
        }
    }
    for seed in seed_vertices {
        suggest_distances_around(
            *seed,
            vertices,
            &adjacency,
            None,
            &mut distances,
            &mut predecessors,
            &mut heap,
        );
    }

    while let Some(QueueState { priority, vertex }) = heap.pop() {
        if priority > queue_priority(distances[vertex], vertex, vertices, None) {
            continue;
        }
        if distances[vertex] >= max_distance_mm {
            break;
        }
        if update_counts[vertex] >= 3 {
            continue;
        }
        update_counts[vertex] += 1;
        suggest_distances_around(
            vertex,
            vertices,
            &adjacency,
            None,
            &mut distances,
            &mut predecessors,
            &mut heap,
        );
    }

    (distances, predecessors)
}

fn suggest_distances_around(
    vertex: usize,
    vertices: &[[f64; 3]],
    adjacency: &[Vec<EdgeAdjacency>],
    target_point: Option<[f64; 3]>,
    distances: &mut [f64],
    predecessors: &mut [Option<usize>],
    heap: &mut BinaryHeap<QueueState>,
) {
    let vertex_distance = distances[vertex];
    for edge in &adjacency[vertex] {
        let dest = edge.dest;
        let mut candidate = vertex_distance + distance_sq(vertices[vertex], vertices[dest]).sqrt();
        if candidate <= vertex_distance {
            candidate = f64::from_bits(vertex_distance.to_bits() + 1);
        }
        if !suggest_vertex_distance(
            dest,
            candidate,
            Some(vertex),
            vertices,
            target_point,
            distances,
            predecessors,
            heap,
        ) {
            for opposite in &edge.opposite_vertices {
                consider_triangle_path(
                    vertex,
                    dest,
                    *opposite,
                    vertices,
                    target_point,
                    distances,
                    predecessors,
                    heap,
                );
            }
        }
    }
}

fn consider_triangle_path(
    mut a: usize,
    mut b: usize,
    c: usize,
    vertices: &[[f64; 3]],
    target_point: Option<[f64; 3]>,
    distances: &mut [f64],
    predecessors: &mut [Option<usize>],
    heap: &mut BinaryHeap<QueueState>,
) {
    let mut va = distances[a];
    let mut vb = distances[b];
    if !va.is_finite() || !vb.is_finite() {
        return;
    }
    if vb < va {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut va, &mut vb);
    }

    let Some(delta) = field_at_c(
        sub(vertices[b], vertices[a]),
        sub(vertices[c], vertices[a]),
        vb - va,
    ) else {
        return;
    };
    let mut candidate = va + delta;
    if candidate <= va {
        candidate = f64::from_bits(va.to_bits() + 1);
    }
    let predecessor = if va <= vb { a } else { b };
    suggest_vertex_distance(
        c,
        candidate,
        Some(predecessor),
        vertices,
        target_point,
        distances,
        predecessors,
        heap,
    );
}

fn suggest_vertex_distance(
    vertex: usize,
    distance: f64,
    predecessor: Option<usize>,
    vertices: &[[f64; 3]],
    target_point: Option<[f64; 3]>,
    distances: &mut [f64],
    predecessors: &mut [Option<usize>],
    heap: &mut BinaryHeap<QueueState>,
) -> bool {
    if distances[vertex] <= distance {
        return false;
    }
    distances[vertex] = distance;
    predecessors[vertex] = predecessor;
    heap.push(QueueState {
        priority: queue_priority(distance, vertex, vertices, target_point),
        vertex,
    });
    true
}

fn grow_one(
    vertices: &[[f64; 3]],
    adjacency: &[Vec<EdgeAdjacency>],
    target_point: Option<[f64; 3]>,
    distances: &mut [f64],
    predecessors: &mut [Option<usize>],
    update_counts: &mut [u8],
    heap: &mut BinaryHeap<QueueState>,
) -> Option<usize> {
    while let Some(QueueState { priority, vertex }) = heap.pop() {
        if priority > queue_priority(distances[vertex], vertex, vertices, target_point) {
            continue;
        }
        if update_counts[vertex] >= 3 {
            continue;
        }
        update_counts[vertex] += 1;
        suggest_distances_around(
            vertex,
            vertices,
            adjacency,
            target_point,
            distances,
            predecessors,
            heap,
        );
        return Some(vertex);
    }
    None
}

fn queue_priority(
    distance: f64,
    vertex: usize,
    vertices: &[[f64; 3]],
    target_point: Option<[f64; 3]>,
) -> f64 {
    distance + target_point.map_or(0.0, |target| distance_sq(vertices[vertex], target).sqrt())
}

fn triangle_point(vertices: &[[f64; 3]], face: [usize; 3], barycentric: [f64; 3]) -> [f64; 3] {
    [
        vertices[face[0]][0] * barycentric[0]
            + vertices[face[1]][0] * barycentric[1]
            + vertices[face[2]][0] * barycentric[2],
        vertices[face[0]][1] * barycentric[0]
            + vertices[face[1]][1] * barycentric[1]
            + vertices[face[2]][1] * barycentric[2],
        vertices[face[0]][2] * barycentric[0]
            + vertices[face[1]][2] * barycentric[1]
            + vertices[face[2]][2] * barycentric[2],
    ]
}

fn field_at_c(b: [f64; 3], c: [f64; 3], vb: f64) -> Option<f64> {
    if vb < 0.0 {
        return None;
    }
    let dot_bc = dot(b, c);
    if dot_bc <= 0.0 {
        return None;
    }
    let blen_sq = dot(b, b);
    let vb_sq = vb * vb;
    if blen_sq <= vb_sq {
        return None;
    }
    let sqr_cos_n = vb_sq / blen_sq;
    let clen_sq = dot(c, c);
    let bc_sq = blen_sq * clen_sq;
    if bc_sq <= 0.0 {
        return None;
    }
    let mut sqr_cos_b0c = dot_bc * dot_bc / bc_sq;
    if sqr_cos_b0c <= sqr_cos_n {
        return None;
    }
    let a = sub(c, b);
    let dot_ba = dot(b, a);
    if dot_ba >= 0.0 {
        let alen_sq = dot(a, a);
        if dot_ba * dot_ba >= sqr_cos_n * blen_sq * alen_sq {
            return None;
        }
    }
    sqr_cos_b0c = sqr_cos_b0c.min(1.0);
    Some(
        clen_sq.sqrt()
            * ((sqr_cos_b0c * sqr_cos_n).sqrt() + ((1.0 - sqr_cos_b0c) * (1.0 - sqr_cos_n)).sqrt()),
    )
}

fn edge_adjacency(vertex_count: usize, faces: &[[usize; 3]]) -> Vec<Vec<EdgeAdjacency>> {
    let mut adjacency = vec![Vec::new(); vertex_count];
    for face in faces {
        for (a, b, c) in [
            (face[0], face[1], face[2]),
            (face[1], face[2], face[0]),
            (face[2], face[0], face[1]),
        ] {
            add_edge_adjacency(&mut adjacency, a, b, c);
            add_edge_adjacency(&mut adjacency, b, a, c);
        }
    }
    adjacency
}

fn add_edge_adjacency(
    adjacency: &mut [Vec<EdgeAdjacency>],
    origin: usize,
    dest: usize,
    opposite: usize,
) {
    if let Some(edge) = adjacency[origin].iter_mut().find(|edge| edge.dest == dest) {
        if !edge.opposite_vertices.contains(&opposite) {
            edge.opposite_vertices.push(opposite);
        }
        return;
    }
    adjacency[origin].push(EdgeAdjacency {
        dest,
        opposite_vertices: vec![opposite],
    });
}

#[cfg(test)]
mod tests {
    use super::surface_distance_field;

    #[test]
    fn triangle_front_update_matches_meshlib_surface_distance_builder() {
        let vertices = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 1.0, 0.0]];
        let faces = [[0, 1, 2]];

        let (distances, predecessors) =
            surface_distance_field(&vertices, &faces, &[0, 1], f64::MAX);

        assert_eq!(distances[0], 0.0);
        assert_eq!(distances[1], 0.0);
        assert!((distances[2] - 1.0).abs() < 1e-9);
        assert_eq!(predecessors[2], Some(0));
    }
}
