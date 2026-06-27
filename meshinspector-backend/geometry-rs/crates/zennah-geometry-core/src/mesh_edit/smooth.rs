use super::topology::ordered_edge;
use nalgebra::{DMatrix, DVector};
use std::collections::{BTreeMap, BTreeSet};

const COTAN_MIN: f64 = -1.0;
const COTAN_MAX: f64 = 10.0;

pub(super) fn smooth_vertices_cotan(
    vertices: &mut [[f64; 3]],
    faces: &[[usize; 3]],
    vertex_indices: &[usize],
    min_sharp_dihedral_angle: f64,
) {
    let smooth_vertices: BTreeSet<usize> = vertex_indices.iter().copied().collect();
    if smooth_vertices.is_empty() {
        return;
    }

    let sharp_vertices = if min_sharp_dihedral_angle < std::f64::consts::PI {
        find_sharp_vertices(vertices, faces, min_sharp_dihedral_angle)
    } else {
        BTreeSet::new()
    };
    let free_vertices: Vec<usize> = smooth_vertices
        .into_iter()
        .filter(|vertex| !sharp_vertices.contains(vertex))
        .collect();
    if free_vertices.is_empty() {
        return;
    }

    let topology = CotanTopology::from_mesh(vertices, faces);
    let free_set: BTreeSet<usize> = free_vertices.iter().copied().collect();
    let mut first_layer_fixed = BTreeSet::<usize>::new();
    for vertex in &free_vertices {
        for neighbor in topology.neighbors(*vertex) {
            if !free_set.contains(&neighbor) && !sharp_vertices.contains(&neighbor) {
                first_layer_fixed.insert(neighbor);
            }
        }
    }

    let mut free_to_column = BTreeMap::<usize, usize>::new();
    for (column, vertex) in free_vertices.iter().copied().enumerate() {
        free_to_column.insert(vertex, column);
    }

    let mut rows = Vec::<LaplacianRow>::new();
    for vertex in &free_vertices {
        if let Some(row) = build_laplacian_row(&topology, vertices, *vertex, &free_to_column, true)
        {
            rows.push(row);
        }
    }
    for vertex in first_layer_fixed {
        if let Some(row) = build_laplacian_row(&topology, vertices, vertex, &free_to_column, false)
        {
            rows.push(row);
        }
    }
    if rows.is_empty() {
        return;
    }

    let column_count = free_vertices.len();
    let mut matrix = DMatrix::<f64>::zeros(rows.len(), column_count);
    let mut rhs = [
        DVector::<f64>::zeros(rows.len()),
        DVector::<f64>::zeros(rows.len()),
        DVector::<f64>::zeros(rows.len()),
    ];
    for (row_index, row) in rows.iter().enumerate() {
        for (column, coeff) in &row.coeffs {
            matrix[(row_index, *column)] += *coeff;
        }
        for axis in 0..3 {
            rhs[axis][row_index] = row.rhs[axis];
        }
    }

    let normal_matrix = matrix.transpose() * &matrix;
    let Some(solutions) = solve_axes(&normal_matrix, &matrix, &rhs) else {
        return;
    };
    for (row, vertex) in free_vertices.iter().copied().enumerate() {
        vertices[vertex] = [solutions[0][row], solutions[1][row], solutions[2][row]];
    }
}

fn build_laplacian_row(
    topology: &CotanTopology,
    vertices: &[[f64; 3]],
    vertex: usize,
    free_to_column: &BTreeMap<usize, usize>,
    free_center: bool,
) -> Option<LaplacianRow> {
    let mut weighted_neighbors = Vec::<(usize, f64)>::new();
    let mut sum_weight = 0.0;
    for neighbor in topology.neighbors(vertex) {
        let weight = topology.weight(vertex, neighbor);
        weighted_neighbors.push((neighbor, weight));
        sum_weight += weight;
    }
    if sum_weight == 0.0 {
        return None;
    }

    let mut coeffs = BTreeMap::<usize, f64>::new();
    let mut rhs = [0.0; 3];
    let mut has_free_coeff = false;

    if free_center {
        let column = free_to_column[&vertex];
        coeffs.insert(column, 1.0);
        has_free_coeff = true;
    } else {
        subtract_scaled(&mut rhs, vertices[vertex], 1.0);
    }

    for (neighbor, weight) in weighted_neighbors {
        let coeff = -weight / sum_weight;
        if let Some(column) = free_to_column.get(&neighbor) {
            *coeffs.entry(*column).or_insert(0.0) += coeff;
            has_free_coeff = true;
        } else {
            subtract_scaled(&mut rhs, vertices[neighbor], coeff);
        }
    }

    has_free_coeff.then_some(LaplacianRow { coeffs, rhs })
}

