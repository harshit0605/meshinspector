use crate::math::{add, scale};
use crate::GeometryError;

pub(super) fn pseudo_index(index: usize, count: usize) -> f64 {
    index as f64 - (count as f64 - 1.0) * 0.5
}

pub(super) fn meshlib_dense_value_at(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    point: [f64; 3],
) -> f64 {
    let coordinate = [
        point[0] / voxel_size[0],
        point[1] / voxel_size[1],
        point[2] / voxel_size[2],
    ];
    let mut base = [0_usize; 3];
    let mut fraction = [0.0_f64; 3];
    for axis in 0..3 {
        let max_coord = (shape[axis] - 1) as f64;
        let clamped = coordinate[axis].clamp(0.0, max_coord);
        let lower = clamped.floor();
        base[axis] = (lower as usize).min(shape[axis] - 1);
        fraction[axis] = clamped - lower;
        if base[axis] + 1 >= shape[axis] {
            fraction[axis] = 0.0;
        }
    }

    let mut total = 0.0_f64;
    for dx in 0..=1 {
        let x = (base[0] + dx).min(shape[0] - 1);
        let wx = if dx == 0 {
            1.0 - fraction[0]
        } else {
            fraction[0]
        };
        for dy in 0..=1 {
            let y = (base[1] + dy).min(shape[1] - 1);
            let wy = if dy == 0 {
                1.0 - fraction[1]
            } else {
                fraction[1]
            };
            for dz in 0..=1 {
                let z = (base[2] + dz).min(shape[2] - 1);
                let wz = if dz == 0 {
                    1.0 - fraction[2]
                } else {
                    fraction[2]
                };
                let index = x + y * shape[0] + z * shape[0] * shape[1];
                total += values[index] as f64 * wx * wy * wz;
            }
        }
    }
    total
}

pub(super) fn fit_polynomial_least_squares(
    samples: &[f64],
    degree: usize,
) -> Result<Vec<f64>, GeometryError> {
    let size = degree + 1;
    let mut ata = vec![vec![0.0_f64; size]; size];
    let mut atb = vec![0.0_f64; size];
    for (index, value) in samples.iter().copied().enumerate() {
        let x = pseudo_index(index, samples.len());
        let mut powers = vec![1.0_f64; size];
        for power in 1..size {
            powers[power] = powers[power - 1] * x;
        }
        for row in 0..size {
            atb[row] += powers[row] * value;
            for col in 0..size {
                ata[row][col] += powers[row] * powers[col];
            }
        }
    }
    solve_linear_system(ata, atb).ok_or_else(|| GeometryError::InvalidSelectionParameter {
        field: "samples",
        value: format!("singular degree-{degree} fit"),
    })
}

