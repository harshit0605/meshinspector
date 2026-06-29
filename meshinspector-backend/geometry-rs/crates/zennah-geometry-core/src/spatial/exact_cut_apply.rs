use super::exact_cut::{exact_cut_preplan, ExactCutPathSegment, ExactCutPreplan};
use super::exact_face_split::{
    split_triangle_with_boundary_segments, split_triangle_with_interior_cycle,
    split_triangle_with_interior_segment, split_triangle_with_interior_spokes,
};
use super::exact_one_mesh::ExactOneMeshContour;
use crate::math::{cross, norm, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::BTreeSet;

mod chains;
mod helpers;
mod paths;
mod polygon;

use self::helpers::{
    effective_epsilon, face_point_position, nearly_equal, ordered_edge, set_shared_interior,
    FacePointPosition,
};
use self::paths::{
    collapsed_cut_segment_paths_and_source_faces_from_preplan,
    cut_edge_paths_and_source_faces_from_preplan, directed_path_is_closed,
    segment_lies_on_shared_boundary_edge,
};
use self::polygon::{boundary_path, dedupe_closed_polygon};

#[derive(Debug, Clone, PartialEq)]
pub struct ExactCutMeshResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub cut_edges: Vec<[usize; 2]>,
    pub cut_edge_paths: Vec<Vec<[usize; 2]>>,
    pub cut_edge_path_closed: Vec<bool>,
    pub cut_edge_path_source_faces: Vec<Vec<Option<usize>>>,
    pub collapsed_cut_segment_paths: Vec<Vec<[usize; 2]>>,
    pub collapsed_cut_segment_path_source_faces: Vec<Vec<Option<usize>>>,
    pub source_face_for_faces: Vec<usize>,
    pub cut_face_source_events: Vec<ExactCutFaceSourceEvent>,
    pub skipped_source_faces: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactCutFaceSourceEvent {
    pub kind: ExactCutFaceSourceEventKind,
    pub source_face: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactCutFaceSourceEventKind {
    Original,
    Split,
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
        cut_face_source_events: Vec::with_capacity(faces.len()),
        skipped_source_faces: BTreeSet::new(),
        epsilon: effective_epsilon(epsilon),
    };

    for (face_index, face) in faces.iter().copied().enumerate() {
        let segment_indices = &segments_by_face[face_index];
        if segment_indices.is_empty() {
            output.push_original_face(face, face_index);
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
            output.push_original_face(face, face_index);
        }
    }

    let mut result = output.finish();
    let (cut_edge_paths, cut_edge_path_source_faces) =
        cut_edge_paths_and_source_faces_from_preplan(preplan, &result.cut_edges);
    result.cut_edge_paths = cut_edge_paths;
    result.cut_edge_path_source_faces = cut_edge_path_source_faces;
    let (collapsed_paths, collapsed_path_source_faces) =
        collapsed_cut_segment_paths_and_source_faces_from_preplan(preplan);
    result.collapsed_cut_segment_paths = collapsed_paths;
    result.collapsed_cut_segment_path_source_faces = collapsed_path_source_faces;
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
    cut_face_source_events: Vec<ExactCutFaceSourceEvent>,
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
        if self
            .try_split_shared_interior_spokes(face, face_index, segment_indices, preplan)
            .is_some()
        {
            return true;
        }
        // The general multi-chain split runs last, so every case the focused
        // strategies above already handle keeps its exact existing output.
        self.try_split_boundary_chain(face, face_index, segment_indices, preplan)
            .is_some()
    }

    fn try_split_shared_interior_spokes(
        &mut self,
        face: [usize; 3],
        face_index: usize,
        segment_indices: &[usize],
        preplan: &ExactCutPreplan,
    ) -> Option<()> {
        let mut interior_vertex = None;
        let mut boundary_points = Vec::with_capacity(segment_indices.len());
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
                (FacePointPosition::Interior, FacePointPosition::Boundary(position)) => {
                    if !set_shared_interior(&mut interior_vertex, from.vertex_index) {
                        return None;
                    }
                    boundary_points.push((to.vertex_index, position));
                }
                (FacePointPosition::Boundary(position), FacePointPosition::Interior) => {
                    if !set_shared_interior(&mut interior_vertex, to.vertex_index) {
                        return None;
                    }
                    boundary_points.push((from.vertex_index, position));
                }
                _ => return None,
            }
        }

        let interior_vertex = interior_vertex?;
        let split_faces = split_triangle_with_interior_spokes(
            face,
            interior_vertex,
            &boundary_points,
            &self.vertices,
            self.epsilon,
        )?;

        for (boundary_vertex, _) in boundary_points {
            self.push_cut_edge([interior_vertex, boundary_vertex]);
        }
        for split_face in split_faces {
            self.push_face(split_face, face_index);
        }
        Some(())
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
        self.push_face_with_event_kind(face, source_face, ExactCutFaceSourceEventKind::Split);
    }

    fn push_original_face(&mut self, face: [usize; 3], source_face: usize) {
        self.push_face_with_event_kind(face, source_face, ExactCutFaceSourceEventKind::Original);
    }

    fn push_face_with_event_kind(
        &mut self,
        face: [usize; 3],
        source_face: usize,
        event_kind: ExactCutFaceSourceEventKind,
    ) {
        self.faces
            .push([face[0] as i64, face[1] as i64, face[2] as i64]);
        self.source_face_for_faces.push(source_face);
        self.cut_face_source_events.push(ExactCutFaceSourceEvent {
            kind: event_kind,
            source_face,
        });
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
            cut_edge_path_source_faces: Vec::new(),
            collapsed_cut_segment_paths: Vec::new(),
            collapsed_cut_segment_path_source_faces: Vec::new(),
            source_face_for_faces: self.source_face_for_faces,
            cut_face_source_events: self.cut_face_source_events,
            skipped_source_faces: self.skipped_source_faces.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests;
