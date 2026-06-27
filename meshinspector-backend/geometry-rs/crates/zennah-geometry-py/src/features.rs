use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::convert::{read_f64_values, read_faces, read_vertices};

mod object_payloads;
use object_payloads::feature_object_descriptor_to_py;

#[pyfunction]
fn feature_pair_measurements(
    py: Python<'_>,
    feature_ids: Vec<String>,
    feature_kinds: Vec<String>,
    centers: PyReadonlyArray2<'_, f64>,
    directions: PyReadonlyArray2<'_, f64>,
    radii: PyReadonlyArray1<'_, f64>,
    lengths: PyReadonlyArray1<'_, f64>,
    pairs: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyList>> {
    let rust_centers = read_vertices(centers)?;
    let rust_directions = read_vertices(directions)?;
    let rust_radii = read_f64_values(radii);
    let rust_lengths = read_f64_values(lengths);
    let rows = pairs.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err(
            "feature pairs must have shape (n, 2)",
        ));
    }
    if feature_ids.len() != feature_kinds.len()
        || feature_ids.len() != rust_centers.len()
        || feature_ids.len() != rust_directions.len()
        || feature_ids.len() != rust_radii.len()
        || feature_ids.len() != rust_lengths.len()
    {
        return Err(PyValueError::new_err(
            "feature ids, kinds, centers, directions, radii, and lengths must have the same length",
        ));
    }

    let features = feature_primitives_from_arrays(
        &feature_ids,
        &feature_kinds,
        &rust_centers,
        &rust_directions,
        &rust_radii,
        &rust_lengths,
    )?;
    let mut rust_pairs = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        if row[0] < 0 || row[1] < 0 {
            return Err(PyValueError::new_err(
                "feature pairs must contain non-negative indices",
            ));
        }
        rust_pairs.push([row[0] as usize, row[1] as usize]);
    }
    let measurements = py
        .detach(|| zennah_geometry_core::feature_pair_measurements(&features, &rust_pairs))
        .map_err(PyValueError::new_err)?;
    let output = PyList::empty(py);
    for measurement in measurements {
        let row = PyDict::new(py);
        row.set_item("first_index", measurement.first_index)?;
        row.set_item("second_index", measurement.second_index)?;
        row.set_item("first_feature_id", measurement.first_feature_id)?;
        row.set_item("second_feature_id", measurement.second_feature_id)?;
        row.set_item("first_kind", measurement.first_kind.as_str())?;
        row.set_item("second_kind", measurement.second_kind.as_str())?;
        row.set_item("distance", distance_part_to_py(py, &measurement.distance)?)?;
        row.set_item(
            "center_distance",
            distance_part_to_py(py, &measurement.center_distance)?,
        )?;
        row.set_item("angle", angle_part_to_py(py, &measurement.angle)?)?;
        let intersections = PyList::empty(py);
        for intersection in &measurement.intersections {
            intersections.append(intersection_to_py(py, intersection)?)?;
        }
        row.set_item("intersections", intersections)?;
        row.set_item("meshlib_reference", measurement.meshlib_reference)?;
        output.append(row)?;
    }
    Ok(output.unbind())
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn feature_object_descriptors(
    py: Python<'_>,
    feature_ids: Vec<String>,
    feature_kinds: Vec<String>,
    centers: PyReadonlyArray2<'_, f64>,
    directions: PyReadonlyArray2<'_, f64>,
    radii: PyReadonlyArray1<'_, f64>,
    lengths: PyReadonlyArray1<'_, f64>,
    infinite_extent_mm: f64,
) -> PyResult<Py<PyList>> {
    let rust_centers = read_vertices(centers)?;
    let rust_directions = read_vertices(directions)?;
    let rust_radii = read_f64_values(radii);
    let rust_lengths = read_f64_values(lengths);
    if feature_ids.len() != feature_kinds.len()
        || feature_ids.len() != rust_centers.len()
        || feature_ids.len() != rust_directions.len()
        || feature_ids.len() != rust_radii.len()
        || feature_ids.len() != rust_lengths.len()
    {
        return Err(PyValueError::new_err(
            "feature ids, kinds, centers, directions, radii, and lengths must have the same length",
        ));
    }

    let features = feature_primitives_from_arrays(
        &feature_ids,
        &feature_kinds,
        &rust_centers,
        &rust_directions,
        &rust_radii,
        &rust_lengths,
    )?;
    let descriptors = py
        .detach(|| zennah_geometry_core::feature_object_descriptors(&features, infinite_extent_mm))
        .map_err(PyValueError::new_err)?;
    let output = PyList::empty(py);
    for descriptor in descriptors {
        output.append(feature_object_descriptor_to_py(py, &descriptor)?)?;
    }
    Ok(output.unbind())
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn refine_feature_primitives(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    feature_ids: Vec<String>,
    feature_kinds: Vec<String>,
    centers: PyReadonlyArray2<'_, f64>,
    directions: PyReadonlyArray2<'_, f64>,
    radii: PyReadonlyArray1<'_, f64>,
    lengths: PyReadonlyArray1<'_, f64>,
    distance_limit_mm: f64,
    normal_tolerance_degrees: f64,
    max_iterations: usize,
) -> PyResult<Py<PyList>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_centers = read_vertices(centers)?;
    let rust_directions = read_vertices(directions)?;
    let rust_radii = read_f64_values(radii);
    let rust_lengths = read_f64_values(lengths);
    if feature_ids.len() != feature_kinds.len()
        || feature_ids.len() != rust_centers.len()
        || feature_ids.len() != rust_directions.len()
        || feature_ids.len() != rust_radii.len()
        || feature_ids.len() != rust_lengths.len()
    {
        return Err(PyValueError::new_err(
            "feature ids, kinds, centers, directions, radii, and lengths must have the same length",
        ));
    }

    let features = feature_primitives_from_arrays(
        &feature_ids,
        &feature_kinds,
        &rust_centers,
        &rust_directions,
        &rust_radii,
        &rust_lengths,
    )?;
    let options = zennah_geometry_core::FeatureRefineOptions {
        distance_limit: distance_limit_mm,
        normal_tolerance_degrees,
        max_iterations,
    };
    let refinements = py
        .detach(|| {
            zennah_geometry_core::refine_feature_primitives(
                &rust_vertices,
                &rust_faces,
                &features,
                options,
            )
        })
        .map_err(PyValueError::new_err)?;
    let output = PyList::empty(py);
    for refinement in refinements {
        let row = PyDict::new(py);
        row.set_item("feature_id", refinement.feature_id)?;
        row.set_item("kind", refinement.kind.as_str())?;
        row.set_item("primitive", primitive_to_py(py, &refinement.primitive)?)?;
        row.set_item(
            "selected_vertex_indices",
            refinement.selected_vertex_indices,
        )?;
        row.set_item("selected_count", refinement.selected_count)?;
        row.set_item("iterations", refinement.iterations)?;
        row.set_item("converged", refinement.converged)?;
        row.set_item("meshlib_reference", refinement.meshlib_reference)?;
        output.append(row)?;
    }
    Ok(output.unbind())
}

