use super::{assemble_classified_boolean_with_stitch_at_tolerance, prepare_part_faces};
use crate::spatial::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperation};
use crate::spatial::exact_classify::{
    exact_classify_components_with_cut_paths, ExactCutPathClassificationInput,
    ExactMeshPartClassification,
};
use crate::spatial::exact_coplanar::same_oriented_coplanar_overlap_faces;
use crate::spatial::exact_cut_apply::ExactCutMeshResult;
use crate::spatial::exact_stitch::ExactStitchPlan;
use crate::GeometryError;
use std::collections::BTreeSet;

pub(in crate::spatial) fn exact_assemble_union_with_coplanar_first_wins(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    stitch_plan: Option<&ExactStitchPlan>,
    epsilon: f64,
) -> Result<ExactBooleanAssemblyResult, GeometryError> {
    let first_classification =
        exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
            vertices: &first.vertices,
            faces_i64: &first.faces,
            other_vertices: &second.vertices,
            other_faces_i64: &second.faces,
            cut_edges: &first.cut_edges,
            cut_edge_paths: &first.cut_edge_paths,
            need_inside: false,
            origin_is_first: true,
            epsilon,
        })?;
    let second_classification =
        exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
            vertices: &second.vertices,
            faces_i64: &second.faces,
            other_vertices: &first.vertices,
            other_faces_i64: &first.faces,
            cut_edges: &second.cut_edges,
            cut_edge_paths: &second.cut_edge_paths,
            need_inside: false,
            origin_is_first: false,
            epsilon,
        })?;
    let first_overlap_faces = same_oriented_coplanar_overlap_faces(
        &first.vertices,
        &first.faces,
        &second.vertices,
        &second.faces,
        epsilon,
    )?;
    let prepare_first_faces = prepare_part_faces(
        Some(&first_classification),
        &first_classification.selected_faces,
    );
    let prepare_second_faces = prepare_part_faces(
        Some(&second_classification),
        &second_classification.selected_faces,
    );
    let first_classification =
        coplanar_union_first_wins_classification(first_classification, &first_overlap_faces, true);
    let second_classification =
        coplanar_union_first_wins_classification(second_classification, &BTreeSet::new(), false);
    let mut assembly = assemble_classified_boolean_with_stitch_at_tolerance(
        first,
        second,
        Some(&first_classification),
        Some(&second_classification),
        stitch_plan,
        ExactBooleanOperation::Union,
        epsilon,
    );
    assembly.prepare_first_faces = prepare_first_faces;
    assembly.prepare_second_faces = prepare_second_faces;
    Ok(assembly)
}

pub(in crate::spatial) fn exact_assemble_intersection_with_coplanar_sampling(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    stitch_plan: Option<&ExactStitchPlan>,
    epsilon: f64,
) -> Result<ExactBooleanAssemblyResult, GeometryError> {
    let first_classification =
        exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
            vertices: &first.vertices,
            faces_i64: &first.faces,
            other_vertices: &second.vertices,
            other_faces_i64: &second.faces,
            cut_edges: &first.cut_edges,
            cut_edge_paths: &first.cut_edge_paths,
            need_inside: true,
            origin_is_first: true,
            epsilon,
        })?;
    let second_classification =
        exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
            vertices: &second.vertices,
            faces_i64: &second.faces,
            other_vertices: &first.vertices,
            other_faces_i64: &first.faces,
            cut_edges: &second.cut_edges,
            cut_edge_paths: &second.cut_edge_paths,
            need_inside: true,
            origin_is_first: false,
            epsilon,
        })?;
    let second_overlap_faces = same_oriented_coplanar_overlap_faces(
        &second.vertices,
        &second.faces,
        &first.vertices,
        &first.faces,
        epsilon,
    )?;
    let prepare_first_faces = prepare_part_faces(
        Some(&first_classification),
        &first_classification.selected_faces,
    );
    let prepare_second_faces = prepare_part_faces(
        Some(&second_classification),
        &second_classification.selected_faces,
    );
    let first_classification = coplanar_intersection_first_wins_classification(
        first_classification,
        &BTreeSet::new(),
        true,
    );
    let second_classification = coplanar_intersection_first_wins_classification(
        second_classification,
        &second_overlap_faces,
        false,
    );
    let mut assembly = assemble_classified_boolean_with_stitch_at_tolerance(
        first,
        second,
        Some(&first_classification),
        Some(&second_classification),
        stitch_plan,
        ExactBooleanOperation::Intersection,
        epsilon,
    );
    assembly.prepare_first_faces = prepare_first_faces;
    assembly.prepare_second_faces = prepare_second_faces;
    Ok(assembly)
}

