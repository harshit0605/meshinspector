use super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOperation, ExactBooleanOutputMesh,
    ExactBooleanOutputMeshSource,
};
use super::exact_boolean_diagnostics::meshlib::{
    meshlib_prepared_base_export, MeshlibRewriteDiagnosticsInput,
};
use super::exact_cut_apply::ExactCutMeshResult;
use crate::mesh::mesh_health;
use crate::GeometryError;

const EXACT_BOOLEAN_OUTPUT_SELF_INTERSECTION_FACE_BUDGET: usize = 20_000;

pub(super) struct ExactBooleanOutputMeshInput<'a> {
    pub(super) first_cut: &'a super::exact_fill_apply::ExactCutHoleFillResult,
    pub(super) second_cut: &'a super::exact_fill_apply::ExactCutHoleFillResult,
    pub(super) assembly: &'a ExactBooleanAssemblyResult,
    pub(super) operation: ExactBooleanOperation,
    pub(super) epsilon: f64,
}

pub(super) fn exact_boolean_public_output_mesh(
    input: ExactBooleanOutputMeshInput<'_>,
) -> Result<ExactBooleanOutputMesh, GeometryError> {
    if let Some(output) = meshlib_prepared_base_output(&input)? {
        return Ok(output);
    }
    Ok(ExactBooleanOutputMesh {
        vertices: input.assembly.vertices.clone(),
        faces: input.assembly.faces.clone(),
        source: ExactBooleanOutputMeshSource::Assembly,
    })
}

fn meshlib_prepared_base_output(
    input: &ExactBooleanOutputMeshInput<'_>,
) -> Result<Option<ExactBooleanOutputMesh>, GeometryError> {
    let Some(export) = meshlib_prepared_base_export(MeshlibRewriteDiagnosticsInput {
        first_cut: cut_mesh(input.first_cut),
        second_cut: cut_mesh(input.second_cut),
        first_added_face_ranges: &input.first_cut.added_face_ranges,
        second_added_face_ranges: &input.second_cut.added_face_ranges,
        stitch_plan: None,
        assembly: input.assembly,
        operation: input.operation,
        epsilon: input.epsilon,
    })?
    else {
        return Ok(None);
    };
    if export.faces.is_empty() {
        return Ok(None);
    }
    if !faces_are_in_vertex_range(&export.vertices, &export.faces) {
        return Ok(None);
    }
    let health = mesh_health(
        &export.vertices,
        &export.faces,
        true,
        Some(EXACT_BOOLEAN_OUTPUT_SELF_INTERSECTION_FACE_BUDGET),
        input.epsilon,
    )?;
    if !health.is_closed || health.boundary_edge_count != 0 || health.nonmanifold_edge_count != 0 {
        return Ok(None);
    }
    Ok(Some(ExactBooleanOutputMesh {
        vertices: export.vertices,
        faces: export.faces,
        source: ExactBooleanOutputMeshSource::MeshlibPreparedBaseExport,
    }))
}

fn cut_mesh(fill: &super::exact_fill_apply::ExactCutHoleFillResult) -> &ExactCutMeshResult {
    &fill.mesh
}

fn faces_are_in_vertex_range(vertices: &[[f64; 3]], faces: &[[i64; 3]]) -> bool {
    faces.iter().all(|face| {
        face.iter()
            .all(|&vertex| vertex >= 0 && (vertex as usize) < vertices.len())
    })
}
