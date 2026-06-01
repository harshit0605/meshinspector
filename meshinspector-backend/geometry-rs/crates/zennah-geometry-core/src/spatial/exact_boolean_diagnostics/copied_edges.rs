use super::super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use super::super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_meshlib_near_stitch::ExactMeshlibNearStitchSourceInput;
use super::super::exact_splice_apply::ExactMeshlibCopiedEdgeTranslationInput;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactMeshlibCopiedEdgePlan {
    pub prepared_faces: usize,
    pub prepared_vertices: usize,
    pub virtual_copied_vertices: usize,
    pub prepared_edges: usize,
    pub mapped_edges: usize,
    pub copied_edges: usize,
    pub copied_edges_mapped_to_existing_output: usize,
    pub copied_edges_mapped_to_output: usize,
    pub copied_edges_missing_output_vertices: usize,
}

impl ExactMeshlibCopiedEdgePlan {
    pub(super) fn ready_for_record_translation(&self) -> bool {
        self.copied_edges_missing_output_vertices == 0
    }
}

pub(super) fn exact_meshlib_copied_edge_plan(
    first_cut: &ExactCutMeshResult,
    second_cut: &ExactCutMeshResult,
    assembly: &ExactBooleanAssemblyResult,
    incoming_operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
) -> ExactMeshlibCopiedEdgePlan {
    let (cut_mesh, prepared_faces, vertex_map) = match incoming_operand {
        ExactBooleanOperand::First => (
            first_cut,
            &assembly.prepare_first_faces,
            &assembly.first_output_vertex_for_cut_vertex,
        ),
        ExactBooleanOperand::Second => (
            second_cut,
            &assembly.prepare_second_faces,
            &assembly.second_output_vertex_for_cut_vertex,
        ),
    };
    let prepared_vertices = prepared_region_vertices(cut_mesh, prepared_faces);
    let prepared_edges = prepared_region_edges(cut_mesh, prepared_faces);
    let mapped_edges = mapped_incoming_edges(assembly, incoming_operand, record_commands);
    let copied_edges = prepared_edges
        .difference(&mapped_edges)
        .copied()
        .collect::<BTreeSet<_>>();
    let copied_edges_mapped_to_existing_output = copied_edges
        .iter()
        .filter(|edge| output_edge_for_cut_edge(**edge, vertex_map).is_some())
        .count();
    let contour_vertex_maps = contour_vertex_maps(assembly, incoming_operand, record_commands);
    let copied_vertex_map = copied_vertex_map(
        vertex_map,
        &prepared_vertices,
        assembly.vertices.len(),
        &contour_vertex_maps,
    );
    let copied_edges_mapped_to_output = copied_edges
        .iter()
        .filter(|edge| output_edge_for_cut_edge(**edge, &copied_vertex_map).is_some())
        .count();

    ExactMeshlibCopiedEdgePlan {
        prepared_faces: prepared_faces.len(),
        prepared_vertices: prepared_vertices.len(),
        virtual_copied_vertices: copied_vertex_map
            .iter()
            .zip(vertex_map.iter().copied().chain(std::iter::repeat(None)))
            .filter(|(copied, original)| copied.is_some() && original.is_none())
            .count(),
        prepared_edges: prepared_edges.len(),
        mapped_edges: mapped_edges.len(),
        copied_edges: copied_edges.len(),
        copied_edges_mapped_to_existing_output,
        copied_edges_mapped_to_output,
        copied_edges_missing_output_vertices: copied_edges
            .len()
            .saturating_sub(copied_edges_mapped_to_output),
    }
}

