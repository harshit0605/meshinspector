use super::super::exact_boolean::ExactBooleanOutputMesh;
use super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET;
use crate::mesh::{mesh_health, mesh_stats};
use crate::{GeometryError, MeshHealth, MeshStats};

pub(super) fn output_mesh_diagnostics(
    output: &ExactBooleanOutputMesh,
    epsilon: f64,
) -> Result<(MeshStats, MeshHealth), GeometryError> {
    let stats = mesh_stats(&output.vertices, &output.faces)?;
    let health = mesh_health(
        &output.vertices,
        &output.faces,
        true,
        Some(EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET),
        epsilon,
    )?;
    Ok((stats, health))
}
