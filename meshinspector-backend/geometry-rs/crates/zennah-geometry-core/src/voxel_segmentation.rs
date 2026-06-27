#[rustfmt::skip]
use crate::{marching_tetrahedra, GeometryError, VoxelMaskMeshResult, VoxelSegmentationMeshResult, VoxelSegmentationOptions, VoxelSegmentationResult};
use std::collections::VecDeque;

const RESIDUAL_EPSILON: f64 = 1.0e-9;

#[derive(Clone, Debug)]
struct Edge {
    to: usize,
    rev: usize,
    cap: f64,
}

#[derive(Debug)]
struct Dinic {
    graph: Vec<Vec<Edge>>,
}

impl Dinic {
    fn new(nodes: usize) -> Self {
        Self {
            graph: vec![Vec::new(); nodes],
        }
    }

    fn add_edge(&mut self, from: usize, to: usize, cap: f64) {
        if cap <= RESIDUAL_EPSILON {
            return;
        }
        let reverse_to = self.graph[to].len();
        let reverse_from = self.graph[from].len();
        self.graph[from].push(Edge {
            to,
            rev: reverse_to,
            cap,
        });
        self.graph[to].push(Edge {
            to: from,
            rev: reverse_from,
            cap: 0.0,
        });
    }

    fn max_flow(&mut self, source: usize, sink: usize) -> f64 {
        let mut flow = 0.0;
        loop {
            let levels = self.levels(source);
            if levels[sink] < 0 {
                break;
            }
            let mut next_edges = vec![0_usize; self.graph.len()];
            loop {
                let pushed = self.push(source, sink, f64::INFINITY, &levels, &mut next_edges);
                if pushed <= RESIDUAL_EPSILON {
                    break;
                }
                flow += pushed;
            }
        }
        flow
    }

    fn levels(&self, source: usize) -> Vec<i32> {
        let mut levels = vec![-1; self.graph.len()];
        let mut queue = VecDeque::new();
        levels[source] = 0;
        queue.push_back(source);
        while let Some(node) = queue.pop_front() {
            for edge in &self.graph[node] {
                if edge.cap > RESIDUAL_EPSILON && levels[edge.to] < 0 {
                    levels[edge.to] = levels[node] + 1;
                    queue.push_back(edge.to);
                }
            }
        }
        levels
    }

    fn push(
        &mut self,
        node: usize,
        sink: usize,
        flow: f64,
        levels: &[i32],
        next_edges: &mut [usize],
    ) -> f64 {
        if node == sink {
            return flow;
        }
        while next_edges[node] < self.graph[node].len() {
            let edge_index = next_edges[node];
            let edge = self.graph[node][edge_index].clone();
            if edge.cap > RESIDUAL_EPSILON && levels[node] + 1 == levels[edge.to] {
                let pushed = self.push(edge.to, sink, flow.min(edge.cap), levels, next_edges);
                if pushed > RESIDUAL_EPSILON {
                    let reverse_index = edge.rev;
                    self.graph[node][edge_index].cap -= pushed;
                    self.graph[edge.to][reverse_index].cap += pushed;
                    return pushed;
                }
            }
            next_edges[node] += 1;
        }
        0.0
    }

    fn reachable_from(&self, source: usize) -> Vec<bool> {
        let mut reachable = vec![false; self.graph.len()];
        let mut queue = VecDeque::new();
        reachable[source] = true;
        queue.push_back(source);
        while let Some(node) = queue.pop_front() {
            for edge in &self.graph[node] {
                if edge.cap > RESIDUAL_EPSILON && !reachable[edge.to] {
                    reachable[edge.to] = true;
                    queue.push_back(edge.to);
                }
            }
        }
        reachable
    }
}

