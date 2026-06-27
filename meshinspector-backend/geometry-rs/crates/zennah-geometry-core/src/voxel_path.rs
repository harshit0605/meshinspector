use crate::{
    GeometryError, VoxelPathBuildFourEntry, VoxelPathBuildFourResult, VoxelPathMetric,
    VoxelPathOptions, VoxelPathPlane, VoxelPathResult,
};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

const INVALID_QUARTER_MASK: u8 = 0;

#[derive(Clone, Copy, Debug)]
struct QuarterParams {
    start: [isize; 3],
    stop: [isize; 3],
    diff: [isize; 3],
    abs_diff: [isize; 3],
}

#[derive(Clone, Copy, Debug)]
struct PathInfo {
    prev: Option<usize>,
    metric: f32,
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    voxel: usize,
    metric: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.voxel == other.voxel && self.metric.to_bits() == other.metric.to_bits()
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .metric
            .total_cmp(&self.metric)
            .then_with(|| other.voxel.cmp(&self.voxel))
    }
}

pub fn voxel_path_values(
    values: &[f32],
    shape: [usize; 3],
    start: [usize; 3],
    finish: [usize; 3],
    metric: VoxelPathMetric,
    options: VoxelPathOptions,
) -> Result<VoxelPathResult, GeometryError> {
    validate_shape(shape)?;
    let expected_values = shape
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge { shape })?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    for (index, value) in values.iter().copied().enumerate() {
        if value.is_nan() {
            return Err(GeometryError::InvalidVoxelValue { index, value });
        }
    }
    validate_coord("start", start, shape)?;
    validate_coord("finish", finish, shape)?;
    if !options.max_dist_ratio.is_finite() || options.max_dist_ratio <= 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_dist_ratio",
            value: options.max_dist_ratio.to_string(),
        });
    }
    if !options.exponent_modifier.is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "exponent_modifier",
            value: options.exponent_modifier.to_string(),
        });
    }
    if options.quarters_mask == INVALID_QUARTER_MASK {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "quarters_mask",
            value: options.quarters_mask.to_string(),
        });
    }

    let start_index = linear_index(start, shape);
    let finish_index = linear_index(finish, shape);
    if start_index == finish_index {
        return Ok(VoxelPathResult {
            voxel_indices: vec![start_index],
            coordinates: vec![start],
            total_metric: 0.0,
        });
    }

    let quarter_params = setup_quarter_params(shape, start_index, finish_index);
    let max_dist_sq =
        dist_sq(shape, start_index, finish_index) * options.max_dist_ratio * options.max_dist_ratio;
    let mut infos: Vec<Option<PathInfo>> = vec![None; expected_values];
    let mut queue = BinaryHeap::new();
    infos[finish_index] = Some(PathInfo {
        prev: None,
        metric: 0.0,
    });
    add_neighbor_steps(
        values,
        shape,
        metric,
        options,
        quarter_params,
        max_dist_sq,
        finish_index,
        0.0,
        &mut infos,
        &mut queue,
    );

    while let Some(candidate) = queue.pop() {
        let Some(info) = infos[candidate.voxel] else {
            continue;
        };
        if info.metric < candidate.metric {
            continue;
        }
        if candidate.voxel == start_index {
            break;
        }
        add_neighbor_steps(
            values,
            shape,
            metric,
            options,
            quarter_params,
            max_dist_sq,
            candidate.voxel,
            candidate.metric,
            &mut infos,
            &mut queue,
        );
    }

    let Some(start_info) = infos[start_index] else {
        return Ok(VoxelPathResult::default());
    };
    let mut voxel_indices = Vec::new();
    let mut coordinates = Vec::new();
    let mut current = start_index;
    loop {
        voxel_indices.push(current);
        coordinates.push(linear_coord(shape, current));
        let Some(info) = infos[current] else {
            break;
        };
        let Some(prev) = info.prev else {
            break;
        };
        current = prev;
    }

    Ok(VoxelPathResult {
        voxel_indices,
        coordinates,
        total_metric: start_info.metric,
    })
}

