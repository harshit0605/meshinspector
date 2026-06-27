use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet};

use super::{flip_edge, input, topology};

mod attributes;
mod candidates;
mod fast;
mod geometry;
mod helpers;
mod qem;
mod types;

use attributes::{
    output_vertex_colors, output_vertex_uvs, update_vertex_colors_before_collapse,
    update_vertex_uvs_before_collapse, validate_vertex_colors, validate_vertex_uvs,
};
use candidates::{next_collapse_plan, next_delone_flip_candidate, remap_twin_map_after_flip};
use helpers::{
    collapse_plan, finish_mesh, normalize_edges_to_collapse, normalize_not_flippable_edges,
    normalize_twin_map, output_edges_to_collapse, output_not_flippable_edges, output_twin_map,
    proportional_quota, remap_edges_to_collapse_after_collapse,
    remap_not_flippable_edges_after_collapse, remap_twin_map_after_collapse, subdivide_part_region,
};
use qem::compute_vertex_forms;
use types::DecimateMeshState;

const MESHLIB_UNBOUNDED_FLOAT_LIMIT: f64 = f32::MAX as f64;
const MESHLIB_INT_MAX: usize = i32::MAX as usize;

pub use types::{DecimateMeshOptions, DecimateMeshResult, DecimateMeshStrategy};

pub fn decimate_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    mut options: DecimateMeshOptions,
) -> Result<DecimateMeshResult, GeometryError> {
    validate_options(&options)?;
    let vertices = vertices.to_vec();
    let faces = input::validate_faces(faces_i64, vertices.len())?;
    let not_flippable_edges =
        normalize_not_flippable_edges(&options.not_flippable_edges, vertices.len())?;
    let edges_to_collapse =
        normalize_edges_to_collapse(options.edges_to_collapse.as_deref(), vertices.len())?;
    let twin_map = normalize_twin_map(&options.twin_map, vertices.len())?;
    let vertex_uvs = validate_vertex_uvs(options.vertex_uvs.take(), vertices.len())?;
    let vertex_colors = validate_vertex_colors(options.vertex_colors.take(), vertices.len())?;
    let region = input::initial_region(faces.len(), options.region_faces.as_deref())?;
    apply_meshlib_target_face_limit(&mut options, &region);
    apply_meshlib_default_half_face_guard(&mut options, &region);

    let state = if options.subdivide_parts > 1 {
        decimate_mesh_subdivide_parts(
            vertices,
            faces,
            region,
            &options,
            &not_flippable_edges,
            &edges_to_collapse,
            &twin_map,
            vertex_uvs,
            vertex_colors,
        )
    } else {
        decimate_mesh_serial_state(
            vertices,
            faces,
            region.clone(),
            region,
            &options,
            &not_flippable_edges,
            &edges_to_collapse,
            &twin_map,
            vertex_uvs,
            vertex_colors,
        )
    };

    let DecimateMeshState {
        vertices,
        faces,
        region: _,
        verts_deleted,
        faces_deleted,
        error_introduced,
        not_flippable_edges,
        edges_to_collapse,
        twin_map,
        vertex_uvs,
        vertex_colors,
    } = state;
    let not_flippable_edges = output_not_flippable_edges(
        &faces,
        &not_flippable_edges,
        options.pack_mesh,
        vertices.len(),
    );
    let edges_to_collapse = output_edges_to_collapse(
        &faces,
        edges_to_collapse.as_ref(),
        options.pack_mesh,
        vertices.len(),
    );
    let twin_map = output_twin_map(&faces, &twin_map, options.pack_mesh, vertices.len());
    let vertex_uvs = output_vertex_uvs(&faces, vertex_uvs, options.pack_mesh, vertices.len());
    let vertex_colors =
        output_vertex_colors(&faces, vertex_colors, options.pack_mesh, vertices.len());
    Ok(DecimateMeshResult {
        mesh: finish_mesh(vertices, faces, options.pack_mesh),
        verts_deleted,
        faces_deleted,
        error_introduced,
        cancelled: false,
        not_flippable_edges,
        edges_to_collapse,
        twin_map,
        vertex_uvs,
        vertex_colors,
    })
}

