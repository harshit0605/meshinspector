use super::{filtered_prepared_faces, MeshlibRewriteDiagnosticsInput};
use crate::spatial::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use crate::spatial::exact_boolean_topology::ExactMeshlibTopologyRewritePlan;
use crate::spatial::exact_cut_apply::ExactCutMeshResult;
use crate::spatial::exact_stitch::{ExactStitchEdgePair, ExactStitchPlan};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MeshlibPreparedPart {
    pub(super) operand: ExactBooleanOperand,
    pub(super) vertices: Vec<[f64; 3]>,
    pub(super) faces: Vec<[i64; 3]>,
    pub(super) cut_to_part_vertices: Vec<Option<usize>>,
    pub(super) cut_to_part_faces: Vec<Option<usize>>,
    pub(super) cut_to_part_directed_edges: BTreeMap<[usize; 2], [usize; 2]>,
    pub(super) remapped_cut_edge_paths: Vec<Vec<[usize; 2]>>,
    pub(super) remapped_cut_edge_path_missing_edges: Vec<usize>,
    pub(super) missing_cut_path_edge_details: Vec<MeshlibPreparedMissingCutEdge>,
    pub(super) remapped_cut_edge_path_closed: Vec<bool>,
    pub(super) remapped_cut_edge_paths_complete: bool,
    pub(super) skipped_faces: Vec<usize>,
}
mod geometry;
#[cfg(test)]
use geometry::edge_points;
use geometry::{
    directed_face_edges, edge_coordinate_relation, effective_epsilon, ordered_edge, reverse_edge,
    valid_face_vertices, EdgeCoordinateRelation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MeshlibPreparedMissingCutEdge {
    pub(super) path_index: usize,
    pub(super) edge_index: usize,
    pub(super) edge: [usize; 2],
    pub(super) prepared_adjacent_faces: Vec<usize>,
    pub(super) unprepared_adjacent_faces: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MeshlibPreparedConnectParts {
    pub(super) base_operand: ExactBooleanOperand,
    pub(super) incoming_operand: ExactBooleanOperand,
    pub(super) base: MeshlibPreparedPart,
    pub(super) incoming: MeshlibPreparedPart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MeshlibPreparedConnectPartsSummary {
    pub(super) base_faces: usize,
    pub(super) incoming_faces: usize,
    pub(super) base_vertices: usize,
    pub(super) incoming_vertices: usize,
    pub(super) unstitched_vertices: usize,
    pub(super) unstitched_faces: usize,
    pub(super) path_pairs: usize,
    pub(super) path_count_mismatch: bool,
    pub(super) path_length_mismatches: usize,
    pub(super) path_closed_mismatches: usize,
    pub(super) path_coordinate_mismatches: usize,
    pub(super) path_same_direction_edges: usize,
    pub(super) path_reversed_edges: usize,
    pub(super) base_mapped_cut_path_edges: usize,
    pub(super) incoming_mapped_cut_path_edges: usize,
    pub(super) base_missing_cut_path_edges: usize,
    pub(super) incoming_missing_cut_path_edges: usize,
    pub(super) base_cut_paths_complete: bool,
    pub(super) incoming_cut_paths_complete: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeshlibPreparedConnectPathEdgeRelation {
    SameDirection,
    Reversed,
    Mismatch,
    MissingBase,
    MissingIncoming,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MeshlibPreparedConnectPathEdgeDiagnostic {
    pub(super) path_index: usize,
    pub(super) pair_index: usize,
    pub(super) base_edge_index: usize,
    pub(super) incoming_edge_index: usize,
    pub(super) base_edge: [usize; 2],
    pub(super) incoming_edge: [usize; 2],
    pub(super) mapped_base_edge: Option<[usize; 2]>,
    pub(super) mapped_incoming_edge: Option<[usize; 2]>,
    pub(super) base_points: Option<[[f64; 3]; 2]>,
    pub(super) incoming_points: Option<[[f64; 3]; 2]>,
    pub(super) relation: MeshlibPreparedConnectPathEdgeRelation,
}

impl MeshlibPreparedConnectParts {
    pub(super) fn summary(
        &self,
        epsilon: f64,
        stitch_plan: Option<&ExactStitchPlan>,
    ) -> MeshlibPreparedConnectPartsSummary {
        let path_summary = self.path_summary(epsilon, stitch_plan);
        MeshlibPreparedConnectPartsSummary {
            base_faces: self.base.faces.len(),
            incoming_faces: self.incoming.faces.len(),
            base_vertices: self.base.vertices.len(),
            incoming_vertices: self.incoming.vertices.len(),
            unstitched_vertices: self.base.vertices.len() + self.incoming.vertices.len(),
            unstitched_faces: self.base.faces.len() + self.incoming.faces.len(),
            path_pairs: path_summary.path_pairs,
            path_count_mismatch: path_summary.path_count_mismatch,
            path_length_mismatches: path_summary.path_length_mismatches,
            path_closed_mismatches: path_summary.path_closed_mismatches,
            path_coordinate_mismatches: path_summary.path_coordinate_mismatches,
            path_same_direction_edges: path_summary.path_same_direction_edges,
            path_reversed_edges: path_summary.path_reversed_edges,
            base_mapped_cut_path_edges: path_summary.base_mapped_cut_path_edges,
            incoming_mapped_cut_path_edges: path_summary.incoming_mapped_cut_path_edges,
            base_missing_cut_path_edges: path_summary.base_missing_cut_path_edges,
            incoming_missing_cut_path_edges: path_summary.incoming_missing_cut_path_edges,
            base_cut_paths_complete: self.base.remapped_cut_edge_paths_complete,
            incoming_cut_paths_complete: self.incoming.remapped_cut_edge_paths_complete,
        }
    }

    fn path_summary(
        &self,
        epsilon: f64,
        stitch_plan: Option<&ExactStitchPlan>,
    ) -> MeshlibPreparedConnectPathSummary {
        if let Some(stitch_plan) = stitch_plan {
            return self.path_summary_from_stitch_plan(epsilon, stitch_plan);
        }
        self.path_summary_from_raw_cut_paths(epsilon)
    }

    fn path_summary_from_raw_cut_paths(&self, epsilon: f64) -> MeshlibPreparedConnectPathSummary {
        let mut summary = MeshlibPreparedConnectPathSummary {
            path_pairs: self
                .base
                .remapped_cut_edge_paths
                .len()
                .min(self.incoming.remapped_cut_edge_paths.len()),
            path_count_mismatch: self.base.remapped_cut_edge_paths.len()
                != self.incoming.remapped_cut_edge_paths.len(),
            ..MeshlibPreparedConnectPathSummary::default()
        };
        let tolerance_sq = effective_epsilon(epsilon).powi(2);
        for path_index in 0..summary.path_pairs {
            let base_path = &self.base.remapped_cut_edge_paths[path_index];
            let incoming_path = &self.incoming.remapped_cut_edge_paths[path_index];
            if base_path.len() != incoming_path.len() {
                summary.path_length_mismatches += 1;
                continue;
            }
            if self
                .base
                .remapped_cut_edge_path_closed
                .get(path_index)
                .copied()
                .unwrap_or(false)
                != self
                    .incoming
                    .remapped_cut_edge_path_closed
                    .get(path_index)
                    .copied()
                    .unwrap_or(false)
            {
                summary.path_closed_mismatches += 1;
            }
            for (base_edge, incoming_edge) in base_path.iter().zip(incoming_path) {
                match edge_coordinate_relation(
                    &self.base.vertices,
                    &self.incoming.vertices,
                    *base_edge,
                    *incoming_edge,
                    tolerance_sq,
                ) {
                    EdgeCoordinateRelation::SameDirection => {
                        summary.path_same_direction_edges += 1;
                    }
                    EdgeCoordinateRelation::Reversed => {
                        summary.path_reversed_edges += 1;
                    }
                    EdgeCoordinateRelation::Mismatch => {
                        summary.path_coordinate_mismatches += 1;
                    }
                }
            }
        }
        summary.base_mapped_cut_path_edges = self.base.remapped_cut_edge_path_edges();
        summary.incoming_mapped_cut_path_edges = self.incoming.remapped_cut_edge_path_edges();
        summary.base_missing_cut_path_edges = self.base.remapped_cut_edge_path_missing_edges();
        summary.incoming_missing_cut_path_edges =
            self.incoming.remapped_cut_edge_path_missing_edges();
        summary
    }

    fn path_summary_from_stitch_plan(
        &self,
        epsilon: f64,
        stitch_plan: &ExactStitchPlan,
    ) -> MeshlibPreparedConnectPathSummary {
        let mut summary = MeshlibPreparedConnectPathSummary {
            path_pairs: stitch_plan.paths.len(),
            ..MeshlibPreparedConnectPathSummary::default()
        };
        let tolerance_sq = effective_epsilon(epsilon).powi(2);
        for path in &stitch_plan.paths {
            for pair_index in &path.pair_indices {
                let Some(pair) = stitch_plan.pairs.get(*pair_index) else {
                    summary.path_length_mismatches += 1;
                    continue;
                };
                let base_edge = stitch_pair_edge_for_operand(pair, self.base_operand);
                let incoming_edge = stitch_pair_edge_for_operand(pair, self.incoming_operand);
                let mapped_base = self
                    .base
                    .cut_to_part_directed_edges
                    .get(&base_edge)
                    .copied();
                let mapped_incoming = self
                    .incoming
                    .cut_to_part_directed_edges
                    .get(&incoming_edge)
                    .copied();
                match mapped_base {
                    Some(_) => summary.base_mapped_cut_path_edges += 1,
                    None => summary.base_missing_cut_path_edges += 1,
                }
                match mapped_incoming {
                    Some(_) => summary.incoming_mapped_cut_path_edges += 1,
                    None => summary.incoming_missing_cut_path_edges += 1,
                }
                let (Some(mapped_base), Some(mapped_incoming)) = (mapped_base, mapped_incoming)
                else {
                    continue;
                };
                match edge_coordinate_relation(
                    &self.base.vertices,
                    &self.incoming.vertices,
                    mapped_base,
                    mapped_incoming,
                    tolerance_sq,
                ) {
                    EdgeCoordinateRelation::SameDirection => {
                        summary.path_same_direction_edges += 1;
                    }
                    EdgeCoordinateRelation::Reversed => {
                        summary.path_reversed_edges += 1;
                    }
                    EdgeCoordinateRelation::Mismatch => {
                        summary.path_coordinate_mismatches += 1;
                    }
                }
            }
        }
        summary
    }

    #[cfg(test)]
    pub(super) fn path_edge_diagnostics_from_stitch_plan(
        &self,
        epsilon: f64,
        stitch_plan: &ExactStitchPlan,
    ) -> Vec<MeshlibPreparedConnectPathEdgeDiagnostic> {
        let tolerance_sq = effective_epsilon(epsilon).powi(2);
        stitch_plan
            .paths
            .iter()
            .enumerate()
            .flat_map(|(path_index, path)| {
                path.pair_indices.iter().filter_map(move |pair_index| {
                    let pair = stitch_plan.pairs.get(*pair_index)?;
                    let base_edge = stitch_pair_edge_for_operand(pair, self.base_operand);
                    let incoming_edge = stitch_pair_edge_for_operand(pair, self.incoming_operand);
                    let mapped_base_edge = self
                        .base
                        .cut_to_part_directed_edges
                        .get(&base_edge)
                        .copied();
                    let mapped_incoming_edge = self
                        .incoming
                        .cut_to_part_directed_edges
                        .get(&incoming_edge)
                        .copied();
                    let base_points =
                        mapped_base_edge.and_then(|edge| edge_points(&self.base.vertices, edge));
                    let incoming_points = mapped_incoming_edge
                        .and_then(|edge| edge_points(&self.incoming.vertices, edge));
                    let relation = match (mapped_base_edge, mapped_incoming_edge) {
                        (None, _) => MeshlibPreparedConnectPathEdgeRelation::MissingBase,
                        (_, None) => MeshlibPreparedConnectPathEdgeRelation::MissingIncoming,
                        (Some(base), Some(incoming)) => match edge_coordinate_relation(
                            &self.base.vertices,
                            &self.incoming.vertices,
                            base,
                            incoming,
                            tolerance_sq,
                        ) {
                            EdgeCoordinateRelation::SameDirection => {
                                MeshlibPreparedConnectPathEdgeRelation::SameDirection
                            }
                            EdgeCoordinateRelation::Reversed => {
                                MeshlibPreparedConnectPathEdgeRelation::Reversed
                            }
                            EdgeCoordinateRelation::Mismatch => {
                                MeshlibPreparedConnectPathEdgeRelation::Mismatch
                            }
                        },
                    };
                    Some(MeshlibPreparedConnectPathEdgeDiagnostic {
                        path_index,
                        pair_index: *pair_index,
                        base_edge_index: stitch_pair_edge_index_for_operand(
                            pair,
                            self.base_operand,
                        ),
                        incoming_edge_index: stitch_pair_edge_index_for_operand(
                            pair,
                            self.incoming_operand,
                        ),
                        base_edge,
                        incoming_edge,
                        mapped_base_edge,
                        mapped_incoming_edge,
                        base_points,
                        incoming_points,
                        relation,
                    })
                })
            })
            .collect()
    }
}

impl MeshlibPreparedPart {
    fn remapped_cut_edge_path_edges(&self) -> usize {
        self.remapped_cut_edge_paths
            .iter()
            .map(Vec::len)
            .sum::<usize>()
    }

    fn remapped_cut_edge_path_missing_edges(&self) -> usize {
        self.remapped_cut_edge_path_missing_edges
            .iter()
            .sum::<usize>()
    }
}

#[derive(Default)]
struct MeshlibPreparedConnectPathSummary {
    path_pairs: usize,
    path_count_mismatch: bool,
    path_length_mismatches: usize,
    path_closed_mismatches: usize,
    path_coordinate_mismatches: usize,
    path_same_direction_edges: usize,
    path_reversed_edges: usize,
    base_mapped_cut_path_edges: usize,
    incoming_mapped_cut_path_edges: usize,
    base_missing_cut_path_edges: usize,
    incoming_missing_cut_path_edges: usize,
}

pub(super) fn meshlib_prepare_connect_parts(
    input: &MeshlibRewriteDiagnosticsInput<'_>,
    topology_rewrite: &ExactMeshlibTopologyRewritePlan,
) -> MeshlibPreparedConnectParts {
    let base_prepared_faces = filtered_prepared_faces(input, topology_rewrite.base_operand);
    let incoming_prepared_faces = filtered_prepared_faces(input, topology_rewrite.incoming_operand);
    let base = meshlib_prepare_part(MeshlibPreparedPartInput {
        cut_mesh: operand_cut_mesh(input, topology_rewrite.base_operand),
        prepared_faces: &base_prepared_faces,
        operand: topology_rewrite.base_operand,
        flip_orientation: operand_flip(input.assembly, topology_rewrite.base_operand),
    });
    let incoming = meshlib_prepare_part(MeshlibPreparedPartInput {
        cut_mesh: operand_cut_mesh(input, topology_rewrite.incoming_operand),
        prepared_faces: &incoming_prepared_faces,
        operand: topology_rewrite.incoming_operand,
        flip_orientation: operand_flip(input.assembly, topology_rewrite.incoming_operand),
    });
    MeshlibPreparedConnectParts {
        base_operand: topology_rewrite.base_operand,
        incoming_operand: topology_rewrite.incoming_operand,
        base,
        incoming,
    }
}

struct MeshlibPreparedPartInput<'a> {
    cut_mesh: &'a ExactCutMeshResult,
    prepared_faces: &'a [usize],
    operand: ExactBooleanOperand,
    flip_orientation: bool,
}

fn meshlib_prepare_part(input: MeshlibPreparedPartInput<'_>) -> MeshlibPreparedPart {
    let mut builder = MeshlibPreparedPartBuilder::new(input.cut_mesh, input.operand);
    for cut_face in input.prepared_faces {
        builder.copy_face(*cut_face, input.flip_orientation);
    }
    builder.finish()
}

struct MeshlibPreparedPartBuilder<'a> {
    cut_mesh: &'a ExactCutMeshResult,
    operand: ExactBooleanOperand,
    vertices: Vec<[f64; 3]>,
    faces: Vec<[i64; 3]>,
    cut_to_part_vertices: Vec<Option<usize>>,
    cut_to_part_faces: Vec<Option<usize>>,
    cut_to_part_directed_edges: BTreeMap<[usize; 2], [usize; 2]>,
    copied_undirected_edges: BTreeSet<[usize; 2]>,
    skipped_faces: Vec<usize>,
}

impl<'a> MeshlibPreparedPartBuilder<'a> {
    fn new(cut_mesh: &'a ExactCutMeshResult, operand: ExactBooleanOperand) -> Self {
        Self {
            cut_mesh,
            operand,
            vertices: Vec::new(),
            faces: Vec::new(),
            cut_to_part_vertices: vec![None; cut_mesh.vertices.len()],
            cut_to_part_faces: vec![None; cut_mesh.faces.len()],
            cut_to_part_directed_edges: BTreeMap::new(),
            copied_undirected_edges: BTreeSet::new(),
            skipped_faces: Vec::new(),
        }
    }

    fn copy_face(&mut self, cut_face: usize, flip_orientation: bool) {
        let Some(face) = self.cut_mesh.faces.get(cut_face).copied() else {
            self.skipped_faces.push(cut_face);
            return;
        };
        let Some(source_vertices) = valid_face_vertices(face, self.cut_mesh.vertices.len()) else {
            self.skipped_faces.push(cut_face);
            return;
        };
        let Some(mapped) = self.mapped_face_vertices(source_vertices) else {
            self.skipped_faces.push(cut_face);
            return;
        };
        let part_face = self.faces.len();
        self.cut_to_part_faces[cut_face] = Some(part_face);
        if flip_orientation {
            self.faces
                .push([mapped[0] as i64, mapped[2] as i64, mapped[1] as i64]);
        } else {
            self.faces
                .push([mapped[0] as i64, mapped[1] as i64, mapped[2] as i64]);
        }
        self.copy_face_edges(source_vertices);
    }

    fn mapped_face_vertices(&mut self, source_vertices: [usize; 3]) -> Option<[usize; 3]> {
        Some([
            self.mapped_vertex(source_vertices[0])?,
            self.mapped_vertex(source_vertices[1])?,
            self.mapped_vertex(source_vertices[2])?,
        ])
    }

    fn mapped_vertex(&mut self, source_vertex: usize) -> Option<usize> {
        if let Some(mapped) = self.cut_to_part_vertices.get(source_vertex).copied()? {
            return Some(mapped);
        }
        let point = *self.cut_mesh.vertices.get(source_vertex)?;
        let mapped = self.vertices.len();
        self.vertices.push(point);
        self.cut_to_part_vertices[source_vertex] = Some(mapped);
        Some(mapped)
    }

    fn copy_face_edges(&mut self, source_vertices: [usize; 3]) {
        for edge in directed_face_edges(source_vertices) {
            self.copy_directed_edge(edge);
        }
    }

    fn copy_directed_edge(&mut self, edge: [usize; 2]) {
        let key = ordered_edge(edge);
        self.copied_undirected_edges.insert(key);
        let Some(mapped) = self.mapped_directed_edge_by_vertex_map(edge) else {
            return;
        };
        self.cut_to_part_directed_edges
            .entry(edge)
            .or_insert(mapped);
        self.cut_to_part_directed_edges
            .entry(reverse_edge(edge))
            .or_insert(reverse_edge(mapped));
    }

    fn mapped_directed_edge_by_vertex_map(&self, edge: [usize; 2]) -> Option<[usize; 2]> {
        let mapped = [
            self.cut_to_part_vertices.get(edge[0]).copied().flatten()?,
            self.cut_to_part_vertices.get(edge[1]).copied().flatten()?,
        ];
        (mapped[0] != mapped[1]).then_some(mapped)
    }

    fn finish(self) -> MeshlibPreparedPart {
        let remapped = remapped_cut_edge_paths(
            self.cut_mesh,
            &self.copied_undirected_edges,
            &self.cut_to_part_directed_edges,
            &self.cut_to_part_faces,
        );
        MeshlibPreparedPart {
            operand: self.operand,
            vertices: self.vertices,
            faces: self.faces,
            cut_to_part_vertices: self.cut_to_part_vertices,
            cut_to_part_faces: self.cut_to_part_faces,
            cut_to_part_directed_edges: self.cut_to_part_directed_edges,
            remapped_cut_edge_paths: remapped.paths,
            remapped_cut_edge_path_missing_edges: remapped.missing_edges,
            missing_cut_path_edge_details: remapped.missing_edge_details,
            remapped_cut_edge_path_closed: remapped.closed,
            remapped_cut_edge_paths_complete: remapped.complete,
            skipped_faces: self.skipped_faces,
        }
    }
}

struct RemappedCutEdgePaths {
    paths: Vec<Vec<[usize; 2]>>,
    missing_edges: Vec<usize>,
    missing_edge_details: Vec<MeshlibPreparedMissingCutEdge>,
    closed: Vec<bool>,
    complete: bool,
}

fn remapped_cut_edge_paths(
    cut_mesh: &ExactCutMeshResult,
    copied_undirected_edges: &BTreeSet<[usize; 2]>,
    cut_to_part_directed_edges: &BTreeMap<[usize; 2], [usize; 2]>,
    cut_to_part_faces: &[Option<usize>],
) -> RemappedCutEdgePaths {
    let mut paths = Vec::new();
    let mut missing_edges = Vec::new();
    let mut missing_edge_details = Vec::new();
    let mut closed = Vec::new();
    let mut complete = true;
    for (path_index, source_path) in cut_mesh.cut_edge_paths.iter().enumerate() {
        let source_closed = cut_mesh
            .cut_edge_path_closed
            .get(path_index)
            .copied()
            .unwrap_or(false);
        let mut missing = 0;
        let mut path_complete = true;
        let mut mapped_path = Vec::with_capacity(source_path.len());
        for (edge_index, edge) in source_path.iter().enumerate() {
            let edge_in_part = copied_undirected_edges.contains(&ordered_edge(*edge));
            let mapped = edge_in_part
                .then(|| cut_to_part_directed_edges.get(edge).copied())
                .flatten();
            match mapped {
                Some(mapped) => mapped_path.push(mapped),
                None => {
                    complete = false;
                    path_complete = false;
                    missing += 1;
                    missing_edge_details.push(missing_cut_path_edge_detail(
                        cut_mesh,
                        cut_to_part_faces,
                        path_index,
                        edge_index,
                        *edge,
                    ));
                }
            }
        }
        paths.push(mapped_path);
        missing_edges.push(missing);
        closed.push(source_closed && path_complete);
    }
    RemappedCutEdgePaths {
        paths,
        missing_edges,
        missing_edge_details,
        closed,
        complete,
    }
}

fn missing_cut_path_edge_detail(
    cut_mesh: &ExactCutMeshResult,
    cut_to_part_faces: &[Option<usize>],
    path_index: usize,
    edge_index: usize,
    edge: [usize; 2],
) -> MeshlibPreparedMissingCutEdge {
    let mut prepared_adjacent_faces = Vec::new();
    let mut unprepared_adjacent_faces = Vec::new();
    let key = ordered_edge(edge);
    for (face_index, face) in cut_mesh.faces.iter().enumerate() {
        let Some(face_vertices) = valid_face_vertices(*face, cut_mesh.vertices.len()) else {
            continue;
        };
        if !directed_face_edges(face_vertices)
            .into_iter()
            .any(|face_edge| ordered_edge(face_edge) == key)
        {
            continue;
        }
        if cut_to_part_faces
            .get(face_index)
            .copied()
            .flatten()
            .is_some()
        {
            prepared_adjacent_faces.push(face_index);
        } else {
            unprepared_adjacent_faces.push(face_index);
        }
    }
    MeshlibPreparedMissingCutEdge {
        path_index,
        edge_index,
        edge,
        prepared_adjacent_faces,
        unprepared_adjacent_faces,
    }
}

fn operand_cut_mesh<'a>(
    input: &'a MeshlibRewriteDiagnosticsInput<'_>,
    operand: ExactBooleanOperand,
) -> &'a ExactCutMeshResult {
    match operand {
        ExactBooleanOperand::First => input.first_cut,
        ExactBooleanOperand::Second => input.second_cut,
    }
}

fn operand_flip(assembly: &ExactBooleanAssemblyResult, operand: ExactBooleanOperand) -> bool {
    match operand {
        ExactBooleanOperand::First => assembly.flipped_first,
        ExactBooleanOperand::Second => assembly.flipped_second,
    }
}

fn stitch_pair_edge_for_operand(
    pair: &ExactStitchEdgePair,
    operand: ExactBooleanOperand,
) -> [usize; 2] {
    match operand {
        ExactBooleanOperand::First => pair.first_edge,
        ExactBooleanOperand::Second => pair.second_edge,
    }
}

#[cfg(test)]
fn stitch_pair_edge_index_for_operand(
    pair: &ExactStitchEdgePair,
    operand: ExactBooleanOperand,
) -> usize {
    match operand {
        ExactBooleanOperand::First => pair.first_edge_index,
        ExactBooleanOperand::Second => pair.second_edge_index,
    }
}

#[cfg(test)]
mod tests;
