use super::super::exact_boolean::{ExactBooleanOperand, ExactBooleanOutputFaceSource};
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::output_topology::OutputFaceTopology;
use super::source_records::ExactMeshlibPreparedSourceRecord;
use std::collections::BTreeMap;

mod diagnostics;
mod maps;
mod source_topology;
#[cfg(test)]
mod tests;

pub(crate) use diagnostics::{
    ExactMeshlibCopiedSourceEdgeDiagnostic, ExactMeshlibCopiedSourceEdgeLookupDiagnostic,
    ExactMeshlibCopiedSourceEdgeStatus,
};
use maps::{connect_prepared_parts_vertex_map, copied_face_map, copied_vertex_map};
use source_topology::SourcePreparedTopology;

#[derive(Debug, Clone)]
pub(crate) struct ExactMeshlibCopiedEdgeTranslationInput<'a> {
    pub cut_mesh: &'a ExactCutMeshResult,
    pub prepared_faces: &'a [usize],
    pub vertex_map: &'a [Option<usize>],
    pub contour_vertex_maps: Vec<([usize; 2], [usize; 2])>,
    pub contour_vertex_map_source_indices: Vec<Option<usize>>,
    pub face_sources: &'a [ExactBooleanOutputFaceSource],
    pub incoming_operand: ExactBooleanOperand,
    pub first_virtual_vertex: usize,
    pub append_prepared_faces: bool,
    pub flip_orientation: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibCopiedEdgeTranslationSummary {
    pub copied_edges: usize,
    pub translated_records: usize,
    pub translated_face_records: usize,
    pub failed_records: usize,
}

pub(crate) struct ExactMeshlibPreparedCopiedEdges {
    source: SourcePreparedTopology,
    vertex_map: Vec<Option<usize>>,
    face_map: Vec<Option<usize>>,
    edge_map: BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    copied_pairs: Vec<(ExactHalfEdgeId, ExactHalfEdgeId)>,
    prepared_faces: Vec<usize>,
    incoming_operand: ExactBooleanOperand,
    append_prepared_faces: bool,
    flip_orientation: bool,
    summary: ExactMeshlibCopiedEdgeTranslationSummary,
}

struct TranslatedRecord {
    next: ExactHalfEdgeId,
    prev: ExactHalfEdgeId,
    origin: Option<usize>,
    left: Option<usize>,
}

