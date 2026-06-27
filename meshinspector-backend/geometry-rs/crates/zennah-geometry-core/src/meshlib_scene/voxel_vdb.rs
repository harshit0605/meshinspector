use super::export_validation::meshlib_voxel_stats;
use super::import_public::ParsedVoxelModel;

const OPENVDB_MAGIC_BYTES: [u8; 8] = [0x20, 0x42, 0x44, 0x56, 0x00, 0x00, 0x00, 0x00];
const OPENVDB_MIN_SUPPORTED_VERSION: u32 = 213;
const OPENVDB_FILE_VERSION_GRID_INSTANCING: u32 = 216;
const OPENVDB_FILE_VERSION_BOOST_UUID: u32 = 218;
const OPENVDB_FILE_VERSION_SELECTIVE_COMPRESSION: u32 = 220;
const OPENVDB_FILE_VERSION_NODE_MASK_COMPRESSION: u32 = 222;
const OPENVDB_COMPRESS_NONE: u32 = 0;
const OPENVDB_COMPRESS_ZIP: u32 = 0x1;
const OPENVDB_COMPRESS_ACTIVE_MASK: u32 = 0x2;
const OPENVDB_COMPRESS_BLOSC: u32 = 0x4;
const OPENVDB_NO_MASK_OR_INACTIVE_VALS: u8 = 0;
const OPENVDB_NO_MASK_AND_MINUS_BG: u8 = 1;
const OPENVDB_NO_MASK_AND_ONE_INACTIVE_VAL: u8 = 2;
const OPENVDB_MASK_AND_NO_INACTIVE_VALS: u8 = 3;
const OPENVDB_MASK_AND_ONE_INACTIVE_VAL: u8 = 4;
const OPENVDB_MASK_AND_TWO_INACTIVE_VALS: u8 = 5;
const OPENVDB_NO_MASK_AND_ALL_VALS: u8 = 6;
const OPENVDB_FLOAT_TREE_DESCRIPTOR: &str = "Tree_float_5_4_3";
const OPENVDB_FLOAT_TREE_HALF_DESCRIPTOR: &str = "Tree_float_5_4_3_HalfFloat";
const OPENVDB_ROOT_INTERNAL_LOG2_DIM: usize = 5;
const OPENVDB_SECOND_INTERNAL_LOG2_DIM: usize = 4;
const OPENVDB_LEAF_LOG2_DIM: usize = 3;
static OPENVDB_BLOSC_INIT: std::sync::Once = std::sync::Once::new();

pub(super) fn parse_meshlib_vdb_voxel_model(
    model_bytes: &[u8],
    dimensions: [usize; 3],
    voxel_size: [f32; 3],
) -> Result<ParsedVoxelModel, String> {
    if model_bytes.is_empty() {
        return Err("MRU ObjectVoxels .vdb payload is empty".to_string());
    }
    if !has_openvdb_magic(model_bytes) {
        return parse_opaque_meshlib_vdb_voxel_model(dimensions, voxel_size);
    }

    let metadata = parse_openvdb_float_grid_metadata(model_bytes)?;
    Ok(ParsedVoxelModel {
        dimensions: metadata.dimensions,
        voxel_size: metadata.voxel_size,
        origin: metadata.origin,
        grid_level_set: metadata.grid_level_set,
        active_mask_compressed: metadata.active_mask_compressed,
        background_value: metadata.background_value,
        values: metadata.values,
        min_value: metadata.min_value,
        max_value: metadata.max_value,
    })
}

fn parse_opaque_meshlib_vdb_voxel_model(
    dimensions: [usize; 3],
    voxel_size: [f32; 3],
) -> Result<ParsedVoxelModel, String> {
    if dimensions.iter().any(|dimension| *dimension == 0) {
        return Err("MRU ObjectVoxels dimensions must be positive".to_string());
    }
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("MRU ObjectVoxels voxel size must be positive".to_string());
    }
    Ok(ParsedVoxelModel {
        dimensions,
        voxel_size,
        origin: [0, 0, 0],
        grid_level_set: false,
        active_mask_compressed: false,
        background_value: 0.0,
        values: Vec::new(),
        min_value: 0.0,
        max_value: 0.0,
    })
}

fn has_openvdb_magic(model_bytes: &[u8]) -> bool {
    model_bytes
        .get(..OPENVDB_MAGIC_BYTES.len())
        .is_some_and(|prefix| prefix == OPENVDB_MAGIC_BYTES)
}

