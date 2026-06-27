use super::export_validation::meshlib_voxel_stats;
use super::import_public::ParsedVoxelModel;
use crate::RawVoxelScalarType;
use serde_json::Value;

pub(super) fn parse_meshlib_gav_voxel_model(
    model_bytes: &[u8],
) -> Result<ParsedVoxelModel, String> {
    let header_len_bytes = model_bytes
        .get(0..4)
        .ok_or_else(|| "Gav-header size read error".to_string())?;
    let header_len = u32::from_le_bytes([
        header_len_bytes[0],
        header_len_bytes[1],
        header_len_bytes[2],
        header_len_bytes[3],
    ]) as usize;
    let header_end = 4usize
        .checked_add(header_len)
        .ok_or_else(|| "Gav-header size overflows".to_string())?;
    let header_bytes = model_bytes
        .get(4..header_end)
        .ok_or_else(|| "Gav-header read error".to_string())?;
    let header_json: Value = serde_json::from_slice(header_bytes)
        .map_err(|error| format!("Gav-header parse error: {error}"))?;

    let value_type = header_json
        .get("ValueType")
        .and_then(Value::as_str)
        .ok_or_else(|| "Gav-header misses ValueType".to_string())?;
    let scalar_type = meshlib_gav_scalar_type(value_type)?;
    let dimensions = meshlib_gav_dimensions(header_json.get("Dimensions"))?;
    let voxel_size = meshlib_gav_voxel_size(header_json.get("VoxelSize"))?;
    if header_json
        .get("Compression")
        .and_then(Value::as_str)
        .is_some()
    {
        return Err("Compressed Gav-files are not supported".to_string());
    }

    let values =
        meshlib_raw_voxel_values(&model_bytes[header_end..], dimensions, scalar_type, ".gav")?;
    let (min_value, max_value) = meshlib_voxel_stats(&values);
    Ok(ParsedVoxelModel {
        dimensions,
        voxel_size,
        origin: [0, 0, 0],
        grid_level_set: false,
        active_mask_compressed: false,
        background_value: max_value,
        values,
        min_value,
        max_value,
    })
}

fn meshlib_gav_scalar_type(value_type: &str) -> Result<RawVoxelScalarType, String> {
    match value_type {
        "UChar" => Ok(RawVoxelScalarType::UInt8),
        "UInt16" => Ok(RawVoxelScalarType::UInt16),
        "UInt32" => Ok(RawVoxelScalarType::UInt32),
        "Char" => Ok(RawVoxelScalarType::Int8),
        "Int16" => Ok(RawVoxelScalarType::Int16),
        "Int32" => Ok(RawVoxelScalarType::Int32),
        "Float" => Ok(RawVoxelScalarType::Float32),
        _ => Err(format!(
            "Gav-header ValueType has unknown value: {value_type}"
        )),
    }
}

fn meshlib_gav_dimensions(value: Option<&Value>) -> Result<[usize; 3], String> {
    let Some(value) = value else {
        return Err("Gav-header misses Dimensions".to_string());
    };
    if !value.is_object() {
        return Err("Gav-header misses Dimensions".to_string());
    }
    let mut dimensions = [0usize; 3];
    for (axis, key) in ["X", "Y", "Z"].iter().enumerate() {
        let Some(raw) = value.get(*key).and_then(Value::as_i64) else {
            return Err("Gav-header misses Dimensions".to_string());
        };
        dimensions[axis] =
            usize::try_from(raw).map_err(|_| "Gav-header misses Dimensions".to_string())?;
    }
    Ok(dimensions)
}

fn meshlib_gav_voxel_size(value: Option<&Value>) -> Result<[f32; 3], String> {
    let Some(value) = value else {
        return Err("Gav-header misses VoxelSize".to_string());
    };
    if !value.is_object() {
        return Err("Gav-header misses VoxelSize".to_string());
    }
    let mut voxel_size = [0.0f32; 3];
    for (axis, key) in ["X", "Y", "Z"].iter().enumerate() {
        let Some(raw) = value.get(*key).and_then(Value::as_f64) else {
            return Err("Gav-header misses VoxelSize".to_string());
        };
        voxel_size[axis] = raw as f32;
    }
    Ok(voxel_size)
}

fn meshlib_raw_voxel_values(
    model_bytes: &[u8],
    dimensions: [usize; 3],
    scalar_type: RawVoxelScalarType,
    extension: &str,
) -> Result<Vec<f32>, String> {
    if dimensions.iter().any(|dimension| *dimension == 0) {
        return Err("MRU ObjectVoxels dimensions must be positive".to_string());
    }
    let value_count = dimensions
        .iter()
        .try_fold(1usize, |product, dimension| product.checked_mul(*dimension))
        .ok_or_else(|| "MRU ObjectVoxels dimensions overflow".to_string())?;
    let unit_size = meshlib_raw_voxel_scalar_size(scalar_type);
    let expected_len = value_count
        .checked_mul(unit_size)
        .ok_or_else(|| "MRU ObjectVoxels value byte count overflows".to_string())?;
    if model_bytes.len() != expected_len {
        return Err(format!(
            "MRU ObjectVoxels {extension} payload length mismatch: expected {expected_len} bytes, got {}",
            model_bytes.len()
        ));
    }
    let mut values = Vec::with_capacity(value_count);
    for chunk in model_bytes.chunks_exact(unit_size) {
        values.push(meshlib_raw_voxel_scalar_value(scalar_type, chunk));
    }
    Ok(values)
}

fn meshlib_raw_voxel_scalar_size(scalar_type: RawVoxelScalarType) -> usize {
    match scalar_type {
        RawVoxelScalarType::UInt8 | RawVoxelScalarType::Int8 => 1,
        RawVoxelScalarType::UInt16 | RawVoxelScalarType::Int16 => 2,
        RawVoxelScalarType::UInt32 | RawVoxelScalarType::Int32 | RawVoxelScalarType::Float32 => 4,
        RawVoxelScalarType::UInt64 | RawVoxelScalarType::Int64 | RawVoxelScalarType::Float64 => 8,
        RawVoxelScalarType::Float32_4 => 16,
    }
}

fn meshlib_raw_voxel_scalar_value(scalar_type: RawVoxelScalarType, bytes: &[u8]) -> f32 {
    match scalar_type {
        RawVoxelScalarType::UInt8 => bytes[0] as f32 / u8::MAX as f32,
        RawVoxelScalarType::Int8 => {
            let value = bytes[0] as i8;
            ((value as f64 - i8::MIN as f64) / (i8::MAX as f64 - i8::MIN as f64)) as f32
        }
        RawVoxelScalarType::UInt16 => {
            let value = u16::from_le_bytes([bytes[0], bytes[1]]);
            value as f32 / u16::MAX as f32
        }
        RawVoxelScalarType::Int16 => {
            let value = i16::from_le_bytes([bytes[0], bytes[1]]);
            ((value as f64 - i16::MIN as f64) / (i16::MAX as f64 - i16::MIN as f64)) as f32
        }
        RawVoxelScalarType::UInt32 => {
            let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            value as f32 / u32::MAX as f32
        }
        RawVoxelScalarType::Int32 => {
            let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            ((value as f64 - i32::MIN as f64) / (i32::MAX as f64 - i32::MIN as f64)) as f32
        }
        RawVoxelScalarType::Float32 => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
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
            ((value as f64 - i64::MIN as f64) / (i64::MAX as f64 - i64::MIN as f64)) as f32
        }
        RawVoxelScalarType::Float64 => {
            let value = f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            value as f32
        }
        RawVoxelScalarType::Float32_4 => {
            f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]])
        }
    }
}
