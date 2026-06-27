use crate::GeometryError;
use std::collections::BTreeSet;

use super::helpers::dot;

pub(super) fn validate_vertex_uvs(
    vertex_uvs: Option<Vec<[f64; 2]>>,
    vertex_count: usize,
) -> Result<Option<Vec<[f64; 2]>>, GeometryError> {
    let Some(uvs) = vertex_uvs else {
        return Ok(None);
    };
    if uvs.len() != vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_uvs",
            value: format!("{} values for {vertex_count} vertices", uvs.len()),
        });
    }
    for uv in &uvs {
        if !uv[0].is_finite() || !uv[1].is_finite() {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "vertex_uvs",
                value: format!("{uv:?}"),
            });
        }
    }
    Ok(Some(uvs))
}

pub(super) fn validate_vertex_colors(
    vertex_colors: Option<Vec<[u8; 4]>>,
    vertex_count: usize,
) -> Result<Option<Vec<[u8; 4]>>, GeometryError> {
    let Some(colors) = vertex_colors else {
        return Ok(None);
    };
    if colors.len() != vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "vertex_colors",
            value: format!("{} values for {vertex_count} vertices", colors.len()),
        });
    }
    Ok(Some(colors))
}

pub(super) fn update_vertex_uvs_before_collapse(
    vertex_uvs: &mut [[f64; 2]],
    vertices: &[[f64; 3]],
    kept_vertex: usize,
    dropped_vertex: usize,
    collapse_pos: [f64; 3],
) {
    let Some(ratio) = collapse_ratio(vertices, kept_vertex, dropped_vertex, collapse_pos) else {
        return;
    };
    if ratio <= 0.0 {
        return;
    }
    if ratio >= 1.0 {
        vertex_uvs[kept_vertex] = vertex_uvs[dropped_vertex];
        return;
    }
    vertex_uvs[kept_vertex] = [
        (1.0 - ratio) * vertex_uvs[kept_vertex][0] + ratio * vertex_uvs[dropped_vertex][0],
        (1.0 - ratio) * vertex_uvs[kept_vertex][1] + ratio * vertex_uvs[dropped_vertex][1],
    ];
}

pub(super) fn update_vertex_colors_before_collapse(
    vertex_colors: &mut [[u8; 4]],
    vertices: &[[f64; 3]],
    kept_vertex: usize,
    dropped_vertex: usize,
    collapse_pos: [f64; 3],
) {
    let Some(ratio) = collapse_ratio(vertices, kept_vertex, dropped_vertex, collapse_pos) else {
        return;
    };
    if ratio <= 0.0 {
        return;
    }
    if ratio >= 1.0 {
        vertex_colors[kept_vertex] = vertex_colors[dropped_vertex];
        return;
    }
    vertex_colors[kept_vertex] = meshlib_interpolate_color(
        vertex_colors[kept_vertex],
        vertex_colors[dropped_vertex],
        ratio,
    );
}

fn collapse_ratio(
    vertices: &[[f64; 3]],
    kept_vertex: usize,
    dropped_vertex: usize,
    collapse_pos: [f64; 3],
) -> Option<f64> {
    let kept_pos = vertices[kept_vertex];
    let dropped_pos = vertices[dropped_vertex];
    let edge_vec = [
        dropped_pos[0] - kept_pos[0],
        dropped_pos[1] - kept_pos[1],
        dropped_pos[2] - kept_pos[2],
    ];
    let collapse_vec = [
        collapse_pos[0] - kept_pos[0],
        collapse_pos[1] - kept_pos[1],
        collapse_pos[2] - kept_pos[2],
    ];
    let edge_len_sq = dot(edge_vec, edge_vec);
    if edge_len_sq <= 1e-24 {
        return None;
    }
    let dt = dot(collapse_vec, edge_vec);
    Some(dt / edge_len_sq)
}

fn meshlib_interpolate_color(left: [u8; 4], right: [u8; 4], ratio: f64) -> [u8; 4] {
    let left_scale = 1.0 - ratio;
    [
        meshlib_scaled_color_component(left[0], left_scale)
            .saturating_add(meshlib_scaled_color_component(right[0], ratio)),
        meshlib_scaled_color_component(left[1], left_scale)
            .saturating_add(meshlib_scaled_color_component(right[1], ratio)),
        meshlib_scaled_color_component(left[2], left_scale)
            .saturating_add(meshlib_scaled_color_component(right[2], ratio)),
        meshlib_scaled_color_component(left[3], left_scale)
            .saturating_add(meshlib_scaled_color_component(right[3], ratio)),
    ]
}

fn meshlib_scaled_color_component(component: u8, scale: f64) -> u8 {
    (scale * f64::from(component)).clamp(0.0, 255.0) as u8
}

pub(super) fn output_vertex_uvs(
    faces: &[[usize; 3]],
    vertex_uvs: Option<Vec<[f64; 2]>>,
    pack_mesh: bool,
    vertex_count: usize,
) -> Option<Vec<[f64; 2]>> {
    let vertex_uvs = vertex_uvs?;
    if !pack_mesh {
        return Some(vertex_uvs);
    }
    let used: BTreeSet<usize> = faces.iter().flat_map(|face| face.iter().copied()).collect();
    let mut output = Vec::with_capacity(used.len());
    for old_index in 0..vertex_count {
        if used.contains(&old_index) {
            output.push(vertex_uvs[old_index]);
        }
    }
    Some(output)
}

pub(super) fn output_vertex_colors(
    faces: &[[usize; 3]],
    vertex_colors: Option<Vec<[u8; 4]>>,
    pack_mesh: bool,
    vertex_count: usize,
) -> Option<Vec<[u8; 4]>> {
    let vertex_colors = vertex_colors?;
    if !pack_mesh {
        return Some(vertex_colors);
    }
    let used: BTreeSet<usize> = faces.iter().flat_map(|face| face.iter().copied()).collect();
    let mut output = Vec::with_capacity(used.len());
    for old_index in 0..vertex_count {
        if used.contains(&old_index) {
            output.push(vertex_colors[old_index]);
        }
    }
    Some(output)
}
