#[derive(Clone, Copy)]
struct OpenVdbReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

#[derive(Debug, Clone)]
struct OpenVdbNodeMask {
    bit_count: usize,
    bytes: Vec<u8>,
}

impl OpenVdbNodeMask {
    fn empty(bit_count: usize) -> Self {
        let byte_count = bit_count.div_ceil(8);
        Self {
            bit_count,
            bytes: vec![0; byte_count],
        }
    }

    fn is_on(&self, offset: usize) -> bool {
        if offset >= self.bit_count {
            return false;
        }
        self.bytes[offset / 8] & (1_u8 << (offset % 8)) != 0
    }

    fn count_on(&self) -> usize {
        self.bytes
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum()
    }

    fn on_offsets(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.bit_count).filter(|offset| self.is_on(*offset))
    }
}

impl<'a> OpenVdbReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn seek(&mut self, offset: usize, context: &str) -> Result<(), String> {
        if offset > self.bytes.len() {
            return Err(format!("{context} points outside the OpenVDB payload"));
        }
        self.offset = offset;
        Ok(())
    }

    fn read_magic(&mut self) -> Result<(), String> {
        let bytes = self.read_exact::<8>("OpenVDB magic")?;
        if bytes != OPENVDB_MAGIC_BYTES {
            return Err("OpenVDB magic bytes mismatched".to_string());
        }
        Ok(())
    }

    fn read_grid_descriptor(&mut self, file_version: u32) -> Result<OpenVdbGridDescriptor, String> {
        let name = self.read_name("OpenVDB grid name")?;
        let grid_type = self.read_name("OpenVDB grid type")?;
        if file_version >= OPENVDB_FILE_VERSION_GRID_INSTANCING {
            let _instance_parent = self.read_name("OpenVDB grid instance parent")?;
        } else {
            return Err(format!(
                "Unsupported OpenVDB file version {file_version}: grid instancing is not available"
            ));
        }
        let grid_pos = usize::try_from(self.read_u64("OpenVDB grid offset")?)
            .map_err(|_| "OpenVDB grid offset is too large".to_string())?;
        let _block_pos = self.read_u64("OpenVDB grid block offset")?;
        let end_pos = usize::try_from(self.read_u64("OpenVDB grid end offset")?)
            .map_err(|_| "OpenVDB grid end offset is too large".to_string())?;
        if grid_pos > self.bytes.len() || end_pos > self.bytes.len() || grid_pos > end_pos {
            return Err(format!("OpenVDB grid {name} has invalid grid offsets"));
        }
        Ok(OpenVdbGridDescriptor {
            name,
            grid_type,
            grid_pos,
            end_pos,
        })
    }

    fn read_metadata(&mut self, context: &str) -> Result<OpenVdbMetadata, String> {
        let entry_count = self.read_u32(context)?;
        let mut metadata = OpenVdbMetadata::default();
        for _ in 0..entry_count {
            let name = self.read_name("OpenVDB metadata name")?;
            let data_type = self.read_name("OpenVDB metadata type")?;
            let len = usize::try_from(self.read_u32("OpenVDB metadata byte length")?)
                .map_err(|_| "OpenVDB metadata byte length is too large".to_string())?;
            match data_type.as_str() {
                "string" => {
                    let value = self.read_string(len, "OpenVDB string metadata")?;
                    match name.as_str() {
                        "class" => metadata.class_name = Some(value),
                        "value_type" => metadata.value_type = Some(value),
                        _ => {}
                    }
                }
                "vec3i" => {
                    if len < 12 {
                        return Err(format!("OpenVDB vec3i metadata {name} is truncated"));
                    }
                    let value = [
                        self.read_i32(&name)?,
                        self.read_i32(&name)?,
                        self.read_i32(&name)?,
                    ];
                    if len > 12 {
                        self.skip(len - 12, "OpenVDB vec3i metadata padding")?;
                    }
                    match name.as_str() {
                        "file_bbox_min" => metadata.file_bbox_min = Some(value),
                        "file_bbox_max" => metadata.file_bbox_max = Some(value),
                        _ => {}
                    }
                }
                "bool" | "int32" | "int64" | "float" => {
                    self.skip(len, "OpenVDB scalar metadata")?;
                }
                _ => {
                    self.skip(len, "OpenVDB unknown metadata")?;
                }
            }
        }
        Ok(metadata)
    }

    fn read_transform_voxel_size(&mut self) -> Result<[f32; 3], String> {
        let transform_name = self.read_name("OpenVDB transform name")?;
        let voxel_size = match transform_name.as_str() {
            "UniformScaleMap" => {
                let _scale_values = self.read_dvec3("OpenVDB transform scale values")?;
                let voxel_size = self.read_dvec3("OpenVDB transform voxel size")?;
                let _scale_values_inverse =
                    self.read_dvec3("OpenVDB transform inverse scale values")?;
                let _inv_scale_sqr = self.read_dvec3("OpenVDB transform inverse scale square")?;
                let _inv_twice_scale = self.read_dvec3("OpenVDB transform inverse twice scale")?;
                voxel_size
            }
            "UniformScaleTranslateMap" | "ScaleTranslateMap" => {
                let _translation = self.read_dvec3("OpenVDB transform translation")?;
                let _scale_values = self.read_dvec3("OpenVDB transform scale values")?;
                let voxel_size = self.read_dvec3("OpenVDB transform voxel size")?;
                let _scale_values_inverse =
                    self.read_dvec3("OpenVDB transform inverse scale values")?;
                let _inv_scale_sqr = self.read_dvec3("OpenVDB transform inverse scale square")?;
                let _inv_twice_scale = self.read_dvec3("OpenVDB transform inverse twice scale")?;
                voxel_size
            }
            _ => {
                return Err(format!(
                    "Unsupported OpenVDB transform map: {transform_name}"
                ));
            }
        };
        let mut result = [0.0f32; 3];
        for axis in 0..3 {
            if !voxel_size[axis].is_finite() || voxel_size[axis] <= 0.0 {
                return Err("OpenVDB transform voxel size must be positive".to_string());
            }
            result[axis] = voxel_size[axis] as f32;
            if !result[axis].is_finite() || result[axis] <= 0.0 {
                return Err("OpenVDB transform voxel size is outside f32 range".to_string());
            }
        }
        Ok(result)
    }

    fn read_name(&mut self, context: &str) -> Result<String, String> {
        let len = usize::try_from(self.read_u32(context)?)
            .map_err(|_| format!("{context} length is too large"))?;
        self.read_string(len, context)
    }

    fn read_string(&mut self, len: usize, context: &str) -> Result<String, String> {
        let bytes = self.read_slice(len, context)?;
        std::str::from_utf8(bytes)
            .map(str::to_string)
            .map_err(|error| format!("{context} is not UTF-8: {error}"))
    }

    fn read_fixed_string(&mut self, len: usize, context: &str) -> Result<String, String> {
        self.read_string(len, context)
    }

    fn read_dvec3(&mut self, context: &str) -> Result<[f64; 3], String> {
        Ok([
            self.read_f64(context)?,
            self.read_f64(context)?,
            self.read_f64(context)?,
        ])
    }

    fn read_coord(&mut self, context: &str) -> Result<[i32; 3], String> {
        Ok([
            self.read_i32(context)?,
            self.read_i32(context)?,
            self.read_i32(context)?,
        ])
    }

    fn read_node_mask(
        &mut self,
        log2_dim: usize,
        context: &str,
    ) -> Result<OpenVdbNodeMask, String> {
        let bit_count = 1usize
            .checked_shl(
                u32::try_from(3 * log2_dim)
                    .map_err(|_| format!("{context} bit count is too large"))?,
            )
            .ok_or_else(|| format!("{context} bit count is too large"))?;
        self.read_node_mask_bits(bit_count, context)
    }

    fn read_node_mask_bits(
        &mut self,
        bit_count: usize,
        context: &str,
    ) -> Result<OpenVdbNodeMask, String> {
        let byte_count = bit_count.div_ceil(8);
        let bytes = self.read_slice(byte_count, context)?.to_vec();
        Ok(OpenVdbNodeMask { bit_count, bytes })
    }

    fn read_openvdb_float_values(
        &mut self,
        count: usize,
        value_mask: &OpenVdbNodeMask,
        background: f32,
        from_half: bool,
        compression: u32,
        context: &str,
    ) -> Result<Vec<f32>, String> {
        let compression_metadata = self.read_u8(context)?;
        if !matches!(
            compression_metadata,
            OPENVDB_NO_MASK_OR_INACTIVE_VALS
                | OPENVDB_NO_MASK_AND_MINUS_BG
                | OPENVDB_NO_MASK_AND_ONE_INACTIVE_VAL
                | OPENVDB_MASK_AND_NO_INACTIVE_VALS
                | OPENVDB_MASK_AND_ONE_INACTIVE_VAL
                | OPENVDB_MASK_AND_TWO_INACTIVE_VALS
                | OPENVDB_NO_MASK_AND_ALL_VALS
        ) {
            return Err(format!(
                "{context} uses unsupported OpenVDB active-mask value compression metadata {compression_metadata}"
            ));
        }

        let mut inactive_value0 = match compression_metadata {
            OPENVDB_NO_MASK_AND_MINUS_BG => -background,
            _ => background,
        };
        let mut inactive_value1 = background;
        if matches!(
            compression_metadata,
            OPENVDB_NO_MASK_AND_ONE_INACTIVE_VAL
                | OPENVDB_MASK_AND_ONE_INACTIVE_VAL
                | OPENVDB_MASK_AND_TWO_INACTIVE_VALS
        ) {
            inactive_value0 = self.read_f32(context)?;
        }
        if compression_metadata == OPENVDB_MASK_AND_TWO_INACTIVE_VALS {
            inactive_value1 = self.read_f32(context)?;
        }
        let selection_mask = if matches!(
            compression_metadata,
            OPENVDB_MASK_AND_NO_INACTIVE_VALS
                | OPENVDB_MASK_AND_ONE_INACTIVE_VAL
                | OPENVDB_MASK_AND_TWO_INACTIVE_VALS
        ) {
            self.read_node_mask_bits(count, context)?
        } else {
            OpenVdbNodeMask::empty(count)
        };

        let mask_compressed = compression & OPENVDB_COMPRESS_ACTIVE_MASK != 0;
        let temporary_count =
            if mask_compressed && compression_metadata != OPENVDB_NO_MASK_AND_ALL_VALS {
                value_mask.count_on()
            } else {
                count
            };
        let value_byte_size = if from_half { 2usize } else { 4usize };
        let value_byte_count = temporary_count
            .checked_mul(value_byte_size)
            .ok_or_else(|| format!("{context} byte count overflows"))?;
        let value_bytes = self.read_openvdb_value_bytes(value_byte_count, compression, context)?;
        let mut value_reader = OpenVdbReader::new(&value_bytes);
        let mut temporary_values = Vec::with_capacity(temporary_count);
        for _ in 0..temporary_count {
            temporary_values.push(if from_half {
                value_reader.read_f16(context)?
            } else {
                value_reader.read_f32(context)?
            });
        }
        if value_reader.offset != value_bytes.len() {
            return Err(format!("{context} value buffer has trailing bytes"));
        }

        if !mask_compressed || temporary_count == count {
            return Ok(temporary_values);
        }

        let mut values = Vec::with_capacity(count);
        let mut active_index = 0usize;
        for offset in 0..count {
            if value_mask.is_on(offset) {
                let value = temporary_values
                    .get(active_index)
                    .copied()
                    .ok_or_else(|| format!("{context} active values are truncated"))?;
                values.push(value);
                active_index += 1;
            } else if selection_mask.is_on(offset) {
                values.push(inactive_value1);
            } else {
                values.push(inactive_value0);
            }
        }
        if active_index != temporary_values.len() {
            return Err(format!(
                "{context} active value count does not match value mask"
            ));
        }
        Ok(values)
    }

    fn read_openvdb_value_bytes(
        &mut self,
        byte_count: usize,
        compression: u32,
        context: &str,
    ) -> Result<Vec<u8>, String> {
        if compression & OPENVDB_COMPRESS_BLOSC != 0 {
            return self.read_openvdb_blosc_bytes(byte_count, context);
        }
        if compression & OPENVDB_COMPRESS_ZIP != 0 {
            return self.read_openvdb_zip_bytes(byte_count, context);
        }
        Ok(self.read_slice(byte_count, context)?.to_vec())
    }

    fn read_openvdb_zip_bytes(
        &mut self,
        byte_count: usize,
        context: &str,
    ) -> Result<Vec<u8>, String> {
        let chunk_size = self.read_i64(&format!("{context} zip chunk byte count"))?;
        if chunk_size <= 0 {
            let stored_len =
                usize::try_from(chunk_size.checked_neg().ok_or_else(|| {
                    format!("{context} zip chunk byte count cannot be represented")
                })?)
                .map_err(|_| format!("{context} zip chunk byte count is too large"))?;
            if stored_len != byte_count {
                return Err(format!(
                    "{context} expected {byte_count} uncompressed zip bytes, got {stored_len}"
                ));
            }
            return Ok(self.read_slice(byte_count, context)?.to_vec());
        }

        let compressed_len = usize::try_from(chunk_size)
            .map_err(|_| format!("{context} compressed zip chunk is too large"))?;
        let compressed_bytes = self.read_slice(compressed_len, context)?;
        let mut decoder = flate2::read::ZlibDecoder::new(compressed_bytes);
        let mut decompressed = Vec::with_capacity(byte_count);
        std::io::Read::read_to_end(&mut decoder, &mut decompressed)
            .map_err(|error| format!("{context} zlib decompression failed: {error}"))?;
        if decompressed.len() != byte_count {
            return Err(format!(
                "{context} expected {byte_count} decompressed zip bytes, got {}",
                decompressed.len()
            ));
        }
        Ok(decompressed)
    }

    fn read_openvdb_blosc_bytes(
        &mut self,
        byte_count: usize,
        context: &str,
    ) -> Result<Vec<u8>, String> {
        let chunk_size = self.read_i64(&format!("{context} blosc chunk byte count"))?;
        if chunk_size <= 0 {
            let stored_len = usize::try_from(chunk_size.checked_neg().ok_or_else(|| {
                format!("{context} blosc chunk byte count cannot be represented")
            })?)
            .map_err(|_| format!("{context} blosc chunk byte count is too large"))?;
            if stored_len != byte_count {
                return Err(format!(
                    "{context} expected {byte_count} uncompressed blosc bytes, got {stored_len}"
                ));
            }
            return Ok(self.read_slice(byte_count, context)?.to_vec());
        }

        let compressed_len = usize::try_from(chunk_size)
            .map_err(|_| format!("{context} compressed blosc chunk is too large"))?;
        let compressed_bytes = self.read_slice(compressed_len, context)?;
        OPENVDB_BLOSC_INIT.call_once(|| unsafe {
            blosc_src::blosc_init();
        });
        let mut decompressed = vec![0_u8; byte_count];
        let decompressed_len = unsafe {
            blosc_src::blosc_decompress_ctx(
                compressed_bytes.as_ptr().cast(),
                decompressed.as_mut_ptr().cast(),
                decompressed.len(),
                1,
            )
        };
        if decompressed_len < 1 {
            return Err(format!(
                "{context} blosc decompression failed with status {decompressed_len}"
            ));
        }
        let decompressed_len = usize::try_from(decompressed_len)
            .map_err(|_| format!("{context} blosc decompressed byte count is invalid"))?;
        if decompressed_len != byte_count {
            return Err(format!(
                "{context} expected {byte_count} decompressed blosc bytes, got {decompressed_len}"
            ));
        }
        Ok(decompressed)
    }

    fn read_u8(&mut self, context: &str) -> Result<u8, String> {
        Ok(self.read_exact::<1>(context)?[0])
    }

    fn read_bool(&mut self, context: &str) -> Result<bool, String> {
        Ok(self.read_u8(context)? != 0)
    }

    fn read_u32(&mut self, context: &str) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.read_exact::<4>(context)?))
    }

    fn read_u16(&mut self, context: &str) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.read_exact::<2>(context)?))
    }

    fn read_u64(&mut self, context: &str) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.read_exact::<8>(context)?))
    }

    fn read_i32(&mut self, context: &str) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.read_exact::<4>(context)?))
    }

    fn read_i64(&mut self, context: &str) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.read_exact::<8>(context)?))
    }

    fn read_f32(&mut self, context: &str) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.read_exact::<4>(context)?))
    }

    fn read_f16(&mut self, context: &str) -> Result<f32, String> {
        Ok(openvdb_half_to_f32(self.read_u16(context)?))
    }

    fn read_f64(&mut self, context: &str) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.read_exact::<8>(context)?))
    }

    fn read_exact<const N: usize>(&mut self, context: &str) -> Result<[u8; N], String> {
        let slice = self.read_slice(N, context)?;
        let mut bytes = [0u8; N];
        bytes.copy_from_slice(slice);
        Ok(bytes)
    }

    fn read_slice(&mut self, len: usize, context: &str) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| format!("{context} length overflows"))?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| format!("{context} is truncated"))?;
        self.offset = end;
        Ok(slice)
    }

    fn skip(&mut self, len: usize, context: &str) -> Result<(), String> {
        let _ = self.read_slice(len, context)?;
        Ok(())
    }
}

fn openvdb_half_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exponent = ((bits >> 10) & 0x1f) as i32;
    let fraction = (bits & 0x03ff) as u32;
    if exponent == 0 {
        if fraction == 0 {
            return f32::from_bits(sign);
        }
        let mut normalized_fraction = fraction;
        let mut normalized_exponent = -14;
        while normalized_fraction & 0x0400 == 0 {
            normalized_fraction <<= 1;
            normalized_exponent -= 1;
        }
        normalized_fraction &= 0x03ff;
        return f32::from_bits(
            sign | (((normalized_exponent + 127) as u32) << 23) | (normalized_fraction << 13),
        );
    }
    if exponent == 31 {
        return f32::from_bits(sign | 0x7f80_0000 | (fraction << 13));
    }
    f32::from_bits(sign | (((exponent - 15 + 127) as u32) << 23) | (fraction << 13))
}
