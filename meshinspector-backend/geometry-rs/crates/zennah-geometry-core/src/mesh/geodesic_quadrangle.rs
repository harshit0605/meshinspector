use crate::math::{add, cross, distance_sq, dot, norm, scale, sub};
use crate::types::GeometryError;

use super::base::validate_faces;
use super::geodesic::mesh_geodesic_path;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeodesicQuadranglePath {
    pub start_vertex: usize,
    pub end_vertex: usize,
    pub start_face_index: usize,
    pub end_face_index: usize,
    pub shared_edge: [usize; 2],
    pub crossing_t: f64,
    pub crossing_point: [f64; 3],
    pub points: Vec<[f64; 3]>,
    pub edge_lengths: Vec<f64>,
    pub length_mm: f64,
    pub graph_vertex_indices: Vec<usize>,
    pub graph_length_mm: f64,
    pub unfolded_quadrangle_convex: bool,
    pub meshlib_reference: &'static str,
}

pub fn mesh_geodesic_quadrangle_path(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    start_vertex: usize,
    end_vertex: usize,
) -> Result<MeshGeodesicQuadranglePath, GeometryError> {
    validate_vertex_id("start_vertex", start_vertex, vertices.len())?;
    validate_vertex_id("end_vertex", end_vertex, vertices.len())?;
    if start_vertex == end_vertex {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "surface_path",
            value: "start_end_same_vertex".to_string(),
        });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    let Some((start_face_index, end_face_index, shared_edge)) =
        adjacent_quadrangle_faces(&faces, start_vertex, end_vertex)
    else {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "surface_path",
            value: "requires_two_adjacent_triangles_with_opposite_start_end_vertices".to_string(),
        });
    };

    let crossing_t = shortest_path_in_quadrangle(
        vertices[shared_edge[0]],
        vertices[start_vertex],
        vertices[shared_edge[1]],
        vertices[end_vertex],
    );
    let crossing_point = add(
        scale(vertices[shared_edge[0]], 1.0 - crossing_t),
        scale(vertices[shared_edge[1]], crossing_t),
    );
    let start_to_crossing = distance_sq(vertices[start_vertex], crossing_point).sqrt();
    let crossing_to_end = distance_sq(crossing_point, vertices[end_vertex]).sqrt();
    let graph = mesh_geodesic_path(vertices, faces_i64, start_vertex, end_vertex, f64::INFINITY)?;
    Ok(MeshGeodesicQuadranglePath {
        start_vertex,
        end_vertex,
        start_face_index,
        end_face_index,
        shared_edge,
        crossing_t,
        crossing_point,
        points: vec![vertices[start_vertex], crossing_point, vertices[end_vertex]],
        edge_lengths: vec![start_to_crossing, crossing_to_end],
        length_mm: start_to_crossing + crossing_to_end,
        graph_vertex_indices: graph.vertex_indices,
        graph_length_mm: graph.length_mm,
        unfolded_quadrangle_convex: crossing_t > 0.0 && crossing_t < 1.0,
        meshlib_reference: "MR::shortestPathInQuadrangle / MR::reducePath",
    })
}

fn validate_vertex_id(
    field: &'static str,
    vertex: usize,
    vertex_count: usize,
) -> Result<(), GeometryError> {
    if vertex >= vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("{vertex} for {vertex_count} vertices"),
        });
    }
    Ok(())
}

fn adjacent_quadrangle_faces(
    faces: &[[usize; 3]],
    start_vertex: usize,
    end_vertex: usize,
) -> Option<(usize, usize, [usize; 2])> {
    for (start_face_index, start_face) in faces.iter().enumerate() {
        if !start_face.contains(&start_vertex) || start_face.contains(&end_vertex) {
            continue;
        }
        for (end_face_index, end_face) in faces.iter().enumerate() {
            if start_face_index == end_face_index
                || !end_face.contains(&end_vertex)
                || end_face.contains(&start_vertex)
            {
                continue;
            }
            let shared = start_face
                .iter()
                .copied()
                .filter(|vertex| end_face.contains(vertex))
                .collect::<Vec<_>>();
            if shared.len() == 2 {
                return Some((start_face_index, end_face_index, [shared[0], shared[1]]));
            }
        }
    }
    None
}

fn shortest_path_in_quadrangle(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let vec_b = sub(b, a);
    let vec_c = sub(c, a);
    let vec_d = sub(d, a);
    let unfold_b = [norm(vec_b), 0.0];
    let unfold_c = unfold_on_plane(vec_b, vec_c, unfold_b, true);
    let unfold_d = unfold_on_plane(vec_c, vec_d, unfold_c, true);
    line_intersection(unfold_c, unfold_b, unfold_d).clamp(0.0, 1.0)
}

fn unfold_on_plane(b: [f64; 3], c: [f64; 3], d: [f64; 2], to_left_from_origin: bool) -> [f64; 2] {
    let dot_bc = dot(b, c);
    let cross_bc = norm(cross(b, c));
    let dd = dot2(d, d);
    if dd <= 0.0 {
        return [0.0, 0.0];
    }
    let orthogonal = if to_left_from_origin {
        [-d[1], d[0]]
    } else {
        [d[1], -d[0]]
    };
    [
        (dot_bc * d[0] + cross_bc * orthogonal[0]) / dd,
        (dot_bc * d[1] + cross_bc * orthogonal[1]) / dd,
    ]
}

fn line_intersection(b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> f64 {
    let c1 = cross2(d, c);
    let c2 = cross2(sub2(c, b), sub2(d, b));
    let denominator = c1 + c2;
    if denominator == 0.0 {
        return 0.0;
    }
    c1 / denominator
}

fn sub2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
