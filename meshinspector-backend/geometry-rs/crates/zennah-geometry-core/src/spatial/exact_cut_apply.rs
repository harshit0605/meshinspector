use super::exact_cut::{
    exact_cut_preplan, ExactCutPathSegment, ExactCutPreplan, ExactCutPrimitive,
};
use super::exact_face_split::{
    split_triangle_with_boundary_segments, split_triangle_with_interior_cycle,
    split_triangle_with_interior_segment, split_triangle_with_interior_spokes,
};
use super::exact_one_mesh::ExactOneMeshContour;
use crate::math::{cross, dot, norm, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::BTreeSet;

mod paths;
mod polygon;

use self::paths::{
    cut_edge_paths_from_preplan, directed_path_is_closed, segment_lies_on_shared_boundary_edge,
};
use self::polygon::{boundary_path, dedupe_closed_polygon};

#[derive(Debug, Clone, PartialEq)]
pub struct ExactCutMeshResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub cut_edges: Vec<[usize; 2]>,
    pub cut_edge_paths: Vec<Vec<[usize; 2]>>,
    pub cut_edge_path_closed: Vec<bool>,
    pub source_face_for_faces: Vec<usize>,
    pub skipped_source_faces: Vec<usize>,
}

pub fn exact_cut_mesh_by_contours(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    contours: &[ExactOneMeshContour],
    epsilon: f64,
) -> Result<ExactCutMeshResult, GeometryError> {
    let preplan = exact_cut_preplan(vertices, faces_i64, contours, epsilon)?;
    exact_cut_mesh_from_preplan(vertices, faces_i64, &preplan, epsilon)
}

pub fn exact_cut_mesh_from_preplan(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    preplan: &ExactCutPreplan,
    epsilon: f64,
) -> Result<ExactCutMeshResult, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut segments_by_face = vec![Vec::new(); faces.len()];
    for (segment_index, segment) in preplan.path_segments.iter().enumerate() {
        if segment.source_faces.len() == 1 || segment_lies_on_shared_boundary_edge(segment, preplan)
        {
            for face in &segment.source_faces {
                let face = *face;
                if face < segments_by_face.len() {
                    segments_by_face[face].push(segment_index);
                }
            }
        }
    }

    let mut output = CutMeshBuilder {
        vertices: preplan.vertices_after_preplan.clone(),
        faces: Vec::with_capacity(faces.len()),
        cut_edges: Vec::new(),
        source_face_for_faces: Vec::with_capacity(faces.len()),
        skipped_source_faces: BTreeSet::new(),
        epsilon: effective_epsilon(epsilon),
    };

    for (face_index, face) in faces.iter().copied().enumerate() {
        let segment_indices = &segments_by_face[face_index];
        if segment_indices.is_empty() {
            output.push_face(face, face_index);
            continue;
        }
        if segment_indices.len() == 1 {
            let segment = &preplan.path_segments[segment_indices[0]];
            if output.try_split_face(face, face_index, segment, preplan) {
                continue;
            }
        } else if output.try_split_multi_segment_face(face, face_index, segment_indices, preplan) {
            continue;
        }
        {
            output.skipped_source_faces.insert(face_index);
            output.push_face(face, face_index);
        }
    }

    let mut result = output.finish();
    result.cut_edge_paths = cut_edge_paths_from_preplan(preplan, &result.cut_edges);
    result.cut_edge_path_closed = result
        .cut_edge_paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            preplan
                .contour_closed
                .get(index)
                .copied()
                .unwrap_or_default()
                && directed_path_is_closed(path)
        })
        .collect();
    Ok(result)
}

struct CutMeshBuilder {
    vertices: Vec<[f64; 3]>,
    faces: Vec<[i64; 3]>,
    cut_edges: Vec<[usize; 2]>,
    source_face_for_faces: Vec<usize>,
    skipped_source_faces: BTreeSet<usize>,
    epsilon: f64,
}

