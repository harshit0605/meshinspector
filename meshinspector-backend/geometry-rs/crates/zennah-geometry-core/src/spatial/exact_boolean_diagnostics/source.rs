use super::super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use super::super::exact_coplanar::same_oriented_coplanar_overlap_faces;
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct FaceSourceSummary {
    pub selected_first_faces: usize,
    pub selected_second_faces: usize,
    pub first_source_face_groups: usize,
    pub second_source_face_groups: usize,
    pub duplicate_first_source_faces: usize,
    pub duplicate_second_source_faces: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct RawFaceSelectionSummary {
    pub raw_selected_faces: [usize; 2],
    pub overlap_faces: [usize; 2],
    pub boundary_misses: [[usize; 2]; 2],
    pub selection_delta_faces: [i64; 2],
}

pub(super) fn face_source_summary(assembly: &ExactBooleanAssemblyResult) -> FaceSourceSummary {
    let (first_source_face_groups, duplicate_first_source_faces) =
        duplicate_source_faces(assembly, ExactBooleanOperand::First);
    let (second_source_face_groups, duplicate_second_source_faces) =
        duplicate_source_faces(assembly, ExactBooleanOperand::Second);
    FaceSourceSummary {
        selected_first_faces: assembly.selected_first_faces.len(),
        selected_second_faces: assembly.selected_second_faces.len(),
        first_source_face_groups,
        second_source_face_groups,
        duplicate_first_source_faces,
        duplicate_second_source_faces,
    }
}

pub(super) fn raw_face_selection_summary(
    input: &super::ExactBooleanPipelineDiagnosticInputs<'_>,
    actual: &FaceSourceSummary,
) -> Result<RawFaceSelectionSummary, GeometryError> {
    let first = input.first_cut;
    let second = input.second_cut;
    let first_prepare_faces = &input.assembly.prepare_first_faces;
    let second_prepare_faces = &input.assembly.prepare_second_faces;
    let first_raw = first_prepare_faces.len();
    let second_raw = second_prepare_faces.len();
    let first_overlap_faces = same_oriented_coplanar_overlap_faces(
        &first.mesh.vertices,
        &first.mesh.faces,
        &second.mesh.vertices,
        &second.mesh.faces,
        input.epsilon,
    )?
    .len();
    let second_overlap_faces = same_oriented_coplanar_overlap_faces(
        &second.mesh.vertices,
        &second.mesh.faces,
        &first.mesh.vertices,
        &first.mesh.faces,
        input.epsilon,
    )?
    .len();
    Ok(RawFaceSelectionSummary {
        raw_selected_faces: [first_raw, second_raw],
        overlap_faces: [first_overlap_faces, second_overlap_faces],
        boundary_misses: boundary_misses(input, first_prepare_faces, second_prepare_faces),
        selection_delta_faces: [
            actual.selected_first_faces as i64 - first_raw as i64,
            actual.selected_second_faces as i64 - second_raw as i64,
        ],
    })
}

fn boundary_misses(
    input: &super::ExactBooleanPipelineDiagnosticInputs<'_>,
    first_raw_faces: &[usize],
    second_raw_faces: &[usize],
) -> [[usize; 2]; 2] {
    [
        [
            contour_boundary_misses(
                &input.first_cut.mesh.faces,
                first_raw_faces,
                input
                    .first_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
            contour_boundary_misses(
                &input.second_cut.mesh.faces,
                second_raw_faces,
                input
                    .second_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
        ],
        [
            contour_boundary_misses(
                &input.first_cut.mesh.faces,
                &input.assembly.selected_first_faces,
                input
                    .first_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
            contour_boundary_misses(
                &input.second_cut.mesh.faces,
                &input.assembly.selected_second_faces,
                input
                    .second_cut
                    .mesh
                    .cut_edge_paths
                    .iter()
                    .flatten()
                    .copied(),
            ),
        ],
    ]
}

pub(super) fn duplicate_face_counts(faces: &[[i64; 3]]) -> (usize, usize) {
    let mut face_keys = BTreeMap::<[i64; 3], usize>::new();
    for face in faces {
        let mut key = *face;
        key.sort_unstable();
        *face_keys.entry(key).or_default() += 1;
    }
    let duplicate_groups = face_keys.values().filter(|&&count| count > 1).count();
    let duplicate_faces = face_keys
        .values()
        .filter(|&&count| count > 1)
        .map(|count| count - 1)
        .sum();
    (duplicate_groups, duplicate_faces)
}

fn contour_boundary_misses(
    faces: &[[i64; 3]],
    selected_faces: &[usize],
    contour_edges: impl Iterator<Item = [usize; 2]>,
) -> usize {
    let selected_faces = selected_faces.iter().copied().collect::<BTreeSet<_>>();
    contour_edges
        .filter(|edge| selected_edge_incidence(faces, &selected_faces, *edge) != 1)
        .count()
}

fn selected_edge_incidence(
    faces: &[[i64; 3]],
    selected_faces: &BTreeSet<usize>,
    edge: [usize; 2],
) -> usize {
    let edge = ordered_edge(edge);
    selected_faces
        .iter()
        .filter(|&&face_index| {
            faces
                .get(face_index)
                .map(|face| face_has_edge(*face, edge))
                .unwrap_or(false)
        })
        .count()
}

fn face_has_edge(face: [i64; 3], edge: [usize; 2]) -> bool {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
        .into_iter()
        .map(|edge| [edge[0] as usize, edge[1] as usize])
        .any(|candidate| ordered_edge(candidate) == edge)
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn duplicate_source_faces(
    assembly: &ExactBooleanAssemblyResult,
    operand: ExactBooleanOperand,
) -> (usize, usize) {
    let mut source_faces = BTreeMap::<usize, usize>::new();
    for source in &assembly.face_sources {
        if source.operand == operand {
            *source_faces.entry(source.source_face).or_default() += 1;
        }
    }
    let duplicate_groups = source_faces.values().filter(|&&count| count > 1).count();
    let duplicate_faces = source_faces
        .values()
        .filter(|&&count| count > 1)
        .map(|count| count - 1)
        .sum();
    (duplicate_groups, duplicate_faces)
}
