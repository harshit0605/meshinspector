use super::{truncated_f64_to_u8, PlyBinaryEndian, PlyProperty, PlyScalarType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum PlyValue {
    Integer(i64),
    Unsigned(u64),
    Float(f64),
}

impl PlyValue {
    pub(super) fn as_f64(self) -> f64 {
        match self {
            Self::Integer(value) => value as f64,
            Self::Unsigned(value) => value as f64,
            Self::Float(value) => value,
        }
    }

    pub(super) fn as_meshlib_list_count(self, error: &str) -> Result<usize, String> {
        let count = match self {
            Self::Integer(value) if (i32::MIN as i64..=i32::MAX as i64).contains(&value) => {
                value as i32
            }
            Self::Unsigned(value) if value <= u32::MAX as u64 => value as u32 as i32,
            Self::Float(value)
                if value.is_finite() && value >= i32::MIN as f64 && value <= i32::MAX as f64 =>
            {
                value.trunc() as i32
            }
            _ => return Err(error.to_string()),
        };
        if count < 0 {
            return Err(error.to_string());
        }
        Ok(count as usize)
    }

    pub(super) fn as_i64(self, error: &str) -> Result<i64, String> {
        match self {
            Self::Integer(value) => Ok(value),
            Self::Unsigned(value) if value <= i64::MAX as u64 => Ok(value as i64),
            Self::Float(value) if value.is_finite() => Ok(value.trunc() as i64),
            _ => Err(error.to_string()),
        }
    }

    pub(super) fn as_u8(self, error: &str) -> Result<u8, String> {
        match self {
            Self::Integer(value) => Ok(value as u8),
            Self::Unsigned(value) => Ok(value as u8),
            Self::Float(value) if value.is_finite() => truncated_f64_to_u8(value, error),
            _ => Err(error.to_string()),
        }
    }
}

pub(super) struct PlyBinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    endian: PlyBinaryEndian,
}

impl<'a> PlyBinaryReader<'a> {
    pub(super) fn new(bytes: &'a [u8], endian: PlyBinaryEndian) -> Self {
        Self {
            bytes,
            offset: 0,
            endian,
        }
    }

    fn read_value(&mut self, ty: PlyScalarType, error: &str) -> Result<PlyValue, String> {
        Ok(match ty {
            PlyScalarType::Char => {
                PlyValue::Integer(i8::from_le_bytes(self.read_array(error)?) as i64)
            }
            PlyScalarType::UChar => {
                PlyValue::Unsigned(u8::from_le_bytes(self.read_array(error)?) as u64)
            }
            PlyScalarType::Short => PlyValue::Integer(self.read_i16(error)? as i64),
            PlyScalarType::UShort => PlyValue::Unsigned(self.read_u16(error)? as u64),
            PlyScalarType::Int => PlyValue::Integer(self.read_i32(error)? as i64),
            PlyScalarType::UInt => PlyValue::Unsigned(self.read_u32(error)? as u64),
            PlyScalarType::Float => PlyValue::Float(self.read_f32(error)? as f64),
            PlyScalarType::Double => PlyValue::Float(self.read_f64(error)?),
        })
    }

    fn read_i16(&mut self, error: &str) -> Result<i16, String> {
        let bytes = self.read_array(error)?;
        Ok(match self.endian {
            PlyBinaryEndian::Little => i16::from_le_bytes(bytes),
            PlyBinaryEndian::Big => i16::from_be_bytes(bytes),
        })
    }

    fn read_u16(&mut self, error: &str) -> Result<u16, String> {
        let bytes = self.read_array(error)?;
        Ok(match self.endian {
            PlyBinaryEndian::Little => u16::from_le_bytes(bytes),
            PlyBinaryEndian::Big => u16::from_be_bytes(bytes),
        })
    }

    fn read_i32(&mut self, error: &str) -> Result<i32, String> {
        let bytes = self.read_array(error)?;
        Ok(match self.endian {
            PlyBinaryEndian::Little => i32::from_le_bytes(bytes),
            PlyBinaryEndian::Big => i32::from_be_bytes(bytes),
        })
    }

    fn read_u32(&mut self, error: &str) -> Result<u32, String> {
        let bytes = self.read_array(error)?;
        Ok(match self.endian {
            PlyBinaryEndian::Little => u32::from_le_bytes(bytes),
            PlyBinaryEndian::Big => u32::from_be_bytes(bytes),
        })
    }

    fn read_f32(&mut self, error: &str) -> Result<f32, String> {
        let bytes = self.read_array(error)?;
        Ok(match self.endian {
            PlyBinaryEndian::Little => f32::from_le_bytes(bytes),
            PlyBinaryEndian::Big => f32::from_be_bytes(bytes),
        })
    }

    fn read_f64(&mut self, error: &str) -> Result<f64, String> {
        let bytes = self.read_array(error)?;
        Ok(match self.endian {
            PlyBinaryEndian::Little => f64::from_le_bytes(bytes),
            PlyBinaryEndian::Big => f64::from_be_bytes(bytes),
        })
    }

    fn read_array<const N: usize>(&mut self, error: &str) -> Result<[u8; N], String> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| error.to_string())?;
        if end > self.bytes.len() {
            return Err(error.to_string());
        }
        let output = self.bytes[self.offset..end].try_into().unwrap();
        self.offset = end;
        Ok(output)
    }

    fn skip_bytes(&mut self, count: usize, error: &str) -> Result<(), String> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| error.to_string())?;
        if end > self.bytes.len() {
            return Err(error.to_string());
        }
        self.offset = end;
        Ok(())
    }
}

pub(super) fn read_binary_property_row(
    reader: &mut PlyBinaryReader<'_>,
    properties: &[PlyProperty],
    error: &str,
) -> Result<Vec<Option<PlyValue>>, String> {
    properties
        .iter()
        .map(|property| match property {
            PlyProperty::Scalar { ty, .. } => reader.read_value(*ty, error).map(Some),
            PlyProperty::List {
                count_ty, item_ty, ..
            } => {
                let count = reader
                    .read_value(*count_ty, error)?
                    .as_meshlib_list_count(error)?;
                let size = count
                    .checked_mul(item_ty.byte_len())
                    .ok_or_else(|| error.to_string())?;
                reader.skip_bytes(size, error)?;
                Ok(None)
            }
        })
        .collect()
}

pub(super) fn binary_scalar_value(
    values: &[Option<PlyValue>],
    index: usize,
    error: &str,
) -> Result<PlyValue, String> {
    values
        .get(index)
        .copied()
        .flatten()
        .ok_or_else(|| error.to_string())
}

pub(super) fn skip_binary_property(
    reader: &mut PlyBinaryReader<'_>,
    property: &PlyProperty,
    error: &str,
) -> Result<(), String> {
    match property {
        PlyProperty::Scalar { ty, .. } => reader.skip_bytes(ty.byte_len(), error),
        PlyProperty::List {
            count_ty, item_ty, ..
        } => {
            let count = reader
                .read_value(*count_ty, error)?
                .as_meshlib_list_count(error)?;
            let size = count
                .checked_mul(item_ty.byte_len())
                .ok_or_else(|| error.to_string())?;
            reader.skip_bytes(size, error)
        }
    }
}
