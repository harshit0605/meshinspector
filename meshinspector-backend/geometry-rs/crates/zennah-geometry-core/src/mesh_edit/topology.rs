use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EdgeRecord {
    pub rank: usize,
    pub org: usize,
    pub dest: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EdgeState {
    records: BTreeMap<[usize; 2], EdgeRecord>,
    next_rank: usize,
}

impl EdgeState {
    pub(super) fn from_faces(faces: &[[usize; 3]]) -> Self {
        EdgeRankBuilder::from_faces(faces).into_edge_state()
    }

    pub(super) fn record(&self, edge: [usize; 2]) -> EdgeRecord {
        self.records.get(&edge).copied().unwrap_or(EdgeRecord {
            rank: usize::MAX,
            org: edge[0],
            dest: edge[1],
        })
    }

    /// Whether the (ordered) edge currently exists. Equivalent to scanning the
    /// faces for the edge (`mesh_has_edge`) but O(log E) instead of O(F): the
    /// edge set is maintained incrementally through splits and flips.
    pub(super) fn contains(&self, edge: [usize; 2]) -> bool {
        self.records.contains_key(&edge)
    }

    /// Iterate the live edge set with each edge's rank. The set is maintained
    /// incrementally across splits and Delone flips, so this is the same edge
    /// set `edge_incident_faces` would rebuild from the faces — but without the
    /// per-iteration rebuild. Used by the subdivide fast candidate scan.
    pub(super) fn iter_edges(&self) -> impl Iterator<Item = ([usize; 2], usize)> + '_ {
        self.records
            .iter()
            .map(|(edge, record)| (*edge, record.rank))
    }

    pub(super) fn split_edge(
        &mut self,
        edge: [usize; 2],
        new_vertex: usize,
        connector_vertices: &[usize],
    ) {
        let record = self.record(edge);
        self.records.remove(&edge);
        self.records.insert(
            ordered_edge(new_vertex, record.dest),
            EdgeRecord {
                rank: record.rank,
                org: new_vertex,
                dest: record.dest,
            },
        );
        self.insert_new(record.org, new_vertex);
        for connector_vertex in connector_vertices {
            self.insert_new(new_vertex, *connector_vertex);
        }
    }

    pub(super) fn flip_edge(&mut self, old_edge: [usize; 2], new_org: usize, new_dest: usize) {
        let record = self.record(old_edge);
        self.records.remove(&old_edge);
        self.records.insert(
            ordered_edge(new_org, new_dest),
            EdgeRecord {
                rank: record.rank,
                org: new_org,
                dest: new_dest,
            },
        );
    }

    fn insert_new(&mut self, org: usize, dest: usize) {
        self.records.insert(
            ordered_edge(org, dest),
            EdgeRecord {
                rank: self.next_rank,
                org,
                dest,
            },
        );
        self.next_rank += 1;
    }
}

/// Vertex → incident face indices, kept sorted ascending and maintained
/// incrementally as faces are split and flipped. Lets subdivision look up the
/// faces on an edge or around a vertex in O(degree) instead of rescanning every
/// face (or rebuilding the whole edge→faces map) on every split — the work that
/// made subdivision quadratic in mesh size. Ascending order matches the
/// face-index iteration order of the original full scans, so the chosen split
/// and flip edges — and therefore the output — are unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VertexFaces {
    faces_of: Vec<Vec<usize>>,
}

impl VertexFaces {
    pub(super) fn from_faces(faces: &[[usize; 3]], vertex_count: usize) -> Self {
        let mut faces_of = vec![Vec::new(); vertex_count];
        for (face_index, face) in faces.iter().enumerate() {
            for vertex in face {
                faces_of[*vertex].push(face_index);
            }
        }
        Self { faces_of }
    }

    pub(super) fn add_vertex(&mut self) {
        self.faces_of.push(Vec::new());
    }

    pub(super) fn faces_around(&self, vertex: usize) -> &[usize] {
        &self.faces_of[vertex]
    }

    /// Faces incident to the edge `(a, b)` (ascending face index) — the sorted
    /// intersection of the two endpoints' face lists.
    pub(super) fn edge_faces(&self, a: usize, b: usize) -> Vec<usize> {
        let left = &self.faces_of[a];
        let right = &self.faces_of[b];
        let mut shared = Vec::new();
        let (mut i, mut j) = (0, 0);
        while i < left.len() && j < right.len() {
            match left[i].cmp(&right[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    shared.push(left[i]);
                    i += 1;
                    j += 1;
                }
            }
        }
        shared
    }

