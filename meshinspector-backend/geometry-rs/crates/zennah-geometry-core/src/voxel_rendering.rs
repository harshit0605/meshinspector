use std::collections::HashSet;

use crate::{
    GeometryError, VoxelVolumeRenderDataResult, VoxelVolumeRenderLutResult,
    VoxelVolumeRenderRayResult,
};

mod lighting;

const VOLUME_RENDER_LUT_REFERENCE: &str = "RenderVolumeObject::bindVolume_ denseMap";
const VOLUME_RENDER_RAY_REFERENCE: &str = "MRVolumeShader fixed-step ray compositing";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VolumeRenderShadingMode {
    None,
    ValueGradient,
    AlphaGradient,
}

impl VolumeRenderShadingMode {
    fn parse(value: &str) -> Result<Self, GeometryError> {
        match normalize_selector(value).as_str() {
            "" | "none" | "off" | "disabled" => Ok(Self::None),
            "valuegradient" | "densegradient" | "value" | "shadingmode1" => Ok(Self::ValueGradient),
            "alphagradient" | "alpha" | "alphagrad" | "shadingmode2" => Ok(Self::AlphaGradient),
            _ => Err(GeometryError::InvalidSelectionParameter {
                field: "shading_mode",
                value: value.to_string(),
            }),
        }
    }
}

pub fn voxel_volume_render_data_values(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    active_min_corner: [usize; 3],
    active_dimensions: [usize; 3],
    source_min_value: f32,
    source_max_value: f32,
) -> Result<VoxelVolumeRenderDataResult, GeometryError> {
    validate_shape(shape)?;
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "voxel_size",
            value: format!("{voxel_size:?}"),
        });
    }
    if active_dimensions.iter().any(|value| *value == 0) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "active_dimensions",
            value: format!("{active_dimensions:?}"),
        });
    }
    for axis in 0..3 {
        let max_excluded = active_min_corner[axis]
            .checked_add(active_dimensions[axis])
            .ok_or(GeometryError::GridTooLarge { shape })?;
        if max_excluded > shape[axis] {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "active_box",
                value: format!(
                    "min={active_min_corner:?}, dimensions={active_dimensions:?}, shape={shape:?}"
                ),
            });
        }
    }
    if !source_min_value.is_finite()
        || !source_max_value.is_finite()
        || source_max_value <= source_min_value
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "source_scale",
            value: format!("{source_min_value}..{source_max_value}"),
        });
    }

    let expected_values = shape
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge { shape })?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    for (index, value) in values.iter().copied().enumerate() {
        if value.is_nan() {
            return Err(GeometryError::InvalidVoxelValue { index, value });
        }
    }

    let active_values = active_dimensions
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge {
            shape: active_dimensions,
        })?;
    let mut normalized_values = Vec::with_capacity(active_values);
    let mut source_indices = Vec::with_capacity(active_values);
    let mut coordinates = Vec::with_capacity(active_values);
    let denominator = source_max_value - source_min_value;

    for z in active_min_corner[2]..active_min_corner[2] + active_dimensions[2] {
        for y in active_min_corner[1]..active_min_corner[1] + active_dimensions[1] {
            for x in active_min_corner[0]..active_min_corner[0] + active_dimensions[0] {
                let coord = [x, y, z];
                let index = linear_index(coord, shape);
                let normalized = ((values[index] - source_min_value) / denominator).clamp(0.0, 1.0);
                normalized_values.push(normalized);
                source_indices.push(index);
                coordinates.push(coord);
            }
        }
    }

    Ok(VoxelVolumeRenderDataResult {
        dimensions: active_dimensions,
        voxel_size,
        source_indices,
        coordinates,
        values: normalized_values,
        min_value: 0.0,
        max_value: 1.0,
    })
}

