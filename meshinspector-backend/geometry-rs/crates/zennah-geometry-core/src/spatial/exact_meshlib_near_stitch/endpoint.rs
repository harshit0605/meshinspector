use super::super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperand};
use super::super::exact_boolean_topology::ExactMeshlibRecordRewriteCommand;
use super::super::exact_halfedge::ExactHalfEdgeTopology;
use super::topology::{ordered_edge, OperandTopology, SourceEdgeWalkResult};
use super::{
    ExactMeshlibNearStitchEdgeUpdateCommand, ExactMeshlibNearStitchEndpoint,
    ExactMeshlibNearStitchPlan,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OpenPathEndpoint {
    Start,
    End,
}

#[derive(Clone, Copy)]
pub(super) struct EndpointSourceContext<'a> {
    pub(super) assembly: &'a ExactBooleanAssemblyResult,
    pub(super) incoming_operand: ExactBooleanOperand,
    pub(super) base: &'a OperandTopology,
    pub(super) incoming: &'a OperandTopology,
    pub(super) output: &'a OperandTopology,
    pub(super) mapped_incoming_edges: &'a BTreeSet<[usize; 2]>,
    pub(super) flip_orientation: bool,
}

#[derive(Clone, Copy)]
pub(super) struct PreparedEndpointContext<'a> {
    pub(super) base: &'a OperandTopology,
    pub(super) incoming: &'a OperandTopology,
    pub(super) mapped_incoming_output_edges: &'a BTreeSet<[usize; 2]>,
    pub(super) mapped_incoming_source_edges: &'a BTreeSet<usize>,
    pub(super) flip_orientation: bool,
}

#[cfg(test)]
pub(super) fn push_endpoint_update(
    plan: &mut ExactMeshlibNearStitchPlan,
    command: Option<&ExactMeshlibRecordRewriteCommand>,
    endpoint: OpenPathEndpoint,
    base: &OperandTopology,
    incoming: &OperandTopology,
    output: &OperandTopology,
    mapped_incoming_edges: &BTreeSet<[usize; 2]>,
) {
    plan.expected_updates += 1;
    let Some(command) = command else {
        plan.blocked_updates += 1;
        return;
    };
    let update = match endpoint {
        OpenPathEndpoint::Start => {
            start_endpoint_update(command, base, incoming, mapped_incoming_edges)
                .or_else(|| start_endpoint_update(command, output, output, mapped_incoming_edges))
        }
        OpenPathEndpoint::End => {
            end_endpoint_update(command, base, incoming, mapped_incoming_edges)
                .or_else(|| end_endpoint_update(command, output, output, mapped_incoming_edges))
        }
    };
    if let Some(update) = update {
        plan.commands.push(update);
    } else {
        plan.blocked_updates += 1;
    }
}

pub(super) fn push_prepared_endpoint_update(
    plan: &mut ExactMeshlibNearStitchPlan,
    command: Option<&ExactMeshlibRecordRewriteCommand>,
    endpoint: OpenPathEndpoint,
    context: PreparedEndpointContext<'_>,
) {
    let Some(command) = command else {
        plan.expected_updates += 1;
        plan.blocked_updates += 1;
        return;
    };
    let update = match endpoint {
        OpenPathEndpoint::Start => match start_endpoint_update_from_source_edges(
            command,
            command.this_source_edge,
            command.from_source_edge,
            command.from_source_edge_index,
            context,
        ) {
            PreparedEndpointUpdate::Command(update) => Some(update),
            PreparedEndpointUpdate::SkippedMapped => return,
            PreparedEndpointUpdate::SkippedPreviousLeftSource => {
                plan.expected_updates += 1;
                plan.blocked_updates += 1;
                plan.skipped_previous_left_source_edges += 1;
                return;
            }
            PreparedEndpointUpdate::SkippedNextRightSource => {
                plan.expected_updates += 1;
                plan.blocked_updates += 1;
                plan.skipped_next_right_source_edges += 1;
                return;
            }
            PreparedEndpointUpdate::Missing => return,
        },
        OpenPathEndpoint::End => match end_endpoint_update_from_source_edges(
            command,
            command.this_source_edge,
            command.from_source_edge,
            command.from_source_edge_index,
            context,
        ) {
            PreparedEndpointUpdate::Command(update) => Some(update),
            PreparedEndpointUpdate::SkippedMapped => return,
            PreparedEndpointUpdate::SkippedPreviousLeftSource => {
                plan.expected_updates += 1;
                plan.blocked_updates += 1;
                plan.skipped_previous_left_source_edges += 1;
                return;
            }
            PreparedEndpointUpdate::SkippedNextRightSource => {
                plan.expected_updates += 1;
                plan.blocked_updates += 1;
                plan.skipped_next_right_source_edges += 1;
                return;
            }
            PreparedEndpointUpdate::Missing => return,
        },
    };
    plan.expected_updates += 1;
    if let Some(update) = update {
        plan.commands.push(update);
    } else {
        plan.blocked_updates += 1;
    }
}

