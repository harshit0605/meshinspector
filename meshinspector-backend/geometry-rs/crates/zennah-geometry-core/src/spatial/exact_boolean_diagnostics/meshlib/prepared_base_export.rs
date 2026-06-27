use super::{meshlib_prepared_base_rewrite_plan, MeshlibRewriteDiagnosticsInput};
use crate::spatial::exact_boolean_topology::meshlib_topology_rewrite_plan;
use crate::GeometryError;

pub(in crate::spatial) struct MeshlibPreparedBaseExport {
    pub(in crate::spatial) vertices: Vec<[f64; 3]>,
    pub(in crate::spatial) faces: Vec<[i64; 3]>,
}

pub(in crate::spatial) fn meshlib_prepared_base_export(
    input: MeshlibRewriteDiagnosticsInput<'_>,
) -> Result<Option<MeshlibPreparedBaseExport>, GeometryError> {
    let topology_rewrite = meshlib_topology_rewrite_plan(input.assembly, input.operation);
    if topology_rewrite.record_rewrite_command_edges.is_empty() {
        return Ok(None);
    }
    let prepared_base = meshlib_prepared_base_rewrite_plan(&input, &topology_rewrite);
    if !prepared_base.apply.apply.ready_for_export {
        return Ok(None);
    }
    Ok(Some(MeshlibPreparedBaseExport {
        vertices: prepared_base.apply.vertices,
        faces: prepared_base.apply.apply.exported_face_indices,
    }))
}
