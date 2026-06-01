use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use zennah_geometry_core::{DrainHolePlan, MeshArrays};

use crate::convert::{
    parse_voxel_mesh_extractor, read_f64_values, read_faces, read_i64_values, read_vec3,
    read_vertices,
};

pub(crate) fn mesh_arrays_dict(py: Python<'_>, result: MeshArrays) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    Ok(output.unbind())
}

pub(crate) fn adaptive_hollow_dict(
    py: Python<'_>,
    result: zennah_geometry_core::AdaptiveHollowResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("achieved_weight_g", result.achieved_weight_g)?;
    output.set_item("wall_thickness_mm", result.wall_thickness_mm)?;
    output.set_item("iterations", result.iterations)?;
    output.set_item("warning", result.warning)?;
    output.set_item("original_weight_g", result.original_weight_g)?;
    output.set_item("target_weight_g", result.target_weight_g)?;
    Ok(output.unbind())
}

fn plan_dict(py: Python<'_>, plan: DrainHolePlan) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("center_mm", plan.center_mm)?;
    output.set_item("direction", plan.direction)?;
    output.set_item("radius_mm", plan.radius_mm)?;
    output.set_item("length_mm", plan.length_mm)?;
    Ok(output.unbind())
}

fn read_plans(
    centers: PyReadonlyArray2<'_, f64>,
    directions: PyReadonlyArray2<'_, f64>,
    radii: PyReadonlyArray1<'_, f64>,
    lengths: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<DrainHolePlan>> {
    let rust_centers = crate::convert::read_points(centers)?;
    let rust_directions = crate::convert::read_points(directions)?;
    let rust_radii = read_f64_values(radii);
    let rust_lengths = read_f64_values(lengths);
    if rust_centers.len() != rust_directions.len()
        || rust_centers.len() != rust_radii.len()
        || rust_centers.len() != rust_lengths.len()
    {
        return Err(PyValueError::new_err(
            zennah_geometry_core::GeometryError::DrainHolePlanCountMismatch {
                centers: rust_centers.len(),
                directions: rust_directions.len(),
                radii: rust_radii.len(),
                lengths: rust_lengths.len(),
            }
            .to_string(),
        ));
    }

    Ok(rust_centers
        .into_iter()
        .zip(rust_directions)
        .zip(rust_radii)
        .zip(rust_lengths)
        .map(
            |(((center_mm, direction), radius_mm), length_mm)| DrainHolePlan {
                center_mm,
                direction,
                radius_mm,
                length_mm,
            },
        )
        .collect())
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn protected_hollow_scale_field(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    protect_region_ids: Vec<String>,
    base_thickness_mm: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_vertex_offsets = read_i64_values(vertex_offsets);
    let rust_vertex_indices = read_i64_values(vertex_indices);
    let scales = py
        .detach(|| {
            zennah_geometry_core::protected_hollow_scale_field(
                &rust_vertices,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                &protect_region_ids,
                base_thickness_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(scales.into_pyarray(py).unbind())
}

#[pyfunction]
fn inward_directions_for_hollow(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let directions = py
        .detach(|| zennah_geometry_core::inward_directions_for_hollow(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = directions.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn weighted_inner_offset_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    protect_region_ids: Vec<String>,
    wall_thickness_mm: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_vertex_offsets = read_i64_values(vertex_offsets);
    let rust_vertex_indices = read_i64_values(vertex_indices);
    let displaced = py
        .detach(|| {
            zennah_geometry_core::weighted_inner_offset_vertices(
                &rust_vertices,
                &rust_faces,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                &protect_region_ids,
                wall_thickness_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = displaced.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, region_ids, vertex_offsets, vertex_indices, ring_axis, wall_thickness_mm, hole_diameter_mm = 0.8))]
#[allow(clippy::too_many_arguments)]
fn plan_drain_holes(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    ring_axis: PyReadonlyArray1<'_, f64>,
    wall_thickness_mm: f64,
    hole_diameter_mm: f64,
) -> PyResult<Py<PyList>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_vertex_offsets = read_i64_values(vertex_offsets);
    let rust_vertex_indices = read_i64_values(vertex_indices);
    let rust_axis = read_vec3("ring_axis", ring_axis)?;
    let plans = py
        .detach(|| {
            zennah_geometry_core::plan_drain_holes(
                &rust_vertices,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                rust_axis,
                wall_thickness_mm,
                hole_diameter_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyList::empty(py);
    for plan in plans {
        output.append(plan_dict(py, plan)?)?;
    }
    Ok(output.unbind())
}

#[pyfunction(signature = (center, direction, radius_mm, length_mm, sections = 32))]
fn drain_hole_cutter_mesh(
    py: Python<'_>,
    center: PyReadonlyArray1<'_, f64>,
    direction: PyReadonlyArray1<'_, f64>,
    radius_mm: f64,
    length_mm: f64,
    sections: usize,
) -> PyResult<Py<PyDict>> {
    let plan = DrainHolePlan {
        center_mm: read_vec3("center", center)?,
        direction: read_vec3("direction", direction)?,
        radius_mm,
        length_mm,
    };
    let mesh = py
        .detach(|| zennah_geometry_core::drain_hole_cutter_mesh(plan, sections))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_dict(py, mesh)
}

#[pyfunction(signature = (centers, directions, radii, lengths, sections = 32))]
fn drain_hole_cutters_mesh(
    py: Python<'_>,
    centers: PyReadonlyArray2<'_, f64>,
    directions: PyReadonlyArray2<'_, f64>,
    radii: PyReadonlyArray1<'_, f64>,
    lengths: PyReadonlyArray1<'_, f64>,
    sections: usize,
) -> PyResult<Py<PyDict>> {
    let plans = read_plans(centers, directions, radii, lengths)?;
    let mesh = py
        .detach(|| zennah_geometry_core::drain_hole_cutters_mesh(&plans, sections))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_dict(py, mesh)
}

#[pyfunction(signature = (
    vertices,
    faces,
    target_weight_g,
    material = "gold_18k",
    tolerance_g = 0.1,
    min_thickness_mm = 0.5,
    max_thickness_mm = 3.0,
    max_iterations = 20,
    voxel_size_mm = 0.5,
    padding_mm = None,
    extractor = "marching",
    refine = false
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_hollow_to_weight(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    target_weight_g: f64,
    material: &str,
    tolerance_g: f64,
    min_thickness_mm: f64,
    max_thickness_mm: f64,
    max_iterations: usize,
    voxel_size_mm: f64,
    padding_mm: Option<f64>,
    extractor: &str,
    refine: bool,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_extractor = parse_voxel_mesh_extractor(extractor)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::adaptive_hollow_to_weight(
                &rust_vertices,
                &rust_faces,
                target_weight_g,
                material,
                tolerance_g,
                min_thickness_mm,
                max_thickness_mm,
                max_iterations,
                zennah_geometry_core::VoxelMeshOptions {
                    voxel_size: voxel_size_mm,
                    padding_mm,
                    extractor: rust_extractor,
                    refine,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    adaptive_hollow_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(protected_hollow_scale_field, module)?)?;
    module.add_function(wrap_pyfunction!(inward_directions_for_hollow, module)?)?;
    module.add_function(wrap_pyfunction!(weighted_inner_offset_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(plan_drain_holes, module)?)?;
    module.add_function(wrap_pyfunction!(drain_hole_cutter_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(drain_hole_cutters_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(adaptive_hollow_to_weight, module)?)?;
    Ok(())
}