fn solve_linear_system(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let size = rhs.len();
    for pivot in 0..size {
        let mut best = pivot;
        let mut best_abs = matrix[pivot][pivot].abs();
        for row in (pivot + 1)..size {
            let candidate = matrix[row][pivot].abs();
            if candidate > best_abs {
                best = row;
                best_abs = candidate;
            }
        }
        if best_abs < 1e-12 {
            return None;
        }
        if best != pivot {
            matrix.swap(pivot, best);
            rhs.swap(pivot, best);
        }

        let divisor = matrix[pivot][pivot];
        for col in pivot..size {
            matrix[pivot][col] /= divisor;
        }
        rhs[pivot] /= divisor;

        for row in 0..size {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            if factor.abs() < 1e-20 {
                continue;
            }
            for col in pivot..size {
                matrix[row][col] -= factor * matrix[pivot][col];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    Some(rhs)
}

pub(super) fn polynomial_derivative(coeffs: &[f64]) -> Vec<f64> {
    if coeffs.len() <= 1 {
        return vec![0.0];
    }
    coeffs
        .iter()
        .copied()
        .enumerate()
        .skip(1)
        .map(|(index, coeff)| coeff * index as f64)
        .collect()
}

pub(super) fn polynomial_interval_min_arg(coeffs: &[f64], min_x: f64, max_x: f64) -> f64 {
    let mut candidates = vec![min_x, max_x];
    candidates.extend(real_roots_in_interval(
        &polynomial_derivative(coeffs),
        min_x,
        max_x,
    ));
    candidates
        .into_iter()
        .min_by(|a, b| {
            polynomial_value(coeffs, *a)
                .partial_cmp(&polynomial_value(coeffs, *b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(min_x)
}

fn polynomial_value(coeffs: &[f64], x: f64) -> f64 {
    coeffs
        .iter()
        .rev()
        .fold(0.0, |total, coeff| total * x + coeff)
}

fn real_roots_in_interval(coeffs: &[f64], min_x: f64, max_x: f64) -> Vec<f64> {
    const ROOT_TOL: f64 = 1e-9;
    let degree = trimmed_polynomial_degree(coeffs);
    if degree == 0 {
        return Vec::new();
    }
    if degree == 1 {
        let root = -coeffs[0] / coeffs[1];
        if root >= min_x - ROOT_TOL && root <= max_x + ROOT_TOL {
            return vec![root.clamp(min_x, max_x)];
        }
        return Vec::new();
    }

    let mut split_points = vec![min_x, max_x];
    split_points.extend(real_roots_in_interval(
        &polynomial_derivative(&coeffs[..=degree]),
        min_x,
        max_x,
    ));
    split_points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    split_points.dedup_by(|a, b| (*a - *b).abs() < ROOT_TOL);

    let mut roots = Vec::new();
    for point in &split_points {
        if polynomial_value(coeffs, *point).abs() < 1e-7 {
            push_root(&mut roots, *point, min_x, max_x);
        }
    }
    for window in split_points.windows(2) {
        let left = window[0];
        let right = window[1];
        if right - left < ROOT_TOL {
            continue;
        }
        let left_value = polynomial_value(coeffs, left);
        let right_value = polynomial_value(coeffs, right);
        if left_value.abs() < 1e-7 || right_value.abs() < 1e-7 {
            continue;
        }
        if left_value.signum() == right_value.signum() {
            continue;
        }
        push_root(
            &mut roots,
            bisect_polynomial_root(coeffs, left, right),
            min_x,
            max_x,
        );
    }
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    roots.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    roots
}

fn trimmed_polynomial_degree(coeffs: &[f64]) -> usize {
    coeffs
        .iter()
        .rposition(|coeff| coeff.abs() > 1e-12)
        .unwrap_or(0)
}

fn bisect_polynomial_root(coeffs: &[f64], mut left: f64, mut right: f64) -> f64 {
    let mut left_value = polynomial_value(coeffs, left);
    for _ in 0..80 {
        let mid = 0.5 * (left + right);
        let mid_value = polynomial_value(coeffs, mid);
        if mid_value.abs() < 1e-12 || (right - left).abs() < 1e-12 {
            return mid;
        }
        if left_value.signum() == mid_value.signum() {
            left = mid;
            left_value = mid_value;
        } else {
            right = mid;
        }
    }
    0.5 * (left + right)
}

fn push_root(roots: &mut Vec<f64>, root: f64, min_x: f64, max_x: f64) {
    let root = root.clamp(min_x, max_x);
    if roots.iter().all(|existing| (existing - root).abs() > 1e-7) {
        roots.push(root);
    }
}

pub(super) fn smooth_shift_vectors(
    shifts: &[[f64; 3]],
    neighbors: &[Vec<usize>],
    iterations: usize,
    force: f64,
) -> Vec<[f64; 3]> {
    if iterations == 0 || force <= 0.0 {
        return shifts.to_vec();
    }
    let force = force.clamp(0.0, 1.0);
    let mut current = shifts.to_vec();
    for _ in 0..iterations {
        let previous = current.clone();
        for (index, neighbor_ids) in neighbors.iter().enumerate() {
            if neighbor_ids.is_empty() {
                continue;
            }
            let mut average = [0.0_f64; 3];
            for neighbor in neighbor_ids {
                average = add(average, previous[*neighbor]);
            }
            average = scale(average, 1.0 / neighbor_ids.len() as f64);
            current[index] = add(scale(previous[index], 1.0 - force), scale(average, force));
        }
    }
    current
}