pub fn voxel_volume_render_lut_values(
    lut_type: &str,
    alpha_type: &str,
    alpha_limit: u8,
    one_color: Option<[u8; 4]>,
) -> Result<VoxelVolumeRenderLutResult, GeometryError> {
    let normalized_lut = normalize_selector(lut_type);
    let canonical_lut = match normalized_lut.as_str() {
        "grayshades" | "gray" | "grey" | "greyshades" => "gray_shades",
        "rainbow" => "rainbow",
        "onecolor" | "color" => "one_color",
        _ => {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "lut_type",
                value: lut_type.to_string(),
            });
        }
    };
    let normalized_alpha = normalize_selector(alpha_type);
    let canonical_alpha = match normalized_alpha.as_str() {
        "constant" => "constant",
        "linearincreasing" | "increasing" => "linear_increasing",
        "lineardecreasing" | "decreasing" => "linear_decreasing",
        _ => {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "alpha_type",
                value: alpha_type.to_string(),
            });
        }
    };

    let mut colors_rgba = match canonical_lut {
        "gray_shades" => vec![[255, 255, 255, 255], [0, 0, 0, 255]],
        "one_color" => {
            let color = one_color.unwrap_or([255, 255, 255, 255]);
            vec![color, color]
        }
        "rainbow" => vec![
            [255, 0, 0, 255],
            [255, 127, 0, 255],
            [255, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [75, 0, 130, 255],
            [148, 0, 211, 255],
        ],
        _ => unreachable!("canonical LUT variants are matched above"),
    };

    match canonical_lut {
        "gray_shades" | "one_color" => {
            apply_two_stop_alpha(&mut colors_rgba, canonical_alpha, alpha_limit)
        }
        "rainbow" => apply_rainbow_alpha(&mut colors_rgba, canonical_alpha, alpha_limit),
        _ => unreachable!("canonical LUT variants are matched above"),
    }

    Ok(VoxelVolumeRenderLutResult {
        lut_type: canonical_lut.to_string(),
        alpha_type: canonical_alpha.to_string(),
        alpha_limit,
        colors_rgba,
        meshlib_reference: VOLUME_RENDER_LUT_REFERENCE.to_string(),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn voxel_volume_render_ray_values(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    min_corner: [usize; 3],
    ray_start: [f64; 3],
    ray_direction: [f64; 3],
    sampling_step: f64,
    min_value: f32,
    max_value: f32,
    lut_type: &str,
    alpha_type: &str,
    alpha_limit: u8,
    one_color: Option<[u8; 4]>,
    clipping_plane: Option<[f64; 4]>,
    shading_mode: &str,
    light_pos_eye: Option<[f64; 3]>,
    ambient_strength: f32,
    specular_strength: f32,
    spec_exp: f32,
    active_indices: Option<&[usize]>,
    max_steps: usize,
) -> Result<VoxelVolumeRenderRayResult, GeometryError> {
    validate_volume_render_ray_inputs(
        values,
        shape,
        voxel_size,
        ray_start,
        ray_direction,
        sampling_step,
        min_value,
        max_value,
        clipping_plane,
        light_pos_eye,
        ambient_strength,
        specular_strength,
        spec_exp,
        max_steps,
    )?;
    let shading_mode = VolumeRenderShadingMode::parse(shading_mode)?;

    let lut = voxel_volume_render_lut_values(lut_type, alpha_type, alpha_limit, one_color)?;
    let active_set = active_indices
        .map(|indices| {
            let expected_values = value_count(shape)?;
            let mut set = HashSet::with_capacity(indices.len());
            for index in indices.iter().copied() {
                if index >= expected_values {
                    return Err(GeometryError::InvalidSelectionParameter {
                        field: "active_indices",
                        value: format!("{index} outside 0..{expected_values}"),
                    });
                }
                set.insert(index);
            }
            Ok(set)
        })
        .transpose()?;

    let ray_dir =
        normalize_vec3(ray_direction).ok_or_else(|| GeometryError::InvalidSelectionParameter {
            field: "ray_direction",
            value: format!("{ray_direction:?}"),
        })?;
    let min_point = [
        voxel_size[0] * min_corner[0] as f64,
        voxel_size[1] * min_corner[1] as f64,
        voxel_size[2] * min_corner[2] as f64,
    ];
    let diagonal = [
        shape[0] as f64 * voxel_size[0],
        shape[1] as f64 * voxel_size[1],
        shape[2] as f64 * voxel_size[2],
    ];

    let mut position = ray_start;
    let mut out_color = [0.0_f32, 0.0, 0.0, 0.0];
    let mut first_opaque_world = None;
    let mut visited_indices = Vec::new();
    let mut accepted_indices = Vec::new();
    let mut start_voxel = clamped_start_voxel(position, min_point, voxel_size, shape);
    // Whether the ray has entered the volume yet. Before entry an out-of-bounds
    // sample means the ray is still approaching from outside, so keep stepping;
    // after entry it means the ray has exited and we stop. (Previously any
    // out-of-bounds sample broke immediately, so a fixed-step ray starting outside
    // could terminate before reaching the volume, yielding an empty render.)
    let mut has_entered = false;

    for _ in 0..max_steps {
        if out_color[3] >= 1.0 {
            break;
        }

        if sampling_step <= 0.0 {
            ray_voxel_intersection(
                min_point,
                &mut start_voxel,
                voxel_size,
                &mut position,
                ray_dir,
            );
        } else {
            position = [
                position[0] + ray_dir[0] * sampling_step,
                position[1] + ray_dir[1] * sampling_step,
                position[2] + ray_dir[2] * sampling_step,
            ];
        }
        let text_coord = [
            (position[0] - min_point[0]) / diagonal[0],
            (position[1] - min_point[1]) / diagonal[1],
            (position[2] - min_point[2]) / diagonal[2],
        ];
        if text_coord
            .iter()
            .any(|coord| *coord < -0.001 || *coord > 1.001)
        {
            if has_entered {
                break;
            }
            continue;
        }
        if text_coord.iter().any(|coord| *coord < 0.0 || *coord >= 1.0) {
            continue;
        }
        has_entered = true;
        if clipping_plane.is_some_and(|plane| is_clipped_by_plane(position, plane)) {
            continue;
        }

        let coord = [
            ((text_coord[0] * shape[0] as f64) as usize).min(shape[0] - 1),
            ((text_coord[1] * shape[1] as f64) as usize).min(shape[1] - 1),
            ((text_coord[2] * shape[2] as f64) as usize).min(shape[2] - 1),
        ];
        let index = linear_index(coord, shape);
        visited_indices.push(index);
        if active_set
            .as_ref()
            .is_some_and(|active| !active.contains(&index))
        {
            continue;
        }

        let density = values[index];
        if density < min_value || density > max_value {
            continue;
        }

        let normalized_value = (density - min_value) / (max_value - min_value);
        let mut sample_color = sample_lut_linear(&lut.colors_rgba, normalized_value);
        if sample_color[3] == 0.0 {
            continue;
        }
        let shading_normal = lighting::normal_for_shading(
            shading_mode,
            values,
            shape,
            coord,
            &lut.colors_rgba,
            min_value,
            max_value,
        );
        if shading_mode == VolumeRenderShadingMode::ValueGradient && shading_normal.is_none() {
            continue;
        }
        if let (Some(light), Some(normal)) = (light_pos_eye, shading_normal) {
            lighting::shade_color(
                &mut sample_color,
                position,
                normal,
                light,
                ambient_strength,
                specular_strength,
                spec_exp,
            );
        }

        let previous_alpha = out_color[3];
        let alpha = previous_alpha + sample_color[3] * (1.0 - previous_alpha);
        if alpha == 0.0 {
            continue;
        }
        out_color[0] = (sample_color[3] * sample_color[0] * (1.0 - previous_alpha)
            + out_color[0] * previous_alpha)
            / alpha;
        out_color[1] = (sample_color[3] * sample_color[1] * (1.0 - previous_alpha)
            + out_color[1] * previous_alpha)
            / alpha;
        out_color[2] = (sample_color[3] * sample_color[2] * (1.0 - previous_alpha)
            + out_color[2] * previous_alpha)
            / alpha;
        out_color[3] = if alpha > 0.98 { 1.0 } else { alpha };

        if first_opaque_world.is_none() {
            first_opaque_world = Some(position);
        }
        accepted_indices.push(index);
    }

    Ok(VoxelVolumeRenderRayResult {
        color_rgba: out_color,
        first_opaque_world,
        visited_indices,
        accepted_indices,
        meshlib_reference: VOLUME_RENDER_RAY_REFERENCE.to_string(),
    })
}

fn normalize_selector(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| !matches!(character, '_' | '-' | ' '))
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn validate_volume_render_ray_inputs(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: [f64; 3],
    ray_start: [f64; 3],
    ray_direction: [f64; 3],
    sampling_step: f64,
    min_value: f32,
    max_value: f32,
    clipping_plane: Option<[f64; 4]>,
    light_pos_eye: Option<[f64; 3]>,
    ambient_strength: f32,
    specular_strength: f32,
    spec_exp: f32,
    max_steps: usize,
) -> Result<(), GeometryError> {
    validate_shape(shape)?;
    if values.len() != value_count(shape)? {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    for (index, value) in values.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(GeometryError::InvalidVoxelValue { index, value });
        }
    }
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "voxel_size",
            value: format!("{voxel_size:?}"),
        });
    }
    if ray_start.iter().any(|value| !value.is_finite()) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "ray_start",
            value: format!("{ray_start:?}"),
        });
    }
    if ray_direction.iter().any(|value| !value.is_finite())
        || normalize_vec3(ray_direction).is_none()
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "ray_direction",
            value: format!("{ray_direction:?}"),
        });
    }
    if !sampling_step.is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "sampling_step",
            value: sampling_step.to_string(),
        });
    }
    if !min_value.is_finite() || !max_value.is_finite() || max_value <= min_value {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "value_range",
            value: format!("{min_value}..{max_value}"),
        });
    }
    if clipping_plane.is_some_and(|plane| plane.iter().any(|value| !value.is_finite())) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "clipping_plane",
            value: format!("{clipping_plane:?}"),
        });
    }
    if light_pos_eye.is_some_and(|position| position.iter().any(|value| !value.is_finite())) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "light_pos_eye",
            value: format!("{light_pos_eye:?}"),
        });
    }
    if !ambient_strength.is_finite() || ambient_strength < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "ambient_strength",
            value: ambient_strength.to_string(),
        });
    }
    if !specular_strength.is_finite() || specular_strength < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "specular_strength",
            value: specular_strength.to_string(),
        });
    }
    if !spec_exp.is_finite() || spec_exp < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "spec_exp",
            value: spec_exp.to_string(),
        });
    }
    if max_steps == 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_steps",
            value: max_steps.to_string(),
        });
    }
    Ok(())
}

