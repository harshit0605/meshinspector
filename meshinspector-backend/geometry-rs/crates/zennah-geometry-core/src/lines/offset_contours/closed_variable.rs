use super::math::{
    add2, contour_normal, is_closed_contour, line_intersection_xy, restore_adjacent_edge_average_z,
    scale2, signed_area_xy,
};
use crate::lines::validate_contour;

pub(super) fn offset_closed_clockwise_signed_inward_contour(
    contour: &[[f64; 3]],
    point_offsets: &[f64],
) -> Result<Vec<[f64; 3]>, String> {
    validate_contour(contour)?;
    if contour.len() < 4 || !is_closed_contour(contour) {
        return Err("OffsetContours Type::Offset requires closed contours".to_string());
    }

    let points = &contour[..contour.len() - 1];
    if points.len() != point_offsets.len() {
        return Err("OffsetContours variable offsets must match contour point counts".to_string());
    }
    if signed_area_xy(points) >= 0.0 || point_offsets.iter().any(|offset| *offset > 0.0) {
        return Err(
            "OffsetContours closed negative-offset slice currently supports clockwise contours with negative offsets"
                .to_string(),
        );
    }

    let mut output = Vec::with_capacity(points.len() + 1);
    for index in 0..points.len() {
        let previous = if index == 0 {
            points.len() - 1
        } else {
            index - 1
        };
        let next = (index + 1) % points.len();
        let previous_normal = contour_normal(points[previous], points[index])?;
        let next_normal = contour_normal(points[index], points[next])?;
        let previous_line_start = add2(
            points[previous],
            scale2(previous_normal, point_offsets[previous]),
        );
        let previous_line_end = add2(points[index], scale2(previous_normal, point_offsets[index]));
        let next_line_start = add2(points[index], scale2(next_normal, point_offsets[index]));
        let next_line_end = add2(points[next], scale2(next_normal, point_offsets[next]));
        let mut point = line_intersection_xy(
            previous_line_start,
            previous_line_end,
            next_line_start,
            next_line_end,
        )
        .unwrap_or(next_line_start);
        point[2] = restore_adjacent_edge_average_z(points, index, point);
        output.push(point);
    }
    if let Some(first) = output.first().copied() {
        output.push(first);
    }
    Ok(output)
}
