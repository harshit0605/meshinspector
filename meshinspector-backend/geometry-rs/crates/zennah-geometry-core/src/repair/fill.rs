use super::{centroid, loop_normal, ordered_boundary_loops, orient_faces_outward};
use crate::math::{dot, normalize_vector, sub};
use crate::{GeometryError, HoleFillReport, HoleFillResult};
use metrics::{triangle_fill_metric_with_mode, FillMetricContext, BAD_TRIANGULATION_METRIC};
use triangulation::triangulate_hole_loop_strong_with_max_polygon_subdivisions_and_metric_up_dir;
#[cfg(test)]
use triangulation::{
    optimal_split_steps, triangulate_hole_loop_weight_with_fill_params_for_tests,
    triangulate_hole_loop_with_multiple_edges_resolve_mode,
    triangulate_hole_loop_with_multiple_edges_resolve_mode_and_metric_up_dir,
};
pub(crate) use triangulation::{triangulate_hole_loop, triangulate_hole_loop_strong};

mod metrics;
mod scoring;
mod strong;
mod triangulation;

pub(crate) const DEFAULT_MAX_POLYGON_SUBDIVISIONS: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillHoleMultipleEdgesResolveMode {
    None,
    Simple,
    Strong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillHoleMetricMode {
    Circumscribed,
    MinArea,
    EdgeLength,
    Universal,
    MaxDihedralAngle,
    ParallelPlane,
    ComplexFill,
    MinTriAngle,
    Plane,
    PlaneNormalized,
    ComplexStitch,
    EdgeLengthStitch,
    VerticalStitch,
    VerticalStitchEdgeBased,
}

pub fn fill_planar_holes(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_edges: Option<usize>,
) -> Result<HoleFillResult, GeometryError> {
    let loops = ordered_boundary_loops(vertices, faces_i64)?;
    if loops.is_empty() {
        return Ok(HoleFillResult {
            vertices: vertices.to_vec(),
            faces: faces_i64.to_vec(),
            report: HoleFillReport {
                input_holes: 0,
                filled_holes: 0,
                added_vertices: 0,
                added_faces: 0,
                new_face_indices: Vec::new(),
                skipped_holes: 0,
            },
        });
    }

    let mut output_vertices = vertices.to_vec();
    let mut output_faces = faces_i64.to_vec();
    let mut new_face_indices = Vec::new();
    let mesh_center = centroid(vertices);
    let mut filled = 0_usize;
    let mut skipped = 0_usize;

    for mut boundary_loop in loops.iter().cloned() {
        if boundary_loop.len() < 3 || max_edges.is_some_and(|limit| boundary_loop.len() > limit) {
            skipped += 1;
            continue;
        }
        let points: Vec<[f64; 3]> = boundary_loop
            .iter()
            .map(|vertex_id| vertices[*vertex_id])
            .collect();
        let loop_centroid = centroid(&points);
        let centroid_index = output_vertices.len() as i64;
        output_vertices.push(loop_centroid);

        let normal = loop_normal(&points);
        let outward_hint = sub(loop_centroid, mesh_center);
        if dot(normal, outward_hint) < 0.0 {
            boundary_loop.reverse();
        }

        for index in 0..boundary_loop.len() {
            let vertex_id = boundary_loop[index] as i64;
            let next_id = boundary_loop[(index + 1) % boundary_loop.len()] as i64;
            new_face_indices.push(output_faces.len());
            output_faces.push([vertex_id, next_id, centroid_index]);
        }
        filled += 1;
    }

    Ok(HoleFillResult {
        vertices: output_vertices,
        faces: output_faces,
        report: HoleFillReport {
            input_holes: loops.len(),
            filled_holes: filled,
            added_vertices: filled,
            added_faces: loops
                .iter()
                .filter(|boundary_loop| max_edges.is_none_or(|limit| boundary_loop.len() <= limit))
                .map(Vec::len)
                .sum(),
            new_face_indices,
            skipped_holes: skipped,
        },
    })
}

pub fn service_fill_holes(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_edges: Option<usize>,
) -> Result<HoleFillResult, GeometryError> {
    service_fill_holes_with_fill_params(
        vertices,
        faces_i64,
        max_edges,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMultipleEdgesResolveMode::Simple,
        false,
        false,
        true,
        FillHoleMetricMode::Circumscribed,
    )
}

pub fn service_fill_holes_with_max_polygon_subdivisions(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_edges: Option<usize>,
    max_polygon_subdivisions: usize,
) -> Result<HoleFillResult, GeometryError> {
    service_fill_holes_with_fill_params(
        vertices,
        faces_i64,
        max_edges,
        max_polygon_subdivisions,
        FillHoleMultipleEdgesResolveMode::Simple,
        false,
        false,
        true,
        FillHoleMetricMode::Circumscribed,
    )
}

pub fn service_fill_holes_with_fill_params(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_edges: Option<usize>,
    max_polygon_subdivisions: usize,
    multiple_edges_resolve_mode: FillHoleMultipleEdgesResolveMode,
    make_degenerate_band: bool,
    stop_before_bad_triangulation: bool,
    smooth_bd: bool,
    fill_metric_mode: FillHoleMetricMode,
) -> Result<HoleFillResult, GeometryError> {
    service_fill_holes_with_fill_params_and_metric_up_dir(
        vertices,
        faces_i64,
        max_edges,
        max_polygon_subdivisions,
        multiple_edges_resolve_mode,
        make_degenerate_band,
        stop_before_bad_triangulation,
        smooth_bd,
        fill_metric_mode,
        None,
    )
}

pub fn service_fill_holes_with_fill_params_and_metric_up_dir(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_edges: Option<usize>,
    max_polygon_subdivisions: usize,
    multiple_edges_resolve_mode: FillHoleMultipleEdgesResolveMode,
    make_degenerate_band: bool,
    stop_before_bad_triangulation: bool,
    smooth_bd: bool,
    fill_metric_mode: FillHoleMetricMode,
    fill_metric_up_dir: Option<[f64; 3]>,
) -> Result<HoleFillResult, GeometryError> {
    if max_polygon_subdivisions < 2 {
        return Err(GeometryError::InvalidMaxPolygonSubdivisions {
            max_polygon_subdivisions,
        });
    }
    let fill_metric_up_dir = fill_metric_up_dir.map(normalize_vector).transpose()?;

    let loops = ordered_boundary_loops(vertices, faces_i64)?;
    if loops.is_empty() {
        return Ok(HoleFillResult {
            vertices: vertices.to_vec(),
            faces: faces_i64.to_vec(),
            report: HoleFillReport {
                input_holes: 0,
                filled_holes: 0,
                added_vertices: 0,
                added_faces: 0,
                new_face_indices: Vec::new(),
                skipped_holes: 0,
            },
        });
    }

    let mut output_vertices = vertices.to_vec();
    let mut output_faces = faces_i64.to_vec();
    let mesh_center = centroid(vertices);
    let mut filled = 0_usize;
    let mut added_vertices = 0_usize;
    let mut added_faces = 0_usize;
    let mut new_face_indices = Vec::new();
    let mut skipped = 0_usize;

    for mut boundary_loop in loops.iter().cloned() {
        if boundary_loop.len() < 3 || max_edges.is_some_and(|limit| boundary_loop.len() > limit) {
            skipped += 1;
            continue;
        }

        let points: Vec<[f64; 3]> = boundary_loop
            .iter()
            .map(|vertex_id| vertices[*vertex_id])
            .collect();
        let loop_centroid = centroid(&points);
        let normal = loop_normal(&points);
        let outward_hint = sub(loop_centroid, mesh_center);
        if dot(normal, outward_hint) < 0.0 {
            boundary_loop.reverse();
        }

        let fill_loop = if make_degenerate_band {
            let first_band_face = output_faces.len();
            let duplicate_loop = make_degenerate_band_around_hole(
                &mut output_vertices,
                &mut output_faces,
                &boundary_loop,
            );
            added_vertices += duplicate_loop.len();
            added_faces += boundary_loop.len() * 2;
            new_face_indices.extend(first_band_face..output_faces.len());
            duplicate_loop
        } else {
            boundary_loop
        };
        let new_faces =
            triangulate_hole_loop_strong_with_max_polygon_subdivisions_and_metric_up_dir(
                &output_vertices,
                &output_faces,
                &fill_loop,
                max_polygon_subdivisions,
                multiple_edges_resolve_mode,
                fill_metric_mode,
                smooth_bd,
                fill_metric_up_dir,
            );
        if stop_before_bad_triangulation && patch_triangulation_is_bad(&output_vertices, &new_faces)
        {
            skipped += 1;
            continue;
        }
        added_faces += new_faces.len();
        new_face_indices.extend(output_faces.len()..(output_faces.len() + new_faces.len()));
        output_faces.extend(new_faces);
        filled += 1;
    }

    let output_faces = if filled > 0 {
        orient_faces_outward(&output_vertices, &output_faces)?
    } else {
        output_faces
    };
    Ok(HoleFillResult {
        vertices: output_vertices,
        faces: output_faces,
        report: HoleFillReport {
            input_holes: loops.len(),
            filled_holes: filled,
            added_vertices,
            added_faces,
            new_face_indices,
            skipped_holes: skipped,
        },
    })
}

fn make_degenerate_band_around_hole(
    output_vertices: &mut Vec<[f64; 3]>,
    output_faces: &mut Vec<[i64; 3]>,
    boundary_loop: &[usize],
) -> Vec<usize> {
    let duplicate_loop = boundary_loop
        .iter()
        .map(|&vertex_id| {
            let duplicate_id = output_vertices.len();
            output_vertices.push(output_vertices[vertex_id]);
            duplicate_id
        })
        .collect::<Vec<_>>();
    for index in 0..boundary_loop.len() {
        let next = (index + 1) % boundary_loop.len();
        let a = boundary_loop[index] as i64;
        let b = boundary_loop[next] as i64;
        let inner_a = duplicate_loop[index] as i64;
        let inner_b = duplicate_loop[next] as i64;
        output_faces.push([a, b, inner_b]);
        output_faces.push([a, inner_b, inner_a]);
    }
    duplicate_loop
}

fn patch_triangulation_is_bad(vertices: &[[f64; 3]], faces: &[[i64; 3]]) -> bool {
    faces.is_empty()
        || faces.iter().any(|face| {
            face.iter().any(|vertex_id| *vertex_id < 0)
                || triangle_fill_metric_with_mode(
                    vertices[face[0] as usize],
                    vertices[face[1] as usize],
                    vertices[face[2] as usize],
                    FillHoleMetricMode::Circumscribed,
                    FillMetricContext::default(),
                ) > BAD_TRIANGULATION_METRIC
        })
}

#[cfg(test)]
mod tests;
