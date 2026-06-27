use super::repetitions::point_cloud_local_triangulation_repetitions;

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudTriangulatedTopologyCandidateMesh {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub repetition_counts: [usize; 4],
    pub repeated_3_count: usize,
    pub repeated_2_count: usize,
    pub candidate_face_count: usize,
    pub topology_skipped_face_count: usize,
    pub topology_degenerate_face_count: usize,
    pub topology_nonmanifold_edge_face_count: usize,
    pub topology_nonmanifold_vertex_face_count: usize,
    pub topology_unsafe_retry_face_count: usize,
    pub removed_hole_complicating_face_count: usize,
    pub output_repeated_boundary_vertex_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct TopologyBuildCandidate {
    faces: Vec<[i64; 3]>,
    skipped_face_count: usize,
    reject_counts: TopologyRejectCounts,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct TopologyRejectCounts {
    degenerate_face_count: usize,
    nonmanifold_edge_face_count: usize,
    nonmanifold_vertex_face_count: usize,
    unsafe_retry_face_count: usize,
}

impl TopologyRejectCounts {
    fn add_result(&mut self, result: AddFaceResult) {
        match result {
            AddFaceResult::Success => {}
            AddFaceResult::UnsafeTryLater => self.unsafe_retry_face_count += 1,
            AddFaceResult::FailDegenerateFace => self.degenerate_face_count += 1,
            AddFaceResult::FailNonManifoldEdge => self.nonmanifold_edge_face_count += 1,
            AddFaceResult::FailNonManifoldVertex => self.nonmanifold_vertex_face_count += 1,
        }
    }
}

pub fn point_cloud_triangulate_topology_candidate_mesh(
    points: &[[f64; 3]],
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<PointCloudTriangulatedTopologyCandidateMesh, String> {
    let repetitions = point_cloud_local_triangulation_repetitions(
        points,
        radius,
        num_neighbors,
        boundary_angle,
        max_removes,
        crit_angle,
        normals,
        untrusted_indices,
    )?;
    let repeated_3_count = repetitions.repeated_3.len();
    let repeated_2_count = repetitions.repeated_2.len();
    let candidate_face_count = repeated_3_count + repeated_2_count;
    let topology = meshlib_two_phase_topology_faces(repetitions.repeated_3, repetitions.repeated_2);
    let cleaned = crate::remove_hole_complicating_faces(points, &topology.faces)
        .map_err(|error| error.to_string())?;

    Ok(PointCloudTriangulatedTopologyCandidateMesh {
        vertices: cleaned.vertices,
        faces: cleaned.faces,
        repetition_counts: repetitions.repetition_counts,
        repeated_3_count,
        repeated_2_count,
        candidate_face_count,
        topology_skipped_face_count: topology.skipped_face_count,
        topology_degenerate_face_count: topology.reject_counts.degenerate_face_count,
        topology_nonmanifold_edge_face_count: topology.reject_counts.nonmanifold_edge_face_count,
        topology_nonmanifold_vertex_face_count: topology
            .reject_counts
            .nonmanifold_vertex_face_count,
        topology_unsafe_retry_face_count: topology.reject_counts.unsafe_retry_face_count,
        removed_hole_complicating_face_count: cleaned.report.removed_face_count,
        output_repeated_boundary_vertex_count: cleaned.report.output_repeated_vertex_count,
    })
}

fn meshlib_two_phase_topology_faces(
    mut repeated_3: Vec<[i64; 3]>,
    mut repeated_2: Vec<[i64; 3]>,
) -> TopologyBuildCandidate {
    sort_meshlib_triangles(&mut repeated_3);
    sort_meshlib_triangles(&mut repeated_2);

    let mut topology = MeshBuilderTopology::default();
    let mut accepted = Vec::new();
    let first_pass = topology.add_triangles(repeated_3);
    accepted.extend(first_pass.accepted_faces);

    let mut second_pass_faces = first_pass.skipped_faces;
    second_pass_faces.extend(repeated_2);
    let second_pass = topology.add_triangles(second_pass_faces);
    accepted.extend(second_pass.accepted_faces);

    TopologyBuildCandidate {
        faces: accepted,
        skipped_face_count: second_pass.skipped_faces.len(),
        reject_counts: second_pass.reject_counts,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AddTrianglesPass {
    accepted_faces: Vec<[i64; 3]>,
    skipped_faces: Vec<[i64; 3]>,
    reject_counts: TopologyRejectCounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddFaceResult {
    Success,
    UnsafeTryLater,
    FailDegenerateFace,
    FailNonManifoldEdge,
    FailNonManifoldVertex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HalfEdge {
    org: Option<i64>,
    left: Option<usize>,
    next: usize,
    prev: usize,
}

#[derive(Debug, Default)]
struct MeshBuilderTopology {
    edges: Vec<HalfEdge>,
    face_count: usize,
}

impl MeshBuilderTopology {
    fn add_triangles(&mut self, faces: Vec<[i64; 3]>) -> AddTrianglesPass {
        let mut active = vec![true; faces.len()];
        let mut bad = vec![false; faces.len()];
        let mut last_result = vec![AddFaceResult::UnsafeTryLater; faces.len()];
        let mut accepted_faces = Vec::new();

        loop {
            let mut added_on_pass = 0_usize;
            for face_index in 0..faces.len() {
                if !active[face_index] {
                    continue;
                }
                let result = self.try_add_face(faces[face_index]);
                last_result[face_index] = result;
                match result {
                    AddFaceResult::Success => {
                        active[face_index] = false;
                        accepted_faces.push(faces[face_index]);
                        added_on_pass += 1;
                    }
                    AddFaceResult::UnsafeTryLater => {}
                    _ => {
                        active[face_index] = false;
                        bad[face_index] = true;
                    }
                }
            }
            if added_on_pass == 0 {
                break;
            }
        }

        let mut skipped_faces = Vec::new();
        let mut reject_counts = TopologyRejectCounts::default();
        for face_index in 0..faces.len() {
            if active[face_index] || bad[face_index] {
                skipped_faces.push(faces[face_index]);
                reject_counts.add_result(last_result[face_index]);
            }
        }

        AddTrianglesPass {
            accepted_faces,
            skipped_faces,
            reject_counts,
        }
    }

    fn try_add_face(&mut self, face: [i64; 3]) -> AddFaceResult {
        if face[0] == face[1] || face[0] == face[2] || face[1] == face[2] {
            return AddFaceResult::FailDegenerateFace;
        }

        let mut simple_vertices = [false; 3];
        let mut face_edges = [None; 3];
        let mut only_left_hole_edges = [None; 3];

        for index in 0..3 {
            let vertex = face[index];
            if !self.has_vert(vertex) {
                simple_vertices[index] = true;
                continue;
            }

            let next_index = (index + 1) % 3;
            if let Some(edge) = self.find_edge(face[index], face[next_index]) {
                if self.edges[edge].left.is_some() {
                    return AddFaceResult::FailNonManifoldEdge;
                }
                face_edges[index] = Some(edge);
                simple_vertices[index] = true;
                simple_vertices[next_index] = true;
            }
        }

        for index in 0..3 {
            let prev_index = (index + 2) % 3;
            if let (Some(edge), Some(prev_edge)) = (face_edges[index], face_edges[prev_index]) {
                if self.edges[edge].next != Self::sym(prev_edge) {
                    return AddFaceResult::FailNonManifoldVertex;
                }
            }
            if !simple_vertices[index] {
                only_left_hole_edges[index] = self.edge_with_org_and_only_left_hole(face[index]);
                if only_left_hole_edges[index].is_none() {
                    return AddFaceResult::UnsafeTryLater;
                }
            }
        }

        for edge in &mut face_edges {
            if edge.is_none() {
                *edge = Some(self.make_edge());
            }
        }

        for index in 0..3 {
            let prev_index = (index + 2) % 3;
            let edge = face_edges[index].expect("face edge should be present");
            let prev_sym = Self::sym(face_edges[prev_index].expect("face edge should be present"));

            if self.edges[edge].org == Some(face[index])
                && self.edges[prev_sym].org == Some(face[index])
            {
                debug_assert_eq!(self.edges[edge].next, prev_sym);
                continue;
            }

            if let Some(only_left_hole) = only_left_hole_edges[index] {
                if only_left_hole != edge {
                    self.splice(only_left_hole, edge);
                }
            }

            let previous = self.edges[prev_sym].prev;
            self.splice(edge, previous);
            self.set_org_ring(edge, face[index]);
        }

        let face_id = self.face_count;
        self.face_count += 1;
        self.set_left_face(face_edges[0].expect("face edge should be present"), face_id);
        AddFaceResult::Success
    }

    fn has_vert(&self, vertex: i64) -> bool {
        self.edge_with_org(vertex).is_some()
    }

    fn edge_with_org(&self, vertex: i64) -> Option<usize> {
        self.edges.iter().position(|edge| edge.org == Some(vertex))
    }

    fn edge_with_org_and_only_left_hole(&self, vertex: i64) -> Option<usize> {
        let start = self.edge_with_org(vertex)?;
        let mut current = start;
        let mut hole_edge = None;
        loop {
            if self.edges[current].left.is_none() {
                if hole_edge.is_some() {
                    return None;
                }
                hole_edge = Some(current);
            }
            current = self.edges[current].next;
            if current == start {
                break;
            }
        }
        hole_edge
    }

    fn find_edge(&self, org: i64, dest: i64) -> Option<usize> {
        let start = self.edge_with_org(org)?;
        let mut current = start;
        loop {
            if self.dest(current) == Some(dest) {
                return Some(current);
            }
            current = self.edges[current].next;
            if current == start {
                return None;
            }
        }
    }

    fn dest(&self, edge: usize) -> Option<i64> {
        self.edges[Self::sym(edge)].org
    }

    fn make_edge(&mut self) -> usize {
        let edge = self.edges.len();
        self.edges.push(HalfEdge {
            org: None,
            left: None,
            next: edge,
            prev: edge,
        });
        self.edges.push(HalfEdge {
            org: None,
            left: None,
            next: edge + 1,
            prev: edge + 1,
        });
        edge
    }

    fn splice(&mut self, left: usize, right: usize) {
        if left == right {
            return;
        }
        let left_next = self.edges[left].next;
        let right_next = self.edges[right].next;
        self.edges[left].next = right_next;
        self.edges[right_next].prev = left;
        self.edges[right].next = left_next;
        self.edges[left_next].prev = right;
    }

    fn set_org_ring(&mut self, edge: usize, org: i64) {
        let mut current = edge;
        loop {
            self.edges[current].org = Some(org);
            current = self.edges[current].next;
            if current == edge {
                break;
            }
        }
    }

    fn set_left_face(&mut self, edge: usize, face: usize) {
        let mut current = edge;
        loop {
            self.edges[current].left = Some(face);
            current = self.edges[Self::sym(current)].prev;
            if current == edge {
                break;
            }
        }
    }

    fn sym(edge: usize) -> usize {
        edge ^ 1
    }
}

fn sort_meshlib_triangles(triangles: &mut [[i64; 3]]) {
    triangles.sort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_stage_accepts_opposite_edge_faces_like_meshbuilder() {
        let topology = meshlib_two_phase_topology_faces(vec![[0, 1, 2]], vec![[1, 0, 3]]);

        assert_eq!(topology.faces, vec![[0, 1, 2], [1, 0, 3]]);
        assert_eq!(topology.skipped_face_count, 0);
    }

    #[test]
    fn topology_stage_rejects_left_occupied_directed_edges_like_meshbuilder() {
        let topology = meshlib_two_phase_topology_faces(vec![[0, 1, 2], [0, 1, 3]], Vec::new());

        assert_eq!(topology.faces, vec![[0, 1, 2]]);
        assert_eq!(topology.skipped_face_count, 1);
        assert_eq!(topology.reject_counts.nonmanifold_edge_face_count, 1);
    }

    #[test]
    fn topology_stage_reports_degenerate_faces_like_meshbuilder() {
        let topology = meshlib_two_phase_topology_faces(vec![[0, 1, 1]], Vec::new());

        assert!(topology.faces.is_empty());
        assert_eq!(topology.skipped_face_count, 1);
        assert_eq!(topology.reject_counts.degenerate_face_count, 1);
    }

    #[test]
    fn topology_stage_retries_failed_rep3_before_rep2_like_point_cloud_triangulator() {
        let topology =
            meshlib_two_phase_topology_faces(vec![[0, 1, 2], [0, 1, 3]], vec![[2, 1, 4]]);

        assert_eq!(topology.faces, vec![[0, 1, 2], [2, 1, 4]]);
        assert_eq!(topology.skipped_face_count, 1);
        assert_eq!(topology.reject_counts.nonmanifold_edge_face_count, 1);
    }

    #[test]
    fn topology_stage_retries_temporarily_unsafe_faces_like_meshbuilder() {
        let topology =
            meshlib_two_phase_topology_faces(vec![[0, 1, 2], [0, 3, 4], [0, 5, 6]], Vec::new());

        assert_eq!(topology.faces, vec![[0, 1, 2], [0, 3, 4]]);
        assert_eq!(topology.skipped_face_count, 1);
        assert_eq!(topology.reject_counts.unsafe_retry_face_count, 1);
    }

    #[test]
    fn point_cloud_triangulate_topology_candidate_mesh_keeps_simple_rep3_face() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let mesh = point_cloud_triangulate_topology_candidate_mesh(
            &points,
            1.5,
            0,
            3.0,
            usize::MAX,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("topology candidate mesh should build");

        assert_eq!(mesh.vertices, points);
        assert_eq!(mesh.faces, vec![[0, 1, 2]]);
        assert_eq!(mesh.candidate_face_count, 1);
        assert_eq!(mesh.topology_skipped_face_count, 0);
        assert_eq!(mesh.topology_degenerate_face_count, 0);
        assert_eq!(mesh.topology_nonmanifold_edge_face_count, 0);
        assert_eq!(mesh.topology_nonmanifold_vertex_face_count, 0);
        assert_eq!(mesh.topology_unsafe_retry_face_count, 0);
        assert_eq!(mesh.removed_hole_complicating_face_count, 0);
    }
}