fn feature_primitives_from_arrays(
    feature_ids: &[String],
    feature_kinds: &[String],
    centers: &[[f64; 3]],
    directions: &[[f64; 3]],
    radii: &[f64],
    lengths: &[f64],
) -> PyResult<Vec<zennah_geometry_core::FeaturePrimitive>> {
    feature_ids
        .iter()
        .zip(feature_kinds.iter())
        .enumerate()
        .map(|(index, (feature_id, kind))| {
            Ok(zennah_geometry_core::FeaturePrimitive {
                feature_id: feature_id.clone(),
                kind: parse_feature_kind(kind)?,
                center: centers[index],
                direction: direction_option(directions[index]),
                radius: radii[index],
                length: lengths[index],
            })
        })
        .collect::<PyResult<Vec<_>>>()
}

fn parse_feature_kind(kind: &str) -> PyResult<zennah_geometry_core::FeaturePrimitiveKind> {
    match kind {
        "point" => Ok(zennah_geometry_core::FeaturePrimitiveKind::Point),
        "sphere" => Ok(zennah_geometry_core::FeaturePrimitiveKind::Sphere),
        "line" => Ok(zennah_geometry_core::FeaturePrimitiveKind::Line),
        "plane" => Ok(zennah_geometry_core::FeaturePrimitiveKind::Plane),
        "circle" => Ok(zennah_geometry_core::FeaturePrimitiveKind::Circle),
        "cylinder" => Ok(zennah_geometry_core::FeaturePrimitiveKind::Cylinder),
        "cone" => Ok(zennah_geometry_core::FeaturePrimitiveKind::Cone),
        other => Err(PyValueError::new_err(format!(
            "unsupported feature primitive kind {other:?}"
        ))),
    }
}