pub(super) fn exact_meshlib_copied_edge_translation_input<'a>(
    first_cut: &'a ExactCutMeshResult,
    second_cut: &'a ExactCutMeshResult,
    assembly: &'a ExactBooleanAssemblyResult,
    incoming_operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
) -> ExactMeshlibCopiedEdgeTranslationInput<'a> {
    let (cut_mesh, prepared_faces, vertex_map, flip_orientation) = match incoming_operand {
        ExactBooleanOperand::First => (
            first_cut,
            assembly.prepare_first_faces.as_slice(),
            assembly.first_output_vertex_for_cut_vertex.as_slice(),
            assembly.flipped_first,
        ),
        ExactBooleanOperand::Second => (
            second_cut,
            assembly.prepare_second_faces.as_slice(),
            assembly.second_output_vertex_for_cut_vertex.as_slice(),
            assembly.flipped_second,
        ),
    };
    ExactMeshlibCopiedEdgeTranslationInput {
        cut_mesh,
        prepared_faces,
        vertex_map,
        contour_vertex_maps: contour_vertex_maps(assembly, incoming_operand, record_commands),
        contour_vertex_map_source_indices: contour_vertex_map_source_indices(
            assembly,
            incoming_operand,
            record_commands,
        ),
        face_sources: &assembly.face_sources,
        incoming_operand,
        first_virtual_vertex: assembly.vertices.len(),
        append_prepared_faces: false,
        flip_orientation,
    }
}

pub(super) fn exact_meshlib_near_stitch_source_input<'a>(
    first_cut: &'a ExactCutMeshResult,
    second_cut: &'a ExactCutMeshResult,
    assembly: &'a ExactBooleanAssemblyResult,
    incoming_operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
) -> ExactMeshlibNearStitchSourceInput<'a> {
    let (cut_mesh, prepared_faces, vertex_map) = match incoming_operand {
        ExactBooleanOperand::First => (
            first_cut,
            assembly.prepare_first_faces.as_slice(),
            assembly.first_output_vertex_for_cut_vertex.as_slice(),
        ),
        ExactBooleanOperand::Second => (
            second_cut,
            assembly.prepare_second_faces.as_slice(),
            assembly.second_output_vertex_for_cut_vertex.as_slice(),
        ),
    };
    ExactMeshlibNearStitchSourceInput {
        cut_mesh,
        prepared_faces,
        vertex_map,
        contour_vertex_maps: contour_vertex_maps(assembly, incoming_operand, record_commands),
        contour_vertex_map_source_indices: contour_vertex_map_source_indices(
            assembly,
            incoming_operand,
            record_commands,
        ),
        first_virtual_vertex: assembly.vertices.len(),
        flip_orientation: match incoming_operand {
            ExactBooleanOperand::First => assembly.flipped_first,
            ExactBooleanOperand::Second => assembly.flipped_second,
        },
    }
}

fn prepared_region_vertices(
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
) -> BTreeSet<usize> {
    let mut vertices = BTreeSet::new();
    for face_index in prepared_faces {
        let Some(face) = cut_mesh.faces.get(*face_index) else {
            continue;
        };
        vertices.extend(face.iter().map(|vertex| *vertex as usize));
    }
    vertices
}

fn prepared_region_edges(
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
) -> BTreeSet<[usize; 2]> {
    let mut edges = BTreeSet::new();
    for face_index in prepared_faces {
        let Some(face) = cut_mesh.faces.get(*face_index) else {
            continue;
        };
        let face = [face[0] as usize, face[1] as usize, face[2] as usize];
        for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
            edges.insert(ordered_edge(edge));
        }
    }
    edges
}

fn mapped_incoming_edges(
    assembly: &ExactBooleanAssemblyResult,
    incoming_operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
) -> BTreeSet<[usize; 2]> {
    record_commands
        .iter()
        .filter(|command| command.from_operand == incoming_operand)
        .filter_map(|command| {
            assembly
                .stitched_edge_sources
                .get(command.stitch_pair_index)
        })
        .map(|source| match incoming_operand {
            ExactBooleanOperand::First => source.first_cut_edge,
            ExactBooleanOperand::Second => source.second_cut_edge,
        })
        .map(ordered_edge)
        .collect()
}

