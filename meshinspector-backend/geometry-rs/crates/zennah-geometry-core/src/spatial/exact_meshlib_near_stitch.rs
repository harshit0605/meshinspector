use super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
use super::exact_cut_apply::ExactCutMeshResult;
use std::collections::{BTreeMap, BTreeSet};
mod endpoint;
mod topology;
#[cfg(test)]
use endpoint::push_endpoint_update;
use endpoint::{
    incoming_cut_edge, push_endpoint_update_with_source, push_prepared_endpoint_update,
    EndpointSourceContext, OpenPathEndpoint, PreparedEndpointContext,
};
use topology::{ordered_edge, OperandTopology};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExactMeshlibNearStitchEndpoint {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactMeshlibSourceHalfedgeKey {
    pub face: usize,
    pub edge: [usize; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactMeshlibNearStitchEdgeUpdateCommand {
    pub stitch_pair_index: Option<usize>,
    pub endpoint: Option<ExactMeshlibNearStitchEndpoint>,
    pub source_operand: Option<ExactBooleanOperand>,
    pub previous_source_halfedge: Option<usize>,
    pub next_source_halfedge: Option<usize>,
    pub previous_source_halfedge_key: Option<ExactMeshlibSourceHalfedgeKey>,
    pub next_source_halfedge_key: Option<ExactMeshlibSourceHalfedgeKey>,
    pub previous_source_edge: Option<[usize; 2]>,
    pub next_source_edge: Option<[usize; 2]>,
    pub strict_source_identity: bool,
    pub previous_edge: [usize; 2],
    pub next_edge: [usize; 2],
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct ExactMeshlibNearStitchPlan {
    pub commands: Vec<ExactMeshlibNearStitchEdgeUpdateCommand>,
    pub open_paths: usize,
    pub expected_updates: usize,
    pub blocked_updates: usize,
    pub skipped_previous_left_source_edges: usize,
    pub skipped_next_right_source_edges: usize,
}

#[derive(Debug, Clone)]
pub(super) struct ExactMeshlibNearStitchSourceInput<'a> {
    pub cut_mesh: &'a ExactCutMeshResult,
    pub prepared_faces: &'a [usize],
    pub vertex_map: &'a [Option<usize>],
    pub contour_vertex_maps: Vec<([usize; 2], [usize; 2])>,
    pub contour_vertex_map_source_indices: Vec<Option<usize>>,
    pub first_virtual_vertex: usize,
    pub flip_orientation: bool,
}

#[cfg(test)]
pub(super) fn exact_meshlib_near_stitch_plan(
    assembly: &ExactBooleanAssemblyResult,
    base_operand: ExactBooleanOperand,
    incoming_operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
) -> ExactMeshlibNearStitchPlan {
    let base = OperandTopology::from_assembly(assembly, base_operand);
    let incoming = OperandTopology::from_assembly(assembly, incoming_operand);
    let output = OperandTopology::from_output(assembly);
    let mapped_incoming_edges = record_commands
        .iter()
        .map(|command| ordered_edge(command.from_contour_edge))
        .collect::<BTreeSet<_>>();
    let command_by_stitch_pair = record_commands
        .iter()
        .map(|command| (command.stitch_pair_index, command))
        .collect::<BTreeMap<_, _>>();
    let mut plan = ExactMeshlibNearStitchPlan::default();

    for path in &assembly.stitched_edge_paths {
        if path.closed || path.pair_indices.is_empty() {
            continue;
        }
        plan.open_paths += 1;
        let first = path.pair_indices[0];
        let last = path.pair_indices[path.pair_indices.len() - 1];
        push_endpoint_update(
            &mut plan,
            command_by_stitch_pair.get(&first).copied(),
            OpenPathEndpoint::Start,
            &base,
            &incoming,
            &output,
            &mapped_incoming_edges,
        );
        push_endpoint_update(
            &mut plan,
            command_by_stitch_pair.get(&last).copied(),
            OpenPathEndpoint::End,
            &base,
            &incoming,
            &output,
            &mapped_incoming_edges,
        );
    }

    plan
}

pub(super) fn exact_meshlib_near_stitch_plan_with_source(
    assembly: &ExactBooleanAssemblyResult,
    base_operand: ExactBooleanOperand,
    incoming_operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
    source_input: ExactMeshlibNearStitchSourceInput<'_>,
) -> ExactMeshlibNearStitchPlan {
    let base = OperandTopology::from_assembly(assembly, base_operand);
    let incoming = OperandTopology::from_cut_mesh(
        source_input.cut_mesh,
        source_input.prepared_faces,
        source_input.vertex_map,
        &source_input.contour_vertex_maps,
        &source_input.contour_vertex_map_source_indices,
        source_input.first_virtual_vertex,
        source_input.flip_orientation,
    );
    let output = OperandTopology::from_output(assembly);
    let mapped_incoming_edges = record_commands
        .iter()
        .filter_map(|command| incoming_cut_edge(assembly, incoming_operand, command))
        .map(ordered_edge)
        .collect::<BTreeSet<_>>();
    let command_by_stitch_pair = record_commands
        .iter()
        .map(|command| (command.stitch_pair_index, command))
        .collect::<BTreeMap<_, _>>();
    let mut plan = ExactMeshlibNearStitchPlan::default();

    for path in &assembly.stitched_edge_paths {
        if path.closed || path.pair_indices.is_empty() {
            continue;
        }
        plan.open_paths += 1;
        let first = path.pair_indices[0];
        let last = path.pair_indices[path.pair_indices.len() - 1];
        push_endpoint_update_with_source(
            &mut plan,
            command_by_stitch_pair.get(&first).copied(),
            OpenPathEndpoint::Start,
            EndpointSourceContext {
                assembly,
                incoming_operand,
                base: &base,
                incoming: &incoming,
                output: &output,
                mapped_incoming_edges: &mapped_incoming_edges,
                flip_orientation: source_input.flip_orientation,
            },
        );
        push_endpoint_update_with_source(
            &mut plan,
            command_by_stitch_pair.get(&last).copied(),
            OpenPathEndpoint::End,
            EndpointSourceContext {
                assembly,
                incoming_operand,
                base: &base,
                incoming: &incoming,
                output: &output,
                mapped_incoming_edges: &mapped_incoming_edges,
                flip_orientation: source_input.flip_orientation,
            },
        );
    }

    plan
}

pub(super) fn exact_meshlib_near_stitch_plan_with_prepared_parts(
    assembly: &ExactBooleanAssemblyResult,
    incoming_operand: ExactBooleanOperand,
    record_commands: &[ExactMeshlibRecordRewriteCommand],
    base_input: ExactMeshlibNearStitchSourceInput<'_>,
    incoming_input: ExactMeshlibNearStitchSourceInput<'_>,
) -> ExactMeshlibNearStitchPlan {
    let base = OperandTopology::from_cut_mesh(
        base_input.cut_mesh,
        base_input.prepared_faces,
        base_input.vertex_map,
        &base_input.contour_vertex_maps,
        &base_input.contour_vertex_map_source_indices,
        base_input.first_virtual_vertex,
        base_input.flip_orientation,
    );
    let incoming_source_preflipped = incoming_input.flip_orientation;
    let incoming_connect_flip_orientation = if incoming_source_preflipped {
        false
    } else {
        incoming_input.flip_orientation
    };
    let incoming = OperandTopology::from_cut_mesh_with_fresh_vertex_map_and_orientation(
        incoming_input.cut_mesh,
        incoming_input.prepared_faces,
        &incoming_input.contour_vertex_maps,
        &incoming_input.contour_vertex_map_source_indices,
        incoming_input.first_virtual_vertex,
        incoming_source_preflipped,
        incoming_connect_flip_orientation,
    );
    let mapped_incoming_output_edges = record_commands
        .iter()
        .filter_map(|command| incoming_cut_edge(assembly, incoming_operand, command))
        .map(ordered_edge)
        .collect::<BTreeSet<_>>();
    let mapped_incoming_source_edges = record_commands
        .iter()
        .flat_map(|command| {
            incoming.source_contour_undirected_edge_keys(
                command.from_source_edge,
                command.from_source_edge_index,
                incoming_connect_flip_orientation,
            )
        })
        .collect::<BTreeSet<_>>();
    let command_by_stitch_pair = record_commands
        .iter()
        .map(|command| (command.stitch_pair_index, command))
        .collect::<BTreeMap<_, _>>();
    let mut plan = ExactMeshlibNearStitchPlan::default();

    for path in &assembly.stitched_edge_paths {
        if path.closed || path.pair_indices.is_empty() {
            continue;
        }
        plan.open_paths += 1;
        let first = path.pair_indices[0];
        let last = path.pair_indices[path.pair_indices.len() - 1];
        push_prepared_endpoint_update(
            &mut plan,
            command_by_stitch_pair.get(&first).copied(),
            OpenPathEndpoint::Start,
            PreparedEndpointContext {
                base: &base,
                incoming: &incoming,
                mapped_incoming_output_edges: &mapped_incoming_output_edges,
                mapped_incoming_source_edges: &mapped_incoming_source_edges,
                flip_orientation: incoming_connect_flip_orientation,
            },
        );
        push_prepared_endpoint_update(
            &mut plan,
            command_by_stitch_pair.get(&last).copied(),
            OpenPathEndpoint::End,
            PreparedEndpointContext {
                base: &base,
                incoming: &incoming,
                mapped_incoming_output_edges: &mapped_incoming_output_edges,
                mapped_incoming_source_edges: &mapped_incoming_source_edges,
                flip_orientation: incoming_connect_flip_orientation,
            },
        );
    }

    plan
}

#[cfg(test)]
mod tests;