impl CutMeshBuilder {
    fn try_split_face(
        &mut self,
        face: [usize; 3],
        face_index: usize,
        segment: &ExactCutPathSegment,
        preplan: &ExactCutPreplan,
    ) -> bool {
        let from = &preplan.cut_points[segment.from_point];
        let to = &preplan.cut_points[segment.to_point];
        if from.vertex_index == to.vertex_index {
            return false;
        }

        let Some(from_pos) = face_point_position(
            face,
            face_index,
            from.primitive,
            from.coordinate,
            &self.vertices,
        ) else {
            return false;
        };
        let Some(to_pos) = face_point_position(
            face,
            face_index,
            to.primitive,
            to.coordinate,
            &self.vertices,
        ) else {
            return false;
        };

        match (from_pos, to_pos) {
            (FacePointPosition::Boundary(from_pos), FacePointPosition::Boundary(to_pos)) => {
                if nearly_equal(from_pos, to_pos, self.epsilon) {
                    return false;
                }

                self.push_cut_edge([from.vertex_index, to.vertex_index]);
                let first_path =
                    boundary_path(face, from.vertex_index, from_pos, to.vertex_index, to_pos);
                let second_path =
                    boundary_path(face, to.vertex_index, to_pos, from.vertex_index, from_pos);
                let mut pushed_any = false;
                pushed_any |= self.push_polygon_fan(&first_path, face_index);
                pushed_any |= self.push_polygon_fan(&second_path, face_index);
                pushed_any
            }
            (FacePointPosition::Interior, FacePointPosition::Boundary(boundary_pos)) => self
                .split_interior_to_boundary(
                    face,
                    face_index,
                    from.vertex_index,
                    to.vertex_index,
                    boundary_pos,
                ),
            (FacePointPosition::Boundary(boundary_pos), FacePointPosition::Interior) => self
                .split_interior_to_boundary(
                    face,
                    face_index,
                    to.vertex_index,
                    from.vertex_index,
                    boundary_pos,
                ),
            (FacePointPosition::Interior, FacePointPosition::Interior) => self
                .split_interior_to_interior(
                    face,
                    face_index,
                    from.vertex_index,
                    to.vertex_index,
                    to.coordinate,
                ),
        }
    }

    fn try_split_multi_segment_face(
        &mut self,
        face: [usize; 3],
        face_index: usize,
        segment_indices: &[usize],
        preplan: &ExactCutPreplan,
    ) -> bool {
        if segment_indices.len() < 2 {
            return false;
        }
        if self
            .try_split_interior_cycle(face, face_index, segment_indices, preplan)
            .is_some()
        {
            return true;
        }
        if self
            .try_split_boundary_segments(face, face_index, segment_indices, preplan)
            .is_some()
        {
            return true;
        }
        if self
            .try_split_boundary_segments_with_interior_spokes(
                face,
                face_index,
                segment_indices,
                preplan,
            )
            .is_some()
        {
            return true;
        }

        let mut interior_vertex = None;
        let mut boundary_points = Vec::with_capacity(segment_indices.len());
        for segment_index in segment_indices {
            let segment = &preplan.path_segments[*segment_index];
            let from = &preplan.cut_points[segment.from_point];
            let to = &preplan.cut_points[segment.to_point];
            let Some(from_pos) = face_point_position(
                face,
                face_index,
                from.primitive,
                from.coordinate,
                &self.vertices,
            ) else {
                return false;
            };
            let Some(to_pos) = face_point_position(
                face,
                face_index,
                to.primitive,
                to.coordinate,
                &self.vertices,
            ) else {
                return false;
            };
            match (from_pos, to_pos) {
                (FacePointPosition::Interior, FacePointPosition::Boundary(position)) => {
                    if !set_shared_interior(&mut interior_vertex, from.vertex_index) {
                        return false;
                    }
                    boundary_points.push((to.vertex_index, position));
                }
                (FacePointPosition::Boundary(position), FacePointPosition::Interior) => {
                    if !set_shared_interior(&mut interior_vertex, to.vertex_index) {
                        return false;
                    }
                    boundary_points.push((from.vertex_index, position));
                }
                _ => return false,
            }
        }

        let Some(interior_vertex) = interior_vertex else {
            return false;
        };
        let Some(split_faces) = split_triangle_with_interior_spokes(
            face,
            interior_vertex,
            &boundary_points,
            &self.vertices,
            self.epsilon,
        ) else {
            return false;
        };

        for (boundary_vertex, _) in boundary_points {
            self.push_cut_edge([interior_vertex, boundary_vertex]);
        }
        for split_face in split_faces {
            self.push_face(split_face, face_index);
        }
        true
    }