pub(super) fn push_endpoint_update_with_source(
    plan: &mut ExactMeshlibNearStitchPlan,
    command: Option<&ExactMeshlibRecordRewriteCommand>,
    endpoint: OpenPathEndpoint,
    context: EndpointSourceContext<'_>,
) {
    plan.expected_updates += 1;
    let Some(command) = command else {
        plan.blocked_updates += 1;
        return;
    };
    let Some(from_contour_edge) =
        incoming_cut_edge(context.assembly, context.incoming_operand, command)
    else {
        plan.blocked_updates += 1;
        return;
    };
    let update = match endpoint {
        OpenPathEndpoint::Start => start_endpoint_update_from_edge(
            command,
            from_contour_edge,
            context.base,
            context.incoming,
            context.mapped_incoming_edges,
            context.flip_orientation,
        )
        .or_else(|| {
            start_endpoint_update(
                command,
                context.output,
                context.output,
                context.mapped_incoming_edges,
            )
            .map(strict_source_identity_update)
        }),
        OpenPathEndpoint::End => end_endpoint_update_from_edge(
            command,
            from_contour_edge,
            context.base,
            context.incoming,
            context.mapped_incoming_edges,
            context.flip_orientation,
        )
        .or_else(|| {
            end_endpoint_update(
                command,
                context.output,
                context.output,
                context.mapped_incoming_edges,
            )
            .map(strict_source_identity_update)
        }),
    };
    if let Some(update) = update {
        plan.commands.push(update);
    } else {
        plan.blocked_updates += 1;
    }
}

fn strict_source_identity_update(
    mut command: ExactMeshlibNearStitchEdgeUpdateCommand,
) -> ExactMeshlibNearStitchEdgeUpdateCommand {
    command.strict_source_identity = true;
    command
}

enum PreparedEndpointUpdate {
    Command(ExactMeshlibNearStitchEdgeUpdateCommand),
    SkippedMapped,
    SkippedPreviousLeftSource,
    SkippedNextRightSource,
    Missing,
}

fn start_endpoint_update(
    command: &ExactMeshlibRecordRewriteCommand,
    base: &OperandTopology,
    incoming: &OperandTopology,
    mapped_incoming_edges: &BTreeSet<[usize; 2]>,
) -> Option<ExactMeshlibNearStitchEdgeUpdateCommand> {
    let this_edge = base.contour_boundary_edge(command.this_contour_edge)?;
    let previous = base.topology.prev(ExactHalfEdgeTopology::sym(this_edge));
    let next = incoming.next_unmapped_source_edge(
        command.from_contour_edge,
        mapped_incoming_edges,
        false,
    )?;
    Some(ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(command.stitch_pair_index),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(command.from_operand),
        previous_source_halfedge: None,
        next_source_halfedge: incoming.source_halfedge_index(next),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: incoming.source_halfedge_key(next),
        previous_source_edge: None,
        next_source_edge: incoming.source_directed_edge(next),
        strict_source_identity: false,
        previous_edge: base.directed_edge(previous)?,
        next_edge: incoming.directed_edge(next)?,
    })
}

