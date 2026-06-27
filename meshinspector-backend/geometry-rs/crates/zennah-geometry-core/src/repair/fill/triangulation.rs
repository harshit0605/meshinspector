use super::metrics::fill_metric_context_with_up_dir;
use super::scoring::{
    boundary_edge_contexts, candidate_weight, CandidateRequest, CandidateWeights,
};
use super::strong::triangulate_avoiding_reused_chords;
use super::{
    FillHoleMetricMode, FillHoleMultipleEdgesResolveMode, DEFAULT_MAX_POLYGON_SUBDIVISIONS,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
struct FillCell {
    weight: f64,
    split: Option<usize>,
}

#[derive(Clone, Debug)]
struct TriangulationOutcome {
    faces: Vec<[i64; 3]>,
    #[cfg(test)]
    weight: f64,
}

pub(crate) fn triangulate_hole_loop_strong(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
) -> Vec<[i64; 3]> {
    triangulate_hole_loop_strong_with_max_polygon_subdivisions(
        vertices,
        existing_faces,
        boundary_loop,
        DEFAULT_MAX_POLYGON_SUBDIVISIONS,
        FillHoleMultipleEdgesResolveMode::Simple,
        FillHoleMetricMode::Circumscribed,
        true,
    )
}

pub(super) fn triangulate_hole_loop_strong_with_max_polygon_subdivisions(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    max_polygon_subdivisions: usize,
    multiple_edges_resolve_mode: FillHoleMultipleEdgesResolveMode,
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
) -> Vec<[i64; 3]> {
    triangulate_hole_loop_with_multiple_edges_resolve_mode(
        vertices,
        existing_faces,
        boundary_loop,
        multiple_edges_resolve_mode,
        max_polygon_subdivisions,
        fill_metric_mode,
        smooth_bd,
    )
}

pub(super) fn triangulate_hole_loop_strong_with_max_polygon_subdivisions_and_metric_up_dir(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    max_polygon_subdivisions: usize,
    multiple_edges_resolve_mode: FillHoleMultipleEdgesResolveMode,
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
    fill_metric_up_dir: Option<[f64; 3]>,
) -> Vec<[i64; 3]> {
    triangulate_hole_loop_with_multiple_edges_resolve_mode_and_metric_up_dir(
        vertices,
        existing_faces,
        boundary_loop,
        multiple_edges_resolve_mode,
        max_polygon_subdivisions,
        fill_metric_mode,
        smooth_bd,
        fill_metric_up_dir,
    )
}

pub(crate) fn triangulate_hole_loop_with_multiple_edges_resolve_mode(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    multiple_edges_resolve_mode: FillHoleMultipleEdgesResolveMode,
    max_polygon_subdivisions: usize,
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
) -> Vec<[i64; 3]> {
    triangulate_hole_loop_with_multiple_edges_resolve_mode_and_metric_up_dir(
        vertices,
        existing_faces,
        boundary_loop,
        multiple_edges_resolve_mode,
        max_polygon_subdivisions,
        fill_metric_mode,
        smooth_bd,
        None,
    )
}

pub(crate) fn triangulate_hole_loop_with_multiple_edges_resolve_mode_and_metric_up_dir(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    multiple_edges_resolve_mode: FillHoleMultipleEdgesResolveMode,
    max_polygon_subdivisions: usize,
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
    fill_metric_up_dir: Option<[f64; 3]>,
) -> Vec<[i64; 3]> {
    let n = boundary_loop.len();
    if n < 4 {
        return triangulate_hole_loop_with_metric_context_and_up_dir(
            vertices,
            existing_faces,
            boundary_loop,
            fill_metric_mode,
            smooth_bd,
            fill_metric_up_dir,
        )
        .faces;
    }
    if multiple_edges_resolve_mode == FillHoleMultipleEdgesResolveMode::None {
        return triangulate_hole_loop_with_metric_context_and_up_dir(
            vertices,
            existing_faces,
            boundary_loop,
            fill_metric_mode,
            smooth_bd,
            fill_metric_up_dir,
        )
        .faces;
    }

    let disallowed_chords = existing_nonboundary_loop_chords(existing_faces, boundary_loop);
    if multiple_edges_resolve_mode == FillHoleMultipleEdgesResolveMode::Strong {
        return triangulate_avoiding_reused_chords(
            vertices,
            existing_faces,
            boundary_loop,
            &disallowed_chords,
            max_polygon_subdivisions,
            fill_metric_mode,
            smooth_bd,
            fill_metric_up_dir,
        )
        .or_else(|| {
            triangulate_avoiding_chords(
                vertices,
                existing_faces,
                boundary_loop,
                &disallowed_chords,
                max_polygon_subdivisions,
                fill_metric_mode,
                smooth_bd,
                fill_metric_up_dir,
            )
        })
        .unwrap_or_else(|| {
            triangulate_hole_loop_with_metric_context_and_up_dir(
                vertices,
                existing_faces,
                boundary_loop,
                fill_metric_mode,
                smooth_bd,
                fill_metric_up_dir,
            )
            .faces
        });
    }
    if disallowed_chords.is_empty() {
        return triangulate_hole_loop_with_metric_context_and_up_dir(
            vertices,
            existing_faces,
            boundary_loop,
            fill_metric_mode,
            smooth_bd,
            fill_metric_up_dir,
        )
        .faces;
    }

    triangulate_avoiding_chords(
        vertices,
        existing_faces,
        boundary_loop,
        &disallowed_chords,
        max_polygon_subdivisions,
        fill_metric_mode,
        smooth_bd,
        fill_metric_up_dir,
    )
    .unwrap_or_else(|| {
        triangulate_hole_loop_with_metric_context_and_up_dir(
            vertices,
            existing_faces,
            boundary_loop,
            fill_metric_mode,
            smooth_bd,
            fill_metric_up_dir,
        )
        .faces
    })
}

fn existing_nonboundary_loop_chords(
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
) -> HashSet<(usize, usize)> {
    let n = boundary_loop.len();
    let mut positions: HashMap<usize, Vec<usize>> = HashMap::with_capacity(n);
    for (position, vertex) in boundary_loop.iter().enumerate() {
        positions.entry(*vertex).or_default().push(position);
    }

    let mut chords = HashSet::new();
    for face in existing_faces {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            if a < 0 || b < 0 {
                continue;
            }
            let Some(a_positions) = positions.get(&(a as usize)) else {
                continue;
            };
            let Some(b_positions) = positions.get(&(b as usize)) else {
                continue;
            };
            for &a_pos in a_positions {
                for &b_pos in b_positions {
                    if a_pos == b_pos || loop_positions_are_adjacent(a_pos, b_pos, n) {
                        continue;
                    }
                    chords.insert(ordered_pair(a_pos, b_pos));
                }
            }
        }
    }
    chords
}