pub(in crate::spatial) fn exact_assemble_difference_with_coplanar_sampling(
    first: &ExactCutMeshResult,
    second: &ExactCutMeshResult,
    stitch_plan: Option<&ExactStitchPlan>,
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> Result<ExactBooleanAssemblyResult, GeometryError> {
    let first_classification =
        exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
            vertices: &first.vertices,
            faces_i64: &first.faces,
            other_vertices: &second.vertices,
            other_faces_i64: &second.faces,
            cut_edges: &first.cut_edges,
            cut_edge_paths: &first.cut_edge_paths,
            need_inside: operation == ExactBooleanOperation::DifferenceBA,
            origin_is_first: true,
            epsilon,
        })?;
    let second_classification =
        exact_classify_components_with_cut_paths(ExactCutPathClassificationInput {
            vertices: &second.vertices,
            faces_i64: &second.faces,
            other_vertices: &first.vertices,
            other_faces_i64: &first.faces,
            cut_edges: &second.cut_edges,
            cut_edge_paths: &second.cut_edge_paths,
            need_inside: operation == ExactBooleanOperation::DifferenceAB,
            origin_is_first: false,
            epsilon,
        })?;

    let raw_prepare_first_faces = prepare_part_faces(
        Some(&first_classification),
        &first_classification.selected_faces,
    );
    let raw_prepare_second_faces = prepare_part_faces(
        Some(&second_classification),
        &second_classification.selected_faces,
    );
    let (first_classification, second_classification, prepare_first_faces, prepare_second_faces) =
        match operation {
            ExactBooleanOperation::DifferenceAB => {
                let second_overlap_faces = same_oriented_coplanar_overlap_faces(
                    &second.vertices,
                    &second.faces,
                    &first.vertices,
                    &first.faces,
                    epsilon,
                )?;
                let first_classification =
                    coplanar_difference_minuend_classification(first_classification);
                let second_classification = coplanar_difference_subtrahend_classification(
                    second_classification,
                    &second_overlap_faces,
                );
                (
                    first_classification,
                    second_classification,
                    raw_prepare_first_faces,
                    raw_prepare_second_faces,
                )
            }
            ExactBooleanOperation::DifferenceBA => {
                let first_overlap_faces = same_oriented_coplanar_overlap_faces(
                    &first.vertices,
                    &first.faces,
                    &second.vertices,
                    &second.faces,
                    epsilon,
                )?;
                let first_classification = coplanar_difference_subtrahend_classification(
                    first_classification,
                    &first_overlap_faces,
                );
                let second_classification =
                    coplanar_difference_minuend_classification(second_classification);
                (
                    first_classification,
                    second_classification,
                    raw_prepare_first_faces,
                    raw_prepare_second_faces,
                )
            }
            _ => unreachable!("coplanar difference assembly only accepts difference operations"),
        };

    let mut assembly = assemble_classified_boolean_with_stitch_at_tolerance(
        first,
        second,
        Some(&first_classification),
        Some(&second_classification),
        stitch_plan,
        operation,
        epsilon,
    );
    assembly.prepare_first_faces = prepare_first_faces;
    assembly.prepare_second_faces = prepare_second_faces;
    Ok(assembly)
}

fn coplanar_union_first_wins_classification(
    mut classification: ExactMeshPartClassification,
    first_overlap_faces: &BTreeSet<usize>,
    keep_same_oriented_overlap: bool,
) -> ExactMeshPartClassification {
    let mut selected_faces = BTreeSet::new();
    for component in &mut classification.components {
        let has_same_oriented_overlap = component
            .face_indices
            .iter()
            .any(|face| first_overlap_faces.contains(face));
        component.selected =
            !component.inside_other || (keep_same_oriented_overlap && has_same_oriented_overlap);
        if component.selected {
            selected_faces.extend(component.face_indices.iter().copied());
        }
    }
    classification.selected_faces = selected_faces.into_iter().collect();
    classification
}

fn coplanar_intersection_first_wins_classification(
    mut classification: ExactMeshPartClassification,
    same_oriented_overlap_faces: &BTreeSet<usize>,
    keep_same_oriented_overlap: bool,
) -> ExactMeshPartClassification {
    let mut selected_faces = BTreeSet::new();
    for component in &mut classification.components {
        let has_same_oriented_overlap = component
            .face_indices
            .iter()
            .any(|face| same_oriented_overlap_faces.contains(face));
        component.selected =
            component.inside_other && (keep_same_oriented_overlap || !has_same_oriented_overlap);
        if component.selected {
            selected_faces.extend(component.face_indices.iter().copied());
        }
    }
    classification.selected_faces = selected_faces.into_iter().collect();
    classification
}

fn coplanar_difference_minuend_classification(
    mut classification: ExactMeshPartClassification,
) -> ExactMeshPartClassification {
    let mut selected_faces = BTreeSet::new();
    for component in &mut classification.components {
        component.selected = !component.inside_other;
        if component.selected {
            selected_faces.extend(component.face_indices.iter().copied());
        }
    }
    classification.selected_faces = selected_faces.into_iter().collect();
    classification
}

