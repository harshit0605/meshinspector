use crate::MeshArrays;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimateMeshStrategy {
    MinimizeError,
    ShortestEdgeFirst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecimateMeshOptions {
    pub strategy: DecimateMeshStrategy,
    pub max_error: f64,
    pub max_edge_len: f64,
    pub max_bd_shift: f64,
    pub stabilizer: f64,
    pub target_face_count: Option<usize>,
    pub target_face_ratio: Option<f64>,
    pub subdivide_parts: usize,
    pub decimate_between_parts: bool,
    pub max_deleted_vertices: usize,
    pub max_deleted_faces: usize,
    pub max_triangle_aspect_ratio: f64,
    pub critical_tri_aspect_ratio: f64,
    pub tiny_edge_length: f64,
    pub max_angle_change: f64,
    pub not_flippable_edges: Vec<[usize; 2]>,
    pub edges_to_collapse: Option<Vec<[usize; 2]>>,
    pub twin_map: Vec<[[usize; 2]; 2]>,
    pub collapse_near_not_flippable: bool,
    pub angle_weighted_dist_to_plane: bool,
    pub touch_near_bd_edges: bool,
    pub touch_bd_verts: bool,
    pub optimize_vertex_pos: bool,
    pub pack_mesh: bool,
    pub vertex_uvs: Option<Vec<[f64; 2]>>,
    pub vertex_colors: Option<Vec<[u8; 4]>>,
    pub region_faces: Option<Vec<usize>>,
}

impl Default for DecimateMeshOptions {
    fn default() -> Self {
        Self {
            strategy: DecimateMeshStrategy::MinimizeError,
            max_error: f64::MAX,
            max_edge_len: f64::MAX,
            max_bd_shift: f64::MAX,
            stabilizer: 0.001,
            target_face_count: None,
            target_face_ratio: None,
            subdivide_parts: 1,
            decimate_between_parts: true,
            max_deleted_vertices: usize::MAX,
            max_deleted_faces: usize::MAX,
            max_triangle_aspect_ratio: 20.0,
            critical_tri_aspect_ratio: f64::MAX,
            tiny_edge_length: -1.0,
            max_angle_change: -1.0,
            not_flippable_edges: Vec::new(),
            edges_to_collapse: None,
            twin_map: Vec::new(),
            collapse_near_not_flippable: false,
            angle_weighted_dist_to_plane: false,
            touch_near_bd_edges: true,
            touch_bd_verts: true,
            optimize_vertex_pos: true,
            pack_mesh: false,
            vertex_uvs: None,
            vertex_colors: None,
            region_faces: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecimateMeshResult {
    pub mesh: MeshArrays,
    pub verts_deleted: usize,
    pub faces_deleted: usize,
    pub error_introduced: f64,
    pub cancelled: bool,
    pub not_flippable_edges: Vec<[usize; 2]>,
    pub edges_to_collapse: Vec<[usize; 2]>,
    pub twin_map: Vec<[[usize; 2]; 2]>,
    pub vertex_uvs: Option<Vec<[f64; 2]>>,
    pub vertex_colors: Option<Vec<[u8; 4]>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EdgeCandidate {
    pub(super) edge: [usize; 2],
    pub(super) length_sq: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct QemCandidate {
    pub(super) edge: [usize; 2],
    pub(super) error_sq: f64,
    pub(super) collapse_pos: [f64; 3],
    pub(super) collapse_form: QuadraticForm,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CollapsePlan {
    pub(super) vertices: Vec<[f64; 3]>,
    pub(super) faces: Vec<[usize; 3]>,
    pub(super) candidate_region: Vec<bool>,
    pub(super) tracked_region: Vec<bool>,
    pub(super) faces_deleted: usize,
    pub(super) kept_vertex: usize,
    pub(super) dropped_vertex: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DecimateMeshState {
    pub(super) vertices: Vec<[f64; 3]>,
    pub(super) faces: Vec<[usize; 3]>,
    pub(super) region: Vec<bool>,
    pub(super) verts_deleted: usize,
    pub(super) faces_deleted: usize,
    pub(super) error_introduced: f64,
    pub(super) not_flippable_edges: BTreeSet<[usize; 2]>,
    pub(super) edges_to_collapse: Option<BTreeSet<[usize; 2]>>,
    pub(super) twin_map: BTreeMap<[usize; 2], [usize; 2]>,
    pub(super) vertex_uvs: Option<Vec<[f64; 2]>>,
    pub(super) vertex_colors: Option<Vec<[u8; 4]>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct QuadraticForm {
    pub(super) a: [[f64; 3]; 3],
    pub(super) c: f64,
}

impl QuadraticForm {
    pub(super) fn zero() -> Self {
        Self {
            a: [[0.0; 3]; 3],
            c: 0.0,
        }
    }

    pub(super) fn eval(self, point: [f64; 3]) -> f64 {
        dot(point, mat_vec(self.a, point)) + self.c
    }

    pub(super) fn add_dist_to_origin(&mut self, weight: f64) {
        for index in 0..3 {
            self.a[index][index] += weight;
        }
    }

    pub(super) fn add_dist_to_plane(&mut self, normal: [f64; 3], weight: f64) {
        add_scaled_outer_square(&mut self.a, normal, weight);
    }

    pub(super) fn add_dist_to_line(&mut self, direction: [f64; 3]) {
        let len_sq = distance_sq(direction, [0.0, 0.0, 0.0]);
        if len_sq <= 1e-24 {
            self.add_dist_to_origin(1.0);
            return;
        }
        let unit = scale(direction, 1.0 / len_sq.sqrt());
        for index in 0..3 {
            self.a[index][index] += 1.0;
        }
        add_scaled_outer_square(&mut self.a, unit, -1.0);
    }

    pub(super) fn add(self, other: Self) -> Self {
        let mut sum = Self {
            a: self.a,
            c: self.c + other.c,
        };
        for row in 0..3 {
            for col in 0..3 {
                sum.a[row][col] += other.a[row][col];
            }
        }
        sum
    }
}

fn add_scaled_outer_square(matrix: &mut [[f64; 3]; 3], vector: [f64; 3], scale: f64) {
    for row in 0..3 {
        for col in 0..3 {
            matrix[row][col] += scale * vector[row] * vector[col];
        }
    }
}

fn distance_sq(left: [f64; 3], right: [f64; 3]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    dx * dx + dy * dy + dz * dz
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn mat_vec(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        dot(matrix[0], vector),
        dot(matrix[1], vector),
        dot(matrix[2], vector),
    ]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}