fn start_endpoint_update_from_edge(
    command: &ExactMeshlibRecordRewriteCommand,
    from_contour_edge: [usize; 2],
    base: &OperandTopology,
    incoming: &OperandTopology,
    mapped_incoming_edges: &BTreeSet<[usize; 2]>,
    flip_orientation: bool,
) -> Option<ExactMeshlibNearStitchEdgeUpdateCommand> {
    let this_edge = base.contour_boundary_edge(command.this_contour_edge)?;
    let previous = base.topology.prev(ExactHalfEdgeTopology::sym(this_edge));
    let next = incoming.next_unmapped_source_edge(
        from_contour_edge,
        mapped_incoming_edges,
        flip_orientation,
    )?;
    Some(ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(command.stitch_pair_index),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(command.from_operand),
        previous_source_halfedge: None,
        next_source_halfedge: incoming.source_halfedge_index(next),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: incoming.source_halfedge_key(next),
        previous_source_edge: None,
        next_source_edge: incoming.source_directed_edge(next),
        strict_source_identity: true,
        previous_edge: base.directed_edge(previous)?,
        next_edge: incoming.directed_edge(next)?,
    })
}

fn start_endpoint_update_from_source_edges(
    command: &ExactMeshlibRecordRewriteCommand,
    this_source_edge: [usize; 2],
    from_source_edge: [usize; 2],
    from_source_edge_index: usize,
    context: PreparedEndpointContext<'_>,
) -> PreparedEndpointUpdate {
    let Some(this_edge) = context.base.contour_boundary_edge(this_source_edge) else {
        return PreparedEndpointUpdate::Missing;
    };
    let previous = context
        .base
        .topology
        .prev(ExactHalfEdgeTopology::sym(this_edge));
    let next = match context.incoming.next_unmapped_source_edge_by_source_index(
        from_source_edge,
        from_source_edge_index,
        context.mapped_incoming_source_edges,
        context.flip_orientation,
    ) {
        SourceEdgeWalkResult::Edge(edge) => edge,
        SourceEdgeWalkResult::BlockedOpenSide => {
            return PreparedEndpointUpdate::SkippedNextRightSource;
        }
        SourceEdgeWalkResult::Missing => return PreparedEndpointUpdate::Missing,
    };
    let Some(next_edge) = context.incoming.directed_edge(next) else {
        return PreparedEndpointUpdate::Missing;
    };
    if context
        .mapped_incoming_output_edges
        .contains(&ordered_edge(next_edge))
    {
        return PreparedEndpointUpdate::SkippedMapped;
    }
    let Some(previous_edge) = context.base.directed_edge(previous) else {
        return PreparedEndpointUpdate::Missing;
    };
    PreparedEndpointUpdate::Command(ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(command.stitch_pair_index),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::Start),
        source_operand: Some(command.from_operand),
        previous_source_halfedge: None,
        next_source_halfedge: context.incoming.source_halfedge_index(next),
        previous_source_halfedge_key: None,
        next_source_halfedge_key: context.incoming.source_halfedge_key(next),
        previous_source_edge: None,
        next_source_edge: context.incoming.source_directed_edge(next),
        strict_source_identity: true,
        previous_edge,
        next_edge,
    })
}

fn end_endpoint_update(
    command: &ExactMeshlibRecordRewriteCommand,
    base: &OperandTopology,
    incoming: &OperandTopology,
    mapped_incoming_edges: &BTreeSet<[usize; 2]>,
) -> Option<ExactMeshlibNearStitchEdgeUpdateCommand> {
    let this_face_edge = base.first_directed_face_edge(command.this_contour_edge)?;
    let previous = incoming.previous_unmapped_source_edge(
        command.from_contour_edge,
        mapped_incoming_edges,
        false,
    )?;
    let next = base
        .topology
        .next(ExactHalfEdgeTopology::sym(this_face_edge));
    Some(ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(command.stitch_pair_index),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::End),
        source_operand: Some(command.from_operand),
        previous_source_halfedge: incoming.source_halfedge_index(previous),
        next_source_halfedge: None,
        previous_source_halfedge_key: incoming.source_halfedge_key(previous),
        next_source_halfedge_key: None,
        previous_source_edge: incoming.source_directed_edge(previous),
        next_source_edge: None,
        strict_source_identity: false,
        previous_edge: incoming.directed_edge(previous)?,
        next_edge: base.directed_edge(next)?,
    })
}