fn coplanar_difference_subtrahend_classification(
    mut classification: ExactMeshPartClassification,
    same_oriented_overlap_faces: &BTreeSet<usize>,
) -> ExactMeshPartClassification {
    let mut selected_faces = BTreeSet::new();
    for component in &mut classification.components {
        let has_same_oriented_overlap = component
            .face_indices
            .iter()
            .any(|face| same_oriented_overlap_faces.contains(face));
        component.selected = component.inside_other && !has_same_oriented_overlap;
        if component.selected {
            selected_faces.extend(component.face_indices.iter().copied());
        }
    }
    classification.selected_faces = selected_faces.into_iter().collect();
    classification
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{mesh_health, mesh_stats};
    use crate::spatial::exact_boolean_diagnostics::meshlib::{
        meshlib_prepared_base_export, MeshlibRewriteDiagnosticsInput,
    };
    use crate::spatial::exact_cut_pair::exact_mesh_pair_cut_meshes;
    use crate::spatial::exact_fill_apply::exact_fill_cut_holes;
    use crate::spatial::exact_stitch::exact_stitch_plan_from_cut_meshes;

    fn cube() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
        let vertices = vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        let faces = vec![
            [0, 3, 2],
            [0, 2, 1],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        (vertices, faces)
    }

    #[test]
    fn paired_difference_prepare_masks_preserve_meshlib_prepare_part_faces() {
        let (first_vertices, faces) = cube();
        let second_vertices = first_vertices
            .iter()
            .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
            .collect::<Vec<_>>();
        let cut_meshes =
            exact_mesh_pair_cut_meshes(&first_vertices, &faces, &second_vertices, &faces, 8, 1e-9)
                .unwrap();
        let paired = cut_meshes
            .paired_coplanar_candidate
            .expect("shifted cubes should produce a paired coplanar candidate");
        let first_cut = exact_fill_cut_holes(&paired.first, 1e-9).unwrap();
        let second_cut = exact_fill_cut_holes(&paired.second, 1e-9).unwrap();
        let stitch_plan =
            exact_stitch_plan_from_cut_meshes(&first_cut.mesh, &second_cut.mesh, 1e-9);
        let assembly = exact_assemble_difference_with_coplanar_sampling(
            &first_cut.mesh,
            &second_cut.mesh,
            Some(&stitch_plan),
            ExactBooleanOperation::DifferenceAB,
            1e-9,
        )
        .unwrap();

        assert_eq!(assembly.prepare_first_faces.len(), 20);
        assert_eq!(assembly.prepare_second_faces.len(), 16);
        assert_eq!(assembly.selected_first_faces.len(), 14);
        assert_eq!(assembly.selected_second_faces.len(), 6);
    }

    #[test]
    fn paired_difference_prepared_export_pins_unstitched_prepare_part_gap() {
        let (first_vertices, faces) = cube();
        let second_vertices = first_vertices
            .iter()
            .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
            .collect::<Vec<_>>();
        let cut_meshes =
            exact_mesh_pair_cut_meshes(&first_vertices, &faces, &second_vertices, &faces, 8, 1e-9)
                .unwrap();
        let paired = cut_meshes
            .paired_coplanar_candidate
            .expect("shifted cubes should produce a paired coplanar candidate");
        let first_cut = exact_fill_cut_holes(&paired.first, 1e-9).unwrap();
        let second_cut = exact_fill_cut_holes(&paired.second, 1e-9).unwrap();
        let stitch_plan =
            exact_stitch_plan_from_cut_meshes(&first_cut.mesh, &second_cut.mesh, 1e-9);
        let assembly = exact_assemble_difference_with_coplanar_sampling(
            &first_cut.mesh,
            &second_cut.mesh,
            Some(&stitch_plan),
            ExactBooleanOperation::DifferenceAB,
            1e-9,
        )
        .unwrap();

        let export = meshlib_prepared_base_export(MeshlibRewriteDiagnosticsInput {
            first_cut: &first_cut.mesh,
            second_cut: &second_cut.mesh,
            first_added_face_ranges: &first_cut.added_face_ranges,
            second_added_face_ranges: &second_cut.added_face_ranges,
            stitch_plan: None,
            assembly: &assembly,
            operation: ExactBooleanOperation::DifferenceAB,
            epsilon: 1e-9,
        })
        .unwrap()
        .expect("paired difference prepared-base export should be topology-ready");
        let stats = mesh_stats(&export.vertices, &export.faces).unwrap();
        let health = mesh_health(&export.vertices, &export.faces, true, None, 1e-9).unwrap();

        assert_eq!(stats.face_count, 36);
        assert_eq!(health.boundary_edge_count, 32);
        assert_eq!(health.nonmanifold_edge_count, 0);
        assert!(!health.is_closed);
        assert!((stats.surface_area_mm2 - 24.0).abs() < 1e-6);
        assert!((stats.volume_mm3 - 8.0 / 3.0).abs() < 1e-6);
    }
}
