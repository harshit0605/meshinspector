use super::*;

pub(crate) fn meshlib_compact_bitset_indices(value: Option<&Value>) -> Result<Vec<usize>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.as_object().is_some_and(Map::is_empty) {
        return Ok(Vec::new());
    }
    let Some(size) = value
        .get("size")
        .and_then(Value::as_u64)
        .and_then(|size| usize::try_from(size).ok())
    else {
        return Ok(Vec::new());
    };
    let Some(bits) = value.get("bits").and_then(Value::as_str) else {
        return Ok(Vec::new());
    };
    let bytes = STANDARD
        .decode(bits)
        .map_err(|error| format!("Invalid MeshLib compact BitSet payload: {error}"))?;
    let mut selected = Vec::new();
    for index in 0..size {
        let Some(byte) = bytes.get(index / 8) else {
            continue;
        };
        if byte & (1u8 << (index % 8)) != 0 {
            selected.push(index);
        }
    }
    Ok(selected)
}

pub(crate) fn meshlib_compact_bitset_value(selected_indices: &[usize], size: usize) -> Value {
    let block_count = (size + 63) / 64;
    let mut bytes = vec![0u8; block_count * std::mem::size_of::<u64>()];
    for index in selected_indices
        .iter()
        .copied()
        .filter(|index| *index < size)
    {
        bytes[index / 8] |= 1u8 << (index % 8);
    }
    json!({
        "size": size,
        "bits": STANDARD.encode(bytes),
    })
}
