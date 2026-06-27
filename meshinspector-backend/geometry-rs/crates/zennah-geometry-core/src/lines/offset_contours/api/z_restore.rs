pub fn offset_contours_with_variable_offsets(
    contours: &[Vec<[f64; 3]>],
    offsets: &[Vec<f64>],
    options: OffsetContoursOptions,
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    offset_contours_with_variable_offsets_and_z_options(
        contours,
        offsets,
        options,
        OffsetContoursZOptions::default(),
    )
}

pub fn offset_contours_with_variable_offsets_and_z_options(
    contours: &[Vec<[f64; 3]>],
    offsets: &[Vec<f64>],
    options: OffsetContoursOptions,
    z_options: OffsetContoursZOptions,
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    Ok(offset_contours_with_variable_offsets_and_origins_and_z_options(
        contours, offsets, options, z_options,
    )?
    .contours)
}

pub(super) fn apply_offset_contours_z_options(
    source_contours: &[Vec<[f64; 3]>],
    contours: &mut [Vec<[f64; 3]>],
    z_options: &OffsetContoursZOptions,
) -> Result<(), String> {
    validate_z_options(source_contours, z_options)?;
    match &z_options.restore_mode {
        OffsetContoursZRestoreMode::Default => {}
        OffsetContoursZRestoreMode::Constant(z_value) => {
            for contour in contours.iter_mut() {
                for point in contour {
                    point[2] = *z_value;
                }
            }
        }
        OffsetContoursZRestoreMode::Custom(z_values) => {
            apply_custom_z_restore(source_contours, contours, z_values)?;
        }
    }
    relax_offset_contours_z(contours, z_options.relax_iterations);
    Ok(())
}

fn validate_z_options(
    source_contours: &[Vec<[f64; 3]>],
    z_options: &OffsetContoursZOptions,
) -> Result<(), String> {
    match &z_options.restore_mode {
        OffsetContoursZRestoreMode::Default => Ok(()),
        OffsetContoursZRestoreMode::Constant(z_value) => {
            if z_value.is_finite() {
                Ok(())
            } else {
                Err("OffsetContours z_value must be finite".to_string())
            }
        }
        OffsetContoursZRestoreMode::Custom(z_values) => {
            validate_custom_z_values(source_contours, z_values)
        }
    }
}

fn validate_custom_z_values(
    source_contours: &[Vec<[f64; 3]>],
    z_values: &[Vec<f64>],
) -> Result<(), String> {
    if z_values.len() != source_contours.len() {
        return Err("OffsetContours z_values must match contour count".to_string());
    }
    for (contour_index, (source, z_row)) in source_contours.iter().zip(z_values).enumerate() {
        if z_row.len() != source.len() {
            return Err(format!(
                "OffsetContours z_values row {contour_index} must match source contour point count"
            ));
        }
        if z_row.iter().any(|value| !value.is_finite()) {
            return Err("OffsetContours z_values must be finite".to_string());
        }
    }
    Ok(())
}

fn apply_custom_z_restore(
    source_contours: &[Vec<[f64; 3]>],
    contours: &mut [Vec<[f64; 3]>],
    z_values: &[Vec<f64>],
) -> Result<(), String> {
    for contour in contours.iter_mut() {
        for point in contour.iter_mut() {
            point[2] = nearest_source_z(point, source_contours, z_values).ok_or_else(|| {
                "OffsetContours z_values require at least one source point".to_string()
            })?;
        }
    }
    Ok(())
}

fn nearest_source_z(
    point: &[f64; 3],
    source_contours: &[Vec<[f64; 3]>],
    z_values: &[Vec<f64>],
) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for (source, z_row) in source_contours.iter().zip(z_values) {
        for (source_point, z_value) in source.iter().zip(z_row) {
            let dx = point[0] - source_point[0];
            let dy = point[1] - source_point[1];
            let distance2 = dx * dx + dy * dy;
            if best.is_none_or(|(best_distance2, _)| distance2 < best_distance2) {
                best = Some((distance2, *z_value));
            }
        }
    }
    best.map(|(_, z_value)| z_value)
}

pub(super) fn relax_offset_contours_z(contours: &mut [Vec<[f64; 3]>], iterations: usize) {
    for contour in contours {
        if contour.len() < 3 {
            continue;
        }
        for _ in 0..iterations {
            let points = contour.clone();
            let count = points.len();
            for index in 0..count {
                let mut previous = (index + count - 1) % count;
                let mut next = (index + 1) % count;
                if previous + 1 == count {
                    previous = previous.saturating_sub(1);
                }
                if next == 0 {
                    next += 1;
                }
                if previous >= count || next >= count || previous == next {
                    continue;
                }

                let previous_point = points[previous];
                let current_point = points[index];
                let next_point = points[next];
                let segment = [
                    next_point[0] - previous_point[0],
                    next_point[1] - previous_point[1],
                ];
                let denominator = segment[0] * segment[0] + segment[1] * segment[1];
                if denominator <= 1e-24 {
                    continue;
                }
                let ratio = (((current_point[0] - previous_point[0]) * segment[0]
                    + (current_point[1] - previous_point[1]) * segment[1])
                    / denominator)
                    .clamp(0.0, 1.0);
                let target_z = (1.0 - ratio) * previous_point[2] + ratio * next_point[2];
                contour[index][2] = (target_z + current_point[2]) * 0.5;
            }
        }
    }
}
