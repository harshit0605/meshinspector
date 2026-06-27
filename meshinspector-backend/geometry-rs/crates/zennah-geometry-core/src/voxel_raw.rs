use crate::{RawVoxelParameters, RawVoxelScalarType, RawVoxelVolume};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_raw_voxels(
    path: impl AsRef<Path>,
    parameters: RawVoxelParameters,
) -> Result<RawVoxelVolume, String> {
    let path = path.as_ref();
    let bytes =
        fs::read(path).map_err(|_| format!("Cannot open file for reading {}", path.display()))?;
    raw_voxels_from_bytes(&bytes, path, parameters)
}

pub fn load_raw_voxels_auto(path: impl AsRef<Path>) -> Result<RawVoxelVolume, String> {
    let (resolved_path, parameters) = find_raw_voxel_parameters(path)?;
    load_raw_voxels(resolved_path, parameters)
}

pub fn find_raw_voxel_parameters(
    path: impl AsRef<Path>,
) -> Result<(PathBuf, RawVoxelParameters), String> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err("Path is empty".to_string());
    }

    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ext != "raw" {
        return Err(format!(
            "Extension is not correct, expected \".raw\" current \".{}\"",
            ext
        ));
    }

    let parent = path.parent().filter(|value| !value.as_os_str().is_empty());
    let parent = parent.unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err(format!("{} is not existing directory", parent.display()));
    }

    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Cannot parse filename".to_string())?;
    let mut candidates = Vec::new();
    for entry in fs::read_dir(parent)
        .map_err(|_| format!("{} is not existing directory", parent.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let candidate_name = entry.file_name();
        let candidate_name = candidate_name.to_string_lossy();
        if candidate_name.contains(filename) {
            candidates.push(entry.path());
        }
    }

    if candidates.is_empty() {
        return Err(format!("Cannot find file: {filename}"));
    }
    if candidates.len() > 1 {
        return Err(format!("More than one file exists: {filename}"));
    }

    let resolved_path = candidates.remove(0);
    let resolved_name = resolved_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Cannot parse filename".to_string())?;
    let parameters = parse_meshlib_raw_filename(resolved_name)?;
    Ok((resolved_path, parameters))
}

fn raw_voxels_from_bytes(
    bytes: &[u8],
    path: &Path,
    parameters: RawVoxelParameters,
) -> Result<RawVoxelVolume, String> {
    if parameters
        .dimensions
        .iter()
        .any(|dimension| *dimension == 0)
    {
        return Err("Wrong volume dimension parameter value".to_string());
    }
    if parameters
        .voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("Wrong voxel size parameter value".to_string());
    }

    let voxel_count = parameters
        .dimensions
        .iter()
        .try_fold(1_usize, |acc, dimension| acc.checked_mul(*dimension))
        .ok_or_else(|| "Raw voxel dimensions overflow value count".to_string())?;
    let unit_size = raw_scalar_type_size(parameters.scalar_type);
    let expected_len = voxel_count
        .checked_mul(unit_size)
        .ok_or_else(|| "Raw voxel byte count overflow".to_string())?;
    if bytes.len() < expected_len {
        return Err("Read error".to_string());
    }

    let mut values = Vec::with_capacity(voxel_count);
    let mut min_value = f32::MAX;
    let mut max_value = f32::MIN;
    for index in 0..voxel_count {
        let offset = index * unit_size;
        let value =
            raw_scalar_to_meshlib_value(parameters.scalar_type, &bytes[offset..offset + unit_size]);
        min_value = min_value.min(value);
        max_value = max_value.max(value);
        values.push(value);
    }

    Ok(RawVoxelVolume {
        dimensions: parameters.dimensions,
        voxel_size: parameters.voxel_size,
        grid_level_set: parameters.grid_level_set,
        scalar_type: parameters.scalar_type,
        values,
        min: min_value,
        max: max_value,
        source_path: path.display().to_string(),
    })
}