fn solve_axes(
    normal_matrix: &DMatrix<f64>,
    matrix: &DMatrix<f64>,
    rhs: &[DVector<f64>; 3],
) -> Option<[DVector<f64>; 3]> {
    let normal_rhs = [
        matrix.transpose() * &rhs[0],
        matrix.transpose() * &rhs[1],
        matrix.transpose() * &rhs[2],
    ];
    let cholesky = normal_matrix.clone().cholesky();
    let solve = |values: &DVector<f64>| {
        if let Some(cholesky) = &cholesky {
            return Some(cholesky.solve(values));
        }
        normal_matrix.clone().lu().solve(values)
    };
    Some([
        solve(&normal_rhs[0])?,
        solve(&normal_rhs[1])?,
        solve(&normal_rhs[2])?,
    ])
}

fn find_sharp_vertices(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    min_sharp_dihedral_angle: f64,
) -> BTreeSet<usize> {
    let mut incident_faces = BTreeMap::<[usize; 2], Vec<usize>>::new();
    for (face_index, face) in faces.iter().copied().enumerate() {
        for edge in face_edges(face) {
            incident_faces.entry(edge).or_default().push(face_index);
        }
    }

    let mut sharp_vertices = BTreeSet::<usize>::new();
    let threshold = min_sharp_dihedral_angle.cos();
    for (edge, face_indices) in incident_faces {
        if face_indices.len() != 2 {
            continue;
        }
        let first = face_normal(vertices, faces[face_indices[0]]);
        let second = face_normal(vertices, faces[face_indices[1]]);
        let cosine = dot(first, second) / (length(first) * length(second));
        if cosine.is_finite() && cosine <= threshold {
            sharp_vertices.insert(edge[0]);
            sharp_vertices.insert(edge[1]);
        }
    }
    sharp_vertices
}

#[derive(Debug, Clone)]
struct CotanTopology {
    weights: BTreeMap<[usize; 2], f64>,
    neighbors: BTreeMap<usize, BTreeSet<usize>>,
}

impl CotanTopology {
    fn from_mesh(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> Self {
        let mut weights = BTreeMap::<[usize; 2], f64>::new();
        let mut neighbors = BTreeMap::<usize, BTreeSet<usize>>::new();
        for face in faces.iter().copied() {
            for [a, b, opposite] in opposite_angle_edges(face) {
                let edge = ordered_edge(a, b);
                let cotan = cotan_at_vertex(vertices[a], vertices[b], vertices[opposite]);
                *weights.entry(edge).or_insert(0.0) += cotan;
                neighbors.entry(a).or_default().insert(b);
                neighbors.entry(b).or_default().insert(a);
            }
        }
        for weight in weights.values_mut() {
            *weight = weight.clamp(COTAN_MIN, COTAN_MAX);
        }
        Self { weights, neighbors }
    }

    fn neighbors(&self, vertex: usize) -> Vec<usize> {
        self.neighbors
            .get(&vertex)
            .map(|values| values.iter().copied().collect())
            .unwrap_or_default()
    }

    fn weight(&self, a: usize, b: usize) -> f64 {
        self.weights
            .get(&ordered_edge(a, b))
            .copied()
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Clone)]
struct LaplacianRow {
    coeffs: BTreeMap<usize, f64>,
    rhs: [f64; 3],
}

fn face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [
        ordered_edge(face[0], face[1]),
        ordered_edge(face[1], face[2]),
        ordered_edge(face[2], face[0]),
    ]
}

fn opposite_angle_edges(face: [usize; 3]) -> [[usize; 3]; 3] {
    [
        [face[0], face[1], face[2]],
        [face[1], face[2], face[0]],
        [face[2], face[0], face[1]],
    ]
}

fn cotan_at_vertex(a: [f64; 3], b: [f64; 3], origin: [f64; 3]) -> f64 {
    let left = subtract(a, origin);
    let right = subtract(b, origin);
    let denominator = length(cross(left, right));
    if denominator == 0.0 {
        return 0.0;
    }
    dot(left, right) / denominator
}

fn face_normal(vertices: &[[f64; 3]], face: [usize; 3]) -> [f64; 3] {
    cross(
        subtract(vertices[face[1]], vertices[face[0]]),
        subtract(vertices[face[2]], vertices[face[0]]),
    )
}

fn subtract_scaled(target: &mut [f64; 3], value: [f64; 3], coeff: f64) {
    target[0] -= coeff * value[0];
    target[1] -= coeff * value[1];
    target[2] -= coeff * value[2];
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}
