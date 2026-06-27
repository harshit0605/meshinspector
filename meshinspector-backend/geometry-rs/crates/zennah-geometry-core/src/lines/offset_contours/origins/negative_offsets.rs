fn offset_closed_clockwise_negative_contour_with_origins(
    contour: &[[f64; 3]],
    contour_id: usize,
    offset: f64,
) -> Result<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>), String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours Type::Offset requires closed contours".to_string());
    }

    let points = &contour[..contour.len() - 1];
    if signed_area_xy(points) >= 0.0 {
        return Err(
            "OffsetContours closed negative-offset slice currently supports clockwise contours with negative offsets"
                .to_string(),
        );
    }
    let point_offsets = vec![offset; points.len()];
    let output = offset_closed_clockwise_signed_inward_contour(contour, &point_offsets)?;
    let mut origins = Vec::with_capacity(output.len());
    for (index, point) in output.iter().take(points.len()).enumerate() {
        origins.push(negative_intersection_origin(
            contour_id, index, points, *point,
        ));
    }
    if let Some(first) = origins.first().copied() {
        origins.push(first);
    }
    Ok((output, origins))
}

fn offset_closed_clockwise_negative_variable_contour_with_origins(
    contour: &[[f64; 3]],
    contour_id: usize,
    offsets: &[f64],
) -> Result<(Vec<[f64; 3]>, Vec<OffsetContoursOrigin>), String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours Type::Offset requires closed contours".to_string());
    }
    if contour.len() != offsets.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }

    let points = &contour[..contour.len() - 1];
    let point_offsets = &offsets[..offsets.len() - 1];
    if signed_area_xy(points) >= 0.0 {
        return Err(
            "OffsetContours closed negative-offset slice currently supports clockwise contours with negative offsets"
                .to_string(),
        );
    }
    if (offsets[0] - offsets[offsets.len() - 1]).abs() > 1e-12 {
        return Err(
            "OffsetContours closed variable-offset slice requires matching first and closing offsets"
                .to_string(),
        );
    }
    if point_offsets.iter().any(|offset| *offset > 0.0) {
        return Err(
            "OffsetContours closed negative-offset slice currently supports clockwise contours with negative offsets"
                .to_string(),
        );
    }
    let output = offset_closed_clockwise_signed_inward_contour(contour, point_offsets)?;
    let uniform_offsets = point_offsets
        .iter()
        .all(|offset| (*offset - point_offsets[0]).abs() <= 1e-12);
    let mut origins = Vec::with_capacity(output.len());
    for (index, point) in output.iter().take(points.len()).enumerate() {
        origins.push(if uniform_offsets {
            negative_intersection_origin(contour_id, index, points, *point)
        } else {
            negative_variable_intersection_origin(contour_id, index, points, *point)
        });
    }
    if let Some(first) = origins.first().copied() {
        origins.push(first);
    }
    Ok((output, origins))
}
