use super::sampling::{
    dot, squared_distance, validate_distance_limit, validate_normals, validate_point_rows,
};

pub fn point_cloud_pick_by_ray(
    points: &[[f64; 3]],
    ray_origin: [f64; 3],
    ray_direction: [f64; 3],
    max_distance_to_ray: f64,
    max_depth: f64,
    normals: Option<&[[f64; 3]]>,
    include_backfaces: bool,
) -> Result<Vec<i64>, String> {
    validate_point_rows("point cloud", points, true)?;
    if ray_origin.iter().any(|value| !value.is_finite()) {
        return Err("ray_origin values must be finite".to_string());
    }
    if ray_direction.iter().any(|value| !value.is_finite()) {
        return Err("ray_direction values must be finite".to_string());
    }
    if !max_distance_to_ray.is_finite() || max_distance_to_ray < 0.0 {
        return Err("max_distance_to_ray must be finite and non-negative".to_string());
    }
    validate_distance_limit("max_depth", max_depth)?;
    if let Some(normals) = normals {
        validate_normals(points, normals)?;
    }

    let direction_len_sq = dot(ray_direction, ray_direction);
    if direction_len_sq <= 1e-24 {
        return Err("ray_direction must have non-zero length".to_string());
    }
    let inv_direction_len = 1.0 / direction_len_sq.sqrt();
    let ray_dir = [
        ray_direction[0] * inv_direction_len,
        ray_direction[1] * inv_direction_len,
        ray_direction[2] * inv_direction_len,
    ];
    let camera_dir = [-ray_dir[0], -ray_dir[1], -ray_dir[2]];
    let max_distance_sq = max_distance_to_ray * max_distance_to_ray;

    let mut best = None::<(usize, f64, f64)>;
    for (index, point) in points.iter().enumerate() {
        if !include_backfaces {
            if let Some(normals) = normals {
                if dot(normals[index], camera_dir) < 0.0 {
                    continue;
                }
            }
        }

        let delta = [
            point[0] - ray_origin[0],
            point[1] - ray_origin[1],
            point[2] - ray_origin[2],
        ];
        let depth = dot(delta, ray_dir);
        if depth < 0.0 || depth > max_depth {
            continue;
        }
        let closest = [
            ray_origin[0] + ray_dir[0] * depth,
            ray_origin[1] + ray_dir[1] * depth,
            ray_origin[2] + ray_dir[2] * depth,
        ];
        let distance_sq = squared_distance(*point, closest);
        if distance_sq > max_distance_sq {
            continue;
        }

        let replace = best
            .map(|(best_index, best_depth, best_distance_sq)| {
                depth < best_depth
                    || (depth == best_depth
                        && (distance_sq < best_distance_sq
                            || (distance_sq == best_distance_sq && index < best_index)))
            })
            .unwrap_or(true);
        if replace {
            best = Some((index, depth, distance_sq));
        }
    }

    Ok(best
        .map(|(index, _, _)| vec![index as i64])
        .unwrap_or_default())
}

pub fn select_point_cloud_points_by_screen_polygon(
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    view_projection_4x4: &[f64; 16],
    polygon_xy: &[[f64; 2]],
    include_backfaces: bool,
    _visible_only: bool,
) -> Result<Vec<i64>, String> {
    validate_point_rows("point cloud", points, true)?;
    validate_screen_projection(view_projection_4x4)?;
    validate_screen_points("polygon_xy", polygon_xy, 3)?;
    if let Some(normals) = normals {
        validate_normals(points, normals)?;
    }

    Ok(select_projected_points(
        points,
        normals,
        view_projection_4x4,
        include_backfaces,
        |point| screen_point_is_selectable(point, polygon_xy),
    ))
}