fn copied_vertex_map(
    vertex_map: &[Option<usize>],
    prepared_vertices: &BTreeSet<usize>,
    first_virtual_vertex: usize,
    contour_vertex_maps: &[([usize; 2], [usize; 2])],
) -> Vec<Option<usize>> {
    let mut copied_map = vertex_map.to_vec();
    for (source_edge, output_edge) in contour_vertex_maps {
        set_copied_vertex(&mut copied_map, source_edge[0], output_edge[0]);
        set_copied_vertex(&mut copied_map, source_edge[1], output_edge[1]);
    }
    let mut next_virtual_vertex = first_virtual_vertex;
    for vertex in prepared_vertices {
        if copied_map.len() <= *vertex {
            copied_map.resize(*vertex + 1, None);
        }
        if copied_map[*vertex].is_none() {
            copied_map[*vertex] = Some(next_virtual_vertex);
            next_virtual_vertex += 1;
        }
    }
    copied_map
}

fn contour_vertex_maps(
    assembly: &ExactBooleanAssemblyResult,
    operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
) -> Vec<([usize; 2], [usize; 2])> {
    record_commands_in_meshlib_contour_order(assembly, record_commands)
        .into_iter()
        .filter_map(|command| {
            let source = assembly
                .stitched_edge_sources
                .get(command.stitch_pair_index)?;
            let source_edge = if command.from_operand == operand {
                match operand {
                    ExactBooleanOperand::First => source.first_cut_edge,
                    ExactBooleanOperand::Second => source.second_cut_edge,
                }
            } else if command.this_operand == operand {
                command.this_source_edge
            } else {
                return None;
            };
            let output_edge = if command.from_operand == operand {
                meshlib_target_contour_edge(command)
            } else {
                command.this_contour_edge
            };
            Some((source_edge, output_edge))
        })
        .collect()
}

fn contour_vertex_map_source_indices(
    assembly: &ExactBooleanAssemblyResult,
    operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
) -> Vec<Option<usize>> {
    record_commands_in_meshlib_contour_order(assembly, record_commands)
        .into_iter()
        .filter_map(|command| {
            let _ = assembly
                .stitched_edge_sources
                .get(command.stitch_pair_index)?;
            if command.from_operand == operand {
                Some(Some(command.from_source_edge_index))
            } else if command.this_operand == operand {
                Some(Some(command.this_source_edge_index))
            } else {
                None
            }
        })
        .collect()
}

fn record_commands_in_meshlib_contour_order<'a>(
    assembly: &ExactBooleanAssemblyResult,
    record_commands: &'a [ExactMeshlibRecordRewriteCommand],
) -> Vec<&'a ExactMeshlibRecordRewriteCommand> {
    let mut ordered = Vec::with_capacity(record_commands.len());
    let mut seen = BTreeSet::new();
    for path in &assembly.stitched_edge_paths {
        for pair_index in &path.pair_indices {
            if !seen.insert(*pair_index) {
                continue;
            }
            if let Some(command) = record_commands
                .iter()
                .find(|command| command.stitch_pair_index == *pair_index)
            {
                ordered.push(command);
            }
        }
    }
    for command in record_commands {
        if !seen.contains(&command.stitch_pair_index) {
            ordered.push(command);
        }
    }
    ordered
}

fn meshlib_target_contour_edge(command: &ExactMeshlibRecordRewriteCommand) -> [usize; 2] {
    if command.this_side_synthetic {
        command.this_contour_edge
    } else {
        reverse_edge(command.this_contour_edge)
    }
}

fn set_copied_vertex(copied_map: &mut Vec<Option<usize>>, source: usize, output: usize) {
    if copied_map.len() <= source {
        copied_map.resize(source + 1, None);
    }
    if copied_map[source].is_none() {
        copied_map[source] = Some(output);
    }
}

