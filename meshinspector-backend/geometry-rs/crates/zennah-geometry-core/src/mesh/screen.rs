use super::base::validate_faces;
use crate::GeometryError;
use std::collections::BTreeSet;

pub fn select_faces_by_screen_polygon(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    view_projection_4x4: &[f64; 16],
    polygon_xy: &[[f64; 2]],
    include_backfaces: bool,
    visible_only: bool,
) -> Result<Vec<i64>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if polygon_xy.len() < 3 {
        return Ok(Vec::new());
    }

    let projected_vertices = vertices
        .iter()
        .map(|vertex| project_vertex_to_clip(vertex, view_projection_4x4))
        .collect::<Vec<_>>();
    let projected_faces = faces
        .iter()
        .map(|face| {
            Some([
                projected_vertices[face[0]]?,
                projected_vertices[face[1]]?,
                projected_vertices[face[2]]?,
            ])
        })
        .collect::<Vec<_>>();

    let mut selected = BTreeSet::<usize>::new();
    for (face_index, projected) in projected_faces.iter().enumerate() {
        let Some(projected) = projected else {
            continue;
        };
        if !include_backfaces && !is_front_facing_projected_triangle(projected) {
            continue;
        }
        let samples = screen_polygon_face_samples(projected, polygon_xy);
        if samples.is_empty() {
            continue;
        }
        if visible_only
            && !samples
                .iter()
                .any(|sample| screen_sample_is_visible(*sample, face_index, &projected_faces))
        {
            continue;
        }
        selected.insert(face_index);
    }

    Ok(selected.into_iter().map(|face_id| face_id as i64).collect())
}

pub fn select_faces_by_screen_rect(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    view_projection_4x4: &[f64; 16],
    rect_min_xy: [f64; 2],
    rect_max_xy: [f64; 2],
    include_backfaces: bool,
    visible_only: bool,
) -> Result<Vec<i64>, GeometryError> {
    if !rect_min_xy
        .iter()
        .chain(rect_max_xy.iter())
        .all(|value| value.is_finite())
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "screen_rect_xy",
            value: format!("{rect_min_xy:?} {rect_max_xy:?}"),
        });
    }

    let min_x = rect_min_xy[0].min(rect_max_xy[0]);
    let max_x = rect_min_xy[0].max(rect_max_xy[0]);
    let min_y = rect_min_xy[1].min(rect_max_xy[1]);
    let max_y = rect_min_xy[1].max(rect_max_xy[1]);
    if (max_x - min_x).abs() < 1e-12 || (max_y - min_y).abs() < 1e-12 {
        validate_faces(faces_i64, vertices.len())?;
        return Ok(Vec::new());
    }

    let polygon = [
        [min_x, min_y],
        [max_x, min_y],
        [max_x, max_y],
        [min_x, max_y],
    ];
    select_faces_by_screen_polygon(
        vertices,
        faces_i64,
        view_projection_4x4,
        &polygon,
        include_backfaces,
        visible_only,
    )
}

pub fn select_faces_by_screen_brush(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    view_projection_4x4: &[f64; 16],
    brush_path_xy: &[[f64; 2]],
    radius_px: f64,
    include_backfaces: bool,
    visible_only: bool,
) -> Result<Vec<i64>, GeometryError> {
    if !radius_px.is_finite() || radius_px < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "radius_px",
            value: radius_px.to_string(),
        });
    }
    if !brush_path_xy
        .iter()
        .flatten()
        .all(|value| value.is_finite())
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "brush_path_xy",
            value: format!("{brush_path_xy:?}"),
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    if brush_path_xy.is_empty() {
        return Ok(Vec::new());
    }

    let projected_vertices = vertices
        .iter()
        .map(|vertex| project_vertex_to_clip(vertex, view_projection_4x4))
        .collect::<Vec<_>>();
    let projected_faces = faces
        .iter()
        .map(|face| {
            Some([
                projected_vertices[face[0]]?,
                projected_vertices[face[1]]?,
                projected_vertices[face[2]]?,
            ])
        })
        .collect::<Vec<_>>();

    let mut selected = BTreeSet::<usize>::new();
    for (face_index, projected) in projected_faces.iter().enumerate() {
        let Some(projected) = projected else {
            continue;
        };
        if !include_backfaces && !is_front_facing_projected_triangle(projected) {
            continue;
        }
        let samples = screen_brush_face_samples(projected, brush_path_xy, radius_px);
        if samples.is_empty() {
            continue;
        }
        if visible_only
            && !samples
                .iter()
                .any(|sample| screen_sample_is_visible(*sample, face_index, &projected_faces))
        {
            continue;
        }
        selected.insert(face_index);
    }

    Ok(selected.into_iter().map(|face_id| face_id as i64).collect())
}