fn end_endpoint_update_from_edge(
    command: &ExactMeshlibRecordRewriteCommand,
    from_contour_edge: [usize; 2],
    base: &OperandTopology,
    incoming: &OperandTopology,
    mapped_incoming_edges: &BTreeSet<[usize; 2]>,
    flip_orientation: bool,
) -> Option<ExactMeshlibNearStitchEdgeUpdateCommand> {
    let this_face_edge = base.first_directed_face_edge(command.this_contour_edge)?;
    let previous = incoming.previous_unmapped_source_edge(
        from_contour_edge,
        mapped_incoming_edges,
        flip_orientation,
    )?;
    let next = base
        .topology
        .next(ExactHalfEdgeTopology::sym(this_face_edge));
    Some(ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(command.stitch_pair_index),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::End),
        source_operand: Some(command.from_operand),
        previous_source_halfedge: incoming.source_halfedge_index(previous),
        next_source_halfedge: None,
        previous_source_halfedge_key: incoming.source_halfedge_key(previous),
        next_source_halfedge_key: None,
        previous_source_edge: incoming.source_directed_edge(previous),
        next_source_edge: None,
        strict_source_identity: true,
        previous_edge: incoming.directed_edge(previous)?,
        next_edge: base.directed_edge(next)?,
    })
}

fn end_endpoint_update_from_source_edges(
    command: &ExactMeshlibRecordRewriteCommand,
    this_source_edge: [usize; 2],
    from_source_edge: [usize; 2],
    from_source_edge_index: usize,
    context: PreparedEndpointContext<'_>,
) -> PreparedEndpointUpdate {
    let Some(this_face_edge) = context.base.first_directed_face_edge(this_source_edge) else {
        return PreparedEndpointUpdate::Missing;
    };
    let previous = match context
        .incoming
        .previous_unmapped_source_edge_by_source_index(
            from_source_edge,
            from_source_edge_index,
            context.mapped_incoming_source_edges,
            context.flip_orientation,
        ) {
        SourceEdgeWalkResult::Edge(edge) => edge,
        SourceEdgeWalkResult::BlockedOpenSide => {
            return PreparedEndpointUpdate::SkippedPreviousLeftSource;
        }
        SourceEdgeWalkResult::Missing => return PreparedEndpointUpdate::Missing,
    };
    let Some(previous_edge) = context.incoming.directed_edge(previous) else {
        return PreparedEndpointUpdate::Missing;
    };
    if context
        .mapped_incoming_output_edges
        .contains(&ordered_edge(previous_edge))
    {
        return PreparedEndpointUpdate::SkippedMapped;
    }
    let next = context
        .base
        .topology
        .next(ExactHalfEdgeTopology::sym(this_face_edge));
    let Some(next_edge) = context.base.directed_edge(next) else {
        return PreparedEndpointUpdate::Missing;
    };
    PreparedEndpointUpdate::Command(ExactMeshlibNearStitchEdgeUpdateCommand {
        stitch_pair_index: Some(command.stitch_pair_index),
        endpoint: Some(ExactMeshlibNearStitchEndpoint::End),
        source_operand: Some(command.from_operand),
        previous_source_halfedge: context.incoming.source_halfedge_index(previous),
        next_source_halfedge: None,
        previous_source_halfedge_key: context.incoming.source_halfedge_key(previous),
        next_source_halfedge_key: None,
        previous_source_edge: context.incoming.source_directed_edge(previous),
        next_source_edge: None,
        strict_source_identity: true,
        previous_edge,
        next_edge,
    })
}

pub(super) fn incoming_cut_edge(
    assembly: &ExactBooleanAssemblyResult,
    incoming_operand: ExactBooleanOperand,
    command: &ExactMeshlibRecordRewriteCommand,
) -> Option<[usize; 2]> {
    let source = assembly
        .stitched_edge_sources
        .get(command.stitch_pair_index)?;
    Some(match incoming_operand {
        ExactBooleanOperand::First => source.first_cut_edge,
        ExactBooleanOperand::Second => source.second_cut_edge,
    })
}
