use nalgebra::{Matrix3, SMatrix, SVector};

use super::{add, dot, normalize, scale, sub};

type Vec3 = SVector<f64, 3>;
type Vec6 = SVector<f64, 6>;
type Mat3 = Matrix3<f64>;
type Mat3x6 = SMatrix<f64, 3, 6>;
type Mat6 = SMatrix<f64, 6, 6>;

#[derive(Debug, Clone, Copy)]
pub(super) struct CylinderApproximation {
    pub center: [f64; 3],
    pub direction: [f64; 3],
    pub radius: f64,
    pub length: f64,
    #[cfg(test)]
    pub error: f64,
}

#[derive(Debug)]
struct CylinderPrecompute {
    average: [f64; 3],
    normalized_points: Vec<Vec3>,
    mu: Vec6,
    f0: Mat3,
    f1: Mat3x6,
    f2: Mat6,
}

pub(super) fn approximate_cylinder_meshlib(
    points: &[[f64; 3]],
) -> Result<CylinderApproximation, String> {
    approximate_cylinder_meshlib_with_resolution(points, 180, 180)
}

fn approximate_cylinder_meshlib_with_resolution(
    points: &[[f64; 3]],
    theta_resolution: usize,
    phi_resolution: usize,
) -> Result<CylinderApproximation, String> {
    if points.len() < 6 {
        return Err(format!(
            "Cylinder3Approximation requires at least 6 points, got {}",
            points.len()
        ));
    }
    if theta_resolution == 0 || phi_resolution == 0 {
        return Err("Cylinder3Approximation resolutions must be positive".to_string());
    }

    let precompute = update_precompute_params(points);
    let mut best_w = Vec3::new(0.0, 0.0, 1.0);
    let mut best = cylinder_error(&precompute, &best_w);

    let theta_step = 2.0 * std::f64::consts::PI / theta_resolution as f64;
    let phi_step = std::f64::consts::FRAC_PI_2 / phi_resolution as f64;

    for j in 1..=phi_resolution {
        let phi = phi_step * j as f64;
        let (sin_phi, cos_phi) = phi.sin_cos();
        for i in 0..theta_resolution {
            let theta = theta_step * i as f64;
            let (sin_theta, cos_theta) = theta.sin_cos();
            let candidate_w = Vec3::new(cos_theta * sin_phi, sin_theta * sin_phi, cos_phi);
            let candidate = cylinder_error(&precompute, &candidate_w);
            if candidate.error < best.error {
                best_w = candidate_w;
                best = candidate;
            }
        }
    }

    let direction = normalize([best_w[0], best_w[1], best_w[2]])
        .ok_or_else(|| "Cylinder3Approximation selected a degenerate axis".to_string())?;
    let mut center = add([best.pc[0], best.pc[1], best.pc[2]], precompute.average);
    let radius = best.radius_sq.max(0.0).sqrt();

    let mut h_min = f64::INFINITY;
    let mut h_max = f64::NEG_INFINITY;
    for point in points {
        let h = dot(direction, sub(*point, center));
        h_min = h_min.min(h);
        h_max = h_max.max(h);
    }
    let h_mid = (h_min + h_max) * 0.5;
    center = add(center, scale(direction, h_mid));

    Ok(CylinderApproximation {
        center,
        direction,
        radius,
        length: h_max - h_min,
        #[cfg(test)]
        error: best.error,
    })
}

#[derive(Debug, Clone, Copy)]
struct CylinderError {
    error: f64,
    pc: Vec3,
    radius_sq: f64,
}

