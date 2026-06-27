use super::super::exact_boolean::{ExactBooleanOperand, ExactBooleanOutputFaceSource};
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::output_topology::OutputFaceTopology;
use super::prepared_base_contours::register_prepared_base_contour_edge_indices;
use std::collections::BTreeSet;

pub(crate) struct ExactMeshlibPreparedBaseTopologyInput<'a> {
    pub cut_mesh: &'a ExactCutMeshResult,
    pub prepared_faces: &'a [usize],
    pub vertex_map: &'a [Option<usize>],
    pub contour_vertex_maps: Vec<([usize; 2], [usize; 2])>,
    pub output_vertices: &'a [[f64; 3]],
    pub operand: ExactBooleanOperand,
    pub first_virtual_vertex: usize,
    pub flip_orientation: bool,
}

pub(crate) struct ExactMeshlibPreparedBaseTopology {
    pub topology: OutputFaceTopology,
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub face_sources: Vec<ExactBooleanOutputFaceSource>,
    pub virtual_vertices: usize,
}

pub(crate) fn output_topology_from_prepared_base(
    input: ExactMeshlibPreparedBaseTopologyInput<'_>,
) -> Result<ExactMeshlibPreparedBaseTopology, &'static str> {
    if input.output_vertices.len() != input.first_virtual_vertex {
        return Err("prepared base first virtual vertex must match output vertex count");
    }
    let mut vertex_map = input.vertex_map.to_vec();
    seed_prepared_base_contour_vertices(&mut vertex_map, &input.contour_vertex_maps);
    let mut vertices = input.output_vertices.to_vec();
    let mut next_virtual_vertex = input.first_virtual_vertex;
    let mut faces = Vec::with_capacity(input.prepared_faces.len());
    let mut face_sources = Vec::with_capacity(input.prepared_faces.len());
    for cut_face in input.prepared_faces {
        let Some(face) = input.cut_mesh.faces.get(*cut_face) else {
            continue;
        };
        let mapped = [
            mapped_vertex(
                &mut vertex_map,
                &mut vertices,
                input.cut_mesh,
                face[0] as usize,
                &mut next_virtual_vertex,
            )?,
            mapped_vertex(
                &mut vertex_map,
                &mut vertices,
                input.cut_mesh,
                face[1] as usize,
                &mut next_virtual_vertex,
            )?,
            mapped_vertex(
                &mut vertex_map,
                &mut vertices,
                input.cut_mesh,
                face[2] as usize,
                &mut next_virtual_vertex,
            )?,
        ];
        let face = if input.flip_orientation {
            [mapped[0] as i64, mapped[2] as i64, mapped[1] as i64]
        } else {
            [mapped[0] as i64, mapped[1] as i64, mapped[2] as i64]
        };
        faces.push(face);
        face_sources.push(ExactBooleanOutputFaceSource {
            operand: input.operand,
            cut_face: *cut_face,
            source_face: input
                .cut_mesh
                .source_face_for_faces
                .get(*cut_face)
                .copied()
                .unwrap_or(*cut_face),
        });
    }
    let virtual_vertices = next_virtual_vertex.saturating_sub(input.first_virtual_vertex);
    let mut topology = OutputFaceTopology::from_faces_with_sources(&faces, &face_sources)?;
    topology.use_meshlib_source_edge_identity();
    topology.use_meshlib_direct_record_rewrite();
    register_prepared_base_contour_edge_indices(
        &mut topology,
        input.cut_mesh,
        input.prepared_faces,
        &vertex_map,
        input.operand,
        input.flip_orientation,
    );
    Ok(ExactMeshlibPreparedBaseTopology {
        topology,
        vertices,
        faces,
        face_sources,
        virtual_vertices,
    })
}

fn seed_prepared_base_contour_vertices(
    vertex_map: &mut Vec<Option<usize>>,
    contour_vertex_maps: &[([usize; 2], [usize; 2])],
) {
    let mut seeded = BTreeSet::new();
    for (source_edge, output_edge) in contour_vertex_maps {
        seed_prepared_base_vertex(vertex_map, &mut seeded, source_edge[0], output_edge[0]);
        seed_prepared_base_vertex(vertex_map, &mut seeded, source_edge[1], output_edge[1]);
    }
}

fn seed_prepared_base_vertex(
    vertex_map: &mut Vec<Option<usize>>,
    seeded: &mut BTreeSet<usize>,
    source: usize,
    output: usize,
) {
    if !seeded.insert(source) {
        return;
    }
    if vertex_map.len() <= source {
        vertex_map.resize(source + 1, None);
    }
    if vertex_map[source].is_none() {
        vertex_map[source] = Some(output);
    }
}

fn mapped_vertex(
    vertex_map: &mut Vec<Option<usize>>,
    vertices: &mut Vec<[f64; 3]>,
    cut_mesh: &ExactCutMeshResult,
    source_vertex: usize,
    next_virtual_vertex: &mut usize,
) -> Result<usize, &'static str> {
    if vertex_map.len() <= source_vertex {
        vertex_map.resize(source_vertex + 1, None);
    }
    if let Some(mapped) = vertex_map[source_vertex] {
        if mapped >= vertices.len() {
            return Err("prepared base mapped vertex is outside output vertices");
        }
        return Ok(mapped);
    }
    let mapped = *next_virtual_vertex;
    if mapped != vertices.len() {
        return Err("prepared base virtual vertices must append contiguously");
    }
    let vertex = *cut_mesh
        .vertices
        .get(source_vertex)
        .ok_or("prepared base source vertex is missing")?;
    vertices.push(vertex);
    *next_virtual_vertex += 1;
    vertex_map[source_vertex] = Some(mapped);
    Ok(mapped)
}