pub fn voxel_segmentation_values(
    values: &[f32],
    shape: [usize; 3],
    inside_seeds: &[[usize; 3]],
    outside_seeds: &[[usize; 3]],
    options: VoxelSegmentationOptions,
) -> Result<VoxelSegmentationResult, GeometryError> {
    validate_shape(shape)?;
    if inside_seeds.is_empty() {
        return Err(GeometryError::EmptySeedIndices);
    }
    if !options.exponent_modifier.is_finite() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "exponent_modifier",
            value: options.exponent_modifier.to_string(),
        });
    }
    let expected_values = voxel_count(shape)?;
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
    for seed in inside_seeds {
        validate_coord("inside_seeds", *seed, shape)?;
    }
    for seed in outside_seeds {
        validate_coord("outside_seeds", *seed, shape)?;
    }

    let (min_corner, dimensions) =
        volume_part_bounds(shape, inside_seeds, options.voxels_expansion);
    let part_count = voxel_count(dimensions)?;
    let mut part_values = Vec::with_capacity(part_count);
    for z in 0..dimensions[2] {
        for y in 0..dimensions[1] {
            for x in 0..dimensions[0] {
                let coord = [min_corner[0] + x, min_corner[1] + y, min_corner[2] + z];
                part_values.push(values[linear_index(coord, shape)]);
            }
        }
    }

    let mut inside_mask = vec![false; part_count];
    for seed in inside_seeds {
        let part_coord = [
            seed[0] - min_corner[0],
            seed[1] - min_corner[1],
            seed[2] - min_corner[2],
        ];
        inside_mask[linear_index(part_coord, dimensions)] = true;
    }

    let mut outside_mask = vec![false; part_count];
    for seed in outside_seeds {
        let clamped = clamp_to_part(*seed, min_corner, dimensions);
        outside_mask[linear_index(clamped, dimensions)] = true;
    }
    if options.include_boundary_outside {
        for z in 0..dimensions[2] {
            for y in 0..dimensions[1] {
                for x in 0..dimensions[0] {
                    if is_boundary([x, y, z], dimensions) {
                        outside_mask[linear_index([x, y, z], dimensions)] = true;
                    }
                }
            }
        }
    }
    for (index, is_inside) in inside_mask.iter().copied().enumerate() {
        if is_inside {
            outside_mask[index] = false;
        }
    }

    let selected = graph_cut_source_side(
        &part_values,
        dimensions,
        &inside_mask,
        &outside_mask,
        options.exponent_modifier,
    );

    let mut source_indices = Vec::new();
    let mut part_indices = Vec::new();
    let mut selected_coordinates = Vec::new();
    let mut selected_values = Vec::new();
    for (part_index, is_selected) in selected.into_iter().enumerate() {
        if !is_selected {
            continue;
        }
        let local = linear_coord(part_index, dimensions);
        let coord = [
            min_corner[0] + local[0],
            min_corner[1] + local[1],
            min_corner[2] + local[2],
        ];
        source_indices.push(linear_index(coord, shape));
        part_indices.push(part_index);
        selected_coordinates.push(coord);
        selected_values.push(values[linear_index(coord, shape)]);
    }

    Ok(VoxelSegmentationResult {
        min_corner,
        dimensions,
        source_indices,
        part_indices,
        selected_coordinates,
        selected_values,
    })
}

pub fn voxel_segmentation_mesh_values(
    values: &[f32],
    shape: [usize; 3],
    inside_seeds: &[[usize; 3]],
    outside_seeds: &[[usize; 3]],
    options: VoxelSegmentationOptions,
    voxel_size: [f64; 3],
) -> Result<VoxelSegmentationMeshResult, GeometryError> {
    validate_voxel_size(voxel_size)?;
    let segmentation =
        voxel_segmentation_values(values, shape, inside_seeds, outside_seeds, options)?;
    if segmentation.part_indices.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "segmentation",
            value: "empty mask".to_string(),
        });
    }

    let mut mask_values = vec![1.0_f32; voxel_count(segmentation.dimensions)?];
    for part_index in &segmentation.part_indices {
        mask_values[*part_index] = 0.0;
    }
    let mut mesh = marching_tetrahedra(
        &mask_values,
        [
            segmentation.min_corner[0] as f64,
            segmentation.min_corner[1] as f64,
            segmentation.min_corner[2] as f64,
        ],
        segmentation.dimensions,
        1.0,
        0.5,
    )?;
    if mesh.faces.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "segmentation_mesh",
            value: "empty mesh".to_string(),
        });
    }
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] *= voxel_size[axis];
        }
    }

    Ok(VoxelSegmentationMeshResult {
        segmentation,
        vertices: mesh.vertices,
        faces: mesh.faces,
    })
}

