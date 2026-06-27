use super::{linear_index, sample_lut_linear, VolumeRenderShadingMode};

pub(super) fn normal_for_shading(
    mode: VolumeRenderShadingMode,
    values: &[f32],
    shape: [usize; 3],
    coord: [usize; 3],
    colors_rgba: &[[u8; 4]],
    min_value: f32,
    max_value: f32,
) -> Option<[f32; 3]> {
    match mode {
        VolumeRenderShadingMode::ValueGradient => value_gradient_normal(values, shape, coord),
        VolumeRenderShadingMode::AlphaGradient => {
            alpha_gradient_normal(values, shape, coord, colors_rgba, min_value, max_value)
        }
        VolumeRenderShadingMode::None => None,
    }
}

pub(super) fn shade_color(
    color: &mut [f32; 4],
    position_eye: [f64; 3],
    normal_eye: [f32; 3],
    light_pos_eye: [f64; 3],
    ambient_strength: f32,
    specular_strength: f32,
    spec_exp: f32,
) {
    let Some(direction_to_light_eye) = normalize_vec3([
        light_pos_eye[0] - position_eye[0],
        light_pos_eye[1] - position_eye[1],
        light_pos_eye[2] - position_eye[2],
    ]) else {
        return;
    };

    let mut normal = [
        normal_eye[0] as f64,
        normal_eye[1] as f64,
        normal_eye[2] as f64,
    ];
    let mut dot_prod = dot3(direction_to_light_eye, normal);
    if dot_prod < 0.0 {
        dot_prod = -dot_prod;
        normal = [-normal[0], -normal[1], -normal[2]];
    }

    let reflection_eye = reflect_vec3(
        [
            -direction_to_light_eye[0],
            -direction_to_light_eye[1],
            -direction_to_light_eye[2],
        ],
        normal,
    );
    let dot_prod_specular = normalize_vec3([-position_eye[0], -position_eye[1], -position_eye[2]])
        .map(|surface_to_viewer_eye| dot3(reflection_eye, surface_to_viewer_eye).max(0.0))
        .unwrap_or(0.0);
    let specular_factor = dot_prod_specular.powf(spec_exp as f64);
    let factor = ambient_strength as f64 + dot_prod + specular_factor * specular_strength as f64;
    color[0] *= factor as f32;
    color[1] *= factor as f32;
    color[2] *= factor as f32;
}

fn value_gradient_normal(values: &[f32], shape: [usize; 3], coord: [usize; 3]) -> Option<[f32; 3]> {
    let min_x = sample_volume_clamped(values, shape, coord, 0, -1);
    let max_x = sample_volume_clamped(values, shape, coord, 0, 1);
    let min_y = sample_volume_clamped(values, shape, coord, 1, -1);
    let max_y = sample_volume_clamped(values, shape, coord, 1, 1);
    let min_z = sample_volume_clamped(values, shape, coord, 2, -1);
    let max_z = sample_volume_clamped(values, shape, coord, 2, 1);
    normalize_normal([-(max_x - min_x), -(max_y - min_y), -(max_z - min_z)])
}

fn alpha_gradient_normal(
    values: &[f32],
    shape: [usize; 3],
    coord: [usize; 3],
    colors_rgba: &[[u8; 4]],
    min_value: f32,
    max_value: f32,
) -> Option<[f32; 3]> {
    let alpha_at = |axis, offset| {
        sample_alpha_gradient_value(
            sample_volume_clamped(values, shape, coord, axis, offset),
            colors_rgba,
            min_value,
            max_value,
        )
    };
    normalize_normal([
        -(alpha_at(0, 1) - alpha_at(0, -1)),
        -(alpha_at(1, 1) - alpha_at(1, -1)),
        -(alpha_at(2, 1) - alpha_at(2, -1)),
    ])
}

fn sample_alpha_gradient_value(
    value: f32,
    colors_rgba: &[[u8; 4]],
    min_value: f32,
    max_value: f32,
) -> f32 {
    let normalized = (value - min_value) / (max_value - min_value);
    if !(0.0..=1.0).contains(&normalized) {
        return 0.0;
    }
    sample_lut_linear(colors_rgba, normalized)[3]
}

fn sample_volume_clamped(
    values: &[f32],
    shape: [usize; 3],
    coord: [usize; 3],
    axis: usize,
    offset: isize,
) -> f32 {
    let mut sample = coord;
    sample[axis] = (sample[axis] as isize + offset).clamp(0, shape[axis] as isize - 1) as usize;
    values[linear_index(sample, shape)]
}

fn normalize_normal(value: [f32; 3]) -> Option<[f32; 3]> {
    let squared = value[0] * value[0] + value[1] * value[1] + value[2] * value[2];
    if !squared.is_finite() || squared < 1.0e-5 {
        return None;
    }
    let norm = squared.sqrt();
    Some([value[0] / norm, value[1] / norm, value[2] / norm])
}

fn normalize_vec3(value: [f64; 3]) -> Option<[f64; 3]> {
    let norm = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    if !norm.is_finite() || norm <= 1e-12 {
        return None;
    }
    Some([value[0] / norm, value[1] / norm, value[2] / norm])
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn reflect_vec3(incoming: [f64; 3], normal: [f64; 3]) -> [f64; 3] {
    let factor = 2.0 * dot3(normal, incoming);
    [
        incoming[0] - factor * normal[0],
        incoming[1] - factor * normal[1],
        incoming[2] - factor * normal[2],
    ]
}
