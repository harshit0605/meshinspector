use super::exact_boolean::ExactBooleanStitchedEdgeSource;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactTopologySpliceStatus {
    Missing,
    BoundaryNeedsSplice,
    Manifold,
    NonManifold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactTopologySpliceEntry {
    pub stitched_edge_index: usize,
    pub output_edge: [usize; 2],
    pub first_output_edge: Option<[usize; 2]>,
    pub second_output_edge: Option<[usize; 2]>,
    pub first_stitch_edge: Option<[usize; 2]>,
    pub second_stitch_edge: Option<[usize; 2]>,
    pub first_stitch_edge_synthetic: bool,
    pub second_stitch_edge_synthetic: bool,
    pub incident_faces: Vec<usize>,
    pub status: ExactTopologySpliceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactTopologySplicePlan {
    pub entries: Vec<ExactTopologySpliceEntry>,
    pub missing_edges: usize,
    pub boundary_edges: usize,
    pub manifold_edges: usize,
    pub non_manifold_edges: usize,
    pub requires_splice: bool,
    pub ready_for_splice: bool,
}

pub fn exact_topology_splice_plan(
    faces: &[[i64; 3]],
    stitched_edges: &[ExactBooleanStitchedEdgeSource],
) -> ExactTopologySplicePlan {
    let edge_faces = edge_face_map(faces);
    let mut entries = Vec::with_capacity(stitched_edges.len());
    let mut missing_edges = 0;
    let mut boundary_edges = 0;
    let mut manifold_edges = 0;
    let mut non_manifold_edges = 0;

    for (index, source) in stitched_edges.iter().enumerate() {
        let incident_faces = edge_faces
            .get(&source.output_edge)
            .cloned()
            .unwrap_or_default();
        let status = match incident_faces.len() {
            0 => {
                missing_edges += 1;
                ExactTopologySpliceStatus::Missing
            }
            1 => {
                boundary_edges += 1;
                ExactTopologySpliceStatus::BoundaryNeedsSplice
            }
            2 => {
                manifold_edges += 1;
                ExactTopologySpliceStatus::Manifold
            }
            _ => {
                non_manifold_edges += 1;
                ExactTopologySpliceStatus::NonManifold
            }
        };
        entries.push(ExactTopologySpliceEntry {
            stitched_edge_index: index,
            output_edge: source.output_edge,
            first_output_edge: source.first_output_edge,
            second_output_edge: source.second_output_edge,
            first_stitch_edge: source.first_stitch_edge,
            second_stitch_edge: source.second_stitch_edge,
            first_stitch_edge_synthetic: source.first_stitch_edge_synthetic,
            second_stitch_edge_synthetic: source.second_stitch_edge_synthetic,
            incident_faces,
            status,
        });
    }

    ExactTopologySplicePlan {
        entries,
        missing_edges,
        boundary_edges,
        manifold_edges,
        non_manifold_edges,
        requires_splice: boundary_edges > 0,
        ready_for_splice: missing_edges == 0 && non_manifold_edges == 0,
    }
}

fn edge_face_map(faces: &[[i64; 3]]) -> BTreeMap<[usize; 2], Vec<usize>> {
    let mut edge_faces = BTreeMap::<[usize; 2], Vec<usize>>::new();
    for (face_index, face) in faces.iter().enumerate() {
        let face = [face[0] as usize, face[1] as usize, face[2] as usize];
        for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
            edge_faces
                .entry(ordered_edge(edge))
                .or_default()
                .push(face_index);
        }
    }
    edge_faces
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

    fn stitched_edge(output_edge: [usize; 2]) -> ExactBooleanStitchedEdgeSource {
        ExactBooleanStitchedEdgeSource {
            output_edge,
            first_output_edge: Some(output_edge),
            second_output_edge: Some([output_edge[1], output_edge[0]]),
            first_stitch_edge: Some(output_edge),
            second_stitch_edge: Some([output_edge[1], output_edge[0]]),
            first_stitch_edge_synthetic: false,
            second_stitch_edge_synthetic: false,
            first_edge_index: 0,
            second_edge_index: 0,
            first_cut_edge: output_edge,
            second_cut_edge: output_edge,
        }
    }

    #[test]
    fn exact_topology_splice_plan_classifies_manifold_stitched_edge() {
        let plan = exact_topology_splice_plan(&[[0, 1, 2], [2, 1, 3]], &[stitched_edge([1, 2])]);

        assert_eq!(plan.manifold_edges, 1);
        assert_eq!(plan.entries[0].status, ExactTopologySpliceStatus::Manifold);
        assert!(!plan.requires_splice);
        assert!(plan.ready_for_splice);
    }

    #[test]
    fn exact_topology_splice_plan_marks_boundary_edge_as_splice_work() {
        let plan = exact_topology_splice_plan(&[[0, 1, 2]], &[stitched_edge([1, 2])]);

        assert_eq!(plan.boundary_edges, 1);
        assert_eq!(
            plan.entries[0].status,
            ExactTopologySpliceStatus::BoundaryNeedsSplice
        );
        assert!(plan.requires_splice);
        assert!(plan.ready_for_splice);
    }

    #[test]
    fn exact_topology_splice_plan_marks_non_manifold_edge_not_ready() {
        let plan = exact_topology_splice_plan(
            &[[0, 1, 2], [2, 1, 3], [1, 2, 4]],
            &[stitched_edge([1, 2])],
        );

        assert_eq!(plan.non_manifold_edges, 1);
        assert_eq!(
            plan.entries[0].status,
            ExactTopologySpliceStatus::NonManifold
        );
        assert!(!plan.ready_for_splice);
    }
}
