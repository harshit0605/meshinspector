use nalgebra::{DMatrix, DVector, Matrix3, Vector3};

use super::{
    directed_nearest_pairs, directed_nearest_plane_pairs, mean_square_pair_distance,
    mean_square_plane_distance, result_from_transforms, validate_multiway_inputs,
    validate_multiway_normals, MultiwayIcpRegistrationResult, ObjectTransform,
};

use crate::registration::IcpMode;

pub fn multiway_all_object_point_to_point_icp(
    objects: &[Vec<[f64; 3]>],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_point_iterations(
            objects,
            initial_transforms,
            iteration_limit,
            tolerance,
            fixed_index,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_all_object_point_to_plane_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let iteration_limit = max_iterations.max(1);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (transforms, iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            initial_transforms,
            iteration_limit,
            tolerance,
            fixed_index,
        )?;

    Ok(result_from_transforms(
        transforms,
        iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub fn multiway_all_object_combined_icp(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    max_iterations: usize,
    tolerance: f64,
    mode: IcpMode,
    fixed_object_index: Option<usize>,
) -> Result<MultiwayIcpRegistrationResult, String> {
    let fixed_index = validate_multiway_inputs(objects, mode, fixed_object_index)?;
    validate_multiway_normals(objects, normals)?;
    let iteration_limit = max_iterations.max(3);
    let tolerance = tolerance.max(0.0);
    let initial_transforms = vec![ObjectTransform::identity(); objects.len()];
    let (point_transforms, point_iterations, _, _) =
        run_point_to_point_iterations(objects, initial_transforms, 2, tolerance, fixed_index)?;
    let (transforms, plane_iterations, mean_square_distance, active_pair_count) =
        run_point_to_plane_iterations(
            objects,
            normals,
            point_transforms,
            iteration_limit - 2,
            tolerance,
            fixed_index,
        )?;

    Ok(result_from_transforms(
        transforms,
        point_iterations + plane_iterations,
        mean_square_distance,
        active_pair_count,
        fixed_index,
    ))
}

pub(super) fn run_point_to_point_iterations(
    objects: &[Vec<[f64; 3]>],
    mut transforms: Vec<ObjectTransform>,
    iteration_limit: usize,
    tolerance: f64,
    fixed_index: usize,
) -> Result<(Vec<ObjectTransform>, usize, f64, usize), String> {
    let mut best_transforms = transforms.clone();
    let initial_pairs = directed_nearest_pairs(objects, &transforms)?;
    let mut best_distance = mean_square_pair_distance(&initial_pairs)
        .ok_or_else(|| "multiway all-object ICP produced no active pairs".to_string())?;
    let mut active_pair_count = initial_pairs.len();
    let mut iterations = 0;

    for iteration in 1..=iteration_limit {
        let pairs = directed_nearest_pairs(objects, &transforms)?;
        active_pair_count = pairs.len();
        let deltas = solve_point_to_point_updates(objects.len(), fixed_index, &pairs, 1e-3)?;
        let candidate = apply_updates(&transforms, &deltas);
        let candidate_pairs = directed_nearest_pairs(objects, &candidate)?;
        let current_distance = mean_square_pair_distance(&candidate_pairs)
            .ok_or_else(|| "multiway all-object ICP produced no active pairs".to_string())?;
        iterations = iteration;

        if current_distance + tolerance < best_distance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
            break;
        } else {
            break;
        }

        if best_distance <= tolerance {
            break;
        }
    }

    Ok((
        best_transforms,
        iterations,
        best_distance,
        active_pair_count,
    ))
}

pub(super) fn run_point_to_plane_iterations(
    objects: &[Vec<[f64; 3]>],
    normals: &[Vec<[f64; 3]>],
    mut transforms: Vec<ObjectTransform>,
    iteration_limit: usize,
    tolerance: f64,
    fixed_index: usize,
) -> Result<(Vec<ObjectTransform>, usize, f64, usize), String> {
    let mut best_transforms = transforms.clone();
    let initial_pairs = directed_nearest_plane_pairs(objects, normals, &transforms)?;
    let mut best_distance = mean_square_plane_distance(&initial_pairs).ok_or_else(|| {
        "multiway all-object point-to-plane ICP produced no active pairs".to_string()
    })?;
    let mut active_pair_count = initial_pairs.len();
    let mut iterations = 0;

    for iteration in 1..=iteration_limit {
        let pairs = directed_nearest_plane_pairs(objects, normals, &transforms)?;
        active_pair_count = pairs.len();
        let deltas = solve_point_to_plane_updates(objects.len(), fixed_index, &pairs, 1e-3)?;
        let candidate = apply_updates(&transforms, &deltas);
        let candidate_pairs = directed_nearest_plane_pairs(objects, normals, &candidate)?;
        let current_distance = mean_square_plane_distance(&candidate_pairs).ok_or_else(|| {
            "multiway all-object point-to-plane ICP produced no active pairs".to_string()
        })?;
        iterations = iteration;

        if current_distance + tolerance < best_distance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
        } else if (best_distance - current_distance).abs() <= tolerance {
            transforms = candidate;
            best_transforms = transforms.clone();
            best_distance = current_distance;
            break;
        } else {
            break;
        }

        if best_distance <= tolerance {
            break;
        }
    }

    Ok((
        best_transforms,
        iterations,
        best_distance,
        active_pair_count,
    ))
}

pub(super) fn solve_point_to_point_updates(
    object_count: usize,
    fixed_index: usize,
    pairs: &[super::DirectedPair],
    shift_stabilizer: f64,
) -> Result<Vec<ObjectTransform>, String> {
    let mut system = MultiwaySystem::new(object_count, fixed_index, shift_stabilizer);
    for pair in pairs {
        system.add_point_link(
            pair.source_object,
            pair.source,
            pair.target_object,
            pair.target,
        );
    }
    system.solve("multiway all-object point-to-point solve")
}

pub(super) fn solve_point_to_plane_updates(
    object_count: usize,
    fixed_index: usize,
    pairs: &[super::DirectedPlanePair],
    shift_stabilizer: f64,
) -> Result<Vec<ObjectTransform>, String> {
    let mut system = MultiwaySystem::new(object_count, fixed_index, shift_stabilizer);
    for pair in pairs {
        system.add_plane_link(
            pair.source_object,
            pair.source,
            pair.target_object,
            pair.target,
            pair.target_normal,
        );
    }
    system.solve("multiway all-object point-to-plane solve")
}

pub(super) fn apply_updates(
    transforms: &[ObjectTransform],
    deltas: &[ObjectTransform],
) -> Vec<ObjectTransform> {
    transforms
        .iter()
        .zip(deltas.iter())
        .map(|(transform, delta)| ObjectTransform {
            rotation: delta.rotation * transform.rotation,
            translation: delta.rotation * transform.translation + delta.translation,
        })
        .collect()
}

struct MultiwaySystem {
    object_count: usize,
    fixed_index: usize,
    matrix: DMatrix<f64>,
    rhs: DVector<f64>,
}

impl MultiwaySystem {
    fn new(object_count: usize, fixed_index: usize, shift_stabilizer: f64) -> Self {
        let variable_count = (object_count - 1) * 6;
        let mut matrix = DMatrix::<f64>::zeros(variable_count, variable_count);
        for object_index in 0..object_count {
            let Some(offset) = variable_offset(object_index, fixed_index) else {
                continue;
            };
            let shift_stabilizer_sq = shift_stabilizer * shift_stabilizer;
            for axis in 3..6 {
                matrix[(offset + axis, offset + axis)] += shift_stabilizer_sq;
            }
        }
        Self {
            object_count,
            fixed_index,
            matrix,
            rhs: DVector::<f64>::zeros(variable_count),
        }
    }

    fn add_point_link(
        &mut self,
        object_a: usize,
        point_a: Vector3<f64>,
        object_b: usize,
        point_b: Vector3<f64>,
    ) {
        let coeffs_a = [
            Vector3::new(0.0, -point_a.z, point_a.y),
            Vector3::new(point_a.z, 0.0, -point_a.x),
            Vector3::new(-point_a.y, point_a.x, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        let coeffs_b = [
            Vector3::new(0.0, point_b.z, -point_b.y),
            Vector3::new(-point_b.z, 0.0, point_b.x),
            Vector3::new(point_b.y, -point_b.x, 0.0),
            Vector3::new(-1.0, 0.0, 0.0),
            Vector3::new(0.0, -1.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0),
        ];
        let residual = point_b - point_a;
        for axis in 0..3 {
            let mut coefficients = DVector::<f64>::zeros(self.rhs.len());
            self.write_vector_coefficients(&mut coefficients, object_a, &coeffs_a, axis);
            self.write_vector_coefficients(&mut coefficients, object_b, &coeffs_b, axis);
            self.add_row(coefficients, residual[axis]);
        }
    }

    fn add_plane_link(
        &mut self,
        object_a: usize,
        point_a: Vector3<f64>,
        object_b: usize,
        point_b: Vector3<f64>,
        normal: Vector3<f64>,
    ) {
        let coeffs_a = [
            normal.z * point_a.y - normal.y * point_a.z,
            normal.x * point_a.z - normal.z * point_a.x,
            normal.y * point_a.x - normal.x * point_a.y,
            normal.x,
            normal.y,
            normal.z,
        ];
        let coeffs_b = [
            normal.y * point_b.z - normal.z * point_b.y,
            normal.z * point_b.x - normal.x * point_b.z,
            normal.x * point_b.y - normal.y * point_b.x,
            -normal.x,
            -normal.y,
            -normal.z,
        ];
        let mut coefficients = DVector::<f64>::zeros(self.rhs.len());
        self.write_scalar_coefficients(&mut coefficients, object_a, &coeffs_a);
        self.write_scalar_coefficients(&mut coefficients, object_b, &coeffs_b);
        self.add_row(coefficients, (point_b - point_a).dot(&normal));
    }

    fn write_vector_coefficients(
        &self,
        coefficients: &mut DVector<f64>,
        object_index: usize,
        values: &[Vector3<f64>; 6],
        axis: usize,
    ) {
        let Some(offset) = variable_offset(object_index, self.fixed_index) else {
            return;
        };
        for column in 0..6 {
            coefficients[offset + column] = values[column][axis];
        }
    }

    fn write_scalar_coefficients(
        &self,
        coefficients: &mut DVector<f64>,
        object_index: usize,
        values: &[f64; 6],
    ) {
        let Some(offset) = variable_offset(object_index, self.fixed_index) else {
            return;
        };
        for column in 0..6 {
            coefficients[offset + column] = values[column];
        }
    }

    fn add_row(&mut self, coefficients: DVector<f64>, residual: f64) {
        self.matrix += &coefficients * coefficients.transpose();
        self.rhs += coefficients * residual;
    }

    fn solve(self, context: &str) -> Result<Vec<ObjectTransform>, String> {
        let solution = self
            .matrix
            .svd(true, true)
            .solve(&self.rhs, 1e-12)
            .map_err(|_| format!("{context} failed"))?;
        let mut transforms = Vec::with_capacity(self.object_count);
        for object_index in 0..self.object_count {
            let Some(offset) = variable_offset(object_index, self.fixed_index) else {
                transforms.push(ObjectTransform::identity());
                continue;
            };
            let angles = Vector3::new(solution[offset], solution[offset + 1], solution[offset + 2]);
            let translation = Vector3::new(
                solution[offset + 3],
                solution[offset + 4],
                solution[offset + 5],
            );
            if !angles.iter().all(|value| value.is_finite())
                || !translation.iter().all(|value| value.is_finite())
            {
                return Err(format!("{context} returned a non-finite update"));
            }
            transforms.push(ObjectTransform {
                rotation: rotation_from_axis_vector(angles),
                translation,
            });
        }
        Ok(transforms)
    }
}

fn variable_offset(object_index: usize, fixed_index: usize) -> Option<usize> {
    if object_index == fixed_index {
        return None;
    }
    let variable_index = if object_index < fixed_index {
        object_index
    } else {
        object_index - 1
    };
    Some(variable_index * 6)
}

fn rotation_from_axis_vector(angles: Vector3<f64>) -> Matrix3<f64> {
    let angle = angles.norm();
    let skew = Matrix3::new(
        0.0, -angles.z, angles.y, angles.z, 0.0, -angles.x, -angles.y, angles.x, 0.0,
    );
    if angle <= 1e-12 {
        Matrix3::identity() + skew
    } else {
        let axis_skew = skew / angle;
        Matrix3::identity()
            + axis_skew * angle.sin()
            + (axis_skew * axis_skew) * (1.0 - angle.cos())
    }
}