fn triangulate_avoiding_chords(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    disallowed_chords: &HashSet<(usize, usize)>,
    max_polygon_subdivisions: usize,
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
    fill_metric_up_dir: Option<[f64; 3]>,
) -> Option<Vec<[i64; 3]>> {
    let n = boundary_loop.len();
    let points = boundary_loop
        .iter()
        .map(|index| vertices[*index])
        .collect::<Vec<_>>();
    let metric_context =
        fill_metric_context_with_up_dir(fill_metric_mode, &points, fill_metric_up_dir);
    let boundary_contexts = boundary_edge_contexts(existing_faces, boundary_loop);
    let mut table = vec![
        vec![
            FillCell {
                weight: 0.0,
                split: None,
            };
            n
        ];
        n
    ];

    for span in 2..n {
        for start in 0..(n - span) {
            let end = start + span;
            if chord_is_disallowed(start, end, n, disallowed_chords) {
                table[start][end] = FillCell {
                    weight: f64::INFINITY,
                    split: None,
                };
                continue;
            }

            let mut best = FillCell {
                weight: f64::INFINITY,
                split: None,
            };
            for split in optimal_split_steps(start, end, n, max_polygon_subdivisions) {
                let weight = candidate_weight(
                    CandidateRequest {
                        vertices,
                        points: &points,
                        boundary_loop,
                        boundary_contexts: &boundary_contexts,
                        fill_metric_mode,
                        metric_context,
                        smooth_bd,
                        start,
                        split,
                        end,
                        include_final_edge: start == 0 && end == n - 1,
                    },
                    CandidateWeights {
                        left_weight: table[start][split].weight,
                        left_prev: table[start][split].split,
                        right_weight: table[split][end].weight,
                        right_prev: table[split][end].split,
                    },
                );
                if weight < best.weight {
                    best = FillCell {
                        weight,
                        split: Some(split),
                    };
                }
            }
            table[start][end] = best;
        }
    }

    if !table[0][n - 1].weight.is_finite() {
        return None;
    }

    let mut faces = Vec::with_capacity(n - 2);
    collect_faces(&table, boundary_loop, 0, n - 1, &mut faces);
    (faces.len() == n - 2).then_some(faces)
}

pub(super) fn optimal_split_steps(
    start: usize,
    end: usize,
    loop_size: usize,
    max_polygon_subdivisions: usize,
) -> Vec<usize> {
    if loop_size == 0 || start == end {
        return Vec::new();
    }
    let boundary_steps = if end > start {
        end - start
    } else {
        loop_size - start + end
    };
    if boundary_steps <= 1 {
        return Vec::new();
    }

    let max_polygon_subdivisions = max_polygon_subdivisions.max(2);
    let first_step = (start + 1) % loop_size;
    let split_count = boundary_steps - 1;
    if split_count <= max_polygon_subdivisions {
        return (0..split_count)
            .map(|offset| (first_step + offset) % loop_size)
            .collect();
    }

    let mut optimal_steps = Vec::with_capacity(max_polygon_subdivisions);
    for offset in 0..(max_polygon_subdivisions / 4) {
        optimal_steps.push((first_step + offset) % loop_size);
    }

    let mut big_step =
        (split_count - (max_polygon_subdivisions / 2)) / (max_polygon_subdivisions / 2);
    let mut num_big_steps = max_polygon_subdivisions / 2;
    if big_step < 2 {
        big_step = 2;
        num_big_steps = max_polygon_subdivisions / 4;
    }
    let big_step_half = big_step / 2;
    let new_start = first_step + big_step_half + (max_polygon_subdivisions / 4) - 1;
    for index in 0..num_big_steps {
        optimal_steps.push((new_start + index * big_step) % loop_size);
    }

    for offset in (0..(max_polygon_subdivisions / 4)).rev() {
        optimal_steps.push((first_step + split_count - offset - 1) % loop_size);
    }
    optimal_steps
}

