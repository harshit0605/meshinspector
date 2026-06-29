use super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOperand, ExactBooleanOperation,
    ExactBooleanOutputFaceSource,
};
use super::exact_boolean_paths::{mapped_result_cut_paths, stitched_edge_sources};
use super::exact_classify::{
    exact_classify_components_with_cut_paths_with_orientation_flip, ExactCutPathClassificationInput,
    ExactMeshPartClassification,
};
use super::exact_cut_apply::ExactCutMeshResult;
use super::exact_stitch::{exact_stitch_vertex_map, ExactStitchPlan};
use crate::{GeometryError, MeshArrays};
use std::collections::BTreeSet;

mod coplanar;
pub(super) use coplanar::{
    exact_assemble_difference_with_coplanar_sampling,
    exact_assemble_intersection_with_coplanar_sampling,
    exact_assemble_union_with_coplanar_first_wins,
};

pub fn exact_assemble_boolean_from_cut_meshes(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> Result<ExactBooleanAssemblyResult, GeometryError> {
    exact_assemble_boolean_from_cut_meshes_with_stitch(first, second, None, operation, epsilon)
}

pub(super) fn exact_assemble_boolean_from_cut_meshes_with_stitch(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    stitch_plan: Option<&ExactStitchPlan>,
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> Result<ExactBooleanAssemblyResult, GeometryError> {
    let first_need_inside = first_need_inside(operation);
    let second_need_inside = second_need_inside(operation);
    let first_classification = match first_need_inside {
        Some(need_inside) => Some(exact_classify_components_with_cut_paths_with_orientation_flip(
            ExactCutPathClassificationInput {
                vertices: &first.vertices,
                faces_i64: &first.faces,
                other_vertices: &second.vertices,
                other_faces_i64: &second.faces,
                cut_edges: &first.cut_edges,
                cut_edge_paths: &first.cut_edge_paths,
                need_inside,
                origin_is_first: true,
                epsilon,
            },
        )?),
        None => None,
    };
    let second_classification = match second_need_inside {
        Some(need_inside) => Some(exact_classify_components_with_cut_paths_with_orientation_flip(
            ExactCutPathClassificationInput {
                vertices: &second.vertices,
                faces_i64: &second.faces,
                other_vertices: &first.vertices,
                other_faces_i64: &first.faces,
                cut_edges: &second.cut_edges,
                cut_edge_paths: &second.cut_edge_paths,
                need_inside,
                origin_is_first: false,
                epsilon,
            },
        )?),
        None => None,
    };
    Ok(assemble_classified_boolean_with_stitch_at_tolerance(
        first,
        second,
        first_classification.as_ref(),
        second_classification.as_ref(),
        stitch_plan,
        operation,
        epsilon,
    ))
}

pub fn assemble_classified_boolean(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    first_classification: Option<&ExactMeshPartClassification>,
    second_classification: Option<&ExactMeshPartClassification>,
    operation: ExactBooleanOperation,
) -> ExactBooleanAssemblyResult {
    assemble_classified_boolean_with_stitch_at_tolerance(
        first,
        second,
        first_classification,
        second_classification,
        None,
        operation,
        1e-9,
    )
}

#[cfg(test)]
pub(super) fn assemble_classified_boolean_with_stitch(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    first_classification: Option<&ExactMeshPartClassification>,
    second_classification: Option<&ExactMeshPartClassification>,
    stitch_plan: Option<&ExactStitchPlan>,
    operation: ExactBooleanOperation,
) -> ExactBooleanAssemblyResult {
    assemble_classified_boolean_with_stitch_at_tolerance(
        first,
        second,
        first_classification,
        second_classification,
        stitch_plan,
        operation,
        1e-9,
    )
}

pub(super) fn assemble_classified_boolean_with_stitch_at_tolerance(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    first_classification: Option<&ExactMeshPartClassification>,
    second_classification: Option<&ExactMeshPartClassification>,
    stitch_plan: Option<&ExactStitchPlan>,
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> ExactBooleanAssemblyResult {
    let selected_first = first_classification
        .map(|classification| classification.selected_faces.clone())
        .unwrap_or_default();
    let selected_second = second_classification
        .map(|classification| classification.selected_faces.clone())
        .unwrap_or_default();
    let prepare_first_faces = prepare_part_faces(first_classification, &selected_first);
    let prepare_second_faces = prepare_part_faces(second_classification, &selected_second);
    let flipped_first = operation == ExactBooleanOperation::DifferenceBA;
    let flipped_second = operation == ExactBooleanOperation::DifferenceAB;

    let mut output = BooleanAssemblyBuilder::default();
    let (first_vertex_map, second_vertex_map) = if paths_have_left_hole(operation) {
        let second_vertex_map = output.append_selected(FaceSelection {
            vertices: &second.vertices,
            faces: &second.faces,
            source_face_for_faces: &second.source_face_for_faces,
            selected_faces: &selected_second,
            flip: flipped_second,
            operand: ExactBooleanOperand::Second,
            preassigned_vertices: None,
        });
        let first_preassigned = stitch_plan
            .and_then(|plan| first_preassigned_vertices(plan, first, &second_vertex_map));
        let first_vertex_map = output.append_selected(FaceSelection {
            vertices: &first.vertices,
            faces: &first.faces,
            source_face_for_faces: &first.source_face_for_faces,
            selected_faces: &selected_first,
            flip: flipped_first,
            operand: ExactBooleanOperand::First,
            preassigned_vertices: first_preassigned.as_deref(),
        });
        (first_vertex_map, second_vertex_map)
    } else {
        let first_vertex_map = output.append_selected(FaceSelection {
            vertices: &first.vertices,
            faces: &first.faces,
            source_face_for_faces: &first.source_face_for_faces,
            selected_faces: &selected_first,
            flip: flipped_first,
            operand: ExactBooleanOperand::First,
            preassigned_vertices: None,
        });
        let second_preassigned = stitch_plan
            .and_then(|plan| second_preassigned_vertices(plan, second, &first_vertex_map));
        let second_vertex_map = output.append_selected(FaceSelection {
            vertices: &second.vertices,
            faces: &second.faces,
            source_face_for_faces: &second.source_face_for_faces,
            selected_faces: &selected_second,
            flip: flipped_second,
            operand: ExactBooleanOperand::Second,
            preassigned_vertices: second_preassigned.as_deref(),
        });
        (first_vertex_map, second_vertex_map)
    };
    let stitched_edges = stitch_plan
        .map(|plan| stitched_edge_sources(plan, &first_vertex_map, &second_vertex_map))
        .unwrap_or_default();
    let result_cut = mapped_result_cut_paths(
        operation,
        first,
        second,
        stitch_plan,
        &first_vertex_map,
        &second_vertex_map,
        epsilon,
    );

    ExactBooleanAssemblyResult {
        vertices: output.vertices,
        faces: output.faces,
        face_sources: output.face_sources,
        first_output_vertex_for_cut_vertex: first_vertex_map,
        second_output_vertex_for_cut_vertex: second_vertex_map,
        stitched_edge_sources: stitched_edges.sources,
        stitched_edge_paths: stitched_edges.paths,
        prepare_first_faces,
        prepare_second_faces,
        selected_first_faces: selected_first,
        selected_second_faces: selected_second,
        flipped_first,
        flipped_second,
        first_cut_paths_consistent: first_classification
            .map(|classification| classification.cut_paths_consistent)
            .unwrap_or(true),
        second_cut_paths_consistent: second_classification
            .map(|classification| classification.cut_paths_consistent)
            .unwrap_or(true),
        first_cut_path_side_components: cut_path_side_components(first_classification),
        second_cut_path_side_components: cut_path_side_components(second_classification),
        first_cut_path_overlap_components: cut_path_overlap_components(first_classification),
        second_cut_path_overlap_components: cut_path_overlap_components(second_classification),
        result_cut_paths: result_cut.paths,
        result_cut_path_closed: result_cut.closed,
        result_cut_paths_complete: result_cut.complete,
    }
}

fn prepare_part_faces(
    classification: Option<&ExactMeshPartClassification>,
    selected_faces: &[usize],
) -> Vec<usize> {
    classification
        .filter(|classification| classification.cut_paths_consistent)
        .map(|_| selected_faces.to_vec())
        .unwrap_or_default()
}

fn cut_path_side_components(classification: Option<&ExactMeshPartClassification>) -> [usize; 2] {
    classification
        .map(|classification| {
            [
                classification.cut_left_components,
                classification.cut_right_components,
            ]
        })
        .unwrap_or([0, 0])
}

fn cut_path_overlap_components(classification: Option<&ExactMeshPartClassification>) -> usize {
    classification
        .map(|classification| classification.cut_path_overlap_components)
        .unwrap_or_default()
}

fn second_preassigned_vertices(
    stitch_plan: &ExactStitchPlan,
    second: &ExactCutMeshResult,
    first_vertex_map: &[Option<usize>],
) -> Option<Vec<Option<usize>>> {
    let stitch_vertices = exact_stitch_vertex_map(stitch_plan, second.vertices.len());
    if !stitch_vertices.conflicts.is_empty() {
        return None;
    }
    Some(
        stitch_vertices
            .second_to_first
            .into_iter()
            .map(|first_vertex| {
                first_vertex.and_then(|vertex| first_vertex_map.get(vertex).copied().flatten())
            })
            .collect(),
    )
}

fn first_preassigned_vertices(
    stitch_plan: &ExactStitchPlan,
    first: &ExactCutMeshResult,
    second_vertex_map: &[Option<usize>],
) -> Option<Vec<Option<usize>>> {
    let mut first_to_second = vec![None; first.vertices.len()];
    for pair in &stitch_plan.pairs {
        let endpoint_pairs = if pair.second_reversed {
            [
                (pair.first_edge[0], pair.second_edge[1]),
                (pair.first_edge[1], pair.second_edge[0]),
            ]
        } else {
            [
                (pair.first_edge[0], pair.second_edge[0]),
                (pair.first_edge[1], pair.second_edge[1]),
            ]
        };
        for (first_vertex, second_vertex) in endpoint_pairs {
            let slot = first_to_second.get_mut(first_vertex)?;
            match slot {
                Some(existing) if *existing != second_vertex => return None,
                Some(_) => {}
                None => *slot = Some(second_vertex),
            }
        }
    }
    Some(
        first_to_second
            .into_iter()
            .map(|second_vertex| {
                second_vertex.and_then(|vertex| second_vertex_map.get(vertex).copied().flatten())
            })
            .collect(),
    )
}

fn paths_have_left_hole(operation: ExactBooleanOperation) -> bool {
    operation == ExactBooleanOperation::Intersection
}

#[derive(Default)]
struct BooleanAssemblyBuilder {
    vertices: Vec<[f64; 3]>,
    faces: Vec<[i64; 3]>,
    face_sources: Vec<ExactBooleanOutputFaceSource>,
}

impl BooleanAssemblyBuilder {
    fn append_selected(&mut self, selection: FaceSelection<'_>) -> Vec<Option<usize>> {
        let mut vertex_map = vec![None; selection.vertices.len()];
        if selection.selected_faces.is_empty() {
            return vertex_map;
        }
        let selected = selection
            .selected_faces
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        for face_index in selected {
            let face = selection.faces[face_index];
            let mut mapped = [0_i64; 3];
            for corner in 0..3 {
                let source_vertex = face[corner] as usize;
                let output_vertex = match vertex_map[source_vertex] {
                    Some(output_vertex) => output_vertex,
                    None => {
                        let output_vertex = selection
                            .preassigned_vertices
                            .and_then(|vertices| vertices.get(source_vertex).copied().flatten())
                            .unwrap_or_else(|| {
                                let output_vertex = self.vertices.len();
                                self.vertices.push(selection.vertices[source_vertex]);
                                output_vertex
                            });
                        vertex_map[source_vertex] = Some(output_vertex);
                        output_vertex
                    }
                };
                mapped[corner] = output_vertex as i64;
            }
            if selection.flip {
                mapped.swap(1, 2);
            }
            self.faces.push(mapped);
            self.face_sources.push(ExactBooleanOutputFaceSource {
                operand: selection.operand,
                cut_face: face_index,
                source_face: selection
                    .source_face_for_faces
                    .get(face_index)
                    .copied()
                    .unwrap_or(face_index),
            });
        }
        vertex_map
    }
}

struct FaceSelection<'a> {
    vertices: &'a [[f64; 3]],
    faces: &'a [[i64; 3]],
    source_face_for_faces: &'a [usize],
    selected_faces: &'a [usize],
    flip: bool,
    operand: ExactBooleanOperand,
    preassigned_vertices: Option<&'a [Option<usize>]>,
}

impl From<ExactBooleanAssemblyResult> for MeshArrays {
    fn from(result: ExactBooleanAssemblyResult) -> Self {
        MeshArrays {
            vertices: result.vertices,
            faces: result.faces,
        }
    }
}

fn first_need_inside(operation: ExactBooleanOperation) -> Option<bool> {
    match operation {
        ExactBooleanOperation::Union
        | ExactBooleanOperation::DifferenceAB
        | ExactBooleanOperation::OutsideA => Some(false),
        ExactBooleanOperation::Intersection
        | ExactBooleanOperation::DifferenceBA
        | ExactBooleanOperation::InsideA => Some(true),
        ExactBooleanOperation::InsideB | ExactBooleanOperation::OutsideB => None,
    }
}

fn second_need_inside(operation: ExactBooleanOperation) -> Option<bool> {
    match operation {
        ExactBooleanOperation::Union
        | ExactBooleanOperation::DifferenceBA
        | ExactBooleanOperation::OutsideB => Some(false),
        ExactBooleanOperation::Intersection
        | ExactBooleanOperation::DifferenceAB
        | ExactBooleanOperation::InsideB => Some(true),
        ExactBooleanOperation::InsideA | ExactBooleanOperation::OutsideA => None,
    }
}