pub fn voxel_path_build_four_values(
    values: &[f32],
    shape: [usize; 3],
    start: [usize; 3],
    finish: [usize; 3],
    metric: VoxelPathMetric,
    options: VoxelPathOptions,
) -> Result<VoxelPathBuildFourResult, GeometryError> {
    let mut paths = Vec::with_capacity(4);
    for quarters_mask in [
        VoxelPathOptions::QUARTER_LEFT_LEFT,
        VoxelPathOptions::QUARTER_LEFT_RIGHT,
        VoxelPathOptions::QUARTER_RIGHT_LEFT,
        VoxelPathOptions::QUARTER_RIGHT_RIGHT,
    ] {
        let mut quarter_options = options;
        quarter_options.quarters_mask = quarters_mask;
        let path = voxel_path_values(values, shape, start, finish, metric, quarter_options)?;
        paths.push(VoxelPathBuildFourEntry {
            quarters_mask,
            path,
        });
    }
    Ok(VoxelPathBuildFourResult { paths })
}

#[allow(clippy::too_many_arguments)]
fn add_neighbor_steps(
    values: &[f32],
    shape: [usize; 3],
    metric: VoxelPathMetric,
    options: VoxelPathOptions,
    quarter_params: QuarterParams,
    max_dist_sq: f32,
    back: usize,
    origin_metric: f32,
    infos: &mut [Option<PathInfo>],
    queue: &mut BinaryHeap<Candidate>,
) {
    for candidate in neighbors(shape, back) {
        let step_metric = edge_metric(
            values,
            shape,
            metric,
            options,
            quarter_params,
            max_dist_sq,
            back,
            candidate,
        );
        let candidate_metric = origin_metric + step_metric;
        if candidate_metric >= f32::MAX || candidate_metric.is_nan() {
            continue;
        }
        let previous_metric = infos[candidate].map(|info| info.metric).unwrap_or(f32::MAX);
        if previous_metric > candidate_metric {
            infos[candidate] = Some(PathInfo {
                prev: Some(back),
                metric: candidate_metric,
            });
            queue.push(Candidate {
                voxel: candidate,
                metric: candidate_metric,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn edge_metric(
    values: &[f32],
    shape: [usize; 3],
    metric: VoxelPathMetric,
    options: VoxelPathOptions,
    quarter_params: QuarterParams,
    max_dist_sq: f32,
    first: usize,
    second: usize,
) -> f32 {
    if let Some(axis) = options.plane.axis() {
        let next = linear_coord(shape, second);
        if next[axis] != quarter_params.start[axis] as usize {
            return f32::MAX;
        }
    }
    if !is_in_quarter(shape, quarter_params, second, options.quarters_mask) {
        return f32::MAX;
    }
    if dist_sq(
        shape,
        linear_index_usize(quarter_params.start, shape),
        second,
    ) + dist_sq(
        shape,
        second,
        linear_index_usize(quarter_params.stop, shape),
    ) > max_dist_sq
    {
        return f32::MAX;
    }

    match metric {
        VoxelPathMetric::Difference => {
            let val_start = values[linear_index_usize(quarter_params.start, shape)];
            let val_stop = values[linear_index_usize(quarter_params.stop, shape)];
            let val_first = values[first];
            let val_second = values[second];
            (val_start - val_first).abs()
                + (val_stop - val_first).abs()
                + (val_start - val_second).abs()
                + (val_stop - val_second).abs()
        }
        VoxelPathMetric::Exponent => {
            (options.exponent_modifier * (values[first] + values[second])).exp()
        }
    }
}

fn validate_shape(shape: [usize; 3]) -> Result<(), GeometryError> {
    if shape.iter().any(|value| *value == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    Ok(())
}

fn validate_coord(
    field: &'static str,
    coord: [usize; 3],
    shape: [usize; 3],
) -> Result<(), GeometryError> {
    if coord
        .iter()
        .zip(shape)
        .any(|(value, bound)| *value >= bound)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("{coord:?} outside shape {shape:?}"),
        });
    }
    Ok(())
}

fn setup_quarter_params(shape: [usize; 3], start: usize, stop: usize) -> QuarterParams {
    let start_coord = linear_coord(shape, start);
    let stop_coord = linear_coord(shape, stop);
    let start = [
        start_coord[0] as isize,
        start_coord[1] as isize,
        start_coord[2] as isize,
    ];
    let stop = [
        stop_coord[0] as isize,
        stop_coord[1] as isize,
        stop_coord[2] as isize,
    ];
    let diff = [stop[0] - start[0], stop[1] - start[1], stop[2] - start[2]];
    let abs_diff = [diff[0].abs(), diff[1].abs(), diff[2].abs()];
    QuarterParams {
        start,
        stop,
        diff,
        abs_diff,
    }
}

fn is_in_quarter(shape: [usize; 3], params: QuarterParams, next: usize, quarters_mask: u8) -> bool {
    if quarters_mask & VoxelPathOptions::QUARTER_ALL == VoxelPathOptions::QUARTER_ALL {
        return true;
    }

    let next = linear_coord(shape, next);
    let main_axis = max_axis(params.abs_diff);
    if params.diff[main_axis] == 0 {
        return true;
    }
    let ratio =
        (next[main_axis] as f32 - params.start[main_axis] as f32) / params.diff[main_axis] as f32;
    let mut other1_axis = (main_axis + 1) % 3;
    let mut other2_axis = (main_axis + 2) % 3;
    if params.abs_diff[other2_axis] > params.abs_diff[other1_axis] {
        std::mem::swap(&mut other1_axis, &mut other2_axis);
    }

    let coord_on_axis = [
        params.diff[0] as f32 * ratio + params.start[0] as f32,
        params.diff[1] as f32 * ratio + params.start[1] as f32,
        params.diff[2] as f32 * ratio + params.start[2] as f32,
    ];
    let first_left = (next[other1_axis] as i32) < coord_on_axis[other1_axis] as i32;
    let second_left = (next[other2_axis] as i32) < coord_on_axis[other2_axis] as i32;
    let current_quarter = match (first_left, second_left) {
        (true, true) => VoxelPathOptions::QUARTER_LEFT_LEFT,
        (true, false) => VoxelPathOptions::QUARTER_LEFT_RIGHT,
        (false, true) => VoxelPathOptions::QUARTER_RIGHT_LEFT,
        (false, false) => VoxelPathOptions::QUARTER_RIGHT_RIGHT,
    };

    let start_diff = [
        next[0] as isize - params.start[0],
        next[1] as isize - params.start[1],
        next[2] as isize - params.start[2],
    ];
    let stop_diff = [
        next[0] as isize - params.stop[0],
        next[1] as isize - params.stop[1],
        next[2] as isize - params.stop[2],
    ];
    let start_diff_sq = length_sq(start_diff);
    let stop_diff_sq = length_sq(stop_diff);
    if start_diff_sq < 4 || stop_diff_sq < 4 {
        return true;
    }

    current_quarter & quarters_mask != 0
}

fn max_axis(values: [isize; 3]) -> usize {
    if values[1] > values[0] && values[1] >= values[2] {
        1
    } else if values[2] > values[0] && values[2] > values[1] {
        2
    } else {
        0
    }
}

fn length_sq(values: [isize; 3]) -> isize {
    values[0] * values[0] + values[1] * values[1] + values[2] * values[2]
}

fn dist_sq(shape: [usize; 3], first: usize, second: usize) -> f32 {
    let a = linear_coord(shape, first);
    let b = linear_coord(shape, second);
    let dx = a[0] as f32 - b[0] as f32;
    let dy = a[1] as f32 - b[1] as f32;
    let dz = a[2] as f32 - b[2] as f32;
    dx * dx + dy * dy + dz * dz
}

fn neighbors(shape: [usize; 3], index: usize) -> Vec<usize> {
    let coord = linear_coord(shape, index);
    let mut candidates = Vec::with_capacity(6);
    if coord[0] > 0 {
        candidates.push(index - 1);
    }
    if coord[0] < shape[0] - 1 {
        candidates.push(index + 1);
    }
    if coord[1] > 0 {
        candidates.push(index - shape[0]);
    }
    if coord[1] < shape[1] - 1 {
        candidates.push(index + shape[0]);
    }
    let dim_xy = shape[0] * shape[1];
    if coord[2] > 0 {
        candidates.push(index - dim_xy);
    }
    if coord[2] < shape[2] - 1 {
        candidates.push(index + dim_xy);
    }
    candidates
}

fn linear_index(coord: [usize; 3], shape: [usize; 3]) -> usize {
    coord[0] + coord[1] * shape[0] + coord[2] * shape[0] * shape[1]
}

fn linear_index_usize(coord: [isize; 3], shape: [usize; 3]) -> usize {
    linear_index(
        [coord[0] as usize, coord[1] as usize, coord[2] as usize],
        shape,
    )
}

fn linear_coord(shape: [usize; 3], index: usize) -> [usize; 3] {
    let dim_xy = shape[0] * shape[1];
    let z = index / dim_xy;
    let sum_z = index % dim_xy;
    let y = sum_z / shape[0];
    let x = sum_z % shape[0];
    [x, y, z]
}

impl VoxelPathPlane {
    fn axis(self) -> Option<usize> {
        match self {
            VoxelPathPlane::YZ => Some(0),
            VoxelPathPlane::ZX => Some(1),
            VoxelPathPlane::XY => Some(2),
            VoxelPathPlane::None => None,
        }
    }
}
