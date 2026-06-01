use crate::math::{add, distance_sq, dot, scale, sub};
use crate::mesh::{
    safe_normalize_vector, validate_faces, vertex_neighbor_list, vertex_normals_from_faces,
};
use crate::{GeometryError, SmoothFalloffOptions};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn laplacian_smooth_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    iterations: i64,
    strength: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    smooth_vertices_with_weights(vertices, faces_i64, None, iterations, strength, 0.0)
}

pub fn weighted_laplacian_smooth_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    weights: &[f32],
    iterations: i64,
    strength: f64,
    active_threshold: f32,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    if weights.len() != vertices.len() {
        return Err(GeometryError::WeightCountDoesNotMatchVertices {
            weights: weights.len(),
            vertices: vertices.len(),
        });
    }
    smooth_vertices_with_weights(
        vertices,
        faces_i64,
        Some(weights),
        iterations,
        strength,
        active_threshold,
    )
}

pub fn falloff_weights(
    vertices: &[[f64; 3]],
    seed_indices: &[i64],
    falloff_mm: f64,
    cutoff_multiplier: f64,
) -> Result<Vec<f32>, GeometryError> {
    let seeds = validate_seed_indices(seed_indices, vertices.len())?;
    let scale = falloff_mm.max(1e-3);
    let cutoff = falloff_mm * cutoff_multiplier;
    let output = vertices
        .par_iter()
        .map(|vertex| {
            let best_distance_sq = seeds
                .iter()
                .map(|seed| distance_sq(*vertex, vertices[*seed]))
                .fold(f64::INFINITY, f64::min);
            let distance = best_distance_sq.sqrt();
            if distance > cutoff {
                0.0
            } else {
                (-0.5 * (distance / scale).powi(2)).exp() as f32
            }
        })
        .collect();
    Ok(output)
}

pub fn smooth_vertices_with_falloff(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    seed_indices: &[i64],
    options: SmoothFalloffOptions,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let weights = falloff_weights(
        vertices,
        seed_indices,
        options.falloff_mm,
        options.cutoff_multiplier,
    )?;
    weighted_laplacian_smooth_vertices(
        vertices,
        faces_i64,
        &weights,
        options.iterations,
        options.strength,
        options.active_threshold,
    )
}

pub fn outward_directions(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if vertices.is_empty() {
        return Ok(Vec::new());
    }
    if faces.is_empty() {
        return Ok(vec![[0.0; 3]; vertices.len()]);
    }

    let normals = vertex_normals_from_faces(vertices, &faces);
    let center = scale(
        vertices.iter().copied().fold([0.0_f64; 3], add),
        1.0 / vertices.len() as f64,
    );

    let directions = vertices
        .par_iter()
        .enumerate()
        .map(|(vertex_index, vertex)| {
            let normal = normals[vertex_index];
            let toward_center = safe_normalize_vector(sub(center, *vertex));
            if dot(normal, toward_center) >= 0.0 {
                scale(normal, -1.0)
            } else {
                normal
            }
        })
        .collect();
    Ok(directions)
}

pub fn local_offset_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    seed_indices: &[i64],
    falloff_mm: f64,
    amount_mm: f64,
    cutoff_multiplier: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let directions = outward_directions(vertices, faces_i64)?;
    let weights = falloff_weights(vertices, seed_indices, falloff_mm, cutoff_multiplier)?;
    let displaced = vertices
        .par_iter()
        .enumerate()
        .map(|(vertex_index, vertex)| {
            add(
                *vertex,
                scale(
                    directions[vertex_index],
                    amount_mm * weights[vertex_index] as f64,
                ),
            )
        })
        .collect();
    Ok(displaced)
}

pub fn brush_stroke_weights(
    vertices: &[[f64; 3]],
    seed_indices: &[i64],
    falloff_mm: f64,
    mask_enabled: bool,
    mask_indices: &[i64],
    protected_indices: &[i64],
    cutoff_multiplier: f64,
) -> Result<Vec<f32>, GeometryError> {
    validate_brush_indices("mask", mask_indices, vertices.len())?;
    validate_brush_indices("protected", protected_indices, vertices.len())?;
    let mut weights = falloff_weights(vertices, seed_indices, falloff_mm, cutoff_multiplier)?;
    apply_brush_weight_constraints(
        &mut weights,
        vertices.len(),
        mask_enabled,
        mask_indices,
        protected_indices,
    );
    Ok(weights)
}

