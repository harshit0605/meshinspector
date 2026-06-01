use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_faces, read_vec3, read_vertices};

#[pyclass(name = "RustAABBTree")]
struct RustAabbTree {
    tree: zennah_geometry_core::AabbQueryTree,
}

#[pymethods]
impl RustAabbTree {
    #[getter]
    fn face_count(&self) -> usize {
        self.tree.face_count()
    }

    #[getter]
    fn leaf_size(&self) -> usize {
        self.tree.leaf_size()
    }
}

#[pyfunction(signature = (vertices, faces, leaf_size = 16))]
fn build_aabb_tree(
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    leaf_size: usize,
) -> PyResult<RustAabbTree> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let tree = zennah_geometry_core::AabbQueryTree::build(&rust_vertices, &rust_faces, leaf_size)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(RustAabbTree { tree })
}

#[pyfunction]
fn point_aabb_distance_sq(
    point: PyReadonlyArray1<'_, f64>,
    bbox_min: PyReadonlyArray1<'_, f64>,
    bbox_max: PyReadonlyArray1<'_, f64>,
) -> PyResult<f64> {
    Ok(zennah_geometry_core::point_aabb_distance_sq(
        read_vec3("point", point)?,
        read_vec3("bbox_min", bbox_min)?,
        read_vec3("bbox_max", bbox_max)?,
    ))
}

#[pyfunction(signature = (origin, direction, bbox_min, bbox_max, max_distance = None))]
fn ray_intersects_aabb(
    origin: PyReadonlyArray1<'_, f64>,
    direction: PyReadonlyArray1<'_, f64>,
    bbox_min: PyReadonlyArray1<'_, f64>,
    bbox_max: PyReadonlyArray1<'_, f64>,
    max_distance: Option<f64>,
) -> PyResult<bool> {
    Ok(zennah_geometry_core::ray_intersects_aabb(
        read_vec3("origin", origin)?,
        read_vec3("direction", direction)?,
        read_vec3("bbox_min", bbox_min)?,
        read_vec3("bbox_max", bbox_max)?,
        max_distance.unwrap_or(f64::INFINITY),
    ))
}

#[pyfunction(signature = (tree, origin, direction, max_distance = None))]
fn aabb_ray_candidate_faces(
    py: Python<'_>,
    tree: PyRef<'_, RustAabbTree>,
    origin: PyReadonlyArray1<'_, f64>,
    direction: PyReadonlyArray1<'_, f64>,
    max_distance: Option<f64>,
) -> PyResult<Py<PyArray1<i64>>> {
    let candidates = tree.tree.ray_candidate_faces(
        read_vec3("origin", origin)?,
        read_vec3("direction", direction)?,
        max_distance.unwrap_or(f64::INFINITY),
    );
    let output: Vec<i64> = candidates
        .into_iter()
        .map(|face_id| face_id as i64)
        .collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (tree, epsilon = 0.0))]
fn aabb_overlapping_face_pairs(tree: PyRef<'_, RustAabbTree>, epsilon: f64) -> Vec<(i64, i64)> {
    tree.tree
        .overlapping_face_pairs(epsilon)
        .into_iter()
        .map(|(left, right)| (left as i64, right as i64))
        .collect()
}

#[pyfunction(signature = (tree, point, current_best_sq))]
fn aabb_closest_candidate_faces(
    py: Python<'_>,
    tree: PyRef<'_, RustAabbTree>,
    point: PyReadonlyArray1<'_, f64>,
    current_best_sq: f64,
) -> PyResult<Py<PyArray1<i64>>> {
    let candidates = tree
        .tree
        .closest_candidate_faces(read_vec3("point", point)?, current_best_sq);
    let output: Vec<i64> = candidates
        .into_iter()
        .map(|face_id| face_id as i64)
        .collect();
    Ok(output.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<RustAabbTree>()?;
    module.add_function(wrap_pyfunction!(build_aabb_tree, module)?)?;
    module.add_function(wrap_pyfunction!(point_aabb_distance_sq, module)?)?;
    module.add_function(wrap_pyfunction!(ray_intersects_aabb, module)?)?;
    module.add_function(wrap_pyfunction!(aabb_ray_candidate_faces, module)?)?;
    module.add_function(wrap_pyfunction!(aabb_overlapping_face_pairs, module)?)?;
    module.add_function(wrap_pyfunction!(aabb_closest_candidate_faces, module)?)?;
    Ok(())
}
