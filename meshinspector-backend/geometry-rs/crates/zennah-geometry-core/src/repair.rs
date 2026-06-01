use crate::math::{cross, dot, norm, scale, sub};
use crate::mesh::{signed_volume, surface_area, validate_faces};
use crate::{
    BasicRepairResult, GeometryError, HoleFillReport, HoleFillResult, MeshEditResult, RepairReport,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum MergeKey {
    Quantized([i64; 3]),
    Exact([u64; 3]),
}

#[derive(Clone, Copy, Debug)]
struct TriangulationCell {
    weight: f64,
    split: Option<usize>,
}

pub fn remove_degenerate_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    area_epsilon: f64,
) -> Result<MeshEditResult, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut kept_faces = Vec::with_capacity(faces_i64.len());
    let mut removed = 0_usize;
    for (face_index, face) in faces.iter().enumerate() {
        let unique_vertices = face[0] != face[1] && face[1] != face[2] && face[0] != face[2];
        let area = face_area(vertices, *face);
        if unique_vertices && area > area_epsilon {
            kept_faces.push(faces_i64[face_index]);
        } else {
            removed += 1;
        }
    }
    Ok(MeshEditResult {
        vertices: vertices.to_vec(),
        faces: kept_faces,
        changed_count: removed,
    })
}

pub fn remove_unreferenced_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<MeshEditResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    if vertices.is_empty() {
        return Ok(MeshEditResult {
            vertices: Vec::new(),
            faces: faces_i64.to_vec(),
            changed_count: 0,
        });
    }
    if faces_i64.is_empty() {
        return Ok(MeshEditResult {
            vertices: Vec::new(),
            faces: Vec::new(),
            changed_count: vertices.len(),
        });
    }

    let used: BTreeSet<usize> = faces_i64
        .iter()
        .flat_map(|face| face.iter().map(|vertex| *vertex as usize))
        .collect();
    let mut remap = vec![-1_i64; vertices.len()];
    let mut compact_vertices = Vec::with_capacity(used.len());
    for (new_index, old_index) in used.iter().enumerate() {
        remap[*old_index] = new_index as i64;
        compact_vertices.push(vertices[*old_index]);
    }
    let compact_faces = faces_i64
        .iter()
        .map(|face| {
            [
                remap[face[0] as usize],
                remap[face[1] as usize],
                remap[face[2] as usize],
            ]
        })
        .collect();

    Ok(MeshEditResult {
        vertices: compact_vertices,
        faces: compact_faces,
        changed_count: vertices.len() - used.len(),
    })
}

pub fn merge_close_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    tolerance: f64,
) -> Result<MeshEditResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    if vertices.is_empty() {
        return Ok(MeshEditResult {
            vertices: Vec::new(),
            faces: faces_i64.to_vec(),
            changed_count: 0,
        });
    }

    let tolerance = tolerance.max(0.0);
    let mut groups: HashMap<MergeKey, usize> = HashMap::new();
    let mut first_indices = Vec::new();
    let mut vertex_to_group = Vec::with_capacity(vertices.len());
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        let key = merge_key(*vertex, tolerance);
        let group_index = match groups.get(&key) {
            Some(existing) => *existing,
            None => {
                let new_index = first_indices.len();
                groups.insert(key, new_index);
                first_indices.push(vertex_index);
                new_index
            }
        };
        vertex_to_group.push(group_index as i64);
    }

    let merged_vertices: Vec<[f64; 3]> =
        first_indices.iter().map(|index| vertices[*index]).collect();
    let merged_faces = faces_i64
        .iter()
        .map(|face| {
            [
                vertex_to_group[face[0] as usize],
                vertex_to_group[face[1] as usize],
                vertex_to_group[face[2] as usize],
            ]
        })
        .collect();
    Ok(MeshEditResult {
        vertices: merged_vertices,
        faces: merged_faces,
        changed_count: vertices.len() - first_indices.len(),
    })
}