fn project_vertex_to_clip(vertex: &[f64; 3], view_projection_4x4: &[f64; 16]) -> Option<[f64; 3]> {
    let x = view_projection_4x4[0] * vertex[0]
        + view_projection_4x4[4] * vertex[1]
        + view_projection_4x4[8] * vertex[2]
        + view_projection_4x4[12];
    let y = view_projection_4x4[1] * vertex[0]
        + view_projection_4x4[5] * vertex[1]
        + view_projection_4x4[9] * vertex[2]
        + view_projection_4x4[13];
    let z = view_projection_4x4[2] * vertex[0]
        + view_projection_4x4[6] * vertex[1]
        + view_projection_4x4[10] * vertex[2]
        + view_projection_4x4[14];
    let w = view_projection_4x4[3] * vertex[0]
        + view_projection_4x4[7] * vertex[1]
        + view_projection_4x4[11] * vertex[2]
        + view_projection_4x4[15];
    if !w.is_finite() || w.abs() < 1e-12 {
        return None;
    }
    let projected = [x / w, y / w, z / w];
    projected
        .iter()
        .all(|value| value.is_finite())
        .then_some(projected)
}

fn screen_polygon_face_samples(
    projected: &[[f64; 3]; 3],
    polygon_xy: &[[f64; 2]],
) -> Vec<[f64; 3]> {
    projected_face_samples(projected)
        .into_iter()
        .filter(|sample| clip_point_is_selectable(*sample, polygon_xy))
        .collect()
}

fn screen_brush_face_samples(
    projected: &[[f64; 3]; 3],
    brush_path_xy: &[[f64; 2]],
    radius_px: f64,
) -> Vec<[f64; 3]> {
    projected_face_samples(projected)
        .into_iter()
        .filter(|sample| clip_point_is_near_brush(*sample, brush_path_xy, radius_px))
        .collect()
}