    fn try_split_boundary_segments(
        &mut self,
        face: [usize; 3],
        face_index: usize,
        segment_indices: &[usize],
        preplan: &ExactCutPreplan,
    ) -> Option<()> {
        let mut boundary_points = Vec::with_capacity(segment_indices.len() * 2);
        let mut cut_edges = Vec::with_capacity(segment_indices.len());
        for segment_index in segment_indices {
            let segment = &preplan.path_segments[*segment_index];
            let from = &preplan.cut_points[segment.from_point];
            let to = &preplan.cut_points[segment.to_point];
            let FacePointPosition::Boundary(from_pos) = face_point_position(
                face,
                face_index,
                from.primitive,
                from.coordinate,
                &self.vertices,
            )?
            else {
                return None;
            };
            let FacePointPosition::Boundary(to_pos) = face_point_position(
                face,
                face_index,
                to.primitive,
                to.coordinate,
                &self.vertices,
            )?
            else {
                return None;
            };
            boundary_points.push((from.vertex_index, from_pos));
            boundary_points.push((to.vertex_index, to_pos));
            cut_edges.push([from.vertex_index, to.vertex_index]);
        }

        let split_faces = split_triangle_with_boundary_segments(
            face,
            &boundary_points,
            &cut_edges,
            &self.vertices,
            self.epsilon,
        )?;
        for edge in cut_edges {
            self.push_cut_edge(edge);
        }
        for split_face in split_faces {
            self.push_face(split_face, face_index);
        }
        Some(())
    }

    fn try_split_boundary_segments_with_interior_spokes(
        &mut self,
        face: [usize; 3],
        face_index: usize,
        segment_indices: &[usize],
        preplan: &ExactCutPreplan,
    ) -> Option<()> {
        let mut interior_vertex = None;
        let mut boundary_points = Vec::with_capacity(segment_indices.len() * 2);
        let mut cut_edges = Vec::with_capacity(segment_indices.len());
        let mut has_boundary_piece = false;
        let mut has_spoke = false;
        for segment_index in segment_indices {
            let segment = &preplan.path_segments[*segment_index];
            let from = &preplan.cut_points[segment.from_point];
            let to = &preplan.cut_points[segment.to_point];
            let from_pos = face_point_position(
                face,
                face_index,
                from.primitive,
                from.coordinate,
                &self.vertices,
            )?;
            let to_pos = face_point_position(
                face,
                face_index,
                to.primitive,
                to.coordinate,
                &self.vertices,
            )?;
            match (from_pos, to_pos) {
                (FacePointPosition::Boundary(from_pos), FacePointPosition::Boundary(to_pos)) => {
                    boundary_points.push((from.vertex_index, from_pos));
                    boundary_points.push((to.vertex_index, to_pos));
                    cut_edges.push([from.vertex_index, to.vertex_index]);
                    has_boundary_piece = true;
                }
                (FacePointPosition::Interior, FacePointPosition::Boundary(position)) => {
                    if !set_shared_interior(&mut interior_vertex, from.vertex_index) {
                        return None;
                    }
                    boundary_points.push((to.vertex_index, position));
                    cut_edges.push([from.vertex_index, to.vertex_index]);
                    has_spoke = true;
                }
                (FacePointPosition::Boundary(position), FacePointPosition::Interior) => {
                    if !set_shared_interior(&mut interior_vertex, to.vertex_index) {
                        return None;
                    }
                    boundary_points.push((from.vertex_index, position));
                    cut_edges.push([from.vertex_index, to.vertex_index]);
                    has_spoke = true;
                }
                (FacePointPosition::Interior, FacePointPosition::Interior) => return None,
            }
        }
        if !has_boundary_piece || !has_spoke {
            return None;
        }
        let split_faces = split_triangle_with_interior_spokes(
            face,
            interior_vertex?,
            &boundary_points,
            &self.vertices,
            self.epsilon,
        )?;
        for edge in cut_edges {
            self.push_cut_edge(edge);
        }
        for split_face in split_faces {
            self.push_face(split_face, face_index);
        }
        Some(())
    }

