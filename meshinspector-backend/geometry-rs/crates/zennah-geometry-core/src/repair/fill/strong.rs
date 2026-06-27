use super::metrics::fill_metric_context_with_up_dir;
use super::scoring::{
    boundary_edge_contexts, candidate_weight, CandidateRequest, CandidateWeights,
};
use super::triangulation::{
    chord_is_disallowed, loop_positions_are_adjacent, optimal_split_steps, ordered_pair,
};
use super::FillHoleMetricMode;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
struct StrongCell {
    weight: f64,
    split: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct PositionedFace {
    vertices: [i64; 3],
    positions: [usize; 3],
}

pub(super) fn triangulate_avoiding_reused_chords(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    initial_disallowed_chords: &HashSet<(usize, usize)>,
    max_polygon_subdivisions: usize,
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
    fill_metric_up_dir: Option<[f64; 3]>,
) -> Option<Vec<[i64; 3]>> {
    let mut disallowed_chords = initial_disallowed_chords.clone();
    let max_attempts = boundary_loop.len() * boundary_loop.len() + 1;

    for _ in 0..max_attempts {
        let positioned_faces = triangulate_with_positions(
            vertices,
            existing_faces,
            boundary_loop,
            &disallowed_chords,
            max_polygon_subdivisions,
            fill_metric_mode,
            smooth_bd,
            fill_metric_up_dir,
        )?;
        let reused_chords = reused_nonboundary_chords(&positioned_faces, boundary_loop.len());
        if reused_chords.is_empty() {
            return Some(
                positioned_faces
                    .into_iter()
                    .map(|face| face.vertices)
                    .collect(),
            );
        }

        let mut changed = false;
        for chord in reused_chords {
            changed |= disallowed_chords.insert(chord);
        }
        if !changed {
            return None;
        }
    }

    None
}

fn triangulate_with_positions(
    vertices: &[[f64; 3]],
    existing_faces: &[[i64; 3]],
    boundary_loop: &[usize],
    disallowed_chords: &HashSet<(usize, usize)>,
    max_polygon_subdivisions: usize,
    fill_metric_mode: FillHoleMetricMode,
    smooth_bd: bool,
    fill_metric_up_dir: Option<[f64; 3]>,
) -> Option<Vec<PositionedFace>> {
    let n = boundary_loop.len();
    if n < 3 {
        return Some(Vec::new());
    }
    if n == 3 {
        return Some(vec![PositionedFace {
            vertices: [
                boundary_loop[0] as i64,
                boundary_loop[1] as i64,
                boundary_loop[2] as i64,
            ],
            positions: [0, 1, 2],
        }]);
    }

    let points: Vec<[f64; 3]> = boundary_loop.iter().map(|index| vertices[*index]).collect();
    let metric_context =
        fill_metric_context_with_up_dir(fill_metric_mode, &points, fill_metric_up_dir);
    let boundary_contexts = boundary_edge_contexts(existing_faces, boundary_loop);
    let mut table = vec![
        vec![
            StrongCell {
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
                table[start][end] = StrongCell {
                    weight: f64::INFINITY,
                    split: None,
                };
                continue;
            }

            let mut best = StrongCell {
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
                    best = StrongCell {
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
    collect_positioned_faces(&table, boundary_loop, 0, n - 1, &mut faces);
    (faces.len() == n - 2).then_some(faces)
}

fn collect_positioned_faces(
    table: &[Vec<StrongCell>],
    boundary_loop: &[usize],
    start: usize,
    end: usize,
    faces: &mut Vec<PositionedFace>,
) {
    if end <= start + 1 {
        return;
    }
    let Some(split) = table[start][end].split else {
        return;
    };
    faces.push(PositionedFace {
        vertices: [
            boundary_loop[start] as i64,
            boundary_loop[split] as i64,
            boundary_loop[end] as i64,
        ],
        positions: [start, split, end],
    });
    collect_positioned_faces(table, boundary_loop, start, split, faces);
    collect_positioned_faces(table, boundary_loop, split, end, faces);
}

fn reused_nonboundary_chords(
    faces: &[PositionedFace],
    boundary_loop_len: usize,
) -> HashSet<(usize, usize)> {
    let mut seen: HashMap<(i64, i64), (usize, usize)> = HashMap::new();
    let mut reused = HashSet::new();

    for face in faces {
        for edge_index in 0..3 {
            let next_index = (edge_index + 1) % 3;
            let a_position = face.positions[edge_index];
            let b_position = face.positions[next_index];
            if loop_positions_are_adjacent(a_position, b_position, boundary_loop_len) {
                continue;
            }

            let position_pair = ordered_pair(a_position, b_position);
            let vertex_pair =
                ordered_vertex_pair(face.vertices[edge_index], face.vertices[next_index]);
            if vertex_pair.0 == vertex_pair.1 {
                reused.insert(position_pair);
                continue;
            }
            if let Some(previous_position_pair) = seen.insert(vertex_pair, position_pair) {
                reused.insert(previous_position_pair);
                reused.insert(position_pair);
            }
        }
    }

    reused
}

fn ordered_vertex_pair(a: i64, b: i64) -> (i64, i64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