pub(crate) fn triangulate_hole_loop(
    vertices: &[[f64; 3]],
    boundary_loop: &[usize],
) -> Vec<[i64; 3]> {
    triangulate_hole_loop_with_metric(vertices, boundary_loop, FillHoleMetricMode::Circumscribed)
}

fn triangulate_hole_loop_with_metric(
    vertices: &[[f64; 3]],
    boundary_loop: &[usize],
    fill_metric_mode: FillHoleMetricMode,
) -> Vec<[i64; 3]> {
    triangulate_hole_loop_with_metric_and_context(
        vertices,
        &[],
        boundary_loop,
        fill_metric_mode,
        false,
    )
    .faces
}

fn triangulate_hole_loop_with_metric_and_context(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
) -> TriangulationOutcome {
    triangulate_hole_loop_with_metric_context_and_up_dir(
        vertices,
        existing_faces,
        boundary_loop,
        fill_metric_mode,
        smooth_bd,
        None,
    )
}

fn triangulate_hole_loop_with_metric_context_and_up_dir(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
    fill_metric_up_dir: Option<[f64; 3]>,
) -> TriangulationOutcome {
    let n = boundary_loop.len();
    if n < 3 {
        return TriangulationOutcome {
            faces: Vec::new(),
            #[cfg(test)]
            weight: 0.0,
        };
    }
    if n == 3 {
        return TriangulationOutcome {
            faces: vec![[
                boundary_loop[0] as i64,
                boundary_loop[1] as i64,
                boundary_loop[2] as i64,
            ]],
            #[cfg(test)]
            weight: 0.0,
        };
    }

    let points: Vec<[f64; 3]> = boundary_loop.iter().map(|index| vertices[*index]).collect();
    let metric_context =
        fill_metric_context_with_up_dir(fill_metric_mode, &points, fill_metric_up_dir);
    let boundary_contexts = boundary_edge_contexts(existing_faces, boundary_loop);
    let mut table = vec![
        vec![
            FillCell {
                weight: 0.0,
                split: None,
            };
            n
        ];
        n
    ];

    for span in 2..n {
        for start in 0..(n - span) {
            let end = start + span;
            let mut best = FillCell {
                weight: f64::INFINITY,
                split: None,
            };
            for split in (start + 1)..end {
                let weight = candidate_weight(
                    CandidateRequest {
                        vertices,
                        points: &points,
                        boundary_loop,
                        boundary_contexts: &boundary_contexts,
                        fill_metric_mode,
                        metric_context,
                        smooth_bd,
                        start,
                        split,
                        end,
                        include_final_edge: start == 0 && end == n - 1,
                    },
                    CandidateWeights {
                        left_weight: table[start][split].weight,
                        left_prev: table[start][split].split,
                        right_weight: table[split][end].weight,
                        right_prev: table[split][end].split,
                    },
                );
                if weight < best.weight {
                    best = FillCell {
                        weight,
                        split: Some(split),
                    };
                }
            }
            table[start][end] = best;
        }
    }

    let mut faces = Vec::with_capacity(n - 2);
    collect_faces(&table, boundary_loop, 0, n - 1, &mut faces);
    TriangulationOutcome {
        faces,
        #[cfg(test)]
        weight: table[0][n - 1].weight,
    }
}

fn collect_faces(
    table: &[Vec<FillCell>],
    boundary_loop: &[usize],
    start: usize,
    end: usize,
    faces: &mut Vec<[i64; 3]>,
) {
    if end <= start + 1 {
        return;
    }
    let Some(split) = table[start][end].split else {
        return;
    };
    faces.push([
        boundary_loop[start] as i64,
        boundary_loop[split] as i64,
        boundary_loop[end] as i64,
    ]);
    collect_faces(table, boundary_loop, start, split, faces);
    collect_faces(table, boundary_loop, split, end, faces);
}

pub(super) fn chord_is_disallowed(
    start: usize,
    end: usize,
    n: usize,
    disallowed_chords: &HashSet<(usize, usize)>,
) -> bool {
    !loop_positions_are_adjacent(start, end, n)
        && disallowed_chords.contains(&ordered_pair(start, end))
}

pub(super) fn loop_positions_are_adjacent(a: usize, b: usize, n: usize) -> bool {
    a.abs_diff(b) == 1 || a.abs_diff(b) + 1 == n
}

pub(super) fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
pub(super) fn triangulate_hole_loop_weight_with_fill_params_for_tests(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
) -> f64 {
    triangulate_hole_loop_with_metric_and_context(
        vertices,
        existing_faces,
        boundary_loop,
        fill_metric_mode,
        smooth_bd,
    )
    .weight
}
