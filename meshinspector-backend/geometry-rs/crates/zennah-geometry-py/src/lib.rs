mod aabb_tree;
mod analysis;
mod boolean;
mod compare_service;
mod convert;
mod deform;
mod deform_smooth;
mod deform_target;
mod distance;
mod health_service;
mod hollow;
mod hollow_shell;
mod jewelry;
mod manufacturability;
mod materials;
mod mesh;
mod repair;
mod resize;
mod sdf_grid;
mod sdf_marching;
mod signed_distance;
mod spatial;
mod thickness;
mod topology;
mod voxel;
mod voxel_mesh_ops;

use pyo3::prelude::*;

#[pymodule]
fn _zennah_geometry_rs(module: &Bound<'_, PyModule>) -> PyResult<()> {
    mesh::register(module)?;
    boolean::register(module)?;
    analysis::register(module)?;
    compare_service::register(module)?;
    spatial::register(module)?;
    thickness::register(module)?;
    aabb_tree::register(module)?;
    signed_distance::register(module)?;
    voxel::register(module)?;
    sdf_grid::register(module)?;
    sdf_marching::register(module)?;
    voxel_mesh_ops::register(module)?;
    deform::register(module)?;
    deform_smooth::register(module)?;
    deform_target::register(module)?;
    distance::register(module)?;
    health_service::register(module)?;
    resize::register(module)?;
    jewelry::register(module)?;
    manufacturability::register(module)?;
    materials::register(module)?;
    hollow::register(module)?;
    hollow_shell::register(module)?;
    repair::register(module)?;
    topology::register(module)?;
    Ok(())
}