#[allow(clippy::too_many_arguments)]
fn decimate_mesh_serial_state(
    vertices: Vec<[f64; 3]>,
    faces: Vec<[usize; 3]>,
    candidate_region: Vec<bool>,
    tracked_region: Vec<bool>,
    options: &DecimateMeshOptions,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
    twin_map: &BTreeMap<[usize; 2], [usize; 2]>,
    vertex_uvs: Option<Vec<[f64; 2]>>,
    vertex_colors: Option<Vec<[u8; 4]>>,
) -> DecimateMeshState {
    if fast::fast_eligible(
        options,
        &candidate_region,
        &tracked_region,
        not_flippable_edges,
        edges_to_collapse,
        twin_map.is_empty(),
        &vertex_uvs,
        &vertex_colors,
        faces.len(),
    ) {
        return fast::decimate_mesh_serial_state_fast(
            vertices,
            faces,
            candidate_region,
            tracked_region,
            options,
            not_flippable_edges.clone(),
            edges_to_collapse.clone(),
            twin_map.clone(),
            vertex_uvs,
            vertex_colors,
        );
    }
    decimate_mesh_serial_state_reference(
        vertices,
        faces,
        candidate_region,
        tracked_region,
        options,
        not_flippable_edges,
        edges_to_collapse,
        twin_map,
        vertex_uvs,
        vertex_colors,
    )
}

