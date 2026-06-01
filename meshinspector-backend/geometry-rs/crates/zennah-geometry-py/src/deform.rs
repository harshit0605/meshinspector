use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use zennah_geometry_core::SmoothFalloffOptions;

use crate::convert::{
    read_f32_values, read_f64_values, read_faces, read_i64_values, read_vertices,
};

type PyIndexPair = (Py<PyArray1<i64>>, Py<PyArray1<i64>>);

#[pyfunction(signature = (vertices, faces, iterations = 1, strength = 0.25))]
fn laplacian_smooth_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    iterations: i64,
    strength: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let smoothed = py
        .detach(|| {
            zennah_geometry_core::laplacian_smooth_vertices(
                &rust_vertices,
                &rust_faces,
                iterations,
                strength,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = smoothed.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, faces, weights, iterations = 1, strength = 0.25, active_threshold = 0.02))]
fn weighted_laplacian_smooth_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    weights: PyReadonlyArray1<'_, f32>,
    iterations: i64,
    strength: f64,
    active_threshold: f32,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_weights = read_f32_values(weights);
    let smoothed = py
        .detach(|| {
            zennah_geometry_core::weighted_laplacian_smooth_vertices(
                &rust_vertices,
                &rust_faces,
                &rust_weights,
                iterations,
                strength,
                active_threshold,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = smoothed.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, seed_indices, falloff_mm, cutoff_multiplier = 3.0))]