    pub(super) fn add_face(&mut self, face_index: usize, tri: [usize; 3]) {
        for vertex in tri {
            insert_sorted(&mut self.faces_of[vertex], face_index);
        }
    }

    /// Update the adjacency when `face_index` changes from `old` to `new`.
    pub(super) fn replace_face(&mut self, face_index: usize, old: [usize; 3], new: [usize; 3]) {
        for vertex in old {
            if !new.contains(&vertex) {
                remove_sorted(&mut self.faces_of[vertex], face_index);
            }
        }
        for vertex in new {
            if !old.contains(&vertex) {
                insert_sorted(&mut self.faces_of[vertex], face_index);
            }
        }
    }

    /// Debug-only invariant: the adjacency matches a fresh rebuild from `faces`.
    #[cfg(debug_assertions)]
    pub(super) fn matches_faces(&self, faces: &[[usize; 3]]) -> bool {
        *self == VertexFaces::from_faces(faces, self.faces_of.len())
    }
}

fn insert_sorted(values: &mut Vec<usize>, value: usize) {
    if let Err(position) = values.binary_search(&value) {
        values.insert(position, value);
    }
}

fn remove_sorted(values: &mut Vec<usize>, value: usize) {
    if let Ok(position) = values.binary_search(&value) {
        values.remove(position);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddRankFaceResult {
    Success,
    UnsafeTryLater,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EdgeRankBuilder {
    records: BTreeMap<[usize; 2], EdgeRecord>,
    directed_left: BTreeMap<(usize, usize), bool>,
    open_left_counts: BTreeMap<usize, usize>,
    vertices: BTreeSet<usize>,
    next_rank: usize,
}

impl EdgeRankBuilder {
    fn from_faces(faces: &[[usize; 3]]) -> Self {
        let mut builder = Self {
            records: BTreeMap::new(),
            directed_left: BTreeMap::new(),
            open_left_counts: BTreeMap::new(),
            vertices: BTreeSet::new(),
            next_rank: 0,
        };
        let mut active: BTreeSet<usize> = (0..faces.len()).collect();
        loop {
            let mut added_on_this_pass = 0;
            for face_index in active.iter().copied().collect::<Vec<_>>() {
                match builder.add_face(faces[face_index]) {
                    AddRankFaceResult::Success => {
                        active.remove(&face_index);
                        added_on_this_pass += 1;
                    }
                    AddRankFaceResult::Fail => {
                        active.remove(&face_index);
                    }
                    AddRankFaceResult::UnsafeTryLater => {}
                }
            }
            if active.is_empty() {
                break;
            }
            if added_on_this_pass == 0 {
                for face_index in active.iter().copied().collect::<Vec<_>>() {
                    builder.add_face_lenient(faces[face_index]);
                    active.remove(&face_index);
                }
                break;
            }
        }
        builder
    }

    fn into_edge_state(self) -> EdgeState {
        EdgeState {
            records: self.records,
            next_rank: self.next_rank,
        }
    }

    fn add_face(&mut self, face: [usize; 3]) -> AddRankFaceResult {
        if face[0] == face[1] || face[1] == face[2] || face[2] == face[0] {
            return AddRankFaceResult::Fail;
        }

        let edges = oriented_face_edges(face);
        let mut reusable = [false; 3];
        let mut simple = [false; 3];
        for (index, [org, dest]) in edges.iter().copied().enumerate() {
            if !self.vertices.contains(&org) {
                simple[index] = true;
                continue;
            }
            if self.has_no_left_edge(org, dest) {
                reusable[index] = true;
                simple[index] = true;
                simple[(index + 1) % 3] = true;
            }
        }

        for (index, [org, _]) in edges.iter().copied().enumerate() {
            let previous = (index + 2) % 3;
            if reusable[index]
                && reusable[previous]
                && !self
                    .directed_left
                    .contains_key(&(org, face[(index + 2) % 3]))
            {
                return AddRankFaceResult::UnsafeTryLater;
            }
            if !simple[index] && !self.has_one_left_hole(org) {
                return AddRankFaceResult::UnsafeTryLater;
            }
        }

        self.add_face_lenient(face);
        AddRankFaceResult::Success
    }

    fn add_face_lenient(&mut self, face: [usize; 3]) {
        for [org, dest] in oriented_face_edges(face) {
            if !self.directed_left.contains_key(&(org, dest)) {
                self.create_edge(org, dest);
            }
        }
        for [org, dest] in oriented_face_edges(face) {
            self.mark_directed_left_filled(org, dest);
            self.vertices.insert(org);
            self.vertices.insert(dest);
        }
    }

    fn create_edge(&mut self, org: usize, dest: usize) {
        self.directed_left.insert((org, dest), false);
        self.directed_left.insert((dest, org), false);
        self.increment_open_left(org);
        self.increment_open_left(dest);
        self.records.insert(
            ordered_edge(org, dest),
            EdgeRecord {
                rank: self.next_rank,
                org,
                dest,
            },
        );
        self.next_rank += 1;
    }

    fn mark_directed_left_filled(&mut self, org: usize, dest: usize) {
        if self.directed_left.insert((org, dest), true) == Some(false) {
            self.decrement_open_left(org);
        }
    }

    fn increment_open_left(&mut self, vertex: usize) {
        *self.open_left_counts.entry(vertex).or_insert(0) += 1;
    }

    fn decrement_open_left(&mut self, vertex: usize) {
        let Some(count) = self.open_left_counts.get_mut(&vertex) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count == 0 {
            self.open_left_counts.remove(&vertex);
        }
    }

    fn has_no_left_edge(&self, org: usize, dest: usize) -> bool {
        self.directed_left
            .get(&(org, dest))
            .copied()
            .map(|left| !left)
            .unwrap_or(false)
    }

    fn has_one_left_hole(&self, vertex: usize) -> bool {
        self.open_left_counts.get(&vertex).copied().unwrap_or(0) == 1
    }
}

pub(super) fn face_has_oriented_edge(face: [usize; 3], org: usize, dest: usize) -> bool {
    oriented_face_edges(face)
        .into_iter()
        .any(|edge| edge == [org, dest])
}

pub(super) fn face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [
        ordered_edge(face[0], face[1]),
        ordered_edge(face[1], face[2]),
        ordered_edge(face[2], face[0]),
    ]
}

pub(super) fn opposite_edge(face: [usize; 3], vertex: usize) -> Option<[usize; 2]> {
    if face[0] == vertex {
        Some(ordered_edge(face[1], face[2]))
    } else if face[1] == vertex {
        Some(ordered_edge(face[2], face[0]))
    } else if face[2] == vertex {
        Some(ordered_edge(face[0], face[1]))
    } else {
        None
    }
}

pub(super) fn opposite_vertex(face: [usize; 3], edge: [usize; 2]) -> Option<usize> {
    face.into_iter()
        .find(|vertex| *vertex != edge[0] && *vertex != edge[1])
}

pub(super) fn oriented_edge_with_opposite(
    face: [usize; 3],
    edge: [usize; 2],
) -> Option<(usize, usize, usize)> {
    for start in 0..3 {
        let end = (start + 1) % 3;
        let remaining = (start + 2) % 3;
        let a = face[start];
        let b = face[end];
        if same_undirected_edge([a, b], edge) {
            return Some((a, b, face[remaining]));
        }
    }
    None
}

pub(super) fn split_face_on_edge(
    face: [usize; 3],
    edge: EdgeRecord,
    new_vertex: usize,
) -> ([usize; 3], [usize; 3]) {
    if let Some(opposite) = opposite_vertex(face, ordered_edge(edge.org, edge.dest)) {
        if face_has_oriented_edge(face, edge.org, edge.dest) {
            return (
                [new_vertex, edge.dest, opposite],
                [new_vertex, opposite, edge.org],
            );
        }
        if face_has_oriented_edge(face, edge.dest, edge.org) {
            return (
                [edge.dest, new_vertex, opposite],
                [opposite, new_vertex, edge.org],
            );
        }
    }
    unreachable!("face must contain split edge")
}

pub(super) fn oriented_face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
}

pub(super) fn ordered_edge(a: usize, b: usize) -> [usize; 2] {
    if a <= b {
        [a, b]
    } else {
        [b, a]
    }
}

fn same_undirected_edge(a: [usize; 2], b: [usize; 2]) -> bool {
    ordered_edge(a[0], a[1]) == ordered_edge(b[0], b[1])
}