#[allow(clippy::too_many_arguments)]
fn decimate_mesh_serial_state_reference(
    mut vertices: Vec<[f64; 3]>,
    mut faces: Vec<[usize; 3]>,
    mut candidate_region: Vec<bool>,
    mut tracked_region: Vec<bool>,
    options: &DecimateMeshOptions,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
    twin_map: &BTreeMap<[usize; 2], [usize; 2]>,
    mut vertex_uvs: Option<Vec<[f64; 2]>>,
    mut vertex_colors: Option<Vec<[u8; 4]>>,
) -> DecimateMeshState {
    let mut not_flippable_edges = not_flippable_edges.clone();
    let mut edges_to_collapse = edges_to_collapse.clone();
    let mut twin_map = twin_map.clone();
    let mut vertex_forms = (options.strategy == DecimateMeshStrategy::MinimizeError).then(|| {
        compute_vertex_forms(
            &vertices,
            &faces,
            &candidate_region,
            &not_flippable_edges,
            options.angle_weighted_dist_to_plane,
            options.stabilizer,
        )
    });

    if options.max_deleted_vertices == 0 || options.max_deleted_faces == 0 {
        return DecimateMeshState {
            vertices,
            faces,
            region: tracked_region,
            verts_deleted: 0,
            faces_deleted: 0,
            error_introduced: 0.0,
            not_flippable_edges,
            edges_to_collapse,
            twin_map,
            vertex_uvs,
            vertex_colors,
        };
    }

    let mut verts_deleted = 0;
    let mut faces_deleted = 0;
    let mut flipped_edges = BTreeSet::<[usize; 2]>::new();

    let error_introduced = loop {
        let mut skipped_edges = BTreeSet::<[usize; 2]>::new();
        let collapse_candidate = loop {
            let next = next_collapse_plan(
                &vertices,
                &faces,
                &candidate_region,
                &tracked_region,
                vertex_forms.as_deref(),
                options,
                &not_flippable_edges,
                &edges_to_collapse,
                &mut skipped_edges,
            );
            let Some((candidate, plan)) = next else {
                break None;
            };
            let candidate_edge = candidate.edge();
            let collapse_pos = plan.vertices[plan.kept_vertex];
            let Some(twin_edge) = twin_map.get(&candidate_edge).copied() else {
                break Some((candidate, plan, None));
            };
            if twin_edge == candidate_edge {
                break Some((candidate, plan, None));
            }
            let Some(twin_plan) = collapse_plan(
                &vertices,
                &faces,
                &candidate_region,
                &tracked_region,
                twin_edge,
                collapse_pos,
                options,
            ) else {
                skipped_edges.insert(candidate_edge);
                continue;
            };
            let Some(original_plan_after_twin) = collapse_plan(
                &twin_plan.vertices,
                &twin_plan.faces,
                &twin_plan.candidate_region,
                &twin_plan.tracked_region,
                candidate_edge,
                collapse_pos,
                options,
            ) else {
                skipped_edges.insert(candidate_edge);
                continue;
            };
            break Some((candidate, original_plan_after_twin, Some(twin_plan)));
        };

        let collapse_error_sq = collapse_candidate
            .as_ref()
            .map(|(candidate, _, _)| candidate.error_sq());
        if let Some(flip_candidate) = next_delone_flip_candidate(
            &vertices,
            &faces,
            &candidate_region,
            options,
            &not_flippable_edges,
            &edges_to_collapse,
            &twin_map,
            &flipped_edges,
            collapse_error_sq,
        ) {
            let mut edge_state = topology::EdgeState::from_faces(&faces);
            if let Some(twin) = flip_candidate.twin {
                flip_edge(
                    &mut faces,
                    &mut candidate_region,
                    &mut edge_state,
                    twin.edge,
                    twin.face_pair,
                );
                remap_twin_map_after_flip(
                    &mut twin_map,
                    flip_candidate.edge,
                    flip_candidate.new_edge,
                    twin.edge,
                    twin.new_edge,
                );
                flipped_edges.insert(twin.edge);
                flipped_edges.insert(twin.new_edge);
            }
            flip_edge(
                &mut faces,
                &mut candidate_region,
                &mut edge_state,
                flip_candidate.edge,
                flip_candidate.face_pair,
            );
            flipped_edges.insert(flip_candidate.edge);
            flipped_edges.insert(flip_candidate.new_edge);
            if vertex_forms.is_some() {
                vertex_forms = Some(compute_vertex_forms(
                    &vertices,
                    &faces,
                    &candidate_region,
                    &not_flippable_edges,
                    options.angle_weighted_dist_to_plane,
                    options.stabilizer,
                ));
            }
            continue;
        }

        let Some((candidate, plan, twin_plan)) = collapse_candidate else {
            break options.max_error;
        };

        if verts_deleted + 1 > options.max_deleted_vertices
            || faces_deleted + plan.faces_deleted > options.max_deleted_faces
        {
            break candidate.error_introduced();
        }

        let paired_collapse = twin_plan.is_some();
        if let Some(twin_plan) = twin_plan {
            let kept_vertex = twin_plan.kept_vertex;
            let dropped_vertex = twin_plan.dropped_vertex;
            update_pre_collapse_vertex_attributes(
                &mut vertex_uvs,
                &mut vertex_colors,
                &vertices,
                kept_vertex,
                dropped_vertex,
                twin_plan.vertices[kept_vertex],
            );
            vertices = twin_plan.vertices;
            remap_not_flippable_edges_after_collapse(
                &mut not_flippable_edges,
                kept_vertex,
                dropped_vertex,
            );
            remap_edges_to_collapse_after_collapse(
                &mut edges_to_collapse,
                kept_vertex,
                dropped_vertex,
            );
            remap_twin_map_after_collapse(&mut twin_map, kept_vertex, dropped_vertex);
            verts_deleted += 1;
            faces_deleted += twin_plan.faces_deleted;
        }

        let kept_vertex = plan.kept_vertex;
        let dropped_vertex = plan.dropped_vertex;
        update_pre_collapse_vertex_attributes(
            &mut vertex_uvs,
            &mut vertex_colors,
            &vertices,
            kept_vertex,
            dropped_vertex,
            plan.vertices[kept_vertex],
        );
        vertices = plan.vertices;
        faces = plan.faces;
        candidate_region = plan.candidate_region;
        tracked_region = plan.tracked_region;
        remap_not_flippable_edges_after_collapse(
            &mut not_flippable_edges,
            kept_vertex,
            dropped_vertex,
        );
        remap_edges_to_collapse_after_collapse(&mut edges_to_collapse, kept_vertex, dropped_vertex);
        remap_twin_map_after_collapse(&mut twin_map, kept_vertex, dropped_vertex);
        if paired_collapse {
            if vertex_forms.is_some() {
                vertex_forms = Some(compute_vertex_forms(
                    &vertices,
                    &faces,
                    &candidate_region,
                    &not_flippable_edges,
                    options.angle_weighted_dist_to_plane,
                    options.stabilizer,
                ));
            }
        } else if let (Some(forms), Some(collapse_form)) =
            (vertex_forms.as_mut(), candidate.collapse_form())
        {
            forms[candidate.edge()[0]] = collapse_form;
        }
        verts_deleted += 1;
        faces_deleted += plan.faces_deleted;

        if faces.is_empty() {
            break options.max_error;
        }
    };

    DecimateMeshState {
        vertices,
        faces,
        region: tracked_region,
        verts_deleted,
        faces_deleted,
        error_introduced,
        not_flippable_edges,
        edges_to_collapse,
        twin_map,
        vertex_uvs,
        vertex_colors,
    }
}

