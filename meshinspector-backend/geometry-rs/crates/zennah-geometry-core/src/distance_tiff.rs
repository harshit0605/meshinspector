use crate::distance::{
    axis_aligned_distance_map_model_transform, distance_map_stats, DistanceMapGrid,
    DISTANCE_MAP_NOT_VALID_VALUE,
};
use std::fs::File;
use std::path::Path;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::encoder::{colortype, TiffEncoder};
use tiff::tags::{PhotometricInterpretation, Tag};
use tiff::ColorType;

pub fn distance_map_from_tiff(path: impl AsRef<Path>) -> Result<DistanceMapGrid, String> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err("Path is empty".to_string());
    }

    let file = File::open(path).map_err(|_| format!("Cannot read file: {}", path.display()))?;
    let mut decoder = Decoder::new(file)
        .map_err(|error| format!("Cannot read file: {}: {error}", path.display()))?;
    let (width, height) = decoder
        .dimensions()
        .map_err(|error| format!("Cannot read TIFF dimensions: {}: {error}", path.display()))?;
    let color_type = decoder
        .colortype()
        .map_err(|error| format!("Cannot read TIFF pixel format: {}: {error}", path.display()))?;
    let transform = read_distance_map_transform(&mut decoder, path)?;
    let undo_white_zero = matches!(
        decoder.find_tag_unsigned::<u16>(Tag::PhotometricInterpretation),
        Ok(Some(value)) if value == PhotometricInterpretation::WhiteIsZero.to_u16()
    );
    let sample_count = tiff_sample_count(color_type)?;
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or_else(|| "TIFF dimensions overflow distance-map size".to_string())?;
    let image = decoder
        .read_image()
        .map_err(|error| format!("Cannot read TIFF pixels: {}: {error}", path.display()))?;
    let values = tiff_decoding_result_to_values(image, pixel_count, sample_count, undo_white_zero)?;
    let (valid_count, min_value, max_value) = distance_map_stats(&values);

    Ok(DistanceMapGrid {
        width: width as usize,
        height: height as usize,
        origin: transform.origin,
        pixel_size: transform.pixel_size,
        model_transform: transform.model_transform,
        values,
        valid_count,
        min_value,
        max_value,
    })
}

pub fn distance_map_to_tiff(map: &DistanceMapGrid, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err("Path is empty".to_string());
    }
    validate_tiff_distance_map(map)?;

    let file = File::create(path).map_err(|_| format!("Cannot write file: {}", path.display()))?;
    let mut encoder = TiffEncoder::new(file)
        .map_err(|error| format!("Cannot write file: {}: {error}", path.display()))?;
    let mut image = encoder
        .new_image::<colortype::Gray32Float>(map.width as u32, map.height as u32)
        .map_err(|error| format!("Cannot initialize TIFF image: {}: {error}", path.display()))?;
    image
        .encoder()
        .write_tag(
            Tag::PhotometricInterpretation,
            PhotometricInterpretation::WhiteIsZero.to_u16(),
        )
        .map_err(|error| {
            format!(
                "Cannot write TIFF photometric tag: {}: {error}",
                path.display()
            )
        })?;
    let no_data = format!("{:.16e}", DISTANCE_MAP_NOT_VALID_VALUE);
    image
        .encoder()
        .write_tag(Tag::GdalNodata, no_data.as_str())
        .map_err(|error| format!("Cannot write TIFF NoData tag: {}: {error}", path.display()))?;
    let transform = map
        .model_transform
        .unwrap_or_else(|| axis_aligned_distance_map_model_transform(map.origin, map.pixel_size));
    image
        .encoder()
        .write_tag(Tag::ModelTransformationTag, &transform[..])
        .map_err(|error| {
            format!(
                "Cannot write TIFF model transform tag: {}: {error}",
                path.display()
            )
        })?;
    image
        .write_data(&map.values)
        .map_err(|error| format!("Cannot write TIFF pixels: {}: {error}", path.display()))
}

#[derive(Clone, Copy, Debug)]
struct DistanceMapTiffTransform {
    origin: [f64; 2],
    pixel_size: [f64; 2],
    model_transform: Option<[f64; 16]>,
}

impl Default for DistanceMapTiffTransform {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0],
            pixel_size: [1.0, 1.0],
            model_transform: None,
        }
    }
}

fn read_distance_map_transform(
    decoder: &mut Decoder<File>,
    path: &Path,
) -> Result<DistanceMapTiffTransform, String> {
    if let Some(value) = decoder
        .find_tag(Tag::ModelTransformationTag)
        .map_err(|error| {
            format!(
                "Cannot read TIFF model transform tag: {}: {error}",
                path.display()
            )
        })?
    {
        let matrix = value.into_f64_vec().map_err(|error| {
            format!(
                "Cannot parse TIFF model transform tag: {}: {error}",
                path.display()
            )
        })?;
        if matrix.len() == 16 {
            return Ok(distance_map_transform_from_model_matrix(&matrix));
        }
    }

    let Some(tiepoint_value) = decoder.find_tag(Tag::ModelTiepointTag).map_err(|error| {
        format!(
            "Cannot read TIFF model tiepoint tag: {}: {error}",
            path.display()
        )
    })?
    else {
        return Ok(DistanceMapTiffTransform::default());
    };
    let tiepoint = tiepoint_value.into_f64_vec().map_err(|error| {
        format!(
            "Cannot parse TIFF model tiepoint tag: {}: {error}",
            path.display()
        )
    })?;
    if tiepoint.len() != 6 {
        return Ok(DistanceMapTiffTransform::default());
    }

    let Some(scale_value) = decoder.find_tag(Tag::ModelPixelScaleTag).map_err(|error| {
        format!(
            "Cannot read TIFF model pixel-scale tag: {}: {error}",
            path.display()
        )
    })?
    else {
        return Ok(DistanceMapTiffTransform::default());
    };
    let scale = scale_value.into_f64_vec().map_err(|error| {
        format!(
            "Cannot parse TIFF model pixel-scale tag: {}: {error}",
            path.display()
        )
    })?;
    if scale.len() != 3 {
        return Ok(DistanceMapTiffTransform::default());
    }

    let origin = [tiepoint[0] + tiepoint[3], tiepoint[1] + tiepoint[4]];
    let pixel_size = [scale[0], -scale[1]];
    Ok(DistanceMapTiffTransform {
        origin,
        pixel_size,
        model_transform: Some(axis_aligned_distance_map_model_transform(
            origin, pixel_size,
        )),
    })
}

