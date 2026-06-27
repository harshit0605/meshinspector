use super::super::exact_boolean::ExactBooleanOperand;
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::output_topology::OutputFaceTopology;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn register_prepared_base_contour_edge_indices(
    topology: &mut OutputFaceTopology,
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    vertex_map: &[Option<usize>],
    operand: ExactBooleanOperand,
    flip_orientation: bool,
) {
    let mut used_targets = BTreeSet::new();
    let registered_from_paths = register_prepared_base_contour_paths(
        topology,
        cut_mesh,
        prepared_faces,
        vertex_map,
        operand,
        flip_orientation,
        &mut used_targets,
    );
    if cut_mesh.cut_edge_paths.iter().any(|path| !path.is_empty()) {
        return;
    }

    for (edge_index, source_edge) in cut_mesh.cut_edges.iter().copied().enumerate() {
        if registered_from_paths.contains(&edge_index) {
            continue;
        }
        let Some(target) = prepared_base_contour_target_edge(
            topology,
            cut_mesh,
            prepared_faces,
            vertex_map,
            source_edge,
            flip_orientation,
            &used_targets,
        ) else {
            continue;
        };
        topology.register_meshlib_mapped_contour_edge_index(operand, edge_index, target);
        used_targets.insert(target);
    }
}

fn register_prepared_base_contour_paths(
    topology: &mut OutputFaceTopology,
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    vertex_map: &[Option<usize>],
    operand: ExactBooleanOperand,
    flip_orientation: bool,
    used_targets: &mut BTreeSet<ExactHalfEdgeId>,
) -> BTreeSet<usize> {
    let mut cut_edge_indices = CutEdgeOccurrenceLookup::new(&cut_mesh.cut_edges);
    let mut registered = BTreeSet::new();
    for path in &cut_mesh.cut_edge_paths {
        for source_edge in path {
            let Some(edge_index) = cut_edge_indices.take(*source_edge) else {
                continue;
            };
            let Some(target) = prepared_base_contour_target_edge(
                topology,
                cut_mesh,
                prepared_faces,
                vertex_map,
                *source_edge,
                flip_orientation,
                used_targets,
            ) else {
                continue;
            };
            topology.register_meshlib_mapped_contour_edge_index(operand, edge_index, target);
            registered.insert(edge_index);
            used_targets.insert(target);
        }
    }
    registered
}

struct CutEdgeOccurrenceLookup {
    directed: BTreeMap<[usize; 2], Vec<usize>>,
    undirected: BTreeMap<[usize; 2], Vec<usize>>,
    used: BTreeSet<usize>,
}

impl CutEdgeOccurrenceLookup {
    fn new(cut_edges: &[[usize; 2]]) -> Self {
        let mut lookup = Self {
            directed: BTreeMap::new(),
            undirected: BTreeMap::new(),
            used: BTreeSet::new(),
        };
        for (index, edge) in cut_edges.iter().copied().enumerate() {
            lookup.directed.entry(edge).or_default().push(index);
            lookup
                .undirected
                .entry(ordered_edge(edge))
                .or_default()
                .push(index);
        }
        lookup
    }

    fn take(&mut self, edge: [usize; 2]) -> Option<usize> {
        self.take_from_indices(self.directed.get(&edge).cloned())
            .or_else(|| self.take_from_indices(self.undirected.get(&ordered_edge(edge)).cloned()))
    }

    fn take_from_indices(&mut self, indices: Option<Vec<usize>>) -> Option<usize> {
        indices?.into_iter().find(|&index| self.used.insert(index))
    }
}

fn prepared_base_contour_target_edge(
    topology: &OutputFaceTopology,
    cut_mesh: &ExactCutMeshResult,
    prepared_faces: &[usize],
    vertex_map: &[Option<usize>],
    source_edge: [usize; 2],
    flip_orientation: bool,
    used_targets: &BTreeSet<ExactHalfEdgeId>,
) -> Option<ExactHalfEdgeId> {
    let mapped_source_edge = mapped_source_edge(source_edge, vertex_map);
    let mut candidates = Vec::new();
    let mut output_face_index = 0;
    for cut_face_index in prepared_faces {
        let Some(face) = cut_mesh.faces.get(*cut_face_index) else {
            continue;
        };
        let source_face = [face[0] as usize, face[1] as usize, face[2] as usize];
        for face_edge in source_face_edges(source_face) {
            if ordered_edge(face_edge) != ordered_edge(source_edge) {
                continue;
            }
            let Some(output_face_edge) =
                mapped_output_face_edge(face_edge, vertex_map, flip_orientation)
            else {
                continue;
            };
            let Some(edge_id) = topology.directed_face_edge(output_face_index, output_face_edge)
            else {
                continue;
            };
            let default_target = ExactHalfEdgeTopology::sym(edge_id);
            for target in [default_target, ExactHalfEdgeTopology::sym(default_target)] {
                if used_targets.contains(&target) || topology.topology.left(target).is_some() {
                    continue;
                }
                candidates.push(PreparedBaseContourTarget {
                    edge: target,
                    source_direction_matches: mapped_source_edge
                        .is_some_and(|edge| target_matches_source_edge(topology, target, edge)),
                });
            }
        }
        output_face_index += 1;
    }
    best_prepared_base_contour_target(topology, candidates)
}

#[derive(Clone, Copy)]
struct PreparedBaseContourTarget {
    edge: ExactHalfEdgeId,
    source_direction_matches: bool,
}