fn update_pre_collapse_vertex_attributes(
    vertex_uvs: &mut Option<Vec<[f64; 2]>>,
    vertex_colors: &mut Option<Vec<[u8; 4]>>,
    vertices: &[[f64; 3]],
    kept_vertex: usize,
    dropped_vertex: usize,
    collapse_pos: [f64; 3],
) {
    if let Some(uvs) = vertex_uvs.as_mut() {
        update_vertex_uvs_before_collapse(uvs, vertices, kept_vertex, dropped_vertex, collapse_pos);
    }
    if let Some(colors) = vertex_colors.as_mut() {
        update_vertex_colors_before_collapse(
            colors,
            vertices,
            kept_vertex,
            dropped_vertex,
            collapse_pos,
        );
    }
}

fn decimate_mesh_subdivide_parts(
    mut vertices: Vec<[f64; 3]>,
    mut faces: Vec<[usize; 3]>,
    mut region: Vec<bool>,
    options: &DecimateMeshOptions,
    not_flippable_edges: &BTreeSet<[usize; 2]>,
    edges_to_collapse: &Option<BTreeSet<[usize; 2]>>,
    twin_map: &BTreeMap<[usize; 2], [usize; 2]>,
    mut vertex_uvs: Option<Vec<[f64; 2]>>,
    mut vertex_colors: Option<Vec<[u8; 4]>>,
) -> DecimateMeshState {
    let mut not_flippable_edges = not_flippable_edges.clone();
    let mut edges_to_collapse = edges_to_collapse.clone();
    let mut twin_map = twin_map.clone();
    let initial_selected_faces = region.iter().filter(|selected| **selected).count().max(1);
    let mut verts_deleted = 0;
    let mut faces_deleted = 0;
    let mut error_introduced: f64 = 0.0;

    for part_index in 0..options.subdivide_parts {
        if verts_deleted >= options.max_deleted_vertices
            || faces_deleted >= options.max_deleted_faces
        {
            break;
        }
        let part_region = subdivide_part_region(&region, options.subdivide_parts, part_index);
        let part_face_count = part_region.iter().filter(|selected| **selected).count();
        if part_face_count == 0 {
            continue;
        }

        let mut part_options = options.clone();
        part_options.subdivide_parts = 1;
        part_options.decimate_between_parts = true;
        part_options.pack_mesh = false;
        part_options.touch_near_bd_edges = false;
        part_options.max_deleted_vertices = options
            .max_deleted_vertices
            .saturating_sub(verts_deleted)
            .min(proportional_quota(
                options.max_deleted_vertices,
                part_face_count,
                initial_selected_faces,
            ));
        part_options.max_deleted_faces = options
            .max_deleted_faces
            .saturating_sub(faces_deleted)
            .min(proportional_quota(
                options.max_deleted_faces,
                part_face_count,
                initial_selected_faces,
            ));

        let state = decimate_mesh_serial_state(
            vertices,
            faces,
            part_region,
            region,
            &part_options,
            &not_flippable_edges,
            &edges_to_collapse,
            &twin_map,
            vertex_uvs,
            vertex_colors,
        );
        vertices = state.vertices;
        faces = state.faces;
        region = state.region;
        not_flippable_edges = state.not_flippable_edges;
        edges_to_collapse = state.edges_to_collapse;
        twin_map = state.twin_map;
        vertex_uvs = state.vertex_uvs;
        vertex_colors = state.vertex_colors;
        verts_deleted += state.verts_deleted;
        faces_deleted += state.faces_deleted;
        error_introduced = error_introduced.max(state.error_introduced);
    }

    if options.decimate_between_parts
        && verts_deleted < options.max_deleted_vertices
        && faces_deleted < options.max_deleted_faces
    {
        let mut final_options = options.clone();
        final_options.subdivide_parts = 1;
        final_options.max_deleted_vertices =
            options.max_deleted_vertices.saturating_sub(verts_deleted);
        final_options.max_deleted_faces = options.max_deleted_faces.saturating_sub(faces_deleted);
        let state = decimate_mesh_serial_state(
            vertices,
            faces,
            region.clone(),
            region,
            &final_options,
            &not_flippable_edges,
            &edges_to_collapse,
            &twin_map,
            vertex_uvs,
            vertex_colors,
        );
        vertices = state.vertices;
        faces = state.faces;
        region = state.region;
        not_flippable_edges = state.not_flippable_edges;
        edges_to_collapse = state.edges_to_collapse;
        twin_map = state.twin_map;
        vertex_uvs = state.vertex_uvs;
        vertex_colors = state.vertex_colors;
        verts_deleted += state.verts_deleted;
        faces_deleted += state.faces_deleted;
        error_introduced = error_introduced.max(state.error_introduced);
    }

    DecimateMeshState {
        vertices,
        faces,
        region,
        verts_deleted,
        faces_deleted,
        error_introduced,
        not_flippable_edges,
        edges_to_collapse,
        twin_map,
        vertex_uvs,
        vertex_colors,
    }
}

