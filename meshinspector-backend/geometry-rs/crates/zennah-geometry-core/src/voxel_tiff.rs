use crate::TiffVoxelVolume;
use std::cmp::Ordering;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tiff::decoder::{Decoder, DecodingResult};
use tiff::ColorType;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TiffSliceParameters {
    width: u32,
    height: u32,
    color_type: ColorType,
}

pub fn load_tiff_voxels_dir(
    dir: impl AsRef<Path>,
    voxel_size: [f32; 3],
    grid_level_set: bool,
) -> Result<TiffVoxelVolume, String> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return Err("Given path is not directory".to_string());
    }
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("Wrong voxel size parameter value".to_string());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|_| "Given path is not directory".to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_file() && is_tiff_file(&path) {
            files.push(path);
        }
    }
    if files.len() < 2 {
        return Err("Too few TIFF files in the directory".to_string());
    }
    sort_scan_files_by_name(&mut files);

    let first_params = read_tiff_slice_parameters(&files[0])?;
    let width = usize::try_from(first_params.width)
        .map_err(|_| "TIFF width overflows voxel dimensions".to_string())?;
    let height = usize::try_from(first_params.height)
        .map_err(|_| "TIFF height overflows voxel dimensions".to_string())?;
    let slice_count = files.len();
    let slice_len = width
        .checked_mul(height)
        .ok_or_else(|| "TIFF slice dimensions overflow voxel size".to_string())?;
    let value_count = slice_len
        .checked_mul(slice_count)
        .ok_or_else(|| "TIFF volume dimensions overflow voxel size".to_string())?;
    let mut values = Vec::with_capacity(value_count);
    let mut min_value = f32::MAX;
    let mut max_value = f32::MIN;

    for path in &files {
        let (params, slice_values) = read_tiff_slice_values(path)?;
        if params != first_params {
            return Err("Inconsistent TIFF files".to_string());
        }
        if slice_values.len() != slice_len {
            return Err("Inconsistent TIFF files".to_string());
        }
        for value in slice_values {
            min_value = min_value.min(value);
            max_value = max_value.max(value);
            values.push(value);
        }
    }

    Ok(TiffVoxelVolume {
        dimensions: [width, height, slice_count],
        voxel_size,
        grid_level_set,
        values,
        min: min_value,
        max: max_value,
        source_path: dir.display().to_string(),
        source_files: files
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    })
}

fn is_tiff_file(path: &Path) -> bool {
    read_tiff_slice_parameters(path).is_ok()
}

fn read_tiff_slice_parameters(path: &Path) -> Result<TiffSliceParameters, String> {
    let file = File::open(path).map_err(|_| format!("Cannot read file: {}", path.display()))?;
    let mut decoder = Decoder::new(file)
        .map_err(|error| format!("Cannot read file: {}: {error}", path.display()))?;
    let (width, height) = decoder
        .dimensions()
        .map_err(|error| format!("Cannot read TIFF dimensions: {}: {error}", path.display()))?;
    let color_type = decoder
        .colortype()
        .map_err(|error| format!("Cannot read TIFF pixel format: {}: {error}", path.display()))?;
    tiff_sample_count(color_type).map_err(|error| format!("{error}: {}", path.display()))?;
    Ok(TiffSliceParameters {
        width,
        height,
        color_type,
    })
}

fn read_tiff_slice_values(path: &Path) -> Result<(TiffSliceParameters, Vec<f32>), String> {
    let file = File::open(path).map_err(|_| format!("Cannot read file: {}", path.display()))?;
    let mut decoder = Decoder::new(file)
        .map_err(|error| format!("Cannot read file: {}: {error}", path.display()))?;
    let (width, height) = decoder
        .dimensions()
        .map_err(|error| format!("Cannot read TIFF dimensions: {}: {error}", path.display()))?;
    let color_type = decoder
        .colortype()
        .map_err(|error| format!("Cannot read TIFF pixel format: {}: {error}", path.display()))?;
    let sample_count = tiff_sample_count(color_type)?;
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or_else(|| "TIFF dimensions overflow voxel slice size".to_string())?;
    let image = decoder
        .read_image()
        .map_err(|error| format!("Cannot read TIFF pixels: {}: {error}", path.display()))?;
    let values = tiff_decoding_result_to_values(image, pixel_count, sample_count)?;
    Ok((
        TiffSliceParameters {
            width,
            height,
            color_type,
        },
        values,
    ))
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
            "Unsupported TIFF pixel format for MeshLib-style voxel import: {color_type:?}"
        )),
    }
}

fn tiff_decoding_result_to_values(
    image: DecodingResult,
    pixel_count: usize,
    sample_count: usize,
) -> Result<Vec<f32>, String> {
    match image {
        DecodingResult::U8(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::U16(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::U32(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::U64(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::I8(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::I16(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::I32(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::I64(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
        DecodingResult::F16(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value.to_f32())
        }
        DecodingResult::F32(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value)
        }
        DecodingResult::F64(samples) => {
            tiff_samples_to_values(&samples, pixel_count, sample_count, |value| value as f32)
        }
    }
}

fn tiff_samples_to_values<T: Copy>(
    samples: &[T],
    pixel_count: usize,
    sample_count: usize,
    to_f32: impl Fn(T) -> f32,
) -> Result<Vec<f32>, String> {
    let expected_len = pixel_count
        .checked_mul(sample_count)
        .ok_or_else(|| "TIFF sample count overflows voxel slice size".to_string())?;
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

fn sort_scan_files_by_name(files: &mut [PathBuf]) {
    let order: Vec<f64> = files
        .iter()
        .map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(last_number_in_name)
                .unwrap_or(0.0)
        })
        .collect();
    let mut indices: Vec<usize> = (0..files.len()).collect();
    indices.sort_by(|left, right| {
        order[*left]
            .partial_cmp(&order[*right])
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.cmp(right))
    });
    let sorted = indices
        .into_iter()
        .map(|index| files[index].clone())
        .collect::<Vec<_>>();
    files.clone_from_slice(&sorted);
}

fn last_number_in_name(name: &str) -> f64 {
    let Some(mut end) = name.rfind(|ch: char| ch == '-' || ch == '.' || ch.is_ascii_digit()) else {
        return 0.0;
    };
    end += 1;
    let mut start = end;
    while start > 0 {
        let previous = name.as_bytes()[start - 1] as char;
        if previous != '-' && previous != '.' && !previous.is_ascii_digit() {
            break;
        }
        start -= 1;
    }
    name[start..end].parse::<f64>().unwrap_or(0.0)
}