#[derive(Debug, Clone)]
struct OpenVdbGridDescriptor {
    name: String,
    grid_type: String,
    grid_pos: usize,
    end_pos: usize,
}

#[derive(Debug, Clone, Default)]
struct OpenVdbMetadata {
    file_bbox_min: Option<[i32; 3]>,
    file_bbox_max: Option<[i32; 3]>,
    value_type: Option<String>,
    class_name: Option<String>,
}

#[derive(Debug, Clone)]
struct OpenVdbGridVoxelMetadata {
    dimensions: [usize; 3],
    voxel_size: [f32; 3],
    origin: [i32; 3],
    grid_level_set: bool,
    active_mask_compressed: bool,
    background_value: f32,
    values: Vec<f32>,
    min_value: f32,
    max_value: f32,
}

#[derive(Debug, Clone)]
struct OpenVdbDenseValues {
    dimensions: [usize; 3],
    origin: [i32; 3],
    background_value: f32,
    values: Vec<f32>,
}

fn parse_openvdb_float_grid_metadata(
    model_bytes: &[u8],
) -> Result<OpenVdbGridVoxelMetadata, String> {
    let mut reader = OpenVdbReader::new(model_bytes);
    reader.read_magic()?;
    let file_version = reader.read_u32("OpenVDB file version")?;
    if file_version < OPENVDB_MIN_SUPPORTED_VERSION {
        return Err(format!(
            "Unsupported OpenVDB file version {file_version}; minimum supported version is {OPENVDB_MIN_SUPPORTED_VERSION}"
        ));
    }
    let _library_version_major = reader.read_u32("OpenVDB library major version")?;
    let _library_version_minor = reader.read_u32("OpenVDB library minor version")?;
    if reader.read_u8("OpenVDB grid-offset flag")? != 1 {
        return Err("OpenVDB payload does not include grid offsets".to_string());
    }
    if (OPENVDB_FILE_VERSION_SELECTIVE_COMPRESSION..OPENVDB_FILE_VERSION_NODE_MASK_COMPRESSION)
        .contains(&file_version)
    {
        let _selective_compression = reader.read_u8("OpenVDB archive compression flag")?;
    }
    if file_version >= OPENVDB_FILE_VERSION_BOOST_UUID {
        let _guid = reader.read_fixed_string(36, "OpenVDB archive UUID")?;
    } else {
        return Err(format!(
            "Unsupported OpenVDB file version {file_version}: UUID byte-string archives are not supported"
        ));
    }
    let _archive_metadata = reader.read_metadata("OpenVDB archive metadata")?;
    let grid_count = reader.read_u32("OpenVDB grid count")?;
    if grid_count == 0 {
        return Err("OpenVDB payload contains no grids".to_string());
    }

    let mut first_non_float_grid: Option<String> = None;
    for _ in 0..grid_count {
        let descriptor = reader.read_grid_descriptor(file_version)?;
        let grid_metadata = parse_openvdb_grid_metadata(model_bytes, file_version, &descriptor)?;
        if openvdb_descriptor_is_float_grid(&descriptor, &grid_metadata.metadata) {
            let bbox_dimensions = openvdb_dimensions_from_bbox(&grid_metadata.metadata)?;
            let bbox_origin = grid_metadata
                .metadata
                .file_bbox_min
                .ok_or_else(|| "OpenVDB FloatGrid metadata misses file_bbox_min".to_string())?;
            let decoded_values = parse_openvdb_float_tree_values(
                model_bytes,
                &descriptor,
                &grid_metadata,
                bbox_dimensions,
            )?;
            let (dimensions, origin, background_value, values) =
                if let Some(decoded_values) = decoded_values {
                    (
                        decoded_values.dimensions,
                        decoded_values.origin,
                        decoded_values.background_value,
                        decoded_values.values,
                    )
                } else {
                    (bbox_dimensions, bbox_origin, 0.0, Vec::new())
                };
            let (min_value, max_value) = meshlib_voxel_stats(&values);
            return Ok(OpenVdbGridVoxelMetadata {
                dimensions,
                voxel_size: grid_metadata.voxel_size,
                origin,
                grid_level_set: grid_metadata
                    .metadata
                    .class_name
                    .as_deref()
                    .is_some_and(|class_name| class_name.eq_ignore_ascii_case("level set")),
                active_mask_compressed: grid_metadata.grid_compression
                    & OPENVDB_COMPRESS_ACTIVE_MASK
                    != 0,
                background_value,
                values,
                min_value,
                max_value,
            });
        }
        first_non_float_grid.get_or_insert(descriptor.name.clone());
        reader.seek(descriptor.end_pos, "OpenVDB grid end offset")?;
    }

    Err(format!(
        "Wrong OpenVDB grid type{}",
        first_non_float_grid
            .map(|name| format!(" for grid {name}"))
            .unwrap_or_default()
    ))
}

