use crate::math::distance_sq;
use crate::spatial::first_ray_hits;
use crate::GeometryError;
use rayon::prelude::*;
use std::collections::BTreeSet;

#[path = "distance_tiff.rs"]
mod distance_tiff;
pub use distance_tiff::{distance_map_from_tiff, distance_map_to_tiff};
#[path = "distance_transform.rs"]
mod distance_transform;
pub(crate) use distance_transform::{
    axis_aligned_distance_map_model_transform, distance_map_model_transform_from_frame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DistanceMapGrid {
    pub width: usize,
    pub height: usize,
    pub origin: [f64; 2],
    pub pixel_size: [f64; 2],
    pub model_transform: Option<[f64; 16]>,
    pub values: Vec<f32>,
    pub valid_count: usize,
    pub min_value: f32,
    pub max_value: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsoLineSegments2 {
    pub iso_value: f32,
    pub segments: Vec<[[f64; 2]; 2]>,
}

pub const DISTANCE_MAP_NOT_VALID_VALUE: f32 = f32::MIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMapMergeMode {
    Min,
    Max,
    Subtract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourBooleanMode {
    Union,
    Intersection,
    Subtract,
}

pub fn nearest_distances_to_indices(
    vertices: &[[f64; 3]],
    target_indices: &[i64],
) -> Result<Vec<f64>, GeometryError> {
    let targets = validate_target_indices(target_indices, vertices.len())?;
    Ok(nearest_distances_to_validated_indices(vertices, &targets))
}

pub fn distance_map_from_contours(
    contours: &[Vec<[f64; 2]>],
    width: usize,
    height: usize,
    origin: [f64; 2],
    pixel_size: [f64; 2],
    signed: bool,
) -> Result<DistanceMapGrid, String> {
    validate_distance_map_inputs(contours, width, height, origin, pixel_size)?;
    let segments = contour_segments(contours, signed)?;
    if segments.is_empty() {
        return Err("contours must contain at least one non-degenerate segment".to_string());
    }

    let values: Vec<f32> = (0..width * height)
        .into_par_iter()
        .map(|index| {
            let x = index % width;
            let y = index / width;
            let point = [
                origin[0] + pixel_size[0] * (x as f64 + 0.5),
                origin[1] + pixel_size[1] * (y as f64 + 0.5),
            ];
            let distance = nearest_segment_distance(point, &segments);
            if signed && point_inside_closed_contours(point, contours) {
                -(distance as f32)
            } else {
                distance as f32
            }
        })
        .collect();
    let (min_value, max_value) = values
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
            (min.min(*value), max.max(*value))
        });
    Ok(DistanceMapGrid {
        width,
        height,
        origin,
        pixel_size,
        model_transform: Some(axis_aligned_distance_map_model_transform(
            origin, pixel_size,
        )),
        valid_count: values.len(),
        values,
        min_value,
        max_value,
    })
}

pub fn distance_map_from_mesh(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    width: usize,
    height: usize,
    origin: [f64; 3],
    x_range: [f64; 3],
    y_range: [f64; 3],
    direction: [f64; 3],
    epsilon: f64,
) -> Result<DistanceMapGrid, String> {
    validate_mesh_distance_map_inputs(
        vertices, width, height, origin, x_range, y_range, direction, epsilon,
    )?;

    let origins = mesh_distance_map_ray_origins(width, height, origin, x_range, y_range);
    let directions = vec![direction; origins.len()];
    let hits = first_ray_hits(vertices, faces, &origins, &directions, epsilon, &[])
        .map_err(|error| error.to_string())?;
    let values = hits
        .distances
        .iter()
        .map(|distance| {
            if distance.is_finite() {
                *distance as f32
            } else {
                DISTANCE_MAP_NOT_VALID_VALUE
            }
        })
        .collect::<Vec<_>>();
    let (valid_count, min_value, max_value) = distance_map_stats(&values);
    Ok(DistanceMapGrid {
        width,
        height,
        origin: [origin[0], origin[1]],
        pixel_size: [
            vector_length(x_range) / width as f64,
            vector_length(y_range) / height as f64,
        ],
        model_transform: Some(distance_map_model_transform_from_frame(
            origin, x_range, y_range, direction, width, height,
        )),
        values,
        valid_count,
        min_value,
        max_value,
    })
}

pub fn distance_map_to_iso_segments(
    map: &DistanceMapGrid,
    iso_value: f32,
) -> Result<IsoLineSegments2, String> {
    validate_iso_map(map, iso_value)?;
    if map.width < 2 || map.height < 2 {
        return Ok(IsoLineSegments2 {
            iso_value,
            segments: Vec::new(),
        });
    }

    let mut segments = Vec::new();
    for y in 0..(map.height - 1) {
        for x in 0..(map.width - 1) {
            let values = [
                map.values[y * map.width + x],
                map.values[y * map.width + x + 1],
                map.values[(y + 1) * map.width + x],
                map.values[(y + 1) * map.width + x + 1],
            ];
            let mut configuration = 0_u8;
            for (index, value) in values.iter().enumerate() {
                if *value < iso_value {
                    configuration |= 1 << index;
                }
            }
            for edge_pair in topology_plan(configuration).chunks(2) {
                if edge_pair.len() != 2 {
                    continue;
                }
                let Some(start) = iso_edge_point(x, y, values, edge_pair[0], iso_value) else {
                    continue;
                };
                let Some(end) = iso_edge_point(x, y, values, edge_pair[1], iso_value) else {
                    continue;
                };
                segments.push([map_to_world(map, start), map_to_world(map, end)]);
            }
        }
    }
    Ok(IsoLineSegments2 {
        iso_value,
        segments,
    })
}

pub fn distance_map_merge(
    left: &DistanceMapGrid,
    right: &DistanceMapGrid,
    mode: DistanceMapMergeMode,
) -> Result<DistanceMapGrid, String> {
    validate_merge_map(left, "left")?;
    validate_merge_map(right, "right")?;

    let mut values = vec![DISTANCE_MAP_NOT_VALID_VALUE; left.width * left.height];
    for y in 0..left.height {
        for x in 0..left.width {
            let index = y * left.width + x;
            let left_value = map_value_at(left, x, y);
            let right_value = map_value_at(right, x, y);
            values[index] = match mode {
                DistanceMapMergeMode::Min => merge_pair(left_value, right_value, f32::min),
                DistanceMapMergeMode::Max => merge_pair(left_value, right_value, f32::max),
                DistanceMapMergeMode::Subtract => match (left_value, right_value) {
                    (Some(lhs), Some(rhs)) => lhs - rhs,
                    (Some(lhs), None) if x >= right.width || y >= right.height => lhs,
                    _ => DISTANCE_MAP_NOT_VALID_VALUE,
                },
            };
        }
    }
    let (valid_count, min_value, max_value) = distance_map_stats(&values);
    Ok(DistanceMapGrid {
        width: left.width,
        height: left.height,
        origin: left.origin,
        pixel_size: left.pixel_size,
        model_transform: left.model_transform,
        values,
        valid_count,
        min_value,
        max_value,
    })
}

pub fn distance_map_contour_boolean(
    contours_a: &[Vec<[f64; 2]>],
    contours_b: &[Vec<[f64; 2]>],
    mode: ContourBooleanMode,
    width: usize,
    height: usize,
    origin: [f64; 2],
    pixel_size: [f64; 2],
    iso_value: f32,
) -> Result<IsoLineSegments2, String> {
    let map_a = distance_map_from_contours(contours_a, width, height, origin, pixel_size, true)?;
    let mut map_b =
        distance_map_from_contours(contours_b, width, height, origin, pixel_size, true)?;
    let merged = match mode {
        ContourBooleanMode::Union => distance_map_merge(&map_a, &map_b, DistanceMapMergeMode::Min)?,
        ContourBooleanMode::Intersection => {
            distance_map_merge(&map_a, &map_b, DistanceMapMergeMode::Max)?
        }
        ContourBooleanMode::Subtract => {
            negate_distance_map(&mut map_b);
            distance_map_merge(&map_a, &map_b, DistanceMapMergeMode::Max)?
        }
    };
    distance_map_to_iso_segments(&merged, iso_value)
}

pub(crate) fn nearest_distances_to_validated_indices(
    vertices: &[[f64; 3]],
    targets: &[usize],
) -> Vec<f64> {
    let distances = vertices
        .par_iter()
        .map(|vertex| {
            targets
                .iter()
                .map(|target| distance_sq(*vertex, vertices[*target]))
                .fold(f64::INFINITY, f64::min)
                .sqrt()
        })
        .collect();
    distances
}

fn validate_target_indices(
    target_indices: &[i64],
    vertex_count: usize,
) -> Result<Vec<usize>, GeometryError> {
    if target_indices.is_empty() {
        return Err(GeometryError::EmptySeedIndices);
    }
    let mut unique = BTreeSet::new();
    for target in target_indices {
        if *target < 0 {
            return Err(GeometryError::NegativeSeedIndex { seed: *target });
        }
        let index = *target as usize;
        if index >= vertex_count {
            return Err(GeometryError::SeedIndexOutOfBounds {
                seed: *target,
                vertex_count,
            });
        }
        unique.insert(index);
    }
    Ok(unique.into_iter().collect())
}

fn validate_distance_map_inputs(
    contours: &[Vec<[f64; 2]>],
    width: usize,
    height: usize,
    origin: [f64; 2],
    pixel_size: [f64; 2],
) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("distance map width and height must be positive".to_string());
    }
    if !origin.iter().all(|value| value.is_finite()) {
        return Err("distance map origin must be finite".to_string());
    }
    if !pixel_size
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        return Err("distance map pixel_size values must be positive and finite".to_string());
    }
    if contours.is_empty() {
        return Err("contours must not be empty".to_string());
    }
    for contour in contours {
        if contour.len() < 2 {
            return Err("each contour must contain at least two points".to_string());
        }
        if !contour.iter().flatten().all(|value| value.is_finite()) {
            return Err("contour points must be finite".to_string());
        }
    }
    Ok(())
}