fn update_precompute_params(points: &[[f64; 3]]) -> CylinderPrecompute {
    let average = centroid(points);
    let normalized_points = points
        .iter()
        .copied()
        .map(|point| {
            let normalized = sub(point, average);
            Vec3::new(normalized[0], normalized[1], normalized[2])
        })
        .collect::<Vec<_>>();

    let mut products = Vec::with_capacity(normalized_points.len());
    let mut mu = Vec6::zeros();
    for point in &normalized_points {
        let product = Vec6::new(
            point[0] * point[0],
            point[0] * point[1],
            point[0] * point[2],
            point[1] * point[1],
            point[1] * point[2],
            point[2] * point[2],
        );
        mu[0] += product[0];
        mu[1] += 2.0 * product[1];
        mu[2] += 2.0 * product[2];
        mu[3] += product[3];
        mu[4] += 2.0 * product[4];
        mu[5] += product[5];
        products.push(product);
    }
    mu /= normalized_points.len() as f64;

    let mut f0 = Mat3::zeros();
    let mut f1 = Mat3x6::zeros();
    let mut f2 = Mat6::zeros();
    for (point, product) in normalized_points.iter().zip(products.iter()) {
        let delta = Vec6::new(
            product[0] - mu[0],
            2.0 * product[1] - mu[1],
            2.0 * product[2] - mu[2],
            product[3] - mu[3],
            2.0 * product[4] - mu[4],
            product[5] - mu[5],
        );
        f0[(0, 0)] += product[0];
        f0[(0, 1)] += product[1];
        f0[(0, 2)] += product[2];
        f0[(1, 1)] += product[3];
        f0[(1, 2)] += product[4];
        f0[(2, 2)] += product[5];
        f1 += point * delta.transpose();
        f2 += delta * delta.transpose();
    }
    let count = normalized_points.len() as f64;
    f0 /= count;
    f0[(1, 0)] = f0[(0, 1)];
    f0[(2, 0)] = f0[(0, 2)];
    f0[(2, 1)] = f0[(1, 2)];
    f1 /= count;
    f2 /= count;

    CylinderPrecompute {
        average,
        normalized_points,
        mu,
        f0,
        f1,
        f2,
    }
}

fn cylinder_error(precompute: &CylinderPrecompute, w: &Vec3) -> CylinderError {
    let projection = Mat3::identity() - (w * w.transpose());
    let skew = Mat3::new(0.0, -w[2], w[1], w[2], 0.0, -w[0], -w[1], w[0], 0.0);
    let a = projection * precompute.f0 * projection;
    let hat_a = -(skew * a * skew);
    let hat_aa = hat_a * a;
    let trace = hat_aa.trace();
    if trace == 0.0 {
        return CylinderError {
            error: f64::MAX,
            pc: Vec3::zeros(),
            radius_sq: 0.0,
        };
    }

    let q = hat_a / trace;
    let p_vec = Vec6::new(
        projection[(0, 0)],
        projection[(0, 1)],
        projection[(0, 2)],
        projection[(1, 1)],
        projection[(1, 2)],
        projection[(2, 2)],
    );
    let alpha = precompute.f1 * p_vec;
    let beta = q * alpha;
    let mut error = (p_vec.dot(&(precompute.f2 * p_vec)) - 4.0 * alpha.dot(&beta)
        + 4.0 * beta.dot(&(precompute.f0 * beta)))
        / precompute.normalized_points.len() as f64;
    if error < 0.0 {
        error = error.abs();
    }
    let radius_sq = (p_vec.dot(&precompute.mu) + beta.dot(&beta)).max(0.0);

    CylinderError {
        error,
        pc: beta,
        radius_sq,
    }
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for point in points {
        sum = add(sum, *point);
    }
    scale(sum, 1.0 / points.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{
        length,
        support::{arbitrary_perpendicular, cross},
    };

    #[test]
    fn cylinder_approximation_matches_meshlib_partial_arc_fixture() {
        let center = [1.0, 2.0, 3.0];
        let direction = normalize([3.0, 2.0, 1.0]).unwrap();
        let radius = 1.5;
        let cylinder_length = 10.0;
        let basis_x = arbitrary_perpendicular(direction);
        let basis_y = normalize(cross(direction, basis_x)).unwrap();

        let mut points = Vec::new();
        let resolution = 100;
        let arch_size = std::f64::consts::PI / 1.5;
        let angle_step = arch_size / resolution as f64;
        let z_step = 1.0 / resolution as f64;
        for i in 0..resolution {
            let angle = i as f64 * angle_step;
            let z = i as f64 * z_step - 0.5;
            for local_z in [z, -z] {
                let radial = add(
                    scale(basis_x, radius * angle.cos()),
                    scale(basis_y, radius * angle.sin()),
                );
                points.push(add(
                    add(center, radial),
                    scale(direction, cylinder_length * local_z),
                ));
            }
        }

        let fit = approximate_cylinder_meshlib(&points).unwrap();

        assert!(fit.error <= 0.1, "unexpected cylinder RMS {}", fit.error);
        assert!((fit.radius - radius).abs() <= 0.1);
        assert!((fit.length - cylinder_length).abs() <= 0.1);
        assert!(length(sub(fit.center, center)) <= 0.1);
        assert!(dot(fit.direction, direction).abs() > 0.9);
    }
}