fn apply_meshlib_target_face_limit(options: &mut DecimateMeshOptions, region: &[bool]) {
    let selected_faces = region.iter().filter(|selected| **selected).count();
    let target_faces = options.target_face_count.or_else(|| {
        options
            .target_face_ratio
            .map(|ratio| ((selected_faces as f64 * ratio).floor() as usize).max(1))
    });
    let Some(target_faces) = target_faces else {
        return;
    };

    let max_target_deletions = selected_faces.saturating_sub(target_faces);
    options.max_deleted_faces = options.max_deleted_faces.min(max_target_deletions);
}

fn apply_meshlib_default_half_face_guard(options: &mut DecimateMeshOptions, region: &[bool]) {
    if options.max_error >= MESHLIB_UNBOUNDED_FLOAT_LIMIT
        && options.max_edge_len >= MESHLIB_UNBOUNDED_FLOAT_LIMIT
        && options.max_deleted_faces >= MESHLIB_INT_MAX
        && options.max_deleted_vertices >= MESHLIB_INT_MAX
    {
        options.max_deleted_faces = region.iter().filter(|selected| **selected).count() / 2;
    }
}

fn validate_options(options: &DecimateMeshOptions) -> Result<(), GeometryError> {
    if !options.max_error.is_finite() || options.max_error < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_error",
            value: options.max_error,
        });
    }
    if !options.max_edge_len.is_finite() || options.max_edge_len < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_edge_len",
            value: options.max_edge_len,
        });
    }
    if !options.max_bd_shift.is_finite() || options.max_bd_shift < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_bd_shift",
            value: options.max_bd_shift,
        });
    }
    if !options.stabilizer.is_finite() || options.stabilizer < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "stabilizer",
            value: options.stabilizer,
        });
    }
    if let Some(target_face_count) = options.target_face_count {
        if target_face_count == 0 {
            return Err(GeometryError::InvalidMeshEditInput {
                field: "target_face_count",
                value: target_face_count as f64,
            });
        }
    }
    if let Some(target_face_ratio) = options.target_face_ratio {
        if !target_face_ratio.is_finite() || target_face_ratio <= 0.0 || target_face_ratio > 1.0 {
            return Err(GeometryError::InvalidMeshEditInput {
                field: "target_face_ratio",
                value: target_face_ratio,
            });
        }
    }
    if options.subdivide_parts == 0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "subdivide_parts",
            value: options.subdivide_parts as f64,
        });
    }
    if !options.max_triangle_aspect_ratio.is_finite() || options.max_triangle_aspect_ratio < 1.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_triangle_aspect_ratio",
            value: options.max_triangle_aspect_ratio,
        });
    }
    if !options.critical_tri_aspect_ratio.is_finite() || options.critical_tri_aspect_ratio < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "critical_tri_aspect_ratio",
            value: options.critical_tri_aspect_ratio,
        });
    }
    if !options.tiny_edge_length.is_finite() {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "tiny_edge_length",
            value: options.tiny_edge_length,
        });
    }
    if !options.max_angle_change.is_finite() {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_angle_change",
            value: options.max_angle_change,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
