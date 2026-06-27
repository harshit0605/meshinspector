use super::super::exact_boolean::ExactBooleanOperation;
use super::super::exact_cut_pair::ExactCoplanarContourCutTrial;
use crate::GeometryError;

pub(super) fn trial_usize(
    trial: Option<&ExactCoplanarContourCutTrial>,
    value: impl FnOnce(&ExactCoplanarContourCutTrial) -> usize,
) -> usize {
    trial.map(value).unwrap_or_default()
}

pub(super) fn trial_vec_vec_usize(
    trial: Option<&ExactCoplanarContourCutTrial>,
    value: impl FnOnce(&ExactCoplanarContourCutTrial) -> &Vec<Vec<usize>>,
) -> Vec<Vec<usize>> {
    trial.map(value).cloned().unwrap_or_default()
}

pub(super) fn vertices_have_mixed_inside_state(
    vertices: &[[f64; 3]],
    other_vertices: &[[f64; 3]],
    other_faces: &[[i64; 3]],
    operation: ExactBooleanOperation,
    epsilon: f64,
) -> Result<bool, GeometryError> {
    if vertices.is_empty() || other_faces.is_empty() || !requires_topology_splice(operation) {
        return Ok(false);
    }
    let mut has_inside = false;
    let mut has_outside = false;
    for vertex in vertices {
        let inside = super::super::point_inside_mesh(
            other_vertices,
            other_faces,
            *vertex,
            super::BOOLEAN_DIAGNOSTIC_RAY_DIRECTION,
            epsilon,
        )?;
        has_inside |= inside;
        has_outside |= !inside;
        if has_inside && has_outside {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn requires_topology_splice(operation: ExactBooleanOperation) -> bool {
    matches!(
        operation,
        ExactBooleanOperation::Union
            | ExactBooleanOperation::Intersection
            | ExactBooleanOperation::DifferenceAB
            | ExactBooleanOperation::DifferenceBA
    )
}