#[derive(Debug, Clone)]
struct ParsedOpenVdbGridMetadata {
    metadata: OpenVdbMetadata,
    voxel_size: [f32; 3],
    tree_offset: usize,
    grid_compression: u32,
}

fn parse_openvdb_grid_metadata(
    model_bytes: &[u8],
    file_version: u32,
    descriptor: &OpenVdbGridDescriptor,
) -> Result<ParsedOpenVdbGridMetadata, String> {
    let mut reader = OpenVdbReader::new(model_bytes);
    reader.seek(descriptor.grid_pos, "OpenVDB grid offset")?;
    let grid_compression = if file_version >= OPENVDB_FILE_VERSION_NODE_MASK_COMPRESSION {
        reader.read_u32("OpenVDB grid compression")?
    } else {
        OPENVDB_COMPRESS_NONE
    };
    let metadata = reader.read_metadata("OpenVDB grid metadata")?;
    if file_version < OPENVDB_FILE_VERSION_GRID_INSTANCING {
        return Err(format!(
            "Unsupported OpenVDB file version {file_version}: grid transform is not stored"
        ));
    }
    let voxel_size = reader.read_transform_voxel_size()?;
    Ok(ParsedOpenVdbGridMetadata {
        metadata,
        voxel_size,
        tree_offset: reader.offset,
        grid_compression,
    })
}

fn parse_openvdb_float_tree_values(
    model_bytes: &[u8],
    descriptor: &OpenVdbGridDescriptor,
    grid_metadata: &ParsedOpenVdbGridMetadata,
    dimensions: [usize; 3],
) -> Result<Option<OpenVdbDenseValues>, String> {
    if grid_metadata.tree_offset >= descriptor.end_pos {
        return Ok(None);
    }
    if !openvdb_descriptor_has_supported_float_tree_values(descriptor) {
        return Ok(None);
    }
    if grid_metadata.grid_compression
        & !(OPENVDB_COMPRESS_ACTIVE_MASK | OPENVDB_COMPRESS_ZIP | OPENVDB_COMPRESS_BLOSC)
        != OPENVDB_COMPRESS_NONE
    {
        return Ok(None);
    }
    let from_half = openvdb_descriptor_uses_half_float_buffers(descriptor);

    let min_corner = grid_metadata
        .metadata
        .file_bbox_min
        .ok_or_else(|| "OpenVDB FloatGrid metadata misses file_bbox_min".to_string())?;
    let expected_values = dimensions
        .iter()
        .try_fold(1usize, |acc, dimension| acc.checked_mul(*dimension))
        .ok_or_else(|| "OpenVDB FloatGrid dense dimensions are too large".to_string())?;

    let mut reader = OpenVdbReader::new(model_bytes);
    reader.seek(grid_metadata.tree_offset, "OpenVDB FloatTree offset")?;
    let root_background = reader.read_f32("OpenVDB FloatTree root background")?;
    let mut values = vec![root_background; expected_values];

    let root_tile_count = usize::try_from(reader.read_u32("OpenVDB FloatTree root tile count")?)
        .map_err(|_| "OpenVDB FloatTree root tile count is too large".to_string())?;
    let root_child_count = usize::try_from(reader.read_u32("OpenVDB FloatTree root child count")?)
        .map_err(|_| "OpenVDB FloatTree root child count is too large".to_string())?;
    for _ in 0..root_tile_count {
        let tile_origin = reader.read_coord("OpenVDB FloatTree root tile origin")?;
        let tile_value = reader.read_f32("OpenVDB FloatTree root tile value")?;
        let _tile_active = reader.read_bool("OpenVDB FloatTree root tile active state")?;
        fill_openvdb_dense_tile(
            &mut values,
            dimensions,
            min_corner,
            tile_origin,
            1 << 12,
            tile_value,
        );
    }

    let mut leaves = Vec::new();
    for _ in 0..root_child_count {
        let child_origin = reader.read_coord("OpenVDB FloatTree root child origin")?;
        read_openvdb_internal_topology(
            &mut reader,
            OPENVDB_ROOT_INTERNAL_LOG2_DIM,
            OPENVDB_SECOND_INTERNAL_LOG2_DIM + OPENVDB_LEAF_LOG2_DIM,
            child_origin,
            dimensions,
            min_corner,
            &mut values,
            &mut leaves,
            root_background,
            from_half,
            grid_metadata.grid_compression,
            "OpenVDB FloatTree root internal node",
        )?;
    }

    for leaf in leaves {
        let buffer_mask =
            reader.read_node_mask(OPENVDB_LEAF_LOG2_DIM, "OpenVDB FloatTree leaf buffer mask")?;
        let buffer_values = reader.read_openvdb_float_values(
            1 << (3 * OPENVDB_LEAF_LOG2_DIM),
            &buffer_mask,
            root_background,
            from_half,
            grid_metadata.grid_compression,
            "OpenVDB FloatTree leaf buffer values",
        )?;
        for (offset, value) in buffer_values.iter().enumerate() {
            let local = openvdb_node_offset_to_local_coord(offset, OPENVDB_LEAF_LOG2_DIM);
            let coord = [
                leaf.origin[0] + local[0],
                leaf.origin[1] + local[1],
                leaf.origin[2] + local[2],
            ];
            if let Some(index) = openvdb_dense_index(coord, min_corner, dimensions) {
                values[index] = *value;
            }
        }
    }

    if reader.offset > descriptor.end_pos {
        return Err("OpenVDB FloatTree dense buffers overrun grid payload".to_string());
    }
    Ok(Some(openvdb_pad_degenerate_dense_bbox(
        values,
        dimensions,
        min_corner,
        root_background,
    )?))
}