fn value_count(shape: [usize; 3]) -> Result<usize, GeometryError> {
    shape
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge { shape })
}

fn normalize_vec3(value: [f64; 3]) -> Option<[f64; 3]> {
    let norm = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    if !norm.is_finite() || norm <= 1e-12 {
        return None;
    }
    Some([value[0] / norm, value[1] / norm, value[2] / norm])
}

fn is_clipped_by_plane(position: [f64; 3], clipping_plane: [f64; 4]) -> bool {
    position[0] * clipping_plane[0]
        + position[1] * clipping_plane[1]
        + position[2] * clipping_plane[2]
        > clipping_plane[3]
}

fn clamped_start_voxel(
    ray_start: [f64; 3],
    min_point: [f64; 3],
    voxel_size: [f64; 3],
    shape: [usize; 3],
) -> [isize; 3] {
    let mut start_voxel = [0_isize; 3];
    for axis in 0..3 {
        let voxel = ((ray_start[axis] - min_point[axis]) / voxel_size[axis]).floor() as isize;
        start_voxel[axis] = voxel.clamp(0, shape[axis] as isize - 1);
    }
    start_voxel
}

fn ray_voxel_intersection(
    min_corner: [f64; 3],
    voxel_coord: &mut [isize; 3],
    voxel_size: [f64; 3],
    ray_origin: &mut [f64; 3],
    ray: [f64; 3],
) {
    let mut abs_min_int = -1.0e20_f64;
    let mut abs_max_int = 1.0e20_f64;
    let mut abs_max_int_id = 3_usize;

    for axis in 0..3 {
        if ray[axis].abs() <= 0.001 {
            continue;
        }

        let voxel_min =
            voxel_coord[axis] as f64 * voxel_size[axis] + min_corner[axis] - ray_origin[axis];
        let voxel_max = (voxel_coord[axis] as f64 + 1.0) * voxel_size[axis] + min_corner[axis]
            - ray_origin[axis];
        let mut min_int = voxel_min / ray[axis];
        let mut max_int = voxel_max / ray[axis];
        if ray[axis] < 0.0 {
            std::mem::swap(&mut min_int, &mut max_int);
        }
        if min_int > abs_min_int {
            abs_min_int = min_int;
        }
        if max_int < abs_max_int {
            abs_max_int = max_int;
            abs_max_int_id = axis;
        }
    }

    ray_origin[0] += ray[0] * abs_min_int;
    ray_origin[1] += ray[1] * abs_min_int;
    ray_origin[2] += ray[2] * abs_min_int;

    if abs_max_int_id < 3 {
        voxel_coord[abs_max_int_id] += if ray[abs_max_int_id] >= 0.0 { 1 } else { -1 };
    } else {
        voxel_coord[0] += 1;
        voxel_coord[1] += 1;
        voxel_coord[2] += 1;
    }
}