pub fn select_point_cloud_points_by_screen_rect(
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    view_projection_4x4: &[f64; 16],
    rect_min_xy: [f64; 2],
    rect_max_xy: [f64; 2],
    include_backfaces: bool,
    visible_only: bool,
) -> Result<Vec<i64>, String> {
    if !rect_min_xy
        .iter()
        .chain(rect_max_xy.iter())
        .all(|value| value.is_finite())
    {
        return Err("screen_rect_xy values must be finite".to_string());
    }
    let min_x = rect_min_xy[0].min(rect_max_xy[0]);
    let max_x = rect_min_xy[0].max(rect_max_xy[0]);
    let min_y = rect_min_xy[1].min(rect_max_xy[1]);
    let max_y = rect_min_xy[1].max(rect_max_xy[1]);
    if (max_x - min_x).abs() < 1e-12 || (max_y - min_y).abs() < 1e-12 {
        validate_point_rows("point cloud", points, true)?;
        return Ok(Vec::new());
    }
    let polygon = [
        [min_x, min_y],
        [max_x, min_y],
        [max_x, max_y],
        [min_x, max_y],
    ];
    select_point_cloud_points_by_screen_polygon(
        points,
        normals,
        view_projection_4x4,
        &polygon,
        include_backfaces,
        visible_only,
    )
}

pub fn select_point_cloud_points_by_screen_brush(
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    view_projection_4x4: &[f64; 16],
    brush_path_xy: &[[f64; 2]],
    radius_px: f64,
    include_backfaces: bool,
    _visible_only: bool,
) -> Result<Vec<i64>, String> {
    validate_point_rows("point cloud", points, true)?;
    validate_screen_projection(view_projection_4x4)?;
    validate_screen_points("brush_path_xy", brush_path_xy, 1)?;
    if !radius_px.is_finite() || radius_px < 0.0 {
        return Err("radius_px must be finite and non-negative".to_string());
    }
    if let Some(normals) = normals {
        validate_normals(points, normals)?;
    }

    Ok(select_projected_points(
        points,
        normals,
        view_projection_4x4,
        include_backfaces,
        |point| screen_point_is_near_brush(point, brush_path_xy, radius_px),
    ))
}

fn validate_screen_projection(view_projection_4x4: &[f64; 16]) -> Result<(), String> {
    if view_projection_4x4.iter().any(|value| !value.is_finite()) {
        return Err("view_projection_4x4 values must be finite".to_string());
    }
    Ok(())
}

fn validate_screen_points(name: &str, points: &[[f64; 2]], min_count: usize) -> Result<(), String> {
    if points.len() < min_count {
        return Err(format!("{name} requires at least {min_count} point(s)"));
    }
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err(format!("{name} points must be finite"));
    }
    Ok(())
}

fn select_projected_points(
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    view_projection_4x4: &[f64; 16],
    include_backfaces: bool,
    accepts: impl Fn([f64; 3]) -> bool,
) -> Vec<i64> {
    let camera_dir = [0.0, 0.0, 1.0];
    points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            let projected = project_point_to_clip(point, view_projection_4x4)?;
            if !accepts(projected) {
                return None;
            }
            if !include_backfaces {
                if let Some(normals) = normals {
                    if dot(normals[index], camera_dir) < 0.0 {
                        return None;
                    }
                }
            }
            Some(index as i64)
        })
        .collect()
}

fn project_point_to_clip(point: &[f64; 3], view_projection_4x4: &[f64; 16]) -> Option<[f64; 3]> {
    let x = view_projection_4x4[0] * point[0]
        + view_projection_4x4[4] * point[1]
        + view_projection_4x4[8] * point[2]
        + view_projection_4x4[12];
    let y = view_projection_4x4[1] * point[0]
        + view_projection_4x4[5] * point[1]
        + view_projection_4x4[9] * point[2]
        + view_projection_4x4[13];
    let z = view_projection_4x4[2] * point[0]
        + view_projection_4x4[6] * point[1]
        + view_projection_4x4[10] * point[2]
        + view_projection_4x4[14];
    let w = view_projection_4x4[3] * point[0]
        + view_projection_4x4[7] * point[1]
        + view_projection_4x4[11] * point[2]
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

fn screen_point_is_selectable(point: [f64; 3], polygon_xy: &[[f64; 2]]) -> bool {
    point[0] >= -1.0
        && point[0] <= 1.0
        && point[1] >= -1.0
        && point[1] <= 1.0
        && point_in_polygon_or_on_boundary([point[0], point[1]], polygon_xy)
}

fn screen_point_is_near_brush(point: [f64; 3], brush_path_xy: &[[f64; 2]], radius_px: f64) -> bool {
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