pub(super) fn prepare_meshlib_copied_edges(
    output: &mut OutputFaceTopology,
    input: ExactMeshlibCopiedEdgeTranslationInput<'_>,
) -> Result<ExactMeshlibPreparedCopiedEdges, &'static str> {
    let source_preflipped = input.append_prepared_faces && input.flip_orientation;
    let effective_flip_orientation = input.flip_orientation && !source_preflipped;
    let source = if source_preflipped {
        SourcePreparedTopology::from_cut_mesh_with_orientation(
            input.cut_mesh,
            input.prepared_faces,
            true,
        )?
    } else {
        SourcePreparedTopology::from_cut_mesh(input.cut_mesh, input.prepared_faces)?
    };
    let contour_vertex_maps = source.oriented_contour_vertex_maps(
        &input.contour_vertex_maps,
        &input.contour_vertex_map_source_indices,
        effective_flip_orientation,
    );
    let vertex_map = if input.append_prepared_faces {
        connect_prepared_parts_vertex_map(
            input.cut_mesh,
            input.prepared_faces,
            input.first_virtual_vertex,
            &contour_vertex_maps,
        )
    } else {
        copied_vertex_map(
            input.vertex_map,
            input.cut_mesh,
            input.prepared_faces,
            input.first_virtual_vertex,
            &contour_vertex_maps,
        )
    };
    let face_map = copied_face_map(
        input.cut_mesh.faces.len(),
        input.prepared_faces,
        input.face_sources,
        input.incoming_operand,
        output.face_edges.len(),
    );
    let mut edge_map =
        source.initial_edge_map(output, input.incoming_operand, effective_flip_orientation);
    let mut copied_pairs = Vec::new();

    for source_edge in &source.base_edges {
        if let Some(source_vertices) = source.source_vertices_for_edge(*source_edge) {
            let mapped_output_edge = source.map_edge_like_meshlib(*source_edge, &edge_map);
            if edge_map_contains_undirected(&edge_map, *source_edge) {
                output.record_meshlib_copied_source_edge_status(
                    input.incoming_operand,
                    source_vertices,
                    copied_source_edge_diagnostic(
                        &source,
                        *source_edge,
                        &face_map,
                        mapped_output_edge,
                        ExactMeshlibCopiedSourceEdgeStatus::MappedContour,
                    ),
                    copied_source_edge_diagnostic(
                        &source,
                        ExactHalfEdgeTopology::sym(*source_edge),
                        &face_map,
                        mapped_output_edge.map(ExactHalfEdgeTopology::sym),
                        ExactMeshlibCopiedSourceEdgeStatus::MappedContour,
                    ),
                );
                continue;
            }
        }
        if edge_map_contains_undirected(&edge_map, *source_edge) {
            continue;
        }
        let Some(output_vertices) = source.output_vertices_for_edge(*source_edge, &vertex_map)
        else {
            if let Some(source_vertices) = source.source_vertices_for_edge(*source_edge) {
                output.record_meshlib_copied_source_edge_status(
                    input.incoming_operand,
                    source_vertices,
                    copied_source_edge_diagnostic(
                        &source,
                        *source_edge,
                        &face_map,
                        None,
                        ExactMeshlibCopiedSourceEdgeStatus::MissingOutputVertices,
                    ),
                    copied_source_edge_diagnostic(
                        &source,
                        ExactHalfEdgeTopology::sym(*source_edge),
                        &face_map,
                        None,
                        ExactMeshlibCopiedSourceEdgeStatus::MissingOutputVertices,
                    ),
                );
            }
            continue;
        };
        let Some(source_vertices) = source.source_vertices_for_edge(*source_edge) else {
            continue;
        };
        let output_edge = output
            .topology
            .make_edge(Some(output_vertices[0]), Some(output_vertices[1]));
        output.register_meshlib_copied_edge(
            input.incoming_operand,
            source_vertices,
            output_vertices,
            output_edge,
        );
        edge_map.insert(*source_edge, output_edge);
        edge_map.insert(
            ExactHalfEdgeTopology::sym(*source_edge),
            ExactHalfEdgeTopology::sym(output_edge),
        );
        output.record_meshlib_copied_source_edge_status(
            input.incoming_operand,
            source_vertices,
            copied_source_edge_diagnostic(
                &source,
                *source_edge,
                &face_map,
                Some(output_edge),
                ExactMeshlibCopiedSourceEdgeStatus::Copied,
            ),
            copied_source_edge_diagnostic(
                &source,
                ExactHalfEdgeTopology::sym(*source_edge),
                &face_map,
                Some(ExactHalfEdgeTopology::sym(output_edge)),
                ExactMeshlibCopiedSourceEdgeStatus::Copied,
            ),
        );
        copied_pairs.push((*source_edge, output_edge));
    }

    source.register_source_halfedge_candidates(output, input.incoming_operand, &edge_map);
    source.register_mapped_contour_source_records(
        output,
        input.incoming_operand,
        &edge_map,
        &face_map,
        effective_flip_orientation,
    )?;

    let copied_edge_count = copied_pairs.len();
    Ok(ExactMeshlibPreparedCopiedEdges {
        source,
        vertex_map,
        face_map,
        edge_map,
        copied_pairs,
        prepared_faces: input.prepared_faces.to_vec(),
        incoming_operand: input.incoming_operand,
        append_prepared_faces: input.append_prepared_faces,
        flip_orientation: effective_flip_orientation,
        summary: ExactMeshlibCopiedEdgeTranslationSummary {
            copied_edges: copied_edge_count,
            ..ExactMeshlibCopiedEdgeTranslationSummary::default()
        },
    })
}

pub(super) fn finalize_meshlib_copied_edges(
    output: &mut OutputFaceTopology,
    prepared: ExactMeshlibPreparedCopiedEdges,
) -> Result<ExactMeshlibCopiedEdgeTranslationSummary, &'static str> {
    let ExactMeshlibPreparedCopiedEdges {
        source,
        vertex_map,
        face_map,
        edge_map,
        copied_pairs,
        prepared_faces,
        incoming_operand,
        append_prepared_faces,
        flip_orientation,
        mut summary,
    } = prepared;
    summary = ExactMeshlibCopiedEdgeTranslationSummary {
        copied_edges: copied_pairs.len(),
        ..summary
    };
    for (source_edge, output_edge) in copied_pairs {
        let source_sym = ExactHalfEdgeTopology::sym(source_edge);
        let output_sym = ExactHalfEdgeTopology::sym(output_edge);
        let translated = translate_record(&source, source_edge, &edge_map, &vertex_map, &face_map);
        let translated_sym =
            translate_record(&source, source_sym, &edge_map, &vertex_map, &face_map);
        match (translated, translated_sym) {
            (Some(mut record), Some(mut sym_record)) => {
                if flip_orientation {
                    flip_translated_records(&mut record, &mut sym_record);
                }
                output.topology.set_meshlib_translated_record(
                    output_edge,
                    record.next,
                    record.prev,
                    record.origin,
                    record.left,
                )?;
                output.topology.set_meshlib_translated_record(
                    output_sym,
                    sym_record.next,
                    sym_record.prev,
                    sym_record.origin,
                    sym_record.left,
                )?;
                summary.translated_records += 2;
            }
            _ => {
                summary.failed_records += 2;
            }
        }
    }
    if append_prepared_faces {
        let mapped_source_record_replays = source.mapped_contour_source_record_replays(
            output,
            incoming_operand,
            &edge_map,
            &face_map,
            flip_orientation,
        )?;
        output.apply_meshlib_prepared_mapped_source_records(mapped_source_record_replays)?;
        summary.translated_face_records = append_prepared_face_records(
            output,
            &source,
            &prepared_faces,
            &edge_map,
            &face_map,
            incoming_operand,
            flip_orientation,
        )?;
    }

    Ok(summary)
}