fn sample_lut_linear(colors_rgba: &[[u8; 4]], value: f32) -> [f32; 4] {
    let value = value.clamp(0.0, 1.0);
    let texel = value * colors_rgba.len() as f32 - 0.5;
    let lower = texel.floor();
    let fraction = texel - lower;
    let lower_index = (lower as isize).clamp(0, colors_rgba.len() as isize - 1) as usize;
    let upper_index = (lower as isize + 1).clamp(0, colors_rgba.len() as isize - 1) as usize;
    let mut color = [0.0_f32; 4];
    for channel in 0..4 {
        let lower_value = colors_rgba[lower_index][channel] as f32 / 255.0;
        let upper_value = colors_rgba[upper_index][channel] as f32 / 255.0;
        color[channel] = lower_value * (1.0 - fraction) + upper_value * fraction;
    }
    color
}

fn apply_two_stop_alpha(colors_rgba: &mut [[u8; 4]], alpha_type: &str, alpha_limit: u8) {
    match alpha_type {
        "linear_increasing" => {
            colors_rgba[0][3] = 0;
            colors_rgba[1][3] = alpha_limit;
        }
        "linear_decreasing" => {
            colors_rgba[0][3] = alpha_limit;
            colors_rgba[1][3] = 0;
        }
        "constant" => {
            colors_rgba[0][3] = alpha_limit;
            colors_rgba[1][3] = alpha_limit;
        }
        _ => unreachable!("canonical alpha variants are matched above"),
    }
}