fn validate_mesh_distance_map_inputs(
    vertices: &[[f64; 3]],
    width: usize,
    height: usize,
    origin: [f64; 3],
    x_range: [f64; 3],
    y_range: [f64; 3],
    direction: [f64; 3],
    epsilon: f64,
) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("distance map width and height must be positive".to_string());
    }
    if vertices.is_empty() {
        return Err("mesh vertices must not be empty".to_string());
    }
    if !vertices.iter().flatten().all(|value| value.is_finite()) {
        return Err("mesh vertices must be finite".to_string());
    }
    for (name, vector) in [
        ("origin", origin),
        ("x_range", x_range),
        ("y_range", y_range),
        ("direction", direction),
    ] {
        if !vector.iter().all(|value| value.is_finite()) {
            return Err(format!("{name} must be finite"));
        }
    }
    if vector_length(x_range) <= 0.0 || vector_length(y_range) <= 0.0 {
        return Err("x_range and y_range must be non-zero".to_string());
    }
    if vector_length(direction) <= 0.0 {
        return Err("direction must be non-zero".to_string());
    }
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return Err("epsilon must be positive and finite".to_string());
    }
    Ok(())
}

fn mesh_distance_map_ray_origins(
    width: usize,
    height: usize,
    origin: [f64; 3],
    x_range: [f64; 3],
    y_range: [f64; 3],
) -> Vec<[f64; 3]> {
    (0..width * height)
        .map(|index| {
            let x = index % width;
            let y = index / width;
            let x_t = (x as f64 + 0.5) / width as f64;
            let y_t = (y as f64 + 0.5) / height as f64;
            [
                origin[0] + x_range[0] * x_t + y_range[0] * y_t,
                origin[1] + x_range[1] * x_t + y_range[1] * y_t,
                origin[2] + x_range[2] * x_t + y_range[2] * y_t,
            ]
        })
        .collect()
}

