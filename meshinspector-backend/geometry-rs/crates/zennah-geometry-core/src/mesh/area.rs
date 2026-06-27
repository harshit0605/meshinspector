use super::base::{face_area, validate_faces};
use crate::GeometryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaScalarType {
    Absolute,
    Percentage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaCompareType {
    Less,
    Greater,
}

fn parse_area_scalar_type(value: &str) -> Result<AreaScalarType, GeometryError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "absolute" | "abs" | "area" | "area_mm2" | "mm2" | "square_mm" | "square_millimeters" => {
            Ok(AreaScalarType::Absolute)
        }
        "percentage" | "percent" | "%" => Ok(AreaScalarType::Percentage),
        normalized => Err(GeometryError::InvalidSelectionParameter {
            field: "scalar_type",
            value: normalized.to_string(),
        }),
    }
}

fn parse_area_compare_type(value: &str) -> Result<AreaCompareType, GeometryError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "less" | "lt" | "<" => Ok(AreaCompareType::Less),
        "greater" | "gt" | ">" => Ok(AreaCompareType::Greater),
        normalized => Err(GeometryError::InvalidSelectionParameter {
            field: "compare_type",
            value: normalized.to_string(),
        }),
    }
}

pub fn select_faces_by_area(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    area_threshold: f64,
    scalar_type: &str,
    compare_type: &str,
) -> Result<Vec<i64>, GeometryError> {
    if !area_threshold.is_finite() {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "area_threshold",
            value: area_threshold,
        });
    }
    let scalar = parse_area_scalar_type(scalar_type)?;
    let compare = parse_area_compare_type(compare_type)?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    let face_areas: Vec<f64> = faces.iter().map(|face| face_area(vertices, face)).collect();
    let total_area = face_areas.iter().sum::<f64>();

    if scalar == AreaScalarType::Percentage && total_area <= f64::EPSILON {
        return Ok(Vec::new());
    }

    Ok(face_areas
        .iter()
        .enumerate()
        .filter_map(|(face_index, face_area)| {
            let value = match scalar {
                AreaScalarType::Absolute => *face_area,
                AreaScalarType::Percentage => (*face_area / total_area) * 100.0,
            };
            let matches = match compare {
                AreaCompareType::Less => value < area_threshold,
                AreaCompareType::Greater => value > area_threshold,
            };
            matches.then_some(face_index as i64)
        })
        .collect())
}