fn append_prepared_face_records(
    output: &mut OutputFaceTopology,
    source: &SourcePreparedTopology,
    prepared_faces: &[usize],
    edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    face_map: &[Option<usize>],
    incoming_operand: ExactBooleanOperand,
    flip_orientation: bool,
) -> Result<usize, &'static str> {
    let mut translated = 0;
    for face in prepared_faces {
        let Some(output_face) = face_map.get(*face).copied().flatten() else {
            continue;
        };
        let Some(face_edge) =
            source.mapped_face_edge(output, *face, output_face, edge_map, flip_orientation)
        else {
            return Err("missing MeshLib copied face record edge");
        };
        output.set_meshlib_copied_face_record(output_face, face_edge, incoming_operand)?;
        translated += 1;
    }
    Ok(translated)
}

fn translate_record(
    source: &SourcePreparedTopology,
    source_edge: ExactHalfEdgeId,
    edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    vertex_map: &[Option<usize>],
    face_map: &[Option<usize>],
) -> Option<TranslatedRecord> {
    let next = mapped_next(source, source.topology.next(source_edge), edge_map)?;
    let prev = mapped_prev(source, source.topology.prev(source_edge), edge_map)?;
    let origin = source
        .topology
        .origin(source_edge)
        .and_then(|vertex| vertex_map.get(vertex).copied().flatten());
    let left = source
        .topology
        .left(source_edge)
        .and_then(|face| face_map.get(face).copied().flatten());
    Some(TranslatedRecord {
        next,
        prev,
        origin,
        left,
    })
}

fn flip_translated_records(record: &mut TranslatedRecord, sym_record: &mut TranslatedRecord) {
    std::mem::swap(&mut record.prev, &mut record.next);
    std::mem::swap(&mut sym_record.prev, &mut sym_record.next);
    std::mem::swap(&mut record.left, &mut sym_record.left);
}

fn translate_stitched_record(
    source: &SourcePreparedTopology,
    source_edge: ExactHalfEdgeId,
    edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    face_map: &[Option<usize>],
    flip_orientation: bool,
) -> Option<ExactMeshlibPreparedSourceRecord> {
    let source_sym = ExactHalfEdgeTopology::sym(source_edge);
    if flip_orientation {
        return Some(ExactMeshlibPreparedSourceRecord {
            next: mapped_prev(source, source.topology.prev(source_edge), edge_map)?,
            left: source
                .topology
                .left(source_sym)
                .and_then(|face| face_map.get(face).copied().flatten()),
            sym_prev: mapped_next(source, source.topology.next(source_sym), edge_map)?,
        });
    }
    Some(ExactMeshlibPreparedSourceRecord {
        next: mapped_next(source, source.topology.next(source_edge), edge_map)?,
        left: source
            .topology
            .left(source_edge)
            .and_then(|face| face_map.get(face).copied().flatten()),
        sym_prev: mapped_prev(source, source.topology.prev(source_sym), edge_map)?,
    })
}

fn mapped_next(
    source: &SourcePreparedTopology,
    start: ExactHalfEdgeId,
    edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
) -> Option<ExactHalfEdgeId> {
    let mut current = start;
    for _ in 0..=source.edge_vertices.len() {
        if let Some(mapped) = source.map_edge_like_meshlib(current, edge_map) {
            return Some(mapped);
        }
        current = source.topology.next(current);
    }
    None
}

fn mapped_prev(
    source: &SourcePreparedTopology,
    start: ExactHalfEdgeId,
    edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
) -> Option<ExactHalfEdgeId> {
    let mut current = start;
    for _ in 0..=source.edge_vertices.len() {
        if let Some(mapped) = source.map_edge_like_meshlib(current, edge_map) {
            return Some(mapped);
        }
        current = source.topology.prev(current);
    }
    None
}

fn edge_map_contains_undirected(
    edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
    edge: ExactHalfEdgeId,
) -> bool {
    edge_map.contains_key(&edge) || edge_map.contains_key(&ExactHalfEdgeTopology::sym(edge))
}

fn copied_source_edge_diagnostic(
    source: &SourcePreparedTopology,
    source_edge: ExactHalfEdgeId,
    face_map: &[Option<usize>],
    output_edge: Option<ExactHalfEdgeId>,
    status: ExactMeshlibCopiedSourceEdgeStatus,
) -> ExactMeshlibCopiedSourceEdgeDiagnostic {
    let source_left = source.topology.left(source_edge);
    let source_right = source.topology.right(source_edge);
    ExactMeshlibCopiedSourceEdgeDiagnostic {
        status,
        source_halfedge: Some(source_edge.0),
        source_origin: source.topology.origin(source_edge),
        source_left,
        source_right,
        source_left_mapped_face: source_left.and_then(|face| face_map.get(face).copied().flatten()),
        source_right_mapped_face: source_right
            .and_then(|face| face_map.get(face).copied().flatten()),
        source_next_halfedge: Some(source.topology.next(source_edge).0),
        source_prev_halfedge: Some(source.topology.prev(source_edge).0),
        output_edge_id: output_edge.map(|edge| edge.0),
    }
}
