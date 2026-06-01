use crate::math::norm;
use crate::GeometryError;

pub(crate) fn grid_value_count(shape: [usize; 3]) -> Result<usize, GeometryError> {
    shape
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge { shape })
}

pub(crate) fn grid_index(key: [usize; 3], shape: [usize; 3]) -> usize {
    key[0] * shape[1] * shape[2] + key[1] * shape[2] + key[2]
}

pub(crate) fn sample_sdf_value(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    point: [f64; 3],
) -> f32 {
    let mut base = [0_usize; 3];
    let mut frac = [0.0_f64; 3];
    for axis in 0..3 {
        let upper = shape[axis] - 1;
        let coord = ((point[axis] - origin[axis]) / voxel_size).clamp(0.0, upper as f64 - 1e-9);
        let base_axis = (coord.floor() as usize).min(upper - 1);
        base[axis] = base_axis;
        frac[axis] = coord - base_axis as f64;
    }

    let mut interpolated = 0.0_f64;
    for dx in 0..=1 {
        let wx = if dx == 1 { frac[0] } else { 1.0 - frac[0] };
        for dy in 0..=1 {
            let wy = if dy == 1 { frac[1] } else { 1.0 - frac[1] };
            for dz in 0..=1 {
                let wz = if dz == 1 { frac[2] } else { 1.0 - frac[2] };
                let index = grid_index([base[0] + dx, base[1] + dy, base[2] + dz], shape);
                interpolated += wx * wy * wz * values[index] as f64;
            }
        }
    }
    interpolated as f32
}

pub(crate) fn sample_sdf_gradient(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    point: [f64; 3],
) -> [f32; 3] {
    let mut raw = [0.0_f64; 3];
    for axis in 0..3 {
        let mut positive = point;
        let mut negative = point;
        positive[axis] += voxel_size;
        negative[axis] -= voxel_size;
        let positive_value = sample_sdf_value(values, origin, shape, voxel_size, positive);
        let negative_value = sample_sdf_value(values, origin, shape, voxel_size, negative);
        let component = (positive_value - negative_value) / (2.0_f32 * voxel_size as f32);
        raw[axis] = component as f64;
    }
    let length = norm(raw).max(1e-12);
    [
        (raw[0] / length) as f32,
        (raw[1] / length) as f32,
        (raw[2] / length) as f32,
    ]
}
