use nalgebra::{SMatrix, SVector};

use super::support::{arbitrary_perpendicular, cross};
use super::{add, dot, length, normalize, scale, sub};

type Vec6 = SVector<f64, 6>;
type Mat6 = SMatrix<f64, 6, 6>;

#[derive(Debug, Clone, Copy)]
pub(super) struct ConeApproximation {
    pub apex: [f64; 3],
    pub direction: [f64; 3],
    pub angle: f64,
    pub height: f64,
    pub base_radius: f64,
    pub error: f64,
}

pub(super) fn cone_angle_from_radius_height(radius: f64, height: f64) -> Result<f64, String> {
    if !radius.is_finite() || radius < 0.0 {
        return Err("cone radius must be finite and non-negative".to_string());
    }
    if !height.is_finite() || height <= f64::EPSILON {
        return Err("cone height must be finite and positive".to_string());
    }
    Ok((radius / height).atan())
}

pub(super) fn project_cone_point(
    apex: [f64; 3],
    direction: [f64; 3],
    angle: f64,
    point: [f64; 3],
) -> Result<([f64; 3], [f64; 3]), String> {
    let direction =
        normalize(direction).ok_or_else(|| "cone direction must be non-zero".to_string())?;
    if !angle.is_finite() || !(0.0..std::f64::consts::FRAC_PI_2).contains(&angle) {
        return Err("cone angle must be finite and in [0, pi/2)".to_string());
    }

    let x = sub(point, apex);
    let angle_x = length(cross(direction, x)).atan2(dot(direction, x));
    if angle + std::f64::consts::FRAC_PI_2 < angle_x {
        return Ok((apex, scale(direction, -1.0)));
    }

    let k = scale(direction, dot(x, direction));
    let xk_dir = normalize(sub(x, k)).unwrap_or_else(|| arbitrary_perpendicular(direction));
    let d = add(k, scale(xk_dir, length(k) * angle.tan()));
    let norm_d = normalize(d).ok_or_else(|| "cone projection is degenerate".to_string())?;
    let projection = add(apex, scale(norm_d, dot(norm_d, x)));
    let z = cross(direction, norm_d);
    let normal = normalize(cross(z, norm_d)).unwrap_or_else(|| scale(direction, -1.0));
    Ok((projection, normal))
}

pub(super) fn approximate_cone_meshlib(points: &[[f64; 3]]) -> Result<ConeApproximation, String> {
    if points.len() < 7 {
        return Err(format!(
            "Cone3Approximation requires at least 7 points, got {}",
            points.len()
        ));
    }

    let center = centroid(points);
    let pcm_axis = compute_center_normal_axis(points, center)
        .ok_or_else(|| "Unable to estimate cone axis from points".to_string())?;
    let mut best = solve_from_initial_axis(points, center, pcm_axis)
        .ok_or_else(|| "Unable to initialize cone from principal component axis".to_string())?;

    let phi_resolution = 30usize;
    let theta_resolution = 30usize;
    let theta_step = 2.0 * std::f64::consts::PI / phi_resolution as f64;
    let phi_step = std::f64::consts::FRAC_PI_2 / phi_resolution as f64;
    for j in 0..=phi_resolution {
        let phi = phi_step * j as f64;
        let (sin_phi, cos_phi) = phi.sin_cos();
        for i in 0..theta_resolution {
            let theta = theta_step * i as f64;
            let (sin_theta, cos_theta) = theta.sin_cos();
            let axis = [cos_theta * sin_phi, sin_theta * sin_phi, cos_phi];
            if let Some(candidate) = solve_from_initial_axis(points, center, axis) {
                if candidate.error < best.error {
                    best = candidate;
                }
            }
        }
    }

    Ok(best)
}

fn solve_from_initial_axis(
    points: &[[f64; 3]],
    center: [f64; 3],
    axis: [f64; 3],
) -> Option<ConeApproximation> {
    let initial = compute_initial_cone(points, center, axis)?;
    Some(refine_cone_lm(points, initial))
}