#[allow(clippy::too_many_arguments)]
pub fn region_brush_masks(
    operation: i64,
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    allowed_operation_offsets: &[i64],
    allowed_operations: &[i64],
    editable_region_ids: &[String],
    protected_region_ids: &[String],
    editable_filter_enabled: bool,
    protected_filter_enabled: bool,
    respect_allowed_operations: bool,
) -> Result<(Vec<i64>, Vec<i64>), GeometryError> {
    validate_brush_operation(operation)?;
    validate_brush_index_offsets(
        "region vertex",
        vertex_offsets,
        vertex_indices.len(),
        region_ids.len(),
    )?;
    validate_brush_index_offsets(
        "region operation",
        allowed_operation_offsets,
        allowed_operations.len(),
        region_ids.len(),
    )?;

    let region_by_id = region_index_by_id(region_ids);
    let editable_regions = if editable_filter_enabled {
        let mut selected = select_region_indices(&region_by_id, editable_region_ids)?;
        if respect_allowed_operations {
            selected.retain(|region_index| {
                region_allows_operation(
                    *region_index,
                    operation,
                    allowed_operation_offsets,
                    allowed_operations,
                )
            });
        }
        selected
    } else {
        (0..region_ids.len())
            .filter(|region_index| {
                !respect_allowed_operations
                    || region_allows_operation(
                        *region_index,
                        operation,
                        allowed_operation_offsets,
                        allowed_operations,
                    )
            })
            .collect()
    };

    let mut protected_regions = Vec::new();
    if respect_allowed_operations {
        protected_regions.extend((0..region_ids.len()).filter(|region_index| {
            !region_allows_operation(
                *region_index,
                operation,
                allowed_operation_offsets,
                allowed_operations,
            )
        }));
    }
    if protected_filter_enabled {
        protected_regions.extend(select_region_indices(&region_by_id, protected_region_ids)?);
    }

    Ok((
        union_region_vertex_indices(&editable_regions, vertex_offsets, vertex_indices),
        union_region_vertex_indices(&protected_regions, vertex_offsets, vertex_indices),
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn apply_brush_strokes(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    operations: &[i64],
    seed_offsets: &[i64],
    seed_indices: &[i64],
    mask_enabled: &[i64],
    mask_offsets: &[i64],
    mask_indices: &[i64],
    protected_offsets: &[i64],
    protected_indices: &[i64],
    amounts_mm: &[f64],
    falloffs_mm: &[f64],
    iterations: &[i64],
    strengths: &[f64],
    cutoff_multiplier: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    validate_brush_strokes(
        operations,
        seed_offsets,
        seed_indices.len(),
        amounts_mm,
        falloffs_mm,
        iterations,
        strengths,
    )?;
    if mask_enabled.len() != operations.len() {
        return Err(GeometryError::BrushStrokeCountMismatch {
            operations: operations.len(),
            amounts: amounts_mm.len(),
            falloffs: falloffs_mm.len(),
            iterations: iterations.len(),
            strengths: strengths.len(),
        });
    }
    validate_brush_index_offsets("mask", mask_offsets, mask_indices.len(), operations.len())?;
    validate_brush_index_offsets(
        "protected",
        protected_offsets,
        protected_indices.len(),
        operations.len(),
    )?;
    validate_brush_indices("mask", mask_indices, vertices.len())?;
    validate_brush_indices("protected", protected_indices, vertices.len())?;

    let mut output = vertices.to_vec();
    for stroke_index in 0..operations.len() {
        let seed_start = seed_offsets[stroke_index] as usize;
        let seed_end = seed_offsets[stroke_index + 1] as usize;
        let stroke_seeds = &seed_indices[seed_start..seed_end];
        let mask_start = mask_offsets[stroke_index] as usize;
        let mask_end = mask_offsets[stroke_index + 1] as usize;
        let stroke_mask = &mask_indices[mask_start..mask_end];
        let protected_start = protected_offsets[stroke_index] as usize;
        let protected_end = protected_offsets[stroke_index + 1] as usize;
        let stroke_protected = &protected_indices[protected_start..protected_end];
        let mut weights = falloff_weights(
            &output,
            stroke_seeds,
            falloffs_mm[stroke_index],
            cutoff_multiplier,
        )?;
        apply_brush_weight_constraints(
            &mut weights,
            vertices.len(),
            mask_enabled[stroke_index] != 0,
            stroke_mask,
            stroke_protected,
        );
        output = match operations[stroke_index] {
            0 => local_offset_vertices_with_weights(
                &output,
                faces_i64,
                &weights,
                amounts_mm[stroke_index],
            )?,
            1 => local_offset_vertices_with_weights(
                &output,
                faces_i64,
                &weights,
                -amounts_mm[stroke_index],
            )?,
            2 => weighted_laplacian_smooth_vertices(
                &output,
                faces_i64,
                &weights,
                iterations[stroke_index],
                strengths[stroke_index],
                0.02,
            )?,
            operation => return Err(GeometryError::UnsupportedBrushOperation { operation }),
        };
    }
    Ok(output)
}

fn local_offset_vertices_with_weights(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    weights: &[f32],
    amount_mm: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    if weights.len() != vertices.len() {
        return Err(GeometryError::WeightCountDoesNotMatchVertices {
            weights: weights.len(),
            vertices: vertices.len(),
        });
    }
    let directions = outward_directions(vertices, faces_i64)?;
    let displaced = vertices
        .par_iter()
        .enumerate()
        .map(|(vertex_index, vertex)| {
            add(
                *vertex,
                scale(
                    directions[vertex_index],
                    amount_mm * weights[vertex_index] as f64,
                ),
            )
        })
        .collect();
    Ok(displaced)
}

fn validate_brush_strokes(
    operations: &[i64],
    seed_offsets: &[i64],
    seed_count: usize,
    amounts_mm: &[f64],
    falloffs_mm: &[f64],
    iterations: &[i64],
    strengths: &[f64],
) -> Result<(), GeometryError> {
    if operations.len() != amounts_mm.len()
        || operations.len() != falloffs_mm.len()
        || operations.len() != iterations.len()
        || operations.len() != strengths.len()
    {
        return Err(GeometryError::BrushStrokeCountMismatch {
            operations: operations.len(),
            amounts: amounts_mm.len(),
            falloffs: falloffs_mm.len(),
            iterations: iterations.len(),
            strengths: strengths.len(),
        });
    }
    if seed_offsets.len() != operations.len() + 1 {
        return Err(GeometryError::BrushSeedOffsetCountMismatch {
            offsets: seed_offsets.len(),
            operations: operations.len(),
        });
    }
    let mut previous = 0_i64;
    for offset in seed_offsets {
        if *offset < 0 || *offset as usize > seed_count {
            return Err(GeometryError::InvalidBrushSeedOffset {
                offset: *offset,
                seed_count,
            });
        }
        if *offset < previous {
            return Err(GeometryError::BrushSeedOffsetsNotSorted {
                previous,
                next: *offset,
            });
        }
        previous = *offset;
    }
    if seed_offsets.last().copied().unwrap_or_default() as usize != seed_count {
        return Err(GeometryError::InvalidBrushSeedOffset {
            offset: seed_offsets.last().copied().unwrap_or_default(),
            seed_count,
        });
    }
    Ok(())
}

fn validate_brush_operation(operation: i64) -> Result<(), GeometryError> {
    match operation {
        0..=2 => Ok(()),
        operation => Err(GeometryError::UnsupportedBrushOperation { operation }),
    }
}

fn validate_brush_index_offsets(
    kind: &'static str,
    offsets: &[i64],
    index_count: usize,
    operation_count: usize,
) -> Result<(), GeometryError> {
    if offsets.len() != operation_count + 1 {
        return Err(GeometryError::BrushIndexOffsetCountMismatch {
            kind,
            offsets: offsets.len(),
            operations: operation_count,
        });
    }
    let mut previous = 0_i64;
    for offset in offsets {
        if *offset < 0 || *offset as usize > index_count {
            return Err(GeometryError::InvalidBrushIndexOffset {
                kind,
                offset: *offset,
                index_count,
            });
        }
        if *offset < previous {
            return Err(GeometryError::BrushIndexOffsetsNotSorted {
                kind,
                previous,
                next: *offset,
            });
        }
        previous = *offset;
    }
    if offsets.last().copied().unwrap_or_default() as usize != index_count {
        return Err(GeometryError::InvalidBrushIndexOffset {
            kind,
            offset: offsets.last().copied().unwrap_or_default(),
            index_count,
        });
    }
    Ok(())
}

fn region_index_by_id(region_ids: &[String]) -> BTreeMap<&str, usize> {
    let mut region_by_id = BTreeMap::new();
    for (region_index, region_id) in region_ids.iter().enumerate() {
        region_by_id.insert(region_id.as_str(), region_index);
    }
    region_by_id
}

fn select_region_indices(
    region_by_id: &BTreeMap<&str, usize>,
    region_ids: &[String],
) -> Result<Vec<usize>, GeometryError> {
    let mut selected = Vec::with_capacity(region_ids.len());
    let mut missing = BTreeSet::new();
    for region_id in region_ids {
        if let Some(region_index) = region_by_id.get(region_id.as_str()) {
            selected.push(*region_index);
        } else {
            missing.insert(region_id.clone());
        }
    }
    if missing.is_empty() {
        Ok(selected)
    } else {
        Err(GeometryError::UnknownRegionIds {
            ids: missing.into_iter().collect(),
        })
    }
}

fn region_allows_operation(
    region_index: usize,
    operation: i64,
    allowed_operation_offsets: &[i64],
    allowed_operations: &[i64],
) -> bool {
    let start = allowed_operation_offsets[region_index] as usize;
    let end = allowed_operation_offsets[region_index + 1] as usize;
    allowed_operations[start..end].contains(&operation)
}

fn union_region_vertex_indices(
    region_indices: &[usize],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
) -> Vec<i64> {
    let mut union = BTreeSet::new();
    for region_index in region_indices {
        let start = vertex_offsets[*region_index] as usize;
        let end = vertex_offsets[*region_index + 1] as usize;
        union.extend(vertex_indices[start..end].iter().copied());
    }
    union.into_iter().collect()
}

fn validate_brush_indices(
    kind: &'static str,
    indices: &[i64],
    vertex_count: usize,
) -> Result<(), GeometryError> {
    for index in indices {
        if *index < 0 || *index as usize >= vertex_count {
            return Err(GeometryError::BrushIndexOutOfBounds {
                kind,
                index: *index,
                vertex_count,
            });
        }
    }
    Ok(())
}

fn apply_brush_weight_constraints(
    weights: &mut [f32],
    vertex_count: usize,
    mask_enabled: bool,
    mask_indices: &[i64],
    protected_indices: &[i64],
) {
    if mask_enabled {
        let mut mask = vec![false; vertex_count];
        for index in mask_indices {
            mask[*index as usize] = true;
        }
        for (vertex_index, weight) in weights.iter_mut().enumerate() {
            if !mask[vertex_index] {
                *weight = 0.0;
            }
        }
    }
    for index in protected_indices {
        weights[*index as usize] = 0.0;
    }
}

fn smooth_vertices_with_weights(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    weights: Option<&[f32]>,
    iterations: i64,
    strength: f64,
    active_threshold: f32,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let neighbors = vertex_neighbor_list(vertices.len(), &faces);
    let step_count = iterations.max(0) as usize;
    let clamped_strength = strength.clamp(0.0, 1.0);
    let mut smoothed = vertices.to_vec();

    for _ in 0..step_count {
        let previous = smoothed.clone();
        smoothed
            .par_iter_mut()
            .enumerate()
            .for_each(|(vertex_index, vertex)| {
                let weight = weights
                    .map(|values| values[vertex_index])
                    .unwrap_or(1.0)
                    .clamp(0.0, 1.0);
                if weight <= active_threshold {
                    return;
                }
                let neighbor_ids = &neighbors[vertex_index];
                if neighbor_ids.is_empty() {
                    return;
                }
                let mut target = [0.0_f64; 3];
                for neighbor_id in neighbor_ids {
                    target = add(target, previous[*neighbor_id]);
                }
                let scale_factor = 1.0 / neighbor_ids.len() as f64;
                target = scale(target, scale_factor);
                *vertex = add(
                    previous[vertex_index],
                    scale(
                        sub(target, previous[vertex_index]),
                        clamped_strength * weight as f64,
                    ),
                );
            });
    }

    Ok(smoothed)
}

fn validate_seed_indices(
    seed_indices: &[i64],
    vertex_count: usize,
) -> Result<Vec<usize>, GeometryError> {
    if seed_indices.is_empty() {
        return Err(GeometryError::EmptySeedIndices);
    }
    let mut unique = BTreeSet::new();
    for seed in seed_indices {
        if *seed < 0 {
            return Err(GeometryError::NegativeSeedIndex { seed: *seed });
        }
        let index = *seed as usize;
        if index >= vertex_count {
            return Err(GeometryError::SeedIndexOutOfBounds {
                seed: *seed,
                vertex_count,
            });
        }
        unique.insert(index);
    }
    Ok(unique.into_iter().collect())
}