fn output_edge_for_cut_edge(
    cut_edge: [usize; 2],
    vertex_map: &[Option<usize>],
) -> Option<[usize; 2]> {
    Some([
        *vertex_map.get(cut_edge[0])?.as_ref()?,
        *vertex_map.get(cut_edge[1])?.as_ref()?,
    ])
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

#[cfg(test)]
mod tests {
    use super::super::super::exact_boolean::{
        ExactBooleanOutputFaceSource, ExactBooleanStitchedEdgeSource,
    };
    use super::super::super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
    use super::super::super::exact_stitch::ExactStitchPath;
    use super::*;

    fn cut_mesh() -> ExactCutMeshResult {
        ExactCutMeshResult {
            vertices: vec![[0.0; 3]; 4],
            faces: vec![[0, 1, 2], [2, 1, 3]],
            cut_edges: Vec::new(),
            cut_edge_paths: Vec::new(),
            cut_edge_path_closed: Vec::new(),
            source_face_for_faces: vec![0, 1],
            skipped_source_faces: Vec::new(),
        }
    }

    fn assembly() -> ExactBooleanAssemblyResult {
        ExactBooleanAssemblyResult {
            vertices: vec![[0.0; 3]; 4],
            faces: vec![[0, 1, 2], [2, 1, 3]],
            face_sources: vec![
                ExactBooleanOutputFaceSource {
                    operand: ExactBooleanOperand::First,
                    cut_face: 0,
                    source_face: 0,
                },
                ExactBooleanOutputFaceSource {
                    operand: ExactBooleanOperand::Second,
                    cut_face: 1,
                    source_face: 1,
                },
            ],
            first_output_vertex_for_cut_vertex: vec![Some(0), Some(1), Some(2), None],
            second_output_vertex_for_cut_vertex: vec![None, Some(1), Some(2), Some(3)],
            stitched_edge_sources: vec![ExactBooleanStitchedEdgeSource {
                output_edge: [1, 2],
                first_output_edge: Some([1, 2]),
                second_output_edge: Some([2, 1]),
                first_stitch_edge: Some([1, 2]),
                second_stitch_edge: Some([2, 1]),
                first_stitch_edge_synthetic: false,
                second_stitch_edge_synthetic: false,
                first_edge_index: 0,
                second_edge_index: 0,
                first_cut_edge: [1, 2],
                second_cut_edge: [2, 1],
            }],
            stitched_edge_paths: Vec::new(),
            prepare_first_faces: vec![0],
            prepare_second_faces: vec![1],
            selected_first_faces: vec![0],
            selected_second_faces: vec![1],
            flipped_first: false,
            flipped_second: false,
            first_cut_paths_consistent: true,
            second_cut_paths_consistent: true,
            first_cut_path_side_components: [1, 1],
            second_cut_path_side_components: [1, 1],
            first_cut_path_overlap_components: 0,
            second_cut_path_overlap_components: 0,
            result_cut_paths: Vec::new(),
            result_cut_path_closed: Vec::new(),
            result_cut_paths_complete: true,
        }
    }

    #[test]
    fn copied_edge_plan_subtracts_mapped_incoming_edges() {
        let first = cut_mesh();
        let second = cut_mesh();
        let assembly = assembly();
        let command = ExactMeshlibRecordRewriteCommand {
            stitch_pair_index: 0,
            this_operand: ExactBooleanOperand::First,
            from_operand: ExactBooleanOperand::Second,
            output_edge: [1, 2],
            this_contour_edge: [1, 2],
            from_contour_edge: [2, 1],
            this_source_edge_index: 0,
            from_source_edge_index: 0,
            this_source_edge: [1, 2],
            from_source_edge: [2, 1],
            this_side_synthetic: false,
            from_side_synthetic: false,
            synthetic_sides: 0,
        };

        let plan = exact_meshlib_copied_edge_plan(
            &first,
            &second,
            &assembly,
            ExactBooleanOperand::Second,
            &[command],
        );

        assert_eq!(plan.prepared_faces, 1);
        assert_eq!(plan.prepared_vertices, 3);
        assert_eq!(plan.virtual_copied_vertices, 0);
        assert_eq!(plan.prepared_edges, 3);
        assert_eq!(plan.mapped_edges, 1);
        assert_eq!(plan.copied_edges, 2);
        assert_eq!(plan.copied_edges_mapped_to_existing_output, 2);
        assert_eq!(plan.copied_edges_mapped_to_output, 2);
        assert_eq!(plan.copied_edges_missing_output_vertices, 0);
        assert!(plan.ready_for_record_translation());
    }

    #[test]
    fn contour_vertex_maps_seed_this_and_from_operands() {
        let assembly = assembly();
        let command = ExactMeshlibRecordRewriteCommand {
            stitch_pair_index: 0,
            this_operand: ExactBooleanOperand::First,
            from_operand: ExactBooleanOperand::Second,
            output_edge: [1, 2],
            this_contour_edge: [1, 2],
            from_contour_edge: [2, 1],
            this_source_edge_index: 0,
            from_source_edge_index: 0,
            this_source_edge: [1, 2],
            from_source_edge: [2, 1],
            this_side_synthetic: false,
            from_side_synthetic: false,
            synthetic_sides: 0,
        };

        assert_eq!(
            contour_vertex_maps(&assembly, ExactBooleanOperand::First, &[command]),
            vec![([1, 2], [1, 2])]
        );
        assert_eq!(
            contour_vertex_maps(&assembly, ExactBooleanOperand::Second, &[command]),
            vec![([2, 1], [2, 1])]
        );
    }

    #[test]
    fn contour_vertex_maps_follow_meshlib_path_pair_order() {
        let mut assembly = assembly();
        assembly
            .stitched_edge_sources
            .push(ExactBooleanStitchedEdgeSource {
                output_edge: [0, 3],
                first_output_edge: Some([0, 3]),
                second_output_edge: Some([3, 0]),
                first_stitch_edge: Some([0, 3]),
                second_stitch_edge: Some([3, 0]),
                first_stitch_edge_synthetic: false,
                second_stitch_edge_synthetic: false,
                first_edge_index: 1,
                second_edge_index: 1,
                first_cut_edge: [0, 3],
                second_cut_edge: [3, 0],
            });
        assembly.stitched_edge_paths = vec![ExactStitchPath {
            pair_indices: vec![1, 0],
            closed: false,
        }];
        let commands = [
            ExactMeshlibRecordRewriteCommand {
                stitch_pair_index: 0,
                this_operand: ExactBooleanOperand::First,
                from_operand: ExactBooleanOperand::Second,
                output_edge: [1, 2],
                this_contour_edge: [1, 2],
                from_contour_edge: [2, 1],
                this_source_edge_index: 0,
                from_source_edge_index: 0,
                this_source_edge: [1, 2],
                from_source_edge: [2, 1],
                this_side_synthetic: false,
                from_side_synthetic: false,
                synthetic_sides: 0,
            },
            ExactMeshlibRecordRewriteCommand {
                stitch_pair_index: 1,
                this_operand: ExactBooleanOperand::First,
                from_operand: ExactBooleanOperand::Second,
                output_edge: [0, 3],
                this_contour_edge: [0, 3],
                from_contour_edge: [3, 0],
                this_source_edge_index: 1,
                from_source_edge_index: 1,
                this_source_edge: [0, 3],
                from_source_edge: [3, 0],
                this_side_synthetic: false,
                from_side_synthetic: false,
                synthetic_sides: 0,
            },
        ];

        assert_eq!(
            contour_vertex_maps(&assembly, ExactBooleanOperand::Second, &commands),
            vec![([3, 0], [3, 0]), ([2, 1], [2, 1])]
        );
        assert_eq!(
            contour_vertex_map_source_indices(&assembly, ExactBooleanOperand::Second, &commands),
            vec![Some(1), Some(0)]
        );
    }
}
