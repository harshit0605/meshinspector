use super::*;
use crate::spatial::exact_boolean::ExactBooleanOperand;
use crate::spatial::exact_stitch::{ExactStitchEdgePair, ExactStitchPath, ExactStitchPlan};
use std::collections::BTreeMap;

fn prepared_part(
    operand: ExactBooleanOperand,
    vertices: Vec<[f64; 3]>,
    directed_edges: Vec<([usize; 2], [usize; 2])>,
) -> MeshlibPreparedPart {
    MeshlibPreparedPart {
        operand,
        vertices,
        faces: Vec::new(),
        cut_to_part_vertices: Vec::new(),
        cut_to_part_faces: Vec::new(),
        cut_to_part_directed_edges: BTreeMap::from_iter(directed_edges),
        remapped_cut_edge_paths: Vec::new(),
        remapped_cut_edge_path_missing_edges: Vec::new(),
        missing_cut_path_edge_details: Vec::new(),
        remapped_cut_edge_path_closed: Vec::new(),
        remapped_cut_edge_paths_complete: true,
        skipped_faces: Vec::new(),
    }
}

#[test]
fn path_edge_diagnostics_reports_reversed_mapped_edges() {
    let connect_parts = MeshlibPreparedConnectParts {
        base_operand: ExactBooleanOperand::First,
        incoming_operand: ExactBooleanOperand::Second,
        base: prepared_part(
            ExactBooleanOperand::First,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            vec![([10, 11], [0, 1])],
        ),
        incoming: prepared_part(
            ExactBooleanOperand::Second,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            vec![([40, 50], [1, 0])],
        ),
    };
    let stitch_plan = ExactStitchPlan {
        pairs: vec![ExactStitchEdgePair {
            first_edge_index: 7,
            second_edge_index: 9,
            first_edge: [10, 11],
            second_edge: [40, 50],
            second_reversed: true,
        }],
        paths: vec![ExactStitchPath {
            pair_indices: vec![0],
            closed: false,
        }],
        unmatched_first_edges: Vec::new(),
        unmatched_second_edges: Vec::new(),
        compatible: true,
    };

    let diagnostics = connect_parts.path_edge_diagnostics_from_stitch_plan(1.0e-9, &stitch_plan);

    assert_eq!(diagnostics.len(), 1);
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.path_index, 0);
    assert_eq!(diagnostic.pair_index, 0);
    assert_eq!(diagnostic.base_edge_index, 7);
    assert_eq!(diagnostic.incoming_edge_index, 9);
    assert_eq!(diagnostic.base_edge, [10, 11]);
    assert_eq!(diagnostic.incoming_edge, [40, 50]);
    assert_eq!(diagnostic.mapped_base_edge, Some([0, 1]));
    assert_eq!(diagnostic.mapped_incoming_edge, Some([1, 0]));
    assert_eq!(
        diagnostic.base_points,
        Some([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]])
    );
    assert_eq!(
        diagnostic.incoming_points,
        Some([[1.0, 0.0, 0.0], [0.0, 0.0, 0.0]])
    );
    assert_eq!(
        diagnostic.relation,
        MeshlibPreparedConnectPathEdgeRelation::Reversed
    );
}