    fn try_split_interior_cycle(
        &mut self,
        face: [usize; 3],
        face_index: usize,
        segment_indices: &[usize],
        preplan: &ExactCutPreplan,
    ) -> Option<()> {
        let mut cycle_vertices = Vec::with_capacity(segment_indices.len());
        let mut cut_edges = Vec::with_capacity(segment_indices.len());
        for segment_index in segment_indices {
            let segment = &preplan.path_segments[*segment_index];
            let from = &preplan.cut_points[segment.from_point];
            let to = &preplan.cut_points[segment.to_point];
            if !matches!(
                face_point_position(
                    face,
                    face_index,
                    from.primitive,
                    from.coordinate,
                    &self.vertices
                )?,
                FacePointPosition::Interior
            ) || !matches!(
                face_point_position(
                    face,
                    face_index,
                    to.primitive,
                    to.coordinate,
                    &self.vertices
                )?,
                FacePointPosition::Interior
            ) {
                return None;
            }
            cycle_vertices.push(from.vertex_index);
            cut_edges.push(ordered_edge([from.vertex_index, to.vertex_index]));
        }

        let split_faces = split_triangle_with_interior_cycle(
            face,
            &cycle_vertices,
            &self.vertices,
            self.epsilon,
        )?;
        for edge in cut_edges {
            self.push_cut_edge(edge);
        }
        for split_face in split_faces {
            self.push_face(split_face, face_index);
        }
        Some(())
    }

    fn push_polygon_fan(&mut self, polygon: &[usize], source_face: usize) -> bool {
        let polygon = dedupe_closed_polygon(polygon);
        if polygon.len() < 3 {
            return false;
        }
        let mut best_faces = Vec::new();
        for anchor_index in 0..polygon.len() {
            let mut candidate = Vec::with_capacity(polygon.len() - 2);
            for offset in 1..polygon.len() - 1 {
                let face = [
                    polygon[anchor_index],
                    polygon[(anchor_index + offset) % polygon.len()],
                    polygon[(anchor_index + offset + 1) % polygon.len()],
                ];
                if self.triangle_area(face) > self.epsilon * self.epsilon {
                    candidate.push(face);
                }
            }
            if candidate.len() > best_faces.len() {
                best_faces = candidate;
            }
        }
        let pushed_any = !best_faces.is_empty();
        for face in best_faces {
            self.push_face(face, source_face);
        }
        pushed_any
    }

    fn split_interior_to_boundary(
        &mut self,
        face: [usize; 3],
        source_face: usize,
        interior_vertex: usize,
        boundary_vertex: usize,
        boundary_pos: f64,
    ) -> bool {
        let Some(split_faces) = split_triangle_with_interior_spokes(
            face,
            interior_vertex,
            &[(boundary_vertex, boundary_pos)],
            &self.vertices,
            self.epsilon,
        ) else {
            return false;
        };

        self.push_cut_edge([interior_vertex, boundary_vertex]);
        for split_face in split_faces {
            self.push_face(split_face, source_face);
        }
        true
    }