fn openvdb_pad_degenerate_dense_bbox(
    values: Vec<f32>,
    dimensions: [usize; 3],
    origin: [i32; 3],
    background: f32,
) -> Result<OpenVdbDenseValues, String> {
    let needs_background_halo = dimensions.iter().any(|dimension| *dimension < 2);
    if !needs_background_halo {
        return Ok(OpenVdbDenseValues {
            dimensions,
            origin,
            background_value: background,
            values,
        });
    }

    openvdb_pad_dense_values(values, dimensions, origin, background)
}

pub(super) fn pad_meshlib_vdb_voxels_for_meshing(
    parsed: &mut ParsedVoxelModel,
) -> Result<(), String> {
    let needs_background_halo =
        parsed.active_mask_compressed || parsed.dimensions.iter().any(|dimension| *dimension < 2);
    if !needs_background_halo {
        return Ok(());
    }
    let padded = openvdb_pad_dense_values(
        std::mem::take(&mut parsed.values),
        parsed.dimensions,
        parsed.origin,
        parsed.background_value,
    )?;
    parsed.dimensions = padded.dimensions;
    parsed.origin = padded.origin;
    parsed.values = padded.values;
    let (min_value, max_value) = meshlib_voxel_stats(&parsed.values);
    parsed.min_value = min_value;
    parsed.max_value = max_value;
    Ok(())
}

fn openvdb_pad_dense_values(
    values: Vec<f32>,
    dimensions: [usize; 3],
    origin: [i32; 3],
    background: f32,
) -> Result<OpenVdbDenseValues, String> {
    let padded_dimensions = [
        dimensions[0]
            .checked_add(2)
            .ok_or_else(|| "OpenVDB padded bbox X dimension overflows".to_string())?,
        dimensions[1]
            .checked_add(2)
            .ok_or_else(|| "OpenVDB padded bbox Y dimension overflows".to_string())?,
        dimensions[2]
            .checked_add(2)
            .ok_or_else(|| "OpenVDB padded bbox Z dimension overflows".to_string())?,
    ];
    let padded_origin = [
        origin[0]
            .checked_sub(1)
            .ok_or_else(|| "OpenVDB padded bbox X origin underflows".to_string())?,
        origin[1]
            .checked_sub(1)
            .ok_or_else(|| "OpenVDB padded bbox Y origin underflows".to_string())?,
        origin[2]
            .checked_sub(1)
            .ok_or_else(|| "OpenVDB padded bbox Z origin underflows".to_string())?,
    ];
    let padded_count = padded_dimensions
        .iter()
        .try_fold(1usize, |acc, dimension| acc.checked_mul(*dimension))
        .ok_or_else(|| "OpenVDB padded dense dimensions are too large".to_string())?;
    let mut padded_values = vec![background; padded_count];

    for z in 0..dimensions[2] {
        for y in 0..dimensions[1] {
            for x in 0..dimensions[0] {
                let source_index = x + y * dimensions[0] + z * dimensions[0] * dimensions[1];
                let target_x = x + 1;
                let target_y = y + 1;
                let target_z = z + 1;
                let target_index = target_x
                    + target_y * padded_dimensions[0]
                    + target_z * padded_dimensions[0] * padded_dimensions[1];
                padded_values[target_index] = values[source_index];
            }
        }
    }

    Ok(OpenVdbDenseValues {
        dimensions: padded_dimensions,
        origin: padded_origin,
        background_value: background,
        values: padded_values,
    })
}