fn refine_cone_lm(points: &[[f64; 3]], initial: ConeApproximation) -> ConeApproximation {
    let mut params = cone_to_fit_params(initial);
    let mut cost = residual_cost(points, params);
    let mut lambda = 1e-3;

    for _ in 0..40 {
        let (jtj, jtf) = normal_equations(points, params);
        let mut system = jtj;
        let diagonal_scale = (0..6)
            .map(|index| system[(index, index)].abs())
            .fold(1.0_f64, f64::max);
        for index in 0..6 {
            system[(index, index)] += lambda * diagonal_scale;
        }

        let Some(delta) = system.lu().solve(&(-jtf)) else {
            lambda *= 10.0;
            continue;
        };
        if delta.iter().any(|value| !value.is_finite()) {
            lambda *= 10.0;
            continue;
        }

        let candidate_params = params + delta;
        let candidate_cost = residual_cost(points, candidate_params);
        if candidate_cost.is_finite() && candidate_cost < cost {
            params = candidate_params;
            cost = candidate_cost;
            lambda = (lambda * 0.3).max(1e-12);
            if delta.norm() <= 1e-10 {
                break;
            }
        } else {
            lambda *= 10.0;
        }
    }

    fit_params_to_cone(points, params)
}

fn compute_initial_cone(
    points: &[[f64; 3]],
    center: [f64; 3],
    axis: [f64; 3],
) -> Option<ConeApproximation> {
    let mut direction = normalize(axis)?;
    let mut h_min = f64::INFINITY;
    let mut h_max = f64::NEG_INFINITY;
    let mut hr_pairs = Vec::with_capacity(points.len());
    for point in points {
        let delta = sub(*point, center);
        let h = dot(direction, delta);
        h_min = h_min.min(h);
        h_max = h_max.max(h);
        let radial = sub(delta, scale(direction, h));
        hr_pairs.push((h, length(radial)));
    }

    let (mut slope, intercept, h_average) = best_fit_line(&hr_pairs)?;
    if slope.abs() <= f64::EPSILON {
        return None;
    }
    if slope < 0.0 {
        direction = scale(direction, -1.0);
        slope = -slope;
        std::mem::swap(&mut h_min, &mut h_max);
        h_min = -h_min;
        h_max = -h_max;
    }

    let r_average = slope * h_average + intercept;
    let r_min = r_average + slope * (h_min - h_average);
    let r_max = r_average + slope * (h_max - h_average);
    let h_range = h_max - h_min;
    let r_range = r_max - r_min;
    if h_range <= f64::EPSILON || r_range <= f64::EPSILON {
        return None;
    }

    let angle = r_range.atan2(h_range);
    let tan_angle = angle.tan();
    if tan_angle <= f64::EPSILON || !tan_angle.is_finite() {
        return None;
    }
    let offset = r_max / tan_angle - h_max;
    let apex = sub(center, scale(direction, offset));
    Some(cone_from_parts(points, apex, direction, angle))
}

fn cone_to_fit_params(cone: ConeApproximation) -> Vec6 {
    let cos_angle = cone.angle.cos().max(f64::EPSILON);
    Vec6::new(
        cone.apex[0],
        cone.apex[1],
        cone.apex[2],
        cone.direction[0] / cos_angle,
        cone.direction[1] / cos_angle,
        cone.direction[2] / cos_angle,
    )
}

fn fit_params_to_cone(points: &[[f64; 3]], params: Vec6) -> ConeApproximation {
    let apex = [params[0], params[1], params[2]];
    let weighted_axis = [params[3], params[4], params[5]];
    let axis_len = length(weighted_axis);
    let direction = normalize(weighted_axis).unwrap_or([0.0, 0.0, 1.0]);
    let cos_angle = (1.0 / axis_len.max(f64::EPSILON)).clamp(0.0, 1.0);
    let angle = cos_angle.acos();
    cone_from_parts(points, apex, direction, angle)
}