fn validate_iso_map(map: &DistanceMapGrid, iso_value: f32) -> Result<(), String> {
    if map.width == 0 || map.height == 0 {
        return Err("distance map width and height must be positive".to_string());
    }
    if map.values.len() != map.width * map.height {
        return Err("distance map values must match width * height".to_string());
    }
    if !iso_value.is_finite() {
        return Err("iso_value must be finite".to_string());
    }
    if !map.values.iter().all(|value| value.is_finite()) {
        return Err("distance map values must be finite".to_string());
    }
    if !map
        .pixel_size
        .iter()
        .all(|value| *value > 0.0 && value.is_finite())
    {
        return Err("distance map pixel_size values must be positive and finite".to_string());
    }
    if !map.origin.iter().all(|value| value.is_finite()) {
        return Err("distance map origin must be finite".to_string());
    }
    Ok(())
}

fn vector_length(vector: [f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn validate_merge_map(map: &DistanceMapGrid, side: &str) -> Result<(), String> {
    if map.width == 0 || map.height == 0 {
        return Err(format!(
            "{side} distance map width and height must be positive"
        ));
    }
    if map.values.len() != map.width * map.height {
        return Err(format!(
            "{side} distance map values must match width * height"
        ));
    }
    if !map.values.iter().all(|value| value.is_finite()) {
        return Err(format!("{side} distance map values must be finite"));
    }
    if !map
        .pixel_size
        .iter()
        .all(|value| *value > 0.0 && value.is_finite())
    {
        return Err(format!(
            "{side} distance map pixel_size values must be positive and finite"
        ));
    }
    if !map.origin.iter().all(|value| value.is_finite()) {
        return Err(format!("{side} distance map origin must be finite"));
    }
    Ok(())
}

fn map_value_at(map: &DistanceMapGrid, x: usize, y: usize) -> Option<f32> {
    if x >= map.width || y >= map.height {
        return None;
    }
    let value = map.values[y * map.width + x];
    if value == DISTANCE_MAP_NOT_VALID_VALUE {
        None
    } else {
        Some(value)
    }
}

fn merge_pair(
    left: Option<f32>,
    right: Option<f32>,
    operation: impl FnOnce(f32, f32) -> f32,
) -> f32 {
    match (left, right) {
        (Some(lhs), Some(rhs)) => operation(lhs, rhs),
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => DISTANCE_MAP_NOT_VALID_VALUE,
    }
}

pub(crate) fn distance_map_stats(values: &[f32]) -> (usize, f32, f32) {
    let mut valid_count = 0;
    let mut min_value = f32::MAX;
    let mut max_value = -f32::MAX;
    for value in values {
        if *value == DISTANCE_MAP_NOT_VALID_VALUE {
            continue;
        }
        valid_count += 1;
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    (valid_count, min_value, max_value)
}

fn negate_distance_map(map: &mut DistanceMapGrid) {
    for value in &mut map.values {
        if *value != DISTANCE_MAP_NOT_VALID_VALUE {
            *value = -*value;
        }
    }
    let (valid_count, min_value, max_value) = distance_map_stats(&map.values);
    map.valid_count = valid_count;
    map.min_value = min_value;
    map.max_value = max_value;
}

fn topology_plan(configuration: u8) -> &'static [usize] {
    match configuration {
        1 => &[1, 0],
        2 => &[0, 3],
        3 => &[1, 3],
        4 => &[2, 1],
        5 => &[2, 0],
        6 => &[0, 1, 2, 3],
        7 => &[2, 3],
        8 => &[3, 2],
        9 => &[1, 0, 3, 2],
        10 => &[0, 2],
        11 => &[1, 2],
        12 => &[3, 1],
        13 => &[3, 0],
        14 => &[0, 1],
        _ => &[],
    }
}

fn iso_edge_point(
    x: usize,
    y: usize,
    values: [f32; 4],
    edge: usize,
    iso_value: f32,
) -> Option<[f64; 2]> {
    let (from, to, p0, p1) = match edge {
        0 => (
            values[0],
            values[1],
            [x as f64, y as f64],
            [x as f64 + 1.0, y as f64],
        ),
        1 => (
            values[0],
            values[2],
            [x as f64, y as f64],
            [x as f64, y as f64 + 1.0],
        ),
        2 => (
            values[2],
            values[3],
            [x as f64, y as f64 + 1.0],
            [x as f64 + 1.0, y as f64 + 1.0],
        ),
        3 => (
            values[1],
            values[3],
            [x as f64 + 1.0, y as f64],
            [x as f64 + 1.0, y as f64 + 1.0],
        ),
        _ => return None,
    };
    if (from < iso_value) == (to < iso_value) {
        return None;
    }
    let ratio = ((iso_value - from).abs() / (to - from).abs()) as f64;
    Some([
        (1.0 - ratio) * p0[0] + ratio * p1[0] + 0.5,
        (1.0 - ratio) * p0[1] + ratio * p1[1] + 0.5,
    ])
}

fn map_to_world(map: &DistanceMapGrid, point: [f64; 2]) -> [f64; 2] {
    [
        map.origin[0] + point[0] * map.pixel_size[0],
        map.origin[1] + point[1] * map.pixel_size[1],
    ]
}

#[derive(Debug, Clone, Copy)]
struct Segment2 {
    a: [f64; 2],
    b: [f64; 2],
}

fn contour_segments(contours: &[Vec<[f64; 2]>], signed: bool) -> Result<Vec<Segment2>, String> {
    let mut segments = Vec::new();
    for contour in contours {
        for pair in contour.windows(2) {
            push_segment(&mut segments, pair[0], pair[1]);
        }
        if signed
            && contour_is_closed(contour)
            && !same_point(contour[0], contour[contour.len() - 1])
        {
            push_segment(&mut segments, contour[contour.len() - 1], contour[0]);
        }
    }
    Ok(segments)
}

fn push_segment(segments: &mut Vec<Segment2>, a: [f64; 2], b: [f64; 2]) {
    if ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)) > 1e-24 {
        segments.push(Segment2 { a, b });
    }
}