fn distance_map_transform_from_model_matrix(matrix: &[f64]) -> DistanceMapTiffTransform {
    let model_transform = matrix.try_into().ok();
    let pixel_x_vec = [matrix[0], matrix[4], matrix[8]];
    let pixel_y_vec = [matrix[1], matrix[5], matrix[9]];
    DistanceMapTiffTransform {
        origin: [matrix[3], matrix[7]],
        pixel_size: [
            signed_axis_length(pixel_x_vec, 0),
            signed_axis_length(pixel_y_vec, 1),
        ],
        model_transform,
    }
}

fn signed_axis_length(vector: [f64; 3], axis: usize) -> f64 {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if vector[axis] < 0.0 {
        -length
    } else {
        length
    }
}

fn validate_tiff_distance_map(map: &DistanceMapGrid) -> Result<(), String> {
    if map.width == 0 || map.height == 0 {
        return Err("ObjectDistanceMap is empty".to_string());
    }
    let expected_len = map
        .width
        .checked_mul(map.height)
        .ok_or_else(|| "distance-map dimensions overflow value count".to_string())?;
    if map.values.len() != expected_len {
        return Err("distance-map values must match width * height".to_string());
    }
    if map.width > u32::MAX as usize || map.height > u32::MAX as usize {
        return Err("distance-map dimensions exceed TIFF limits".to_string());
    }
    Ok(())
}

fn tiff_sample_count(color_type: ColorType) -> Result<usize, String> {
    match color_type {
        ColorType::Gray(_) => Ok(1),
        ColorType::RGB(_) => Ok(3),
        ColorType::RGBA(_) => Ok(4),
        ColorType::Multiband { num_samples, .. } if matches!(num_samples, 1 | 3 | 4) => {
            Ok(num_samples as usize)
        }
        _ => Err(format!(
            "Unsupported TIFF pixel format for MeshLib-style distance-map import: {color_type:?}"
        )),
    }
}

fn tiff_decoding_result_to_values(
    image: DecodingResult,
    pixel_count: usize,
    sample_count: usize,
    undo_white_zero: bool,
) -> Result<Vec<f32>, String> {
    match image {
        DecodingResult::U8(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                if undo_white_zero {
                    (u8::MAX - value) as f32
                } else {
                    value as f32
                }
            })
        }
        DecodingResult::U16(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                if undo_white_zero {
                    (u16::MAX - value) as f32
                } else {
                    value as f32
                }
            })
        }
        DecodingResult::U32(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                if undo_white_zero {
                    (u32::MAX - value) as f32
                } else {
                    value as f32
                }
            })
        }
        DecodingResult::U64(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                if undo_white_zero {
                    (u64::MAX - value) as f32
                } else {
                    value as f32
                }
            })
        }
        DecodingResult::I8(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                value as f32
            })
        }
        DecodingResult::I16(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                value as f32
            })
        }
        DecodingResult::I32(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                value as f32
            })
        }
        DecodingResult::I64(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                value as f32
            })
        }
        DecodingResult::F16(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                value.to_f32()
            })
        }
        DecodingResult::F32(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                if undo_white_zero {
                    1.0 - value
                } else {
                    value
                }
            })
        }
        DecodingResult::F64(samples) => {
            tiff_samples_to_distance_values(&samples, pixel_count, sample_count, |value| {
                if undo_white_zero {
                    (1.0 - value) as f32
                } else {
                    value as f32
                }
            })
        }
    }
}

fn tiff_samples_to_distance_values<T: Copy>(
    samples: &[T],
    pixel_count: usize,
    sample_count: usize,
    to_f32: impl Fn(T) -> f32,
) -> Result<Vec<f32>, String> {
    let expected_len = pixel_count
        .checked_mul(sample_count)
        .ok_or_else(|| "TIFF sample count overflows distance-map size".to_string())?;
    if samples.len() != expected_len {
        return Err(format!(
            "TIFF sample count does not match dimensions: expected {expected_len}, got {}",
            samples.len()
        ));
    }

    if sample_count == 1 {
        return Ok(samples.iter().map(|value| to_f32(*value)).collect());
    }

    let mut values = Vec::with_capacity(pixel_count);
    for pixel in samples.chunks_exact(sample_count) {
        let red = to_f32(pixel[0]);
        let green = to_f32(pixel[1]);
        let blue = to_f32(pixel[2]);
        values.push(0.299 * red + 0.587 * green + 0.114 * blue);
    }
    Ok(values)
}