fn apply_rainbow_alpha(colors_rgba: &mut [[u8; 4]], alpha_type: &str, alpha_limit: u8) {
    match alpha_type {
        "linear_increasing" => {
            let alpha_step = alpha_limit as f32 / colors_rgba.len() as f32;
            for (index, color) in colors_rgba.iter_mut().enumerate() {
                color[3] = ((index as f32) * alpha_step).clamp(0.0, alpha_limit as f32) as u8;
            }
        }
        "linear_decreasing" => {
            let alpha_step = alpha_limit as f32 / colors_rgba.len() as f32;
            let last = colors_rgba.len() - 1;
            for index in 0..colors_rgba.len() {
                colors_rgba[last - index][3] =
                    ((index as f32) * alpha_step).clamp(0.0, alpha_limit as f32) as u8;
            }
        }
        "constant" => {
            for color in colors_rgba {
                color[3] = alpha_limit;
            }
        }
        _ => unreachable!("canonical alpha variants are matched above"),
    }
}

fn validate_shape(shape: [usize; 3]) -> Result<(), GeometryError> {
    if shape.iter().any(|value| *value == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    Ok(())
}

fn linear_index(coord: [usize; 3], shape: [usize; 3]) -> usize {
    coord[0] + coord[1] * shape[0] + coord[2] * shape[0] * shape[1]
}