fn direction_option(direction: [f64; 3]) -> Option<[f64; 3]> {
    let length_sq = direction.iter().map(|value| value * value).sum::<f64>();
    if length_sq <= f64::EPSILON {
        None
    } else {
        Some(direction)
    }
}

fn distance_part_to_py(
    py: Python<'_>,
    part: &zennah_geometry_core::FeatureDistancePart,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("status", part.status.as_str())?;
    output.set_item("distance_mm", part.distance_mm)?;
    output.set_item(
        "closest_point_a",
        part.closest_point_a.map(|value| value.to_vec()),
    )?;
    output.set_item(
        "closest_point_b",
        part.closest_point_b.map(|value| value.to_vec()),
    )?;
    Ok(output.unbind())
}

fn angle_part_to_py(
    py: Python<'_>,
    part: &zennah_geometry_core::FeatureAnglePart,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("status", part.status.as_str())?;
    output.set_item("angle_radians", part.angle_radians)?;
    output.set_item("angle_degrees", part.angle_degrees)?;
    output.set_item("point_a", part.point_a.map(|value| value.to_vec()))?;
    output.set_item("point_b", part.point_b.map(|value| value.to_vec()))?;
    output.set_item("direction_a", part.direction_a.map(|value| value.to_vec()))?;
    output.set_item("direction_b", part.direction_b.map(|value| value.to_vec()))?;
    output.set_item("is_surface_normal_a", part.is_surface_normal_a)?;
    output.set_item("is_surface_normal_b", part.is_surface_normal_b)?;
    Ok(output.unbind())
}

fn intersection_to_py(
    py: Python<'_>,
    intersection: &zennah_geometry_core::FeatureIntersectionPrimitive,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("kind", intersection.kind.as_str())?;
    output.set_item("center", intersection.center.to_vec())?;
    output.set_item(
        "direction",
        intersection.direction.map(|value| value.to_vec()),
    )?;
    output.set_item("radius_mm", intersection.radius_mm)?;
    output.set_item("length_mm", intersection.length_mm)?;
    output.set_item(
        "start_point",
        intersection.start_point.map(|value| value.to_vec()),
    )?;
    output.set_item(
        "end_point",
        intersection.end_point.map(|value| value.to_vec()),
    )?;
    output.set_item("meshlib_primitive", intersection.meshlib_primitive)?;
    Ok(output.unbind())
}

fn primitive_to_py(
    py: Python<'_>,
    primitive: &zennah_geometry_core::FeaturePrimitive,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("feature_id", &primitive.feature_id)?;
    output.set_item("kind", primitive.kind.as_str())?;
    output.set_item("center", primitive.center.to_vec())?;
    output.set_item("direction", primitive.direction.map(|value| value.to_vec()))?;
    output.set_item("radius_mm", primitive.radius)?;
    output.set_item("length_mm", primitive.length)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(feature_pair_measurements, module)?)?;
    module.add_function(wrap_pyfunction!(feature_object_descriptors, module)?)?;
    module.add_function(wrap_pyfunction!(refine_feature_primitives, module)?)?;
    Ok(())
}