fn same_point(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12
}

fn contour_is_closed(contour: &[[f64; 2]]) -> bool {
    same_point(contour[0], contour[contour.len() - 1])
        || (contour.len() >= 3 && signed_area(contour).abs() > 1e-12)
}

fn signed_area(contour: &[[f64; 2]]) -> f64 {
    contour
        .windows(2)
        .map(|pair| pair[0][0] * pair[1][1] - pair[1][0] * pair[0][1])
        .sum::<f64>()
        * 0.5
}

fn nearest_segment_distance(point: [f64; 2], segments: &[Segment2]) -> f64 {
    segments
        .iter()
        .map(|segment| point_segment_distance_sq(point, *segment))
        .fold(f64::INFINITY, f64::min)
        .sqrt()
}

fn point_segment_distance_sq(point: [f64; 2], segment: Segment2) -> f64 {
    let edge = [segment.b[0] - segment.a[0], segment.b[1] - segment.a[1]];
    let length_sq = edge[0] * edge[0] + edge[1] * edge[1];
    if length_sq <= 1e-24 {
        return (point[0] - segment.a[0]).powi(2) + (point[1] - segment.a[1]).powi(2);
    }
    let t = (((point[0] - segment.a[0]) * edge[0] + (point[1] - segment.a[1]) * edge[1])
        / length_sq)
        .clamp(0.0, 1.0);
    let closest = [segment.a[0] + edge[0] * t, segment.a[1] + edge[1] * t];
    (point[0] - closest[0]).powi(2) + (point[1] - closest[1]).powi(2)
}

fn point_inside_closed_contours(point: [f64; 2], contours: &[Vec<[f64; 2]>]) -> bool {
    contours
        .iter()
        .filter(|contour| contour.len() >= 3 && contour_is_closed(contour))
        .fold(false, |inside, contour| {
            inside ^ point_inside_contour(point, contour)
        })
}

fn point_inside_contour(point: [f64; 2], contour: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let last = if same_point(contour[0], contour[contour.len() - 1]) {
        contour.len() - 1
    } else {
        contour.len()
    };
    for index in 0..last {
        let a = contour[index];
        let b = contour[(index + 1) % last];
        let crosses_y = (a[1] > point[1]) != (b[1] > point[1]);
        if crosses_y {
            let x_at_y = (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0];
            if point[0] < x_at_y {
                inside = !inside;
            }
        }
    }
    inside
}
