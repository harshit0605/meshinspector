use super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOperand, ExactBooleanOperation,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactMeshlibTopologyRewritePlan {
    pub base_operand: ExactBooleanOperand,
    pub incoming_operand: ExactBooleanOperand,
    pub base_faces: usize,
    pub incoming_faces: usize,
    pub stitched_edges: usize,
    pub mapped_contour_edges: usize,
    pub missing_base_contour_edges: usize,
    pub missing_incoming_contour_edges: usize,
    pub contour_direction_mismatches: usize,
    pub mapped_stitch_contour_edges: usize,
    pub missing_stitch_contour_edges: usize,
    pub synthetic_stitch_contour_edges: usize,
    pub stitch_direction_mismatches: usize,
    pub stitch_metadata_ready: bool,
    pub materialized_stitch_contour_edges: usize,
    pub unmaterialized_stitch_contour_edges: usize,
    pub materialized_synthetic_stitch_sides: usize,
    pub stitch_materialization_direction_mismatches: usize,
    pub stitch_materialization_ready: bool,
    pub record_rewrite_commands: usize,
    pub record_rewrite_blocked_edges: usize,
    pub record_rewrite_synthetic_sides: usize,
    pub record_rewrite_direction_mismatches: usize,
    pub record_rewrite_ready: bool,
    pub record_rewrite_command_edges: Vec<ExactMeshlibRecordRewriteCommand>,
    pub open_stitch_paths: usize,
    pub open_stitch_near_edge_updates: usize,
    pub open_stitch_near_edge_blocked_updates: usize,
    pub open_stitch_near_edge_ready: bool,
    pub ready_for_rewrite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactMeshlibRecordRewriteCommand {
    pub stitch_pair_index: usize,
    pub this_operand: ExactBooleanOperand,
    pub from_operand: ExactBooleanOperand,
    pub output_edge: [usize; 2],
    pub this_contour_edge: [usize; 2],
    pub from_contour_edge: [usize; 2],
    pub this_source_edge_index: usize,
    pub from_source_edge_index: usize,
    pub this_source_edge: [usize; 2],
    pub from_source_edge: [usize; 2],
    pub this_side_synthetic: bool,
    pub from_side_synthetic: bool,
    pub synthetic_sides: usize,
}

pub(super) fn meshlib_topology_rewrite_plan(
    assembly: &ExactBooleanAssemblyResult,
    operation: ExactBooleanOperation,
) -> ExactMeshlibTopologyRewritePlan {
    let (base_operand, incoming_operand) = meshlib_connect_order(operation);
    let base_faces = prepared_face_count(assembly, base_operand);
    let incoming_faces = prepared_face_count(assembly, incoming_operand);
    let directed_edges = directed_face_edges_by_operand(assembly);
    let mut mapped_contour_edges = 0;
    let mut missing_base_contour_edges = 0;
    let mut missing_incoming_contour_edges = 0;
    let mut contour_direction_mismatches = 0;
    let mut mapped_stitch_contour_edges = 0;
    let mut missing_stitch_contour_edges = 0;
    let mut synthetic_stitch_contour_edges = 0;
    let mut stitch_direction_mismatches = 0;
    let mut materialized_stitch_contour_edges = 0;
    let mut unmaterialized_stitch_contour_edges = 0;
    let mut materialized_synthetic_stitch_sides = 0;
    let mut stitch_materialization_direction_mismatches = 0;
    let mut record_rewrite_command_edges = Vec::new();
    let mut materialized_stitch_pairs = vec![false; assembly.stitched_edge_sources.len()];

    for (source_index, source) in assembly.stitched_edge_sources.iter().enumerate() {
        let base_edge = directed_contour_edge(&directed_edges, base_operand, source.output_edge);
        let incoming_edge =
            directed_contour_edge(&directed_edges, incoming_operand, source.output_edge);
        if base_edge.is_none() {
            missing_base_contour_edges += 1;
        }
        if incoming_edge.is_none() {
            missing_incoming_contour_edges += 1;
        }
        if let (Some(base_edge), Some(incoming_edge)) = (base_edge, incoming_edge) {
            mapped_contour_edges += 1;
            if reverse_edge(base_edge) != incoming_edge {
                contour_direction_mismatches += 1;
            }
        }
        let base_stitch_edge = operand_stitch_edge(source, base_operand);
        let incoming_stitch_edge = operand_stitch_edge(source, incoming_operand);
        if base_stitch_edge.is_none() || incoming_stitch_edge.is_none() {
            missing_stitch_contour_edges += 1;
        }
        synthetic_stitch_contour_edges += operand_stitch_edge_synthetic(source, base_operand)
            + operand_stitch_edge_synthetic(source, incoming_operand);
        if let (Some(base_stitch_edge), Some(incoming_stitch_edge)) =
            (base_stitch_edge, incoming_stitch_edge)
        {
            mapped_stitch_contour_edges += 1;
            if reverse_edge(base_stitch_edge) != incoming_stitch_edge {
                stitch_direction_mismatches += 1;
            }
        }
        let materialization = materialized_contour_pair(
            source,
            base_operand,
            incoming_operand,
            base_edge,
            incoming_edge,
        );
        match materialization {
            ExactMeshlibMaterializedContourPair::Ready {
                base_edge,
                incoming_edge,
                base_synthetic,
                incoming_synthetic,
            } => {
                materialized_stitch_contour_edges += 1;
                materialized_synthetic_stitch_sides +=
                    base_synthetic as usize + incoming_synthetic as usize;
                if reverse_edge(base_edge) != incoming_edge {
                    stitch_materialization_direction_mismatches += 1;
                }
                materialized_stitch_pairs[source_index] = true;
                record_rewrite_command_edges.push(ExactMeshlibRecordRewriteCommand {
                    stitch_pair_index: source_index,
                    this_operand: base_operand,
                    from_operand: incoming_operand,
                    output_edge: source.output_edge,
                    this_contour_edge: base_edge,
                    from_contour_edge: incoming_edge,
                    this_source_edge_index: operand_source_edge_index(source, base_operand),
                    from_source_edge_index: operand_source_edge_index(source, incoming_operand),
                    this_source_edge: operand_source_edge(source, base_operand),
                    from_source_edge: operand_source_edge(source, incoming_operand),
                    this_side_synthetic: base_synthetic,
                    from_side_synthetic: incoming_synthetic,
                    synthetic_sides: base_synthetic as usize + incoming_synthetic as usize,
                });
            }
            ExactMeshlibMaterializedContourPair::Blocked => {
                unmaterialized_stitch_contour_edges += 1;
            }
        }
    }
    let open_stitch_near_edges = open_stitch_near_edge_plan(assembly, &materialized_stitch_pairs);
    let stitch_metadata_ready = !assembly.stitched_edge_sources.is_empty()
        && missing_stitch_contour_edges == 0
        && synthetic_stitch_contour_edges == 0
        && stitch_direction_mismatches == 0;
    let stitch_materialization_ready = !assembly.stitched_edge_sources.is_empty()
        && unmaterialized_stitch_contour_edges == 0
        && stitch_materialization_direction_mismatches == 0;
    let record_rewrite_blocked_edges =
        unmaterialized_stitch_contour_edges + stitch_materialization_direction_mismatches;
    let record_rewrite_ready = !assembly.stitched_edge_sources.is_empty()
        && record_rewrite_blocked_edges == 0
        && open_stitch_near_edges.ready;

    ExactMeshlibTopologyRewritePlan {
        base_operand,
        incoming_operand,
        base_faces,
        incoming_faces,
        stitched_edges: assembly.stitched_edge_sources.len(),
        mapped_contour_edges,
        missing_base_contour_edges,
        missing_incoming_contour_edges,
        contour_direction_mismatches,
        mapped_stitch_contour_edges,
        missing_stitch_contour_edges,
        synthetic_stitch_contour_edges,
        stitch_direction_mismatches,
        stitch_metadata_ready,
        materialized_stitch_contour_edges,
        unmaterialized_stitch_contour_edges,
        materialized_synthetic_stitch_sides,
        stitch_materialization_direction_mismatches,
        stitch_materialization_ready,
        record_rewrite_commands: record_rewrite_command_edges.len(),
        record_rewrite_blocked_edges,
        record_rewrite_synthetic_sides: materialized_synthetic_stitch_sides,
        record_rewrite_direction_mismatches: stitch_materialization_direction_mismatches,
        record_rewrite_ready,
        record_rewrite_command_edges,
        open_stitch_paths: open_stitch_near_edges.open_paths,
        open_stitch_near_edge_updates: open_stitch_near_edges.updates,
        open_stitch_near_edge_blocked_updates: open_stitch_near_edges.blocked_updates,
        open_stitch_near_edge_ready: open_stitch_near_edges.ready,
        ready_for_rewrite: !assembly.stitched_edge_sources.is_empty()
            && missing_base_contour_edges == 0
            && missing_incoming_contour_edges == 0
            && contour_direction_mismatches == 0
            && open_stitch_near_edges.ready,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ExactMeshlibOpenStitchNearEdgePlan {
    open_paths: usize,
    updates: usize,
    blocked_updates: usize,
    ready: bool,
}

fn open_stitch_near_edge_plan(
    assembly: &ExactBooleanAssemblyResult,
    materialized_stitch_pairs: &[bool],
) -> ExactMeshlibOpenStitchNearEdgePlan {
    let mut plan = ExactMeshlibOpenStitchNearEdgePlan {
        ready: true,
        ..ExactMeshlibOpenStitchNearEdgePlan::default()
    };
    for path in &assembly.stitched_edge_paths {
        if path.closed || path.pair_indices.is_empty() {
            continue;
        }
        plan.open_paths += 1;
        let first = path.pair_indices[0];
        let last = path.pair_indices[path.pair_indices.len() - 1];
        for source_index in [first, last] {
            if materialized_stitch_pairs
                .get(source_index)
                .copied()
                .unwrap_or(false)
            {
                plan.updates += 1;
            } else {
                plan.blocked_updates += 1;
            }
        }
    }
    plan.ready = plan.blocked_updates == 0;
    plan
}

fn prepared_face_count(
    assembly: &ExactBooleanAssemblyResult,
    operand: ExactBooleanOperand,
) -> usize {
    match operand {
        ExactBooleanOperand::First => assembly.prepare_first_faces.len(),
        ExactBooleanOperand::Second => assembly.prepare_second_faces.len(),
    }
}

fn meshlib_connect_order(
    operation: ExactBooleanOperation,
) -> (ExactBooleanOperand, ExactBooleanOperand) {
    if operation == ExactBooleanOperation::Intersection {
        (ExactBooleanOperand::Second, ExactBooleanOperand::First)
    } else {
        (ExactBooleanOperand::First, ExactBooleanOperand::Second)
    }
}

fn directed_face_edges_by_operand(
    assembly: &ExactBooleanAssemblyResult,
) -> BTreeMap<(ExactBooleanOperand, [usize; 2]), Vec<[usize; 2]>> {
    let mut edges = BTreeMap::<(ExactBooleanOperand, [usize; 2]), Vec<[usize; 2]>>::new();
    for (face, source) in assembly.faces.iter().zip(&assembly.face_sources) {
        let face = [face[0] as usize, face[1] as usize, face[2] as usize];
        for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
            edges
                .entry((source.operand, ordered_edge(edge)))
                .or_default()
                .push(edge);
        }
    }
    edges
}

fn directed_contour_edge(
    edges: &BTreeMap<(ExactBooleanOperand, [usize; 2]), Vec<[usize; 2]>>,
    operand: ExactBooleanOperand,
    output_edge: [usize; 2],
) -> Option<[usize; 2]> {
    edges
        .get(&(operand, ordered_edge(output_edge)))
        .and_then(|edges| edges.first().copied())
}

enum ExactMeshlibMaterializedContourPair {
    Ready {
        base_edge: [usize; 2],
        incoming_edge: [usize; 2],
        base_synthetic: bool,
        incoming_synthetic: bool,
    },
    Blocked,
}

fn materialized_contour_pair(
    source: &super::exact_boolean::ExactBooleanStitchedEdgeSource,
    base_operand: ExactBooleanOperand,
    incoming_operand: ExactBooleanOperand,
    base_face_edge: Option<[usize; 2]>,
    incoming_face_edge: Option<[usize; 2]>,
) -> ExactMeshlibMaterializedContourPair {
    let base = materialized_operand_edge(source, base_operand, base_face_edge);
    let incoming = materialized_operand_edge(source, incoming_operand, incoming_face_edge);
    let anchored = base_face_edge.is_some() || incoming_face_edge.is_some();
    match (base, incoming, anchored) {
        (Some((base_edge, base_synthetic)), Some((incoming_edge, incoming_synthetic)), true) => {
            ExactMeshlibMaterializedContourPair::Ready {
                base_edge,
                incoming_edge,
                base_synthetic,
                incoming_synthetic,
            }
        }
        _ => ExactMeshlibMaterializedContourPair::Blocked,
    }
}

fn materialized_operand_edge(
    source: &super::exact_boolean::ExactBooleanStitchedEdgeSource,
    operand: ExactBooleanOperand,
    face_edge: Option<[usize; 2]>,
) -> Option<([usize; 2], bool)> {
    let stitch_edge = operand_stitch_edge(source, operand);
    if face_edge.is_some() {
        return stitch_edge.or(face_edge).map(|edge| (edge, false));
    }
    (operand_stitch_edge_synthetic(source, operand) > 0)
        .then(|| stitch_edge.map(|edge| (edge, true)))
        .flatten()
}

fn operand_stitch_edge(
    source: &super::exact_boolean::ExactBooleanStitchedEdgeSource,
    operand: ExactBooleanOperand,
) -> Option<[usize; 2]> {
    match operand {
        ExactBooleanOperand::First => source.first_stitch_edge,
        ExactBooleanOperand::Second => source.second_stitch_edge,
    }
}

fn operand_source_edge(
    source: &super::exact_boolean::ExactBooleanStitchedEdgeSource,
    operand: ExactBooleanOperand,
) -> [usize; 2] {
    match operand {
        ExactBooleanOperand::First => source.first_cut_edge,
        ExactBooleanOperand::Second => source.second_cut_edge,
    }
}

fn operand_source_edge_index(
    source: &super::exact_boolean::ExactBooleanStitchedEdgeSource,
    operand: ExactBooleanOperand,
) -> usize {
    match operand {
        ExactBooleanOperand::First => source.first_edge_index,
        ExactBooleanOperand::Second => source.second_edge_index,
    }
}

fn operand_stitch_edge_synthetic(
    source: &super::exact_boolean::ExactBooleanStitchedEdgeSource,
    operand: ExactBooleanOperand,
) -> usize {
    (match operand {
        ExactBooleanOperand::First => source.first_stitch_edge_synthetic,
        ExactBooleanOperand::Second => source.second_stitch_edge_synthetic,
    }) as usize
}

fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

#[cfg(test)]
mod tests {
    use super::super::exact_boolean::{
        ExactBooleanAssemblyResult, ExactBooleanOutputFaceSource, ExactBooleanStitchedEdgeSource,
    };
    use super::super::exact_stitch::ExactStitchPath;
    use super::*;

    fn stitched_edge(first: [usize; 2], second: [usize; 2]) -> ExactBooleanStitchedEdgeSource {
        ExactBooleanStitchedEdgeSource {
            output_edge: ordered_edge(first),
            first_output_edge: Some(first),
            second_output_edge: Some(second),
            first_stitch_edge: Some(first),
            second_stitch_edge: Some(second),
            first_stitch_edge_synthetic: false,
            second_stitch_edge_synthetic: false,
            first_edge_index: 0,
            second_edge_index: 0,
            first_cut_edge: first,
            second_cut_edge: second,
        }
    }

    fn assembly_with_shared_contour() -> ExactBooleanAssemblyResult {
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
                    cut_face: 0,
                    source_face: 0,
                },
            ],
            first_output_vertex_for_cut_vertex: Vec::new(),
            second_output_vertex_for_cut_vertex: Vec::new(),
            stitched_edge_sources: vec![stitched_edge([1, 2], [2, 1])],
            stitched_edge_paths: Vec::new(),
            prepare_first_faces: vec![0],
            prepare_second_faces: vec![0],
            selected_first_faces: vec![0],
            selected_second_faces: vec![0],
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
    fn meshlib_topology_rewrite_plan_uses_union_connect_order() {
        let plan = meshlib_topology_rewrite_plan(
            &assembly_with_shared_contour(),
            ExactBooleanOperation::Union,
        );

        assert_eq!(plan.base_operand, ExactBooleanOperand::First);
        assert_eq!(plan.incoming_operand, ExactBooleanOperand::Second);
        assert_eq!(plan.base_faces, 1);
        assert_eq!(plan.incoming_faces, 1);
        assert_eq!(plan.mapped_contour_edges, 1);
        assert_eq!(plan.missing_base_contour_edges, 0);
        assert_eq!(plan.missing_incoming_contour_edges, 0);
        assert_eq!(plan.contour_direction_mismatches, 0);
        assert_eq!(plan.mapped_stitch_contour_edges, 1);
        assert_eq!(plan.missing_stitch_contour_edges, 0);
        assert_eq!(plan.synthetic_stitch_contour_edges, 0);
        assert_eq!(plan.stitch_direction_mismatches, 0);
        assert!(plan.stitch_metadata_ready);
        assert_eq!(plan.materialized_stitch_contour_edges, 1);
        assert_eq!(plan.unmaterialized_stitch_contour_edges, 0);
        assert_eq!(plan.materialized_synthetic_stitch_sides, 0);
        assert_eq!(plan.stitch_materialization_direction_mismatches, 0);
        assert!(plan.stitch_materialization_ready);
        assert_eq!(plan.record_rewrite_commands, 1);
        assert_eq!(plan.record_rewrite_blocked_edges, 0);
        assert_eq!(plan.record_rewrite_synthetic_sides, 0);
        assert_eq!(plan.record_rewrite_direction_mismatches, 0);
        assert!(plan.record_rewrite_ready);
        assert_eq!(
            plan.record_rewrite_command_edges,
            vec![ExactMeshlibRecordRewriteCommand {
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
            }]
        );
        assert_eq!(plan.open_stitch_paths, 0);
        assert_eq!(plan.open_stitch_near_edge_updates, 0);
        assert_eq!(plan.open_stitch_near_edge_blocked_updates, 0);
        assert!(plan.open_stitch_near_edge_ready);
        assert!(plan.ready_for_rewrite);
    }

    #[test]
    fn meshlib_topology_rewrite_plan_uses_intersection_left_hole_order() {
        let plan = meshlib_topology_rewrite_plan(
            &assembly_with_shared_contour(),
            ExactBooleanOperation::Intersection,
        );

        assert_eq!(plan.base_operand, ExactBooleanOperand::Second);
        assert_eq!(plan.incoming_operand, ExactBooleanOperand::First);
        assert_eq!(plan.mapped_contour_edges, 1);
        assert_eq!(plan.mapped_stitch_contour_edges, 1);
        assert!(plan.stitch_metadata_ready);
        assert_eq!(plan.materialized_stitch_contour_edges, 1);
        assert!(plan.stitch_materialization_ready);
        assert_eq!(plan.record_rewrite_commands, 1);
        assert!(plan.record_rewrite_ready);
        assert_eq!(
            plan.record_rewrite_command_edges,
            vec![ExactMeshlibRecordRewriteCommand {
                stitch_pair_index: 0,
                this_operand: ExactBooleanOperand::Second,
                from_operand: ExactBooleanOperand::First,
                output_edge: [1, 2],
                this_contour_edge: [2, 1],
                from_contour_edge: [1, 2],
                this_source_edge_index: 0,
                from_source_edge_index: 0,
                this_source_edge: [2, 1],
                from_source_edge: [1, 2],
                this_side_synthetic: false,
                from_side_synthetic: false,
                synthetic_sides: 0,
            }]
        );
        assert!(plan.ready_for_rewrite);
    }

    #[test]
    fn meshlib_topology_rewrite_plan_counts_prepare_part_faces() {
        let mut assembly = assembly_with_shared_contour();
        assembly.prepare_first_faces = vec![0, 1];
        assembly.prepare_second_faces = vec![0, 1, 2];

        let union = meshlib_topology_rewrite_plan(&assembly, ExactBooleanOperation::Union);
        assert_eq!(union.base_faces, 2);
        assert_eq!(union.incoming_faces, 3);

        let intersection =
            meshlib_topology_rewrite_plan(&assembly, ExactBooleanOperation::Intersection);
        assert_eq!(intersection.base_faces, 3);
        assert_eq!(intersection.incoming_faces, 2);
    }

    #[test]
    fn meshlib_topology_rewrite_plan_blocks_same_direction_contours() {
        let mut assembly = assembly_with_shared_contour();
        assembly.faces[1] = [1, 2, 3];
        assembly.stitched_edge_sources = vec![stitched_edge([1, 2], [1, 2])];
        let plan = meshlib_topology_rewrite_plan(&assembly, ExactBooleanOperation::Union);

        assert_eq!(plan.mapped_contour_edges, 1);
        assert_eq!(plan.missing_incoming_contour_edges, 0);
        assert_eq!(plan.contour_direction_mismatches, 1);
        assert_eq!(plan.mapped_stitch_contour_edges, 1);
        assert_eq!(plan.stitch_direction_mismatches, 1);
        assert!(!plan.stitch_metadata_ready);
        assert_eq!(plan.materialized_stitch_contour_edges, 1);
        assert_eq!(plan.stitch_materialization_direction_mismatches, 1);
        assert!(!plan.stitch_materialization_ready);
        assert_eq!(plan.record_rewrite_commands, 1);
        assert_eq!(plan.record_rewrite_blocked_edges, 1);
        assert_eq!(plan.record_rewrite_direction_mismatches, 1);
        assert!(!plan.record_rewrite_ready);
        assert!(!plan.ready_for_rewrite);
    }

    #[test]
    fn meshlib_topology_rewrite_plan_blocks_missing_non_synthetic_face_record() {
        let mut assembly = assembly_with_shared_contour();
        assembly.faces = vec![[0, 1, 2]];
        assembly.face_sources = vec![ExactBooleanOutputFaceSource {
            operand: ExactBooleanOperand::First,
            cut_face: 0,
            source_face: 0,
        }];

        let plan = meshlib_topology_rewrite_plan(&assembly, ExactBooleanOperation::Union);

        assert_eq!(plan.mapped_contour_edges, 0);
        assert_eq!(plan.missing_incoming_contour_edges, 1);
        assert_eq!(plan.mapped_stitch_contour_edges, 1);
        assert_eq!(plan.missing_stitch_contour_edges, 0);
        assert!(plan.stitch_metadata_ready);
        assert_eq!(plan.materialized_stitch_contour_edges, 0);
        assert_eq!(plan.unmaterialized_stitch_contour_edges, 1);
        assert!(!plan.stitch_materialization_ready);
        assert_eq!(plan.record_rewrite_commands, 0);
        assert_eq!(plan.record_rewrite_blocked_edges, 1);
        assert!(!plan.record_rewrite_ready);
        assert!(!plan.ready_for_rewrite);
    }

    #[test]
    fn meshlib_topology_rewrite_plan_materializes_synthetic_face_record() {
        let mut assembly = assembly_with_shared_contour();
        assembly.faces = vec![[0, 1, 2]];
        assembly.face_sources = vec![ExactBooleanOutputFaceSource {
            operand: ExactBooleanOperand::First,
            cut_face: 0,
            source_face: 0,
        }];
        assembly.stitched_edge_sources[0].second_stitch_edge_synthetic = true;

        let plan = meshlib_topology_rewrite_plan(&assembly, ExactBooleanOperation::Union);

        assert_eq!(plan.mapped_contour_edges, 0);
        assert_eq!(plan.missing_incoming_contour_edges, 1);
        assert_eq!(plan.materialized_stitch_contour_edges, 1);
        assert_eq!(plan.unmaterialized_stitch_contour_edges, 0);
        assert_eq!(plan.materialized_synthetic_stitch_sides, 1);
        assert!(plan.stitch_materialization_ready);
        assert_eq!(plan.record_rewrite_commands, 1);
        assert_eq!(plan.record_rewrite_synthetic_sides, 1);
        assert!(plan.record_rewrite_ready);
        assert_eq!(
            plan.record_rewrite_command_edges,
            vec![ExactMeshlibRecordRewriteCommand {
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
                from_side_synthetic: true,
                synthetic_sides: 1,
            }]
        );
        assert!(!plan.ready_for_rewrite);
    }

    #[test]
    fn meshlib_topology_rewrite_plan_counts_open_contour_near_edge_updates() {
        let mut assembly = assembly_with_shared_contour();
        assembly.stitched_edge_paths = vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }];

        let plan = meshlib_topology_rewrite_plan(&assembly, ExactBooleanOperation::Union);

        assert_eq!(plan.open_stitch_paths, 1);
        assert_eq!(plan.open_stitch_near_edge_updates, 2);
        assert_eq!(plan.open_stitch_near_edge_blocked_updates, 0);
        assert!(plan.open_stitch_near_edge_ready);
        assert!(plan.record_rewrite_ready);
        assert!(plan.ready_for_rewrite);
    }

    #[test]
    fn meshlib_topology_rewrite_plan_blocks_open_near_edge_updates_without_records() {
        let mut assembly = assembly_with_shared_contour();
        assembly.faces = vec![[0, 1, 2]];
        assembly.face_sources = vec![ExactBooleanOutputFaceSource {
            operand: ExactBooleanOperand::First,
            cut_face: 0,
            source_face: 0,
        }];
        assembly.stitched_edge_paths = vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }];

        let plan = meshlib_topology_rewrite_plan(&assembly, ExactBooleanOperation::Union);

        assert_eq!(plan.open_stitch_paths, 1);
        assert_eq!(plan.open_stitch_near_edge_updates, 0);
        assert_eq!(plan.open_stitch_near_edge_blocked_updates, 2);
        assert!(!plan.open_stitch_near_edge_ready);
        assert!(!plan.record_rewrite_ready);
        assert!(!plan.ready_for_rewrite);
    }
}