pub fn orient_faces_outward(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[i64; 3]>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces_i64.is_empty() || signed_volume(vertices, &faces) >= 0.0 {
        return Ok(faces_i64.to_vec());
    }
    Ok(faces_i64
        .iter()
        .map(|face| [face[0], face[2], face[1]])
        .collect())
}

pub fn basic_repair(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    merge_tolerance: f64,
    area_epsilon: f64,
) -> Result<BasicRepairResult, GeometryError> {
    let input_vertex_count = vertices.len();
    let input_face_count = faces_i64.len();

    let merged = merge_close_vertices(vertices, faces_i64, merge_tolerance)?;
    let degenerate_removed =
        remove_degenerate_faces(&merged.vertices, &merged.faces, area_epsilon)?;
    let unreferenced_removed =
        remove_unreferenced_vertices(&degenerate_removed.vertices, &degenerate_removed.faces)?;
    let report = RepairReport {
        input_vertex_count,
        input_face_count,
        output_vertex_count: unreferenced_removed.vertices.len(),
        output_face_count: unreferenced_removed.faces.len(),
        merged_vertices: merged.changed_count,
        removed_degenerate_faces: degenerate_removed.changed_count,
        removed_unreferenced_vertices: unreferenced_removed.changed_count,
    };

    Ok(BasicRepairResult {
        vertices: unreferenced_removed.vertices,
        faces: unreferenced_removed.faces,
        report,
    })
}

pub fn repaired_surface_area(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<f64, GeometryError> {
    let repaired = basic_repair(vertices, faces_i64, 1e-6, 1e-12)?;
    let faces = validate_faces(&repaired.faces, repaired.vertices.len())?;
    Ok(surface_area(&repaired.vertices, &faces))
}

pub fn ordered_boundary_loops(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<Vec<usize>>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let edges = boundary_edges_in_python_order(&faces);
    if edges.is_empty() {
        return Ok(Vec::new());
    }

    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
    for (a, b) in &edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }
    let mut unused: BTreeSet<(usize, usize)> = edges.into_iter().collect();
    let mut loops = Vec::new();

    while let Some((a, b)) = unused.iter().next().copied() {
        let mut loop_vertices = vec![a, b];
        unused.remove(&(a, b));
        let mut previous = a;
        let mut current = b;

        loop {
            let mut next_vertex = None;
            if let Some(candidates) = adjacency.get(&current) {
                for candidate in candidates {
                    if *candidate == previous {
                        continue;
                    }
                    let edge = ordered_edge(current, *candidate);
                    if unused.contains(&edge) {
                        next_vertex = Some(*candidate);
                        break;
                    }
                }
            }
            let Some(next) = next_vertex else {
                break;
            };
            loop_vertices.push(next);
            unused.remove(&ordered_edge(current, next));
            previous = current;
            current = next;
            if current == loop_vertices[0] {
                loop_vertices.pop();
                break;
            }
        }

        if loop_vertices.len() >= 3 {
            loops.push(loop_vertices);
        }
    }
    Ok(loops)
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
                skipped_holes: 0,
            },
        });
    }

    let mut output_vertices = vertices.to_vec();
    let mut output_faces = faces_i64.to_vec();
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
            skipped_holes: skipped,
        },
    })
}

pub fn service_fill_holes(
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
                skipped_holes: 0,
            },
        });
    }

    let mut output_faces = faces_i64.to_vec();
    let mesh_center = centroid(vertices);
    let mut filled = 0_usize;
    let mut added_faces = 0_usize;
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

        let new_faces = triangulate_hole_loop(vertices, &boundary_loop);
        added_faces += new_faces.len();
        output_faces.extend(new_faces);
        filled += 1;
    }

    let output_faces = if filled > 0 {
        orient_faces_outward(vertices, &output_faces)?
    } else {
        output_faces
    };
    Ok(HoleFillResult {
        vertices: vertices.to_vec(),
        faces: output_faces,
        report: HoleFillReport {
            input_holes: loops.len(),
            filled_holes: filled,
            added_vertices: 0,
            added_faces,
            skipped_holes: skipped,
        },
    })
}