fn cone_from_parts(
    points: &[[f64; 3]],
    apex: [f64; 3],
    direction: [f64; 3],
    angle: f64,
) -> ConeApproximation {
    let mut height = 0.0_f64;
    for point in points {
        height = height.max(dot(sub(*point, apex), direction).abs());
    }
    let base_radius = height * angle.tan();
    let mut cone = ConeApproximation {
        apex,
        direction,
        angle,
        height,
        base_radius,
        error: 0.0,
    };
    cone.error = projection_error(points, cone);
    cone
}

fn residual_cost(points: &[[f64; 3]], params: Vec6) -> f64 {
    let mut cost = 0.0;
    for point in points {
        let residual = cone_residual(params, *point);
        cost += residual * residual;
    }
    cost
}

fn normal_equations(points: &[[f64; 3]], params: Vec6) -> (Mat6, Vec6) {
    let mut jtj = Mat6::zeros();
    let mut jtf = Vec6::zeros();
    for point in points {
        let (residual, jacobian) = cone_residual_and_jacobian(params, *point);
        jtj += jacobian * jacobian.transpose();
        jtf += jacobian * residual;
    }
    (jtj, jtf)
}

fn cone_residual(params: Vec6, point: [f64; 3]) -> f64 {
    let d = [
        params[0] - point[0],
        params[1] - point[1],
        params[2] - point[2],
    ];
    let w = [params[3], params[4], params[5]];
    let d_dot_w = dot(d, w);
    dot(d, d) - d_dot_w * d_dot_w
}

fn cone_residual_and_jacobian(params: Vec6, point: [f64; 3]) -> (f64, Vec6) {
    let d = [
        params[0] - point[0],
        params[1] - point[1],
        params[2] - point[2],
    ];
    let w = [params[3], params[4], params[5]];
    let d_dot_w = dot(d, w);
    let pvw = sub(d, scale(w, d_dot_w));
    let pwd = scale(d, d_dot_w);
    (
        dot(d, d) - d_dot_w * d_dot_w,
        Vec6::new(
            2.0 * pvw[0],
            2.0 * pvw[1],
            2.0 * pvw[2],
            -2.0 * pwd[0],
            -2.0 * pwd[1],
            -2.0 * pwd[2],
        ),
    )
}

fn projection_error(points: &[[f64; 3]], cone: ConeApproximation) -> f64 {
    let mut error = 0.0;
    for point in points {
        let Ok((projection, _)) = project_cone_point(cone.apex, cone.direction, cone.angle, *point)
        else {
            return f64::INFINITY;
        };
        let delta = sub(projection, *point);
        error += dot(delta, delta);
    }
    error / points.len() as f64
}

fn compute_center_normal_axis(points: &[[f64; 3]], center: [f64; 3]) -> Option<[f64; 3]> {
    let mut axis = [0.0; 3];
    for point in points {
        let z = sub(*point, center);
        axis = add(axis, scale(z, dot(z, z)));
    }
    normalize(axis)
}