fn falloff_weights(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    seed_indices: PyReadonlyArray1<'_, i64>,
    falloff_mm: f64,
    cutoff_multiplier: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_seed_indices = read_i64_values(seed_indices);
    let weights = py
        .detach(|| {
            zennah_geometry_core::falloff_weights(
                &rust_vertices,
                &rust_seed_indices,
                falloff_mm,
                cutoff_multiplier,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(weights.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, faces, seed_indices, falloff_mm, iterations = 5, strength = 0.5, active_threshold = 0.02, cutoff_multiplier = 3.0))]
#[allow(clippy::too_many_arguments)]
fn smooth_vertices_with_falloff(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    seed_indices: PyReadonlyArray1<'_, i64>,
    falloff_mm: f64,
    iterations: i64,
    strength: f64,
    active_threshold: f32,
    cutoff_multiplier: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_seed_indices = read_i64_values(seed_indices);
    let smoothed = py
        .detach(|| {
            zennah_geometry_core::smooth_vertices_with_falloff(
                &rust_vertices,
                &rust_faces,
                &rust_seed_indices,
                SmoothFalloffOptions {
                    falloff_mm,
                    iterations,
                    strength,
                    active_threshold,
                    cutoff_multiplier,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = smoothed.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn outward_directions(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let directions = py
        .detach(|| zennah_geometry_core::outward_directions(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = directions.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, faces, seed_indices, falloff_mm, amount_mm, cutoff_multiplier = 3.0))]
fn local_offset_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    seed_indices: PyReadonlyArray1<'_, i64>,
    falloff_mm: f64,
    amount_mm: f64,
    cutoff_multiplier: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_seed_indices = read_i64_values(seed_indices);
    let displaced = py
        .detach(|| {
            zennah_geometry_core::local_offset_vertices(
                &rust_vertices,
                &rust_faces,
                &rust_seed_indices,
                falloff_mm,
                amount_mm,
                cutoff_multiplier,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = displaced.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (
    vertices,
    seed_indices,
    falloff_mm,
    mask_enabled = false,
    mask_indices = None,
    protected_indices = None,
    cutoff_multiplier = 3.0
))]
#[allow(clippy::too_many_arguments)]
fn brush_stroke_weights(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    seed_indices: PyReadonlyArray1<'_, i64>,
    falloff_mm: f64,
    mask_enabled: bool,
    mask_indices: Option<PyReadonlyArray1<'_, i64>>,
    protected_indices: Option<PyReadonlyArray1<'_, i64>>,
    cutoff_multiplier: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_seed_indices = read_i64_values(seed_indices);
    let rust_mask_indices = mask_indices.map(read_i64_values).unwrap_or_default();
    let rust_protected_indices = protected_indices.map(read_i64_values).unwrap_or_default();
    let weights = py
        .detach(|| {
            zennah_geometry_core::brush_stroke_weights(
                &rust_vertices,
                &rust_seed_indices,
                falloff_mm,
                mask_enabled,
                &rust_mask_indices,
                &rust_protected_indices,
                cutoff_multiplier,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(weights.into_pyarray(py).unbind())
}

#[pyfunction(signature = (
    operation,
    region_ids,
    vertex_offsets,
    vertex_indices,
    allowed_operation_offsets,
    allowed_operations,
    editable_region_ids = None,
    protected_region_ids = None,
    editable_filter_enabled = false,
    protected_filter_enabled = false,
    respect_allowed_operations = true
))]
#[allow(clippy::too_many_arguments)]
fn region_brush_masks(
    py: Python<'_>,
    operation: i64,
    region_ids: Vec<String>,
    vertex_offsets: PyReadonlyArray1<'_, i64>,
    vertex_indices: PyReadonlyArray1<'_, i64>,
    allowed_operation_offsets: PyReadonlyArray1<'_, i64>,
    allowed_operations: PyReadonlyArray1<'_, i64>,
    editable_region_ids: Option<Vec<String>>,
    protected_region_ids: Option<Vec<String>>,
    editable_filter_enabled: bool,
    protected_filter_enabled: bool,
    respect_allowed_operations: bool,
) -> PyResult<PyIndexPair> {
    let rust_vertex_offsets = read_i64_values(vertex_offsets);
    let rust_vertex_indices = read_i64_values(vertex_indices);
    let rust_allowed_offsets = read_i64_values(allowed_operation_offsets);
    let rust_allowed_operations = read_i64_values(allowed_operations);
    let rust_editable_region_ids = editable_region_ids.unwrap_or_default();
    let rust_protected_region_ids = protected_region_ids.unwrap_or_default();
    let (editable, protected) = py
        .detach(|| {
            zennah_geometry_core::region_brush_masks(
                operation,
                &region_ids,
                &rust_vertex_offsets,
                &rust_vertex_indices,
                &rust_allowed_offsets,
                &rust_allowed_operations,
                &rust_editable_region_ids,
                &rust_protected_region_ids,
                editable_filter_enabled,
                protected_filter_enabled,
                respect_allowed_operations,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok((
        editable.into_pyarray(py).unbind(),
        protected.into_pyarray(py).unbind(),
    ))
}

#[pyfunction(signature = (vertices, faces, operations, seed_offsets, seed_indices, mask_enabled, mask_offsets, mask_indices, protected_offsets, protected_indices, amounts_mm, falloffs_mm, iterations, strengths, cutoff_multiplier = 3.0))]
#[allow(clippy::too_many_arguments)]
fn apply_brush_strokes(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    operations: PyReadonlyArray1<'_, i64>,
    seed_offsets: PyReadonlyArray1<'_, i64>,
    seed_indices: PyReadonlyArray1<'_, i64>,
    mask_enabled: PyReadonlyArray1<'_, i64>,
    mask_offsets: PyReadonlyArray1<'_, i64>,
    mask_indices: PyReadonlyArray1<'_, i64>,
    protected_offsets: PyReadonlyArray1<'_, i64>,
    protected_indices: PyReadonlyArray1<'_, i64>,
    amounts_mm: PyReadonlyArray1<'_, f64>,
    falloffs_mm: PyReadonlyArray1<'_, f64>,
    iterations: PyReadonlyArray1<'_, i64>,
    strengths: PyReadonlyArray1<'_, f64>,
    cutoff_multiplier: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_operations = read_i64_values(operations);
    let rust_seed_offsets = read_i64_values(seed_offsets);
    let rust_seed_indices = read_i64_values(seed_indices);
    let rust_mask_enabled = read_i64_values(mask_enabled);
    let rust_mask_offsets = read_i64_values(mask_offsets);
    let rust_mask_indices = read_i64_values(mask_indices);
    let rust_protected_offsets = read_i64_values(protected_offsets);
    let rust_protected_indices = read_i64_values(protected_indices);
    let rust_amounts = read_f64_values(amounts_mm);
    let rust_falloffs = read_f64_values(falloffs_mm);
    let rust_iterations = read_i64_values(iterations);
    let rust_strengths = read_f64_values(strengths);
    let displaced = py
        .detach(|| {
            zennah_geometry_core::apply_brush_strokes(
                &rust_vertices,
                &rust_faces,
                &rust_operations,
                &rust_seed_offsets,
                &rust_seed_indices,
                &rust_mask_enabled,
                &rust_mask_offsets,
                &rust_mask_indices,
                &rust_protected_offsets,
                &rust_protected_indices,
                &rust_amounts,
                &rust_falloffs,
                &rust_iterations,
                &rust_strengths,
                cutoff_multiplier,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = displaced.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(laplacian_smooth_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(
        weighted_laplacian_smooth_vertices,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(falloff_weights, module)?)?;
    module.add_function(wrap_pyfunction!(smooth_vertices_with_falloff, module)?)?;
    module.add_function(wrap_pyfunction!(outward_directions, module)?)?;
    module.add_function(wrap_pyfunction!(local_offset_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(brush_stroke_weights, module)?)?;
    module.add_function(wrap_pyfunction!(region_brush_masks, module)?)?;
    module.add_function(wrap_pyfunction!(apply_brush_strokes, module)?)?;
    Ok(())
}