fn parse_meshlib_raw_filename(filename: &str) -> Result<RawVoxelParameters, String> {
    let parse_error = || format!("Cannot parse filename: {filename}");
    let w_end = filename.find('_').ok_or_else(parse_error)?;
    let width = parse_dimension(&filename[1..w_end], filename)?;

    let h_end = filename[w_end + 1..]
        .find('_')
        .map(|index| index + w_end + 1)
        .ok_or_else(parse_error)?;
    let height = parse_dimension(&filename[w_end + 2..h_end], filename)?;

    let s_end = filename[h_end + 1..]
        .find('_')
        .map(|index| index + h_end + 1)
        .ok_or_else(parse_error)?;
    let slices = parse_dimension(&filename[h_end + 2..s_end], filename)?;

    let xv_end = filename[s_end + 1..]
        .find('_')
        .map(|index| index + s_end + 1)
        .ok_or_else(parse_error)?;
    let x_size = parse_voxel_size(&filename[s_end + 2..xv_end], filename)?;

    let mut voxel_size = [x_size, x_size, x_size];
    let mut grid_level_set = false;
    let marker = filename.as_bytes().get(xv_end + 1).copied();
    if marker == Some(b'G') {
        if let Some(gt_end) = filename[xv_end + 1..].find('_') {
            let gt_end = gt_end + xv_end + 1;
            let gt = &filename[xv_end + 2..gt_end];
            grid_level_set = gt == "1";
        }
    }
    if marker != Some(b'F') {
        let yv_end = filename[xv_end + 1..]
            .find('_')
            .map(|index| index + xv_end + 1)
            .ok_or_else(parse_error)?;
        let y_size = parse_voxel_size(&filename[xv_end + 1..yv_end], filename)?;

        let zv_end = filename[yv_end + 1..]
            .find('_')
            .map(|index| index + yv_end + 1)
            .ok_or_else(parse_error)?;
        let z_size = parse_voxel_size(&filename[yv_end + 1..zv_end], filename)?;
        voxel_size = [x_size, y_size, z_size];

        if let Some(gt_end) = filename[zv_end + 1..].find('_') {
            let gt_end = gt_end + zv_end + 1;
            let gt = &filename[zv_end + 2..gt_end];
            grid_level_set = gt == "1";
        }
    }

    Ok(RawVoxelParameters {
        dimensions: [width, height, slices],
        voxel_size,
        grid_level_set,
        scalar_type: RawVoxelScalarType::Float32,
    })
}

fn parse_dimension(value: &str, filename: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("Cannot parse filename: {filename}"))
}

fn parse_voxel_size(value: &str, filename: &str) -> Result<f32, String> {
    let value = value
        .parse::<f32>()
        .map_err(|_| format!("Cannot parse filename: {filename}"))?;
    Ok(value / 1000.0)
}

fn raw_scalar_type_size(scalar_type: RawVoxelScalarType) -> usize {
    match scalar_type {
        RawVoxelScalarType::UInt8 | RawVoxelScalarType::Int8 => 1,
        RawVoxelScalarType::UInt16 | RawVoxelScalarType::Int16 => 2,
        RawVoxelScalarType::UInt32 | RawVoxelScalarType::Int32 | RawVoxelScalarType::Float32 => 4,
        RawVoxelScalarType::UInt64 | RawVoxelScalarType::Int64 | RawVoxelScalarType::Float64 => 8,
        RawVoxelScalarType::Float32_4 => 16,
    }
}

fn raw_scalar_to_meshlib_value(scalar_type: RawVoxelScalarType, bytes: &[u8]) -> f32 {
    match scalar_type {
        RawVoxelScalarType::UInt8 => bytes[0] as f32 / u8::MAX as f32,
        RawVoxelScalarType::Int8 => {
            let value = bytes[0] as i8;
            (value as f32 - i8::MIN as f32) / (i8::MAX as f32 - i8::MIN as f32)
        }
        RawVoxelScalarType::UInt16 => {
            let value = u16::from_le_bytes([bytes[0], bytes[1]]);
            value as f32 / u16::MAX as f32
        }
        RawVoxelScalarType::Int16 => {
            let value = i16::from_le_bytes([bytes[0], bytes[1]]);
            (value as f32 - i16::MIN as f32) / (i16::MAX as f32 - i16::MIN as f32)
        }
        RawVoxelScalarType::UInt32 => {
            let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            value as f32 / u32::MAX as f32
        }
        RawVoxelScalarType::Int32 => {
            let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            (value as f32 - i32::MIN as f32) / (i32::MAX as f32 - i32::MIN as f32)
        }
        RawVoxelScalarType::UInt64 => {
            let value = u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            value as f32 / u64::MAX as f32
        }
        RawVoxelScalarType::Int64 => {
            let value = i64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            (value as f32 - i64::MIN as f32) / (u64::MAX as f32)
        }
        RawVoxelScalarType::Float32 => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        RawVoxelScalarType::Float64 => {
            let value = f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            // Float scalars carry real values; do NOT normalize (matches Float32).
            // The previous `/ 0.0` produced Infinity for every voxel.
            value as f32
        }
        RawVoxelScalarType::Float32_4 => {
            // 4-channel float: use the 4th channel (offset 12) as the scalar value,
            // unnormalized like Float32. The previous `/ 0.0` produced Infinity.
            f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]])
        }
    }
}
