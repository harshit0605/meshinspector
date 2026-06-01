use super::super::super::exact_boolean::{ExactBooleanOperand, ExactBooleanOutputFaceSource};
use super::super::super::exact_cut_apply::ExactCutMeshResult;
use std::collections::BTreeSet;

pub(super) fn copied_vertex_map(
    vertex_map: &[Option<usize>],
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    first_virtual_vertex: usize,
    contour_vertex_maps: &[([usize; 2], [usize; 2])],
) -> Vec<Option<usize>> {
    let mut copied_map = vertex_map.to_vec();
    for (source_edge, output_edge) in contour_vertex_maps {
        set_copied_vertex(&mut copied_map, source_edge[0], output_edge[0]);
        set_copied_vertex(&mut copied_map, source_edge[1], output_edge[1]);
    }
    fill_missing_prepared_vertices(
        &mut copied_map,
        cut_mesh,
        prepared_faces,
        first_virtual_vertex,
    );
    copied_map
}

pub(super) fn connect_prepared_parts_vertex_map(
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    first_virtual_vertex: usize,
    contour_vertex_maps: &[([usize; 2], [usize; 2])],
) -> Vec<Option<usize>> {
    let mut copied_map = Vec::new();
    for (source_edge, output_edge) in contour_vertex_maps {
        set_copied_vertex(&mut copied_map, source_edge[0], output_edge[0]);
        set_copied_vertex(&mut copied_map, source_edge[1], output_edge[1]);
    }
    fill_missing_prepared_vertices(
        &mut copied_map,
        cut_mesh,
        prepared_faces,
        first_virtual_vertex,
    );
    copied_map
}

pub(super) fn copied_face_map(
    source_faces: usize,
    prepared_faces: &[usize],
    face_sources: &[ExactBooleanOutputFaceSource],
    incoming_operand: ExactBooleanOperand,
    first_virtual_face: usize,
) -> Vec<Option<usize>> {
    let max_prepared_face = prepared_faces.iter().copied().max().unwrap_or(0);
    let mut face_map = vec![None; source_faces.max(max_prepared_face + 1)];
    let prepared_face_set = prepared_faces.iter().copied().collect::<BTreeSet<_>>();
    for (output_face, source) in face_sources.iter().enumerate() {
        if source.operand == incoming_operand
            && source.cut_face < face_map.len()
            && prepared_face_set.contains(&source.cut_face)
        {
            face_map[source.cut_face] = Some(output_face);
        }
    }
    let mut next_virtual_face = first_virtual_face;
    for face in prepared_faces {
        if face_map[*face].is_none() {
            face_map[*face] = Some(next_virtual_face);
            next_virtual_face += 1;
        }
    }
    face_map
}

fn fill_missing_prepared_vertices(
    copied_map: &mut Vec<Option<usize>>,
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    first_virtual_vertex: usize,
) {
    let mut next_virtual_vertex = first_virtual_vertex;
    for vertex in prepared_region_vertices(cut_mesh, prepared_faces) {
        if copied_map.len() <= vertex {
            copied_map.resize(vertex + 1, None);
        }
        if copied_map[vertex].is_none() {
            copied_map[vertex] = Some(next_virtual_vertex);
            next_virtual_vertex += 1;
        }
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

fn prepared_region_vertices(cut_mesh: &ExactCutMeshResult, prepared_faces: &[usize]) -> Vec<usize> {
    let mut vertices = Vec::new();
    for face_index in prepared_faces {
        let Some(face) = cut_mesh.faces.get(*face_index) else {
            continue;
        };
        for vertex in face {
            let vertex = *vertex as usize;
            if !vertices.contains(&vertex) {
                vertices.push(vertex);
            }
        }
    }
    vertices.sort_unstable();
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_vertex_map_keeps_first_meshlib_assignment() {
        let cut_mesh = ExactCutMeshResult {
            vertices: vec![[0.0; 3]; 3],
            faces: vec![[0, 1, 2]],
            cut_edges: Vec::new(),
            cut_edge_paths: Vec::new(),
            cut_edge_path_closed: Vec::new(),
            source_face_for_faces: vec![0],
            skipped_source_faces: Vec::new(),
        };

        let copied = connect_prepared_parts_vertex_map(
            &cut_mesh,
            &[0],
            20,
            &[([0, 1], [10, 11]), ([1, 2], [99, 12])],
        );

        assert_eq!(copied[0], Some(10));
        assert_eq!(
            copied[1],
            Some(11),
            "MeshLib setVmap keeps the first contour vertex assignment"
        );
        assert_eq!(copied[2], Some(12));
    }
}