fn face_area(vertices: &[[f64; 3]], face: [usize; 3]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    norm(cross(sub(b, a), sub(c, a))) * 0.5
}

fn boundary_edges_in_python_order(faces: &[[usize; 3]]) -> Vec<(usize, usize)> {
    let mut counts: HashMap<(usize, usize), usize> = HashMap::new();
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    for face in faces {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let edge = ordered_edge(a, b);
            if seen.insert(edge) {
                order.push(edge);
            }
            *counts.entry(edge).or_default() += 1;
        }
    }
    order
        .into_iter()
        .filter(|edge| counts.get(edge) == Some(&1))
        .collect()
}

fn ordered_edge(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    if points.is_empty() {
        return [0.0; 3];
    }
    let mut total = [0.0; 3];
    for point in points {
        for axis in 0..3 {
            total[axis] += point[axis];
        }
    }
    scale(total, 1.0 / points.len() as f64)
}

fn loop_normal(points: &[[f64; 3]]) -> [f64; 3] {
    let mut normal = [0.0; 3];
    if points.is_empty() {
        return normal;
    }
    for (index, point) in points.iter().enumerate() {
        let next = points[(index + 1) % points.len()];
        normal[0] += (point[1] - next[1]) * (point[2] + next[2]);
        normal[1] += (point[2] - next[2]) * (point[0] + next[0]);
        normal[2] += (point[0] - next[0]) * (point[1] + next[1]);
    }
    let magnitude = norm(normal).max(1e-12);
    scale(normal, 1.0 / magnitude)
}

fn triangulate_hole_loop(vertices: &[[f64; 3]], boundary_loop: &[usize]) -> Vec<[i64; 3]> {
    let n = boundary_loop.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[
            boundary_loop[0] as i64,
            boundary_loop[1] as i64,
            boundary_loop[2] as i64,
        ]];
    }

    let points: Vec<[f64; 3]> = boundary_loop.iter().map(|index| vertices[*index]).collect();
    let mut table = vec![
        vec![
            TriangulationCell {
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
            let mut best = TriangulationCell {
                weight: f64::INFINITY,
                split: None,
            };
            for split in (start + 1)..end {
                let weight = table[start][split].weight
                    + table[split][end].weight
                    + triangle_fill_metric(points[start], points[split], points[end]);
                if weight < best.weight {
                    best = TriangulationCell {
                        weight,
                        split: Some(split),
                    };
                }
            }
            table[start][end] = best;
        }
    }

    let mut faces = Vec::with_capacity(n - 2);
    collect_triangulation_faces(&table, boundary_loop, 0, n - 1, &mut faces);
    faces
}

fn collect_triangulation_faces(
    table: &[Vec<TriangulationCell>],
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
    collect_triangulation_faces(table, boundary_loop, start, split, faces);
    collect_triangulation_faces(table, boundary_loop, split, end, faces);
}

fn triangle_fill_metric(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = norm(sub(b, a));
    let bc = norm(sub(c, b));
    let ca = norm(sub(a, c));
    let area = norm(cross(sub(b, a), sub(c, a))) * 0.5;
    if area <= 1e-12 {
        return f64::MAX / 4.0;
    }
    let circumradius = (ab * bc * ca) / (4.0 * area);
    circumradius + (ab + bc + ca) * 1e-6
}

fn merge_key(vertex: [f64; 3], tolerance: f64) -> MergeKey {
    if tolerance == 0.0 {
        return MergeKey::Exact(vertex.map(exact_float_key));
    }
    MergeKey::Quantized(vertex.map(|value| (value / tolerance).round_ties_even() as i64))
}

fn exact_float_key(value: f64) -> u64 {
    if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    }
}