pub fn voxel_mask_to_mesh_values(
    values: &[f32],
    shape: [usize; 3],
    mask_coordinates: &[[usize; 3]],
    voxel_size: [f64; 3],
    mask_expansion: usize,
    smooth_band_radius: usize,
) -> Result<VoxelMaskMeshResult, GeometryError> {
    validate_shape(shape)?;
    validate_voxel_size(voxel_size)?;
    if mask_coordinates.is_empty() {
        return Err(GeometryError::EmptySeedIndices);
    }
    let expected_values = voxel_count(shape)?;
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
    for coord in mask_coordinates {
        validate_coord("mask_coordinates", *coord, shape)?;
    }

    let mut mask = vec![false; expected_values];
    for coord in mask_coordinates {
        mask[linear_index(*coord, shape)] = true;
    }
    let mut expanded_mask = mask.clone();
    expand_voxel_mask(&mut expanded_mask, shape, mask_expansion);

    let (min_corner, dimensions) = mask_bounds(shape, &expanded_mask)?;
    let part_count = voxel_count(dimensions)?;
    let mut part_values = Vec::with_capacity(part_count);
    let mut part_mask = vec![false; part_count];
    let mut source_indices = Vec::new();
    let mut part_indices = Vec::new();
    let mut selected_coordinates = Vec::new();
    for z in 0..dimensions[2] {
        for y in 0..dimensions[1] {
            for x in 0..dimensions[0] {
                let local = [x, y, z];
                let coord = [min_corner[0] + x, min_corner[1] + y, min_corner[2] + z];
                let source_index = linear_index(coord, shape);
                let part_index = linear_index(local, dimensions);
                part_values.push(values[source_index]);
                if mask[source_index] {
                    part_mask[part_index] = true;
                    source_indices.push(source_index);
                    part_indices.push(part_index);
                    selected_coordinates.push(coord);
                }
            }
        }
    }

    let prepared =
        smooth_mask_volume_values(&part_values, dimensions, &part_mask, smooth_band_radius)?;
    let mesh_values = prepared
        .into_iter()
        .map(|value| 1.0_f32 - value)
        .collect::<Vec<_>>();
    let mut mesh = marching_tetrahedra(
        &mesh_values,
        [
            min_corner[0] as f64,
            min_corner[1] as f64,
            min_corner[2] as f64,
        ],
        dimensions,
        1.0,
        0.5,
    )?;
    if mesh.faces.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "voxel_mask_to_mesh",
            value: "empty mesh".to_string(),
        });
    }
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] *= voxel_size[axis];
        }
    }

    Ok(VoxelMaskMeshResult {
        min_corner,
        dimensions,
        source_indices,
        part_indices,
        selected_coordinates,
        vertices: mesh.vertices,
        faces: mesh.faces,
    })
}

fn graph_cut_source_side(
    values: &[f32],
    shape: [usize; 3],
    inside_mask: &[bool],
    outside_mask: &[bool],
    exponent_modifier: f32,
) -> Vec<bool> {
    let voxel_count = values.len();
    let source = voxel_count;
    let sink = voxel_count + 1;
    let mut graph = Dinic::new(voxel_count + 2);
    let terminal_capacity = f64::from(f32::MAX) / 4.0;

    for index in 0..voxel_count {
        if inside_mask[index] {
            graph.add_edge(source, index, terminal_capacity);
        }
        if outside_mask[index] {
            graph.add_edge(index, sink, terminal_capacity);
        }
        for neighbor in neighbors(index, shape) {
            if outside_mask[index] {
                continue;
            }
            if inside_mask[neighbor] {
                continue;
            }
            if inside_mask[index] && outside_mask[neighbor] {
                continue;
            }
            graph.add_edge(
                index,
                neighbor,
                edge_capacity(values[index], values[neighbor], exponent_modifier),
            );
        }
    }

    graph.max_flow(source, sink);
    let reachable = graph.reachable_from(source);
    reachable.into_iter().take(voxel_count).collect()
}