fn best_fit_line(pairs: &[(f64, f64)]) -> Option<(f64, f64, f64)> {
    let count = pairs.len() as f64;
    let (mut sum_x, mut sum_y) = (0.0, 0.0);
    for (x, y) in pairs {
        sum_x += x;
        sum_y += y;
    }
    let mean_x = sum_x / count;
    let mean_y = sum_y / count;
    let (mut covariance, mut variance) = (0.0, 0.0);
    for (x, y) in pairs {
        covariance += (x - mean_x) * (y - mean_y);
        variance += (x - mean_x) * (x - mean_x);
    }
    if variance <= f64::EPSILON {
        return None;
    }
    let slope = covariance / variance;
    Some((slope, mean_y - slope * mean_x, mean_x))
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

    #[test]
    fn cone_projection_matches_meshlib_apex_fallback_rule() {
        let (projection, normal) = project_cone_point(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            12.0_f64.to_radians(),
            [0.0, 0.0, -1.0],
        )
        .unwrap();
        assert_eq!(projection, [0.0, 0.0, 0.0]);
        assert_eq!(normal, [-0.0, -0.0, -1.0]);
    }

    #[test]
    fn cone_projection_matches_meshlib_side_projection_rule() {
        let angle = (2.0_f64 / 10.0).atan();
        let (projection, normal) =
            project_cone_point([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], angle, [4.0, 0.0, 10.0]).unwrap();

        assert!(projection[0] > 1.9 && projection[0] < 2.5);
        assert!(projection[1].abs() < 1e-12);
        assert!(projection[2] > 9.5 && projection[2] < 10.5);
        assert!(normal[0] > 0.9);
        assert!(normal[2] < 0.0);
    }

    #[test]
    fn cone_approximation_matches_meshlib_partial_arc_fixture() {
        let apex = [1.0, 2.0, 3.0];
        let direction = normalize([3.0, 2.0, 1.0]).unwrap();
        let height = 10.0;
        let angle = 12.0_f64.to_radians();
        let points = meshlib_cone_object_fixture_points(apex, direction, height, angle);

        let cone = approximate_cone_meshlib(&points).unwrap();
        let geometric_angle = angle.sin().atan();

        assert!(axis_similarity(cone.direction, direction) > 0.999);
        assert!(approx(geometric_angle, 1.0e-2).contains(&cone.angle));
        assert!(approx(angle, 0.1).contains(&cone.angle));
        assert!(approx(height, 2.0e-2).contains(&cone.height));
        assert!(approx(height * angle.sin(), 3.0e-2).contains(&cone.base_radius));
        assert!(point_distance(cone.apex, apex) < 3.0e-2);
        assert!(cone.error < 1.0e-4);
    }

    fn meshlib_cone_object_fixture_points(
        apex: [f64; 3],
        direction: [f64; 3],
        height: f64,
        angle: f64,
    ) -> Vec<[f64; 3]> {
        let resolution = 100usize;
        let start_angle = 0.0;
        let arc_angle = std::f64::consts::PI / 1.5;
        let z_step = 1.0 / resolution as f64;
        let angle_step = arc_angle / resolution as f64;
        let x_axis = arbitrary_perpendicular(direction);
        let y_axis = normalize(cross(direction, x_axis)).unwrap();
        let radius_scale = angle.tan() * height;
        let mut points = Vec::with_capacity(resolution * 2);
        for index in 0..resolution {
            let theta = start_angle + index as f64 * angle_step;
            let z = index as f64 * z_step;
            let noise = z.sin() * 1.0e-3;
            let (sin_theta, cos_theta) = theta.sin_cos();
            let radius1 = angle.cos() * z;
            let radius2 = angle.cos() * (1.0 - z);
            points.push(transform_fixture_point(
                apex,
                direction,
                x_axis,
                y_axis,
                height,
                radius_scale,
                [
                    cos_theta * radius1 + noise,
                    sin_theta * radius1 - noise,
                    z + noise,
                ],
            ));
            points.push(transform_fixture_point(
                apex,
                direction,
                x_axis,
                y_axis,
                height,
                radius_scale,
                [
                    cos_theta * radius2 - noise,
                    sin_theta * radius2 + noise,
                    1.0 - z - noise,
                ],
            ));
        }
        points
    }

    fn transform_fixture_point(
        apex: [f64; 3],
        direction: [f64; 3],
        x_axis: [f64; 3],
        y_axis: [f64; 3],
        height: f64,
        radius_scale: f64,
        point: [f64; 3],
    ) -> [f64; 3] {
        add(
            apex,
            add(
                add(
                    scale(x_axis, point[0] * radius_scale),
                    scale(y_axis, point[1] * radius_scale),
                ),
                scale(direction, point[2] * height),
            ),
        )
    }

    fn axis_similarity(left: [f64; 3], right: [f64; 3]) -> f64 {
        dot(left, right).abs()
    }

    fn point_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
        length(sub(left, right))
    }

    fn approx(expected: f64, tolerance: f64) -> std::ops::RangeInclusive<f64> {
        (expected - tolerance)..=(expected + tolerance)
    }
}