fn projected_face_samples(projected: &[[f64; 3]; 3]) -> Vec<[f64; 3]> {
    let mut samples = projected.to_vec();
    samples.push([
        (projected[0][0] + projected[1][0] + projected[2][0]) / 3.0,
        (projected[0][1] + projected[1][1] + projected[2][1]) / 3.0,
        (projected[0][2] + projected[1][2] + projected[2][2]) / 3.0,
    ]);
    let min_x = projected
        .iter()
        .map(|point| point[0])
        .fold(f64::INFINITY, f64::min);
    let max_x = projected
        .iter()
        .map(|point| point[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = projected
        .iter()
        .map(|point| point[1])
        .fold(f64::INFINITY, f64::min);
    let max_y = projected
        .iter()
        .map(|point| point[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let max_span = (max_x - min_x).abs().max((max_y - min_y).abs());
    let steps = ((max_span * 32.0).ceil() as usize).clamp(4, 64);
    let inv_steps = 1.0 / steps as f64;
    for ia in 1..steps {
        for ib in 1..(steps - ia) {
            let ic = steps - ia - ib;
            if ic == 0 {
                continue;
            }
            let weights = [
                ia as f64 * inv_steps,
                ib as f64 * inv_steps,
                ic as f64 * inv_steps,
            ];
            let sample = [
                weights[0] * projected[0][0]
                    + weights[1] * projected[1][0]
                    + weights[2] * projected[2][0],
                weights[0] * projected[0][1]
                    + weights[1] * projected[1][1]
                    + weights[2] * projected[2][1],
                weights[0] * projected[0][2]
                    + weights[1] * projected[1][2]
                    + weights[2] * projected[2][2],
            ];
            samples.push(sample);
        }
    }
    samples
}

fn clip_point_is_selectable(point: [f64; 3], polygon_xy: &[[f64; 2]]) -> bool {
    point[0] >= -1.0
        && point[0] <= 1.0
        && point[1] >= -1.0
        && point[1] <= 1.0
        && point_in_polygon_or_on_boundary([point[0], point[1]], polygon_xy)
}

fn clip_point_is_near_brush(point: [f64; 3], brush_path_xy: &[[f64; 2]], radius_px: f64) -> bool {
    if point[0] < -1.0 || point[0] > 1.0 || point[1] < -1.0 || point[1] > 1.0 {
        return false;
    }
    let point_xy = [point[0], point[1]];
    let radius_sq = radius_px * radius_px + 1e-12;
    if brush_path_xy.len() == 1 {
        return distance_sq_2d(point_xy, brush_path_xy[0]) <= radius_sq;
    }
    brush_path_xy
        .windows(2)
        .any(|segment| point_segment_distance_sq_2d(point_xy, segment[0], segment[1]) <= radius_sq)
}

fn point_in_polygon_or_on_boundary(point: [f64; 2], polygon_xy: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for index in 0..polygon_xy.len() {
        let a = polygon_xy[index];
        let b = polygon_xy[(index + 1) % polygon_xy.len()];
        if point_on_segment(point, a, b) {
            return true;
        }
        let crosses = (a[1] > point[1]) != (b[1] > point[1]);
        if crosses {
            let x_at_y = a[0] + (point[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if point[0] < x_at_y {
                inside = !inside;
            }
        }
    }
    inside
}

fn point_segment_distance_sq_2d(point: [f64; 2], start: [f64; 2], end: [f64; 2]) -> f64 {
    let segment = [end[0] - start[0], end[1] - start[1]];
    let len_sq = segment[0] * segment[0] + segment[1] * segment[1];
    if len_sq <= 1e-18 {
        return distance_sq_2d(point, start);
    }
    let delta = [point[0] - start[0], point[1] - start[1]];
    let t = ((delta[0] * segment[0] + delta[1] * segment[1]) / len_sq).clamp(0.0, 1.0);
    let closest = [start[0] + segment[0] * t, start[1] + segment[1] * t];
    distance_sq_2d(point, closest)
}

fn distance_sq_2d(left: [f64; 2], right: [f64; 2]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    dx * dx + dy * dy
}

fn point_on_segment(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    let cross = (point[1] - a[1]) * (b[0] - a[0]) - (point[0] - a[0]) * (b[1] - a[1]);
    if cross.abs() > 1e-10 {
        return false;
    }
    let min_x = a[0].min(b[0]) - 1e-10;
    let max_x = a[0].max(b[0]) + 1e-10;
    let min_y = a[1].min(b[1]) - 1e-10;
    let max_y = a[1].max(b[1]) + 1e-10;
    point[0] >= min_x && point[0] <= max_x && point[1] >= min_y && point[1] <= max_y
}

fn is_front_facing_projected_triangle(projected: &[[f64; 3]; 3]) -> bool {
    let area2 = (projected[1][0] - projected[0][0]) * (projected[2][1] - projected[0][1])
        - (projected[1][1] - projected[0][1]) * (projected[2][0] - projected[0][0]);
    area2 >= -1e-12
}

fn screen_sample_is_visible(
    sample: [f64; 3],
    face_index: usize,
    projected_faces: &[Option<[[f64; 3]; 3]>],
) -> bool {
    let mut min_depth = f64::INFINITY;
    let sample_xy = [sample[0], sample[1]];
    for projected in projected_faces.iter().flatten() {
        let Some(weights) = projected_triangle_barycentric(sample_xy, projected) else {
            continue;
        };
        let depth = weights[0] * projected[0][2]
            + weights[1] * projected[1][2]
            + weights[2] * projected[2][2];
        min_depth = min_depth.min(depth);
    }
    if !min_depth.is_finite() {
        return true;
    }
    let Some(projected) = projected_faces[face_index] else {
        return false;
    };
    let Some(weights) = projected_triangle_barycentric(sample_xy, &projected) else {
        return false;
    };
    let face_depth =
        weights[0] * projected[0][2] + weights[1] * projected[1][2] + weights[2] * projected[2][2];
    face_depth <= min_depth + 1e-9
}

fn projected_triangle_barycentric(point: [f64; 2], projected: &[[f64; 3]; 3]) -> Option<[f64; 3]> {
    let a = [projected[0][0], projected[0][1]];
    let b = [projected[1][0], projected[1][1]];
    let c = [projected[2][0], projected[2][1]];
    let denominator = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if denominator.abs() < 1e-12 {
        return None;
    }
    let wa = ((b[1] - c[1]) * (point[0] - c[0]) + (c[0] - b[0]) * (point[1] - c[1])) / denominator;
    let wb = ((c[1] - a[1]) * (point[0] - c[0]) + (a[0] - c[0]) * (point[1] - c[1])) / denominator;
    let wc = 1.0 - wa - wb;
    (wa >= -1e-9 && wb >= -1e-9 && wc >= -1e-9).then_some([wa, wb, wc])
}