fn edge_capacity(density_from: f32, density_to: f32, exponent_modifier: f32) -> f64 {
    let max_capacity = f32::MAX / 10.0;
    if exponent_modifier == 0.0 {
        return 1.0;
    }
    let max_delta = max_capacity.ln() / exponent_modifier.abs();
    let delta = density_to - density_from;
    if (exponent_modifier > 0.0 && delta > max_delta)
        || (exponent_modifier < 0.0 && delta < -max_delta)
    {
        return f64::from(max_capacity);
    }
    let capacity = (exponent_modifier * delta).exp();
    if capacity.is_finite() {
        f64::from(capacity)
    } else {
        f64::from(max_capacity)
    }
}

fn volume_part_bounds(
    shape: [usize; 3],
    inside_seeds: &[[usize; 3]],
    voxels_expansion: usize,
) -> ([usize; 3], [usize; 3]) {
    let mut min_corner = inside_seeds[0];
    let mut max_corner = inside_seeds[0];
    for seed in inside_seeds.iter().copied().skip(1) {
        for axis in 0..3 {
            min_corner[axis] = min_corner[axis].min(seed[axis]);
            max_corner[axis] = max_corner[axis].max(seed[axis]);
        }
    }
    for axis in 0..3 {
        min_corner[axis] = min_corner[axis].saturating_sub(voxels_expansion);
        max_corner[axis] = max_corner[axis]
            .saturating_add(voxels_expansion)
            .min(shape[axis] - 1);
    }
    let dimensions = [
        max_corner[0] - min_corner[0] + 1,
        max_corner[1] - min_corner[1] + 1,
        max_corner[2] - min_corner[2] + 1,
    ];
    (min_corner, dimensions)
}

fn clamp_to_part(coord: [usize; 3], min_corner: [usize; 3], dimensions: [usize; 3]) -> [usize; 3] {
    [
        coord[0]
            .saturating_sub(min_corner[0])
            .min(dimensions[0] - 1),
        coord[1]
            .saturating_sub(min_corner[1])
            .min(dimensions[1] - 1),
        coord[2]
            .saturating_sub(min_corner[2])
            .min(dimensions[2] - 1),
    ]
}

fn neighbors(index: usize, shape: [usize; 3]) -> Vec<usize> {
    let coord = linear_coord(index, shape);
    let mut out = Vec::with_capacity(6);
    for axis in 0..3 {
        if coord[axis] > 0 {
            let mut prev = coord;
            prev[axis] -= 1;
            out.push(linear_index(prev, shape));
        }
        if coord[axis] + 1 < shape[axis] {
            let mut next = coord;
            next[axis] += 1;
            out.push(linear_index(next, shape));
        }
    }
    out
}

fn is_boundary(coord: [usize; 3], shape: [usize; 3]) -> bool {
    (0..3).any(|axis| coord[axis] == 0 || coord[axis] + 1 == shape[axis])
}

fn mask_bounds(
    shape: [usize; 3],
    mask: &[bool],
) -> Result<([usize; 3], [usize; 3]), GeometryError> {
    let first_index = mask
        .iter()
        .position(|selected| *selected)
        .ok_or(GeometryError::EmptySeedIndices)?;
    let first = linear_coord(first_index, shape);
    let mut min_corner = first;
    let mut max_corner = first;
    for (index, selected) in mask.iter().copied().enumerate().skip(first_index + 1) {
        if !selected {
            continue;
        }
        let coord = linear_coord(index, shape);
        for axis in 0..3 {
            min_corner[axis] = min_corner[axis].min(coord[axis]);
            max_corner[axis] = max_corner[axis].max(coord[axis]);
        }
    }
    let dimensions = [
        max_corner[0] - min_corner[0] + 1,
        max_corner[1] - min_corner[1] + 1,
        max_corner[2] - min_corner[2] + 1,
    ];
    Ok((min_corner, dimensions))
}