fn openvdb_descriptor_has_supported_float_tree_values(descriptor: &OpenVdbGridDescriptor) -> bool {
    descriptor
        .grid_type
        .eq_ignore_ascii_case(OPENVDB_FLOAT_TREE_DESCRIPTOR)
        || descriptor
            .grid_type
            .eq_ignore_ascii_case(OPENVDB_FLOAT_TREE_HALF_DESCRIPTOR)
}

fn openvdb_descriptor_uses_half_float_buffers(descriptor: &OpenVdbGridDescriptor) -> bool {
    descriptor
        .grid_type
        .eq_ignore_ascii_case(OPENVDB_FLOAT_TREE_HALF_DESCRIPTOR)
}

#[derive(Debug, Clone)]
struct OpenVdbLeafTopology {
    origin: [i32; 3],
}

#[allow(clippy::too_many_arguments)]
fn read_openvdb_internal_topology(
    reader: &mut OpenVdbReader<'_>,
    log2_dim: usize,
    child_total: usize,
    origin: [i32; 3],
    dimensions: [usize; 3],
    min_corner: [i32; 3],
    dense_values: &mut [f32],
    leaves: &mut Vec<OpenVdbLeafTopology>,
    root_background: f32,
    from_half: bool,
    grid_compression: u32,
    context: &str,
) -> Result<(), String> {
    let child_mask = reader.read_node_mask(log2_dim, &format!("{context} child mask"))?;
    let value_mask = reader.read_node_mask(log2_dim, &format!("{context} value mask"))?;
    let tile_values = reader.read_openvdb_float_values(
        1 << (3 * log2_dim),
        &value_mask,
        root_background,
        from_half,
        grid_compression,
        &format!("{context} tile values"),
    )?;

    let child_dim = 1_i32
        .checked_shl(
            u32::try_from(child_total)
                .map_err(|_| "OpenVDB child level is too large".to_string())?,
        )
        .ok_or_else(|| "OpenVDB child dimension is too large".to_string())?;
    for offset in value_mask.on_offsets() {
        if child_mask.is_on(offset) {
            continue;
        }
        let child_origin = openvdb_internal_child_origin(origin, offset, log2_dim, child_total)?;
        fill_openvdb_dense_tile(
            dense_values,
            dimensions,
            min_corner,
            child_origin,
            child_dim,
            tile_values[offset],
        );
    }

    for offset in child_mask.on_offsets() {
        let child_origin = openvdb_internal_child_origin(origin, offset, log2_dim, child_total)?;
        if child_total == OPENVDB_LEAF_LOG2_DIM {
            let _leaf_value_mask = reader.read_node_mask(
                OPENVDB_LEAF_LOG2_DIM,
                "OpenVDB FloatTree leaf topology mask",
            )?;
            leaves.push(OpenVdbLeafTopology {
                origin: child_origin,
            });
        } else {
            read_openvdb_internal_topology(
                reader,
                OPENVDB_SECOND_INTERNAL_LOG2_DIM,
                OPENVDB_LEAF_LOG2_DIM,
                child_origin,
                dimensions,
                min_corner,
                dense_values,
                leaves,
                root_background,
                from_half,
                grid_compression,
                "OpenVDB FloatTree second internal node",
            )?;
        }
    }

    Ok(())
}

fn openvdb_internal_child_origin(
    origin: [i32; 3],
    offset: usize,
    log2_dim: usize,
    child_total: usize,
) -> Result<[i32; 3], String> {
    let local = openvdb_node_offset_to_local_coord(offset, log2_dim);
    let shift =
        u32::try_from(child_total).map_err(|_| "OpenVDB child level is too large".to_string())?;
    Ok([
        origin[0]
            .checked_add(
                local[0]
                    .checked_shl(shift)
                    .ok_or_else(|| "OpenVDB child origin is too large".to_string())?,
            )
            .ok_or_else(|| "OpenVDB child origin overflows".to_string())?,
        origin[1]
            .checked_add(
                local[1]
                    .checked_shl(shift)
                    .ok_or_else(|| "OpenVDB child origin is too large".to_string())?,
            )
            .ok_or_else(|| "OpenVDB child origin overflows".to_string())?,
        origin[2]
            .checked_add(
                local[2]
                    .checked_shl(shift)
                    .ok_or_else(|| "OpenVDB child origin is too large".to_string())?,
            )
            .ok_or_else(|| "OpenVDB child origin overflows".to_string())?,
    ])
}

