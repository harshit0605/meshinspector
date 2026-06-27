from __future__ import annotations

import numpy as np


def synthetic_openvdb_single_dense_leaf(
    values: list[float],
    *,
    leaf_origin: tuple[int, int, int] = (0, 0, 0),
    file_bbox_min: tuple[int, int, int] | None = None,
    file_bbox_max: tuple[int, int, int] | None = None,
    active_offsets: list[int] | None = None,
    buffer_offsets: list[int] | None = None,
    root_background: float = 1000.0,
    active_mask_compression: bool = False,
) -> bytes:
    """Build a minimal uncompressed OpenVDB FloatGrid with one active 8x8x8 leaf."""

    if len(values) != 512:
        raise ValueError("values must contain exactly 512 x-fastest FloatGrid samples")
    if any(origin < 0 or origin > 120 or origin % 8 != 0 for origin in leaf_origin):
        raise ValueError("leaf_origin must contain 8-voxel-aligned offsets in [0, 120]")
    topology_offsets = list(range(512)) if active_offsets is None else active_offsets
    value_offsets = list(range(512)) if buffer_offsets is None else buffer_offsets
    if any(offset < 0 or offset >= 512 for offset in topology_offsets + value_offsets):
        raise ValueError("OpenVDB leaf offsets must be in [0, 511]")
    bbox_min = leaf_origin if file_bbox_min is None else file_bbox_min
    bbox_max = tuple(origin + 7 for origin in leaf_origin) if file_bbox_max is None else file_bbox_max
    if any(max_value < min_value for min_value, max_value in zip(bbox_min, bbox_max, strict=True)):
        raise ValueError("file_bbox_max must be greater than or equal to file_bbox_min")

    def push_u8(buffer: bytearray, value: int) -> None:
        buffer.append(value)

    def push_u32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=False))

    def push_u64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=False))

    def push_i32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=True))

    def push_i64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=True))

    def push_f32(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float32(value).tobytes())

    def push_f64(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float64(value).tobytes())

    def push_string(buffer: bytearray, value: str) -> None:
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_string(buffer: bytearray, name: str, value: str) -> None:
        push_string(buffer, name)
        push_string(buffer, "string")
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_i64(buffer: bytearray, name: str, value: int) -> None:
        push_string(buffer, name)
        push_string(buffer, "int64")
        push_u32(buffer, 8)
        push_i64(buffer, value)

    def push_metadata_vec3i(buffer: bytearray, name: str, vector: tuple[int, int, int]) -> None:
        push_string(buffer, name)
        push_string(buffer, "vec3i")
        push_u32(buffer, 12)
        for value in vector:
            push_i32(buffer, value)

    def push_dvec3(buffer: bytearray, vector: tuple[float, float, float]) -> None:
        for value in vector:
            push_f64(buffer, value)

    def push_node_mask(buffer: bytearray, log2_dim: int, enabled_offsets: list[int]) -> None:
        bit_count = 1 << (3 * log2_dim)
        mask = bytearray(bit_count // 8)
        for offset in enabled_offsets:
            mask[offset // 8] |= 1 << (offset % 8)
        buffer.extend(mask)

    def push_uncompressed_float_values(buffer: bytearray, count: int, value: float) -> None:
        push_u8(buffer, 6)
        for _ in range(count):
            push_f32(buffer, value)

    def push_active_mask_values_header(buffer: bytearray) -> None:
        push_u8(buffer, 0)

    grid = bytearray()
    push_u32(grid, 2 if active_mask_compression else 0)
    push_u32(grid, 5)
    push_metadata_vec3i(grid, "file_bbox_min", bbox_min)
    push_metadata_vec3i(grid, "file_bbox_max", bbox_max)
    push_metadata_i64(grid, "file_voxel_count", len(topology_offsets))
    push_metadata_string(grid, "value_type", "float")
    push_metadata_string(grid, "class", "level set")
    push_string(grid, "UniformScaleMap")
    push_dvec3(grid, (0.5, 0.5, 0.5))
    push_dvec3(grid, (0.5, 0.5, 0.5))
    push_dvec3(grid, (2.0, 2.0, 2.0))
    push_dvec3(grid, (4.0, 4.0, 4.0))
    push_dvec3(grid, (1.0, 1.0, 1.0))

    push_f32(grid, root_background)
    push_u32(grid, 0)
    push_u32(grid, 1)
    push_i32(grid, 0)
    push_i32(grid, 0)
    push_i32(grid, 0)
    push_node_mask(grid, 5, [0])
    push_node_mask(grid, 5, [])
    if active_mask_compression:
        push_active_mask_values_header(grid)
    else:
        push_uncompressed_float_values(grid, 1 << 15, root_background)
    second_level_leaf_offset = (
        (leaf_origin[0] // 8) * 256
        + (leaf_origin[1] // 8) * 16
        + (leaf_origin[2] // 8)
    )
    push_node_mask(grid, 4, [second_level_leaf_offset])
    push_node_mask(grid, 4, [])
    if active_mask_compression:
        push_active_mask_values_header(grid)
    else:
        push_uncompressed_float_values(grid, 1 << 12, root_background)
    push_node_mask(grid, 3, topology_offsets)

    push_node_mask(grid, 3, value_offsets)
    if active_mask_compression:
        push_active_mask_values_header(grid)
        for offset in value_offsets:
            local_x = offset >> 6
            local_y = (offset & 63) >> 3
            local_z = offset & 7
            dense_index = local_x + local_y * 8 + local_z * 64
            push_f32(grid, values[dense_index])
    else:
        push_u8(grid, 6)
        for x in range(8):
            for y in range(8):
                for z in range(8):
                    dense_index = x + y * 8 + z * 64
                    push_f32(grid, values[dense_index])

    payload = bytearray(b"\x20\x42\x44\x56\x00\x00\x00\x00")
    push_u32(payload, 223)
    push_u32(payload, 12)
    push_u32(payload, 0)
    push_u8(payload, 1)
    payload.extend(b"00000000-0000-0000-0000-000000000000")
    push_u32(payload, 0)
    push_u32(payload, 1)
    push_string(payload, "dense_leaf")
    push_string(payload, "Tree_float_5_4_3")
    push_string(payload, "")
    grid_pos_offset = len(payload)
    push_u64(payload, 0)
    block_pos_offset = len(payload)
    push_u64(payload, 0)
    end_pos_offset = len(payload)
    push_u64(payload, 0)
    grid_pos = len(payload)
    payload.extend(grid)
    end_pos = len(payload)
    payload[grid_pos_offset : grid_pos_offset + 8] = grid_pos.to_bytes(8, "little")
    payload[block_pos_offset : block_pos_offset + 8] = end_pos.to_bytes(8, "little")
    payload[end_pos_offset : end_pos_offset + 8] = end_pos.to_bytes(8, "little")
    return bytes(payload)