    fn split_interior_to_interior(
        &mut self,
        face: [usize; 3],
        source_face: usize,
        first_interior_vertex: usize,
        second_interior_vertex: usize,
        second_coordinate: [f64; 3],
    ) -> bool {
        let Some(split_faces) = split_triangle_with_interior_segment(
            face,
            first_interior_vertex,
            second_interior_vertex,
            second_coordinate,
            &self.vertices,
            self.epsilon,
        ) else {
            return false;
        };

        self.push_cut_edge([first_interior_vertex, second_interior_vertex]);
        for split_face in split_faces {
            self.push_face(split_face, source_face);
        }
        true
    }

    fn push_face(&mut self, face: [usize; 3], source_face: usize) {
        self.faces
            .push([face[0] as i64, face[1] as i64, face[2] as i64]);
        self.source_face_for_faces.push(source_face);
    }

    fn push_cut_edge(&mut self, edge: [usize; 2]) {
        let edge = ordered_edge(edge);
        if !self.cut_edges.contains(&edge) {
            self.cut_edges.push(edge);
        }
    }

    fn triangle_area(&self, face: [usize; 3]) -> f64 {
        let a = self.vertices[face[0]];
        let b = self.vertices[face[1]];
        let c = self.vertices[face[2]];
        0.5 * norm(cross(sub(b, a), sub(c, a)))
    }

    fn finish(self) -> ExactCutMeshResult {
        ExactCutMeshResult {
            vertices: self.vertices,
            faces: self.faces,
            cut_edges: self.cut_edges,
            cut_edge_paths: Vec::new(),
            cut_edge_path_closed: Vec::new(),
            source_face_for_faces: self.source_face_for_faces,
            skipped_source_faces: self.skipped_source_faces.into_iter().collect(),
        }
    }
}

fn set_shared_interior(interior_vertex: &mut Option<usize>, candidate: usize) -> bool {
    match interior_vertex {
        Some(existing) => *existing == candidate,
        None => {
            *interior_vertex = Some(candidate);
            true
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FacePointPosition {
    Boundary(f64),
    Interior,
}

fn face_point_position(
    face: [usize; 3],
    face_index: usize,
    primitive: ExactCutPrimitive,
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
) -> Option<FacePointPosition> {
    if let Some(position) = boundary_position(face, primitive, coordinate, vertices) {
        return Some(FacePointPosition::Boundary(position));
    }
    match primitive {
        ExactCutPrimitive::Face(source_face) if source_face == face_index => {
            Some(FacePointPosition::Interior)
        }
        _ => None,
    }
}

fn boundary_position(
    face: [usize; 3],
    primitive: ExactCutPrimitive,
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
) -> Option<f64> {
    match primitive {
        ExactCutPrimitive::Vertex(vertex) => face
            .iter()
            .position(|candidate| *candidate == vertex)
            .map(|index| index as f64),
        ExactCutPrimitive::Edge(edge) => {
            for edge_index in 0..3 {
                let start = face[edge_index];
                let end = face[(edge_index + 1) % 3];
                if ordered_edge(edge) != ordered_edge([start, end]) {
                    continue;
                }
                let parameter = edge_parameter(vertices[start], vertices[end], coordinate);
                return Some(edge_index as f64 + parameter);
            }
            None
        }
        ExactCutPrimitive::Face(_) => None,
    }
}

fn edge_parameter(start: [f64; 3], end: [f64; 3], point: [f64; 3]) -> f64 {
    let edge = sub(end, start);
    let length_sq = dot(edge, edge);
    if length_sq <= f64::EPSILON {
        return 0.0;
    }
    (dot(sub(point, start), edge) / length_sq).clamp(0.0, 1.0)
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn nearly_equal(left: f64, right: f64, epsilon: f64) -> bool {
    (left - right).abs() <= epsilon
}

fn effective_epsilon(epsilon: f64) -> f64 {
    if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    }
}

#[cfg(test)]
mod tests;