fn smooth_mask_volume_values(
    values: &[f32],
    shape: [usize; 3],
    mask: &[bool],
    smooth_band_radius: usize,
) -> Result<Vec<f32>, GeometryError> {
    let inside_count = mask.iter().filter(|selected| **selected).count();
    let outside_count = mask.len().saturating_sub(inside_count);
    if inside_count == 0 || outside_count == 0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "mask_coordinates",
            value: "mask must contain inside and outside voxels after expansion".to_string(),
        });
    }

    let mut inside_sum = 0.0_f64;
    let mut outside_sum = 0.0_f64;
    for (value, selected) in values.iter().copied().zip(mask.iter().copied()) {
        if selected {
            inside_sum += f64::from(value);
        } else {
            outside_sum += f64::from(value);
        }
    }
    let inside_avg = inside_sum / inside_count as f64;
    let outside_avg = outside_sum / outside_count as f64;
    let range = inside_avg - outside_avg;
    if !range.is_finite() || range.abs() <= f64::EPSILON {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "density_range",
            value: format!("inside average {inside_avg}, outside average {outside_avg}"),
        });
    }

    let mut expanded_mask = mask.to_vec();
    let mut shrunken_mask = mask.to_vec();
    expand_voxel_mask(&mut expanded_mask, shape, smooth_band_radius);
    shrink_voxel_mask(&mut shrunken_mask, shape, smooth_band_radius);

    let mut prepared = Vec::with_capacity(values.len());
    for (index, value) in values.iter().copied().enumerate() {
        if shrunken_mask[index] {
            prepared.push(1.0);
        } else if expanded_mask[index] {
            prepared.push(((f64::from(value) - outside_avg) / range).clamp(0.0, 1.0) as f32);
        } else {
            prepared.push(0.0);
        }
    }
    Ok(prepared)
}

fn expand_voxel_mask(mask: &mut [bool], shape: [usize; 3], expansion: usize) {
    for _ in 0..expansion {
        let mut next = mask.to_vec();
        for index in 0..mask.len() {
            if mask[index] {
                continue;
            }
            if neighbors(index, shape)
                .into_iter()
                .any(|neighbor| mask[neighbor])
            {
                next[index] = true;
            }
        }
        mask.copy_from_slice(&next);
    }
}

fn shrink_voxel_mask(mask: &mut [bool], shape: [usize; 3], shrinkage: usize) {
    for _ in 0..shrinkage {
        let mut next = mask.to_vec();
        for index in 0..mask.len() {
            if !mask[index] {
                continue;
            }
            let coord = linear_coord(index, shape);
            let touches_boundary =
                (0..3).any(|axis| coord[axis] == 0 || coord[axis] + 1 == shape[axis]);
            if touches_boundary
                || neighbors(index, shape)
                    .into_iter()
                    .any(|neighbor| !mask[neighbor])
            {
                next[index] = false;
            }
        }
        mask.copy_from_slice(&next);
    }
}

fn validate_shape(shape: [usize; 3]) -> Result<(), GeometryError> {
    if shape.iter().any(|value| *value == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    Ok(())
}

fn validate_voxel_size(voxel_size: [f64; 3]) -> Result<(), GeometryError> {
    if voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "voxel_size",
            value: format!("{voxel_size:?}"),
        });
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

fn voxel_count(shape: [usize; 3]) -> Result<usize, GeometryError> {
    shape
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge { shape })
}

fn linear_index(coord: [usize; 3], shape: [usize; 3]) -> usize {
    coord[0] + coord[1] * shape[0] + coord[2] * shape[0] * shape[1]
}

fn linear_coord(index: usize, shape: [usize; 3]) -> [usize; 3] {
    let xy = shape[0] * shape[1];
    let z = index / xy;
    let rem = index % xy;
    let y = rem / shape[0];
    let x = rem % shape[0];
    [x, y, z]
}