fn best_prepared_base_contour_target(
    topology: &OutputFaceTopology,
    candidates: Vec<PreparedBaseContourTarget>,
) -> Option<ExactHalfEdgeId> {
    candidates
        .iter()
        .copied()
        .find(|candidate| {
            candidate.source_direction_matches
                && prepared_base_contour_boundary_score(topology, candidate.edge) == 2
        })
        .or_else(|| {
            candidates.iter().copied().find(|candidate| {
                candidate.source_direction_matches
                    && prepared_base_contour_boundary_score(topology, candidate.edge) == 1
            })
        })
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .find(|candidate| candidate.source_direction_matches)
        })
        .or_else(|| {
            candidates.iter().copied().find(|candidate| {
                prepared_base_contour_boundary_score(topology, candidate.edge) == 2
            })
        })
        .or_else(|| {
            candidates.iter().copied().find(|candidate| {
                prepared_base_contour_boundary_score(topology, candidate.edge) == 1
            })
        })
        .or_else(|| candidates.first().copied())
        .map(|candidate| candidate.edge)
}

fn prepared_base_contour_boundary_score(
    topology: &OutputFaceTopology,
    target: ExactHalfEdgeId,
) -> u8 {
    let start_edge = topology.topology.prev(ExactHalfEdgeTopology::sym(target));
    let end_edge = topology.topology.next(target);
    let start_ready = topology.topology.left(start_edge).is_none();
    let end_ready = topology.topology.right(end_edge).is_none();
    u8::from(start_ready) + u8::from(end_ready)
}

fn mapped_output_face_edge(
    edge: [usize; 2],
    vertex_map: &[Option<usize>],
    flip_orientation: bool,
) -> Option<[usize; 2]> {
    let mapped = [
        *vertex_map.get(edge[0])?.as_ref()?,
        *vertex_map.get(edge[1])?.as_ref()?,
    ];
    Some(if flip_orientation {
        [mapped[1], mapped[0]]
    } else {
        mapped
    })
}

fn mapped_source_edge(edge: [usize; 2], vertex_map: &[Option<usize>]) -> Option<[usize; 2]> {
    Some([
        *vertex_map.get(edge[0])?.as_ref()?,
        *vertex_map.get(edge[1])?.as_ref()?,
    ])
}

fn target_matches_source_edge(
    topology: &OutputFaceTopology,
    target: ExactHalfEdgeId,
    mapped_source_edge: [usize; 2],
) -> bool {
    topology.topology.origin(target) == Some(mapped_source_edge[0])
        && topology.topology.origin(ExactHalfEdgeTopology::sym(target))
            == Some(mapped_source_edge[1])
}

fn source_face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
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
    use super::*;

    #[test]
    fn best_prepared_base_contour_target_prefers_open_near_stitch_sides() {
        let mut topology = OutputFaceTopology::from_faces(&[[0, 1, 2], [3, 4, 5]]).unwrap();
        let low_face_edge = topology.directed_face_edge(0, [0, 1]).unwrap();
        let high_face_edge = topology.directed_face_edge(1, [3, 4]).unwrap();
        let low = ExactHalfEdgeTopology::sym(low_face_edge);
        let high = ExactHalfEdgeTopology::sym(high_face_edge);
        let low_start = topology.topology.prev(low_face_edge);
        let low_end = topology.topology.next(low);
        let high_start = topology.topology.prev(high_face_edge);
        let high_end = topology.topology.next(high);

        topology
            .topology
            .set_left_direct(low_start, Some(99))
            .unwrap();
        topology
            .topology
            .set_left_direct(ExactHalfEdgeTopology::sym(low_end), Some(98))
            .unwrap();
        topology.topology.set_left_direct(high_start, None).unwrap();
        topology
            .topology
            .set_left_direct(ExactHalfEdgeTopology::sym(high_end), None)
            .unwrap();

        assert_eq!(prepared_base_contour_boundary_score(&topology, high), 2);
        assert!(
            prepared_base_contour_boundary_score(&topology, low)
                < prepared_base_contour_boundary_score(&topology, high)
        );
        assert_eq!(
            best_prepared_base_contour_target(
                &topology,
                vec![
                    PreparedBaseContourTarget {
                        edge: low,
                        source_direction_matches: false,
                    },
                    PreparedBaseContourTarget {
                        edge: high,
                        source_direction_matches: false,
                    },
                ],
            ),
            Some(high)
        );
    }

    #[test]
    fn best_prepared_base_contour_target_prefers_source_direction_match() {
        let topology = OutputFaceTopology::from_faces(&[[0, 1, 2], [3, 4, 5]]).unwrap();
        let first_face_edge = topology.directed_face_edge(0, [0, 1]).unwrap();
        let second_face_edge = topology.directed_face_edge(1, [3, 4]).unwrap();
        let first = ExactHalfEdgeTopology::sym(first_face_edge);
        let second = ExactHalfEdgeTopology::sym(second_face_edge);

        assert_eq!(prepared_base_contour_boundary_score(&topology, first), 2);
        assert_eq!(prepared_base_contour_boundary_score(&topology, second), 2);
        assert_eq!(
            best_prepared_base_contour_target(
                &topology,
                vec![
                    PreparedBaseContourTarget {
                        edge: first,
                        source_direction_matches: false,
                    },
                    PreparedBaseContourTarget {
                        edge: second,
                        source_direction_matches: true,
                    },
                ],
            ),
            Some(second)
        );
    }
}