fn openvdb_node_offset_to_local_coord(offset: usize, log2_dim: usize) -> [i32; 3] {
    let x = offset >> (2 * log2_dim);
    let y = (offset & ((1 << (2 * log2_dim)) - 1)) >> log2_dim;
    let z = offset & ((1 << log2_dim) - 1);
    [x as i32, y as i32, z as i32]
}

fn fill_openvdb_dense_tile(
    values: &mut [f32],
    dimensions: [usize; 3],
    min_corner: [i32; 3],
    tile_origin: [i32; 3],
    tile_dim: i32,
    value: f32,
) {
    if tile_dim <= 0 {
        return;
    }
    let max_corner = [
        min_corner[0] + dimensions[0] as i32 - 1,
        min_corner[1] + dimensions[1] as i32 - 1,
        min_corner[2] + dimensions[2] as i32 - 1,
    ];
    let tile_max = [
        tile_origin[0] + tile_dim - 1,
        tile_origin[1] + tile_dim - 1,
        tile_origin[2] + tile_dim - 1,
    ];
    let start = [
        tile_origin[0].max(min_corner[0]),
        tile_origin[1].max(min_corner[1]),
        tile_origin[2].max(min_corner[2]),
    ];
    let end = [
        tile_max[0].min(max_corner[0]),
        tile_max[1].min(max_corner[1]),
        tile_max[2].min(max_corner[2]),
    ];
    if start[0] > end[0] || start[1] > end[1] || start[2] > end[2] {
        return;
    }
    for z in start[2]..=end[2] {
        for y in start[1]..=end[1] {
            for x in start[0]..=end[0] {
                if let Some(index) = openvdb_dense_index([x, y, z], min_corner, dimensions) {
                    values[index] = value;
                }
            }
        }
    }
}

fn openvdb_dense_index(
    coord: [i32; 3],
    min_corner: [i32; 3],
    dimensions: [usize; 3],
) -> Option<usize> {
    let x = usize::try_from(coord[0].checked_sub(min_corner[0])?).ok()?;
    let y = usize::try_from(coord[1].checked_sub(min_corner[1])?).ok()?;
    let z = usize::try_from(coord[2].checked_sub(min_corner[2])?).ok()?;
    if x >= dimensions[0] || y >= dimensions[1] || z >= dimensions[2] {
        return None;
    }
    Some(x + y * dimensions[0] + z * dimensions[0] * dimensions[1])
}

fn openvdb_descriptor_is_float_grid(
    descriptor: &OpenVdbGridDescriptor,
    metadata: &OpenVdbMetadata,
) -> bool {
    if metadata
        .value_type
        .as_deref()
        .is_some_and(|value_type| value_type.eq_ignore_ascii_case("float"))
    {
        return true;
    }
    descriptor
        .grid_type
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|part| part == "float" || part == "halffloat")
}

fn openvdb_dimensions_from_bbox(metadata: &OpenVdbMetadata) -> Result<[usize; 3], String> {
    let min_corner = metadata
        .file_bbox_min
        .ok_or_else(|| "OpenVDB FloatGrid metadata misses file_bbox_min".to_string())?;
    let max_corner = metadata
        .file_bbox_max
        .ok_or_else(|| "OpenVDB FloatGrid metadata misses file_bbox_max".to_string())?;
    let mut dimensions = [0usize; 3];
    for axis in 0..3 {
        if max_corner[axis] < min_corner[axis] {
            return Err("OpenVDB FloatGrid active voxel bbox is invalid".to_string());
        }
        let span = i64::from(max_corner[axis]) - i64::from(min_corner[axis]) + 1;
        dimensions[axis] = usize::try_from(span)
            .map_err(|_| "OpenVDB FloatGrid active voxel bbox is too large".to_string())?;
    }
    if dimensions.iter().any(|dimension| *dimension == 0) {
        return Err("OpenVDB FloatGrid active voxel bbox is empty".to_string());
    }
    Ok(dimensions)
}

include!("voxel_vdb/reader.rs");
