use crate::math::{cross, norm, scale, sub};
use crate::mesh::{edge_face_map, signed_volume, surface_area, validate_faces};
use crate::{BasicRepairResult, GeometryError, MeshEditResult, RepairReport};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

pub(crate) mod fill;
pub(crate) use fill::triangulate_hole_loop;
pub use fill::{
    fill_planar_holes, service_fill_holes, service_fill_holes_with_fill_params,
    service_fill_holes_with_fill_params_and_metric_up_dir,
    service_fill_holes_with_max_polygon_subdivisions, FillHoleMetricMode,
    FillHoleMultipleEdgesResolveMode,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixSelfIntersectionsRelaxOptions {
    pub touch_is_intersection: bool,
    pub relax_iterations: usize,
    pub max_expand: usize,
    pub force: f64,
    pub epsilon: f64,
}

impl Default for FixSelfIntersectionsRelaxOptions {
    fn default() -> Self {
        Self {
            touch_is_intersection: true,
            relax_iterations: 5,
            max_expand: 3,
            force: 0.5,
            epsilon: 1e-8,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixSelfIntersectionsRelaxReport {
    pub input_vertex_count: usize,
    pub input_face_count: usize,
    pub output_vertex_count: usize,
    pub output_face_count: usize,
    pub input_self_intersections: usize,
    pub output_self_intersections: usize,
    pub relaxed_face_count: usize,
    pub moved_vertex_count: usize,
    pub relax_iterations: usize,
    pub max_expand: usize,
    pub force: f64,
    pub method: String,
    pub subdivide_edge_len_disabled: bool,
    pub topology_changed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixSelfIntersectionsRelaxResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: FixSelfIntersectionsRelaxReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindDisorientationRayMode {
    Positive,
    Shallowest,
    Both,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum MergeKey {
    Quantized([i64; 3]),
    Exact([u64; 3]),
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

pub fn unite_close_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    close_dist: f64,
    unite_only_boundary: bool,
) -> Result<MeshEditResult, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if vertices.is_empty() {
        return Ok(MeshEditResult {
            vertices: Vec::new(),
            faces: faces_i64.to_vec(),
            changed_count: 0,
        });
    }

    let close_dist = close_dist.max(0.0);
    let candidate_vertices = if unite_only_boundary {
        Some(boundary_vertex_set(&faces))
    } else {
        None
    };
    let is_candidate = |vertex: usize| {
        candidate_vertices
            .as_ref()
            .is_none_or(|candidates| candidates.contains(&vertex))
    };

    let mut old_to_new = (0..vertices.len()).collect::<Vec<_>>();
    for vertex in 0..vertices.len() {
        if !is_candidate(vertex) {
            continue;
        }
        let mut smallest_close = vertex;
        for other in 0..vertices.len() {
            if other == vertex || !is_candidate(other) {
                continue;
            }
            if norm(sub(vertices[vertex], vertices[other])) <= close_dist {
                smallest_close = smallest_close.min(other);
            }
        }
        old_to_new[vertex] = smallest_close;
    }

    for vertex in 0..vertices.len() {
        if !is_candidate(vertex) {
            continue;
        }
        let mapped = old_to_new[vertex];
        if mapped == vertex || old_to_new[mapped] == mapped {
            continue;
        }
        let mut smallest_stable = vertex;
        for other in 0..vertices.len() {
            if other == vertex || !is_candidate(other) || old_to_new[other] != other {
                continue;
            }
            if norm(sub(vertices[vertex], vertices[other])) <= close_dist {
                smallest_stable = smallest_stable.min(other);
            }
        }
        old_to_new[vertex] = smallest_stable;
    }

    let changed_count = old_to_new
        .iter()
        .enumerate()
        .filter(|(vertex, mapped)| *vertex != **mapped)
        .count();
    if changed_count == 0 {
        return Ok(MeshEditResult {
            vertices: vertices.to_vec(),
            faces: faces_i64.to_vec(),
            changed_count,
        });
    }

    let mut used = BTreeSet::new();
    let mut remapped_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let remapped = [
            old_to_new[face[0]],
            old_to_new[face[1]],
            old_to_new[face[2]],
        ];
        if remapped[0] == remapped[1] || remapped[1] == remapped[2] || remapped[0] == remapped[2] {
            continue;
        }
        used.extend(remapped);
        remapped_faces.push(remapped);
    }

    let mut compact_map = vec![-1_i64; vertices.len()];
    let mut compact_vertices = Vec::with_capacity(used.len());
    for old_index in used {
        compact_map[old_index] = compact_vertices.len() as i64;
        compact_vertices.push(vertices[old_index]);
    }
    let compact_faces = remapped_faces
        .into_iter()
        .map(|face| {
            [
                compact_map[face[0]],
                compact_map[face[1]],
                compact_map[face[2]],
            ]
        })
        .collect();

    Ok(MeshEditResult {
        vertices: compact_vertices,
        faces: compact_faces,
        changed_count,
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

pub fn flip_normals(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[i64; 3]>, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    Ok(faces_i64
        .iter()
        .map(|face| [face[0], face[2], face[1]])
        .collect())
}

pub fn find_disoriented_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    mode: FindDisorientationRayMode,
    epsilon: f64,
) -> Result<Vec<usize>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let epsilon = epsilon.max(0.0);
    let mut candidates = Vec::new();
    let mut origins = Vec::new();
    let mut directions = Vec::new();
    let mut ignored_faces = Vec::new();

    for (face_index, face) in faces.iter().enumerate() {
        let a = vertices[face[0]];
        let b = vertices[face[1]];
        let c = vertices[face[2]];
        let normal = cross(sub(b, a), sub(c, a));
        let normal_len = norm(normal);
        if normal_len <= epsilon {
            continue;
        }
        let normal = scale(normal, 1.0 / normal_len);
        let center = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];

        candidates.push(face_index);
        origins.push(center);
        directions.push(normal);
        ignored_faces.push(face_index as i64);
    }

    if mode != FindDisorientationRayMode::Positive {
        let candidate_count = candidates.len();
        origins.reserve(candidate_count);
        directions.reserve(candidate_count);
        ignored_faces.reserve(candidate_count);
        for index in 0..candidate_count {
            origins.push(origins[index]);
            directions.push(scale(directions[index], -1.0));
            ignored_faces.push(ignored_faces[index]);
        }
    }

    let hit_counts = crate::spatial::ray_triangle_hit_counts(
        vertices,
        faces_i64,
        &origins,
        &directions,
        epsilon,
        &ignored_faces,
    )?;
    let candidate_count = candidates.len();
    let mut disoriented = Vec::new();

    for (candidate_index, face_index) in candidates.into_iter().enumerate() {
        let positive_counter = hit_counts[candidate_index];
        let positive_valid = positive_counter % 2 == 0;
        let mut result_valid = positive_valid;

        if mode != FindDisorientationRayMode::Positive {
            let negative_counter = hit_counts[candidate_count + candidate_index];
            let negative_valid = negative_counter % 2 == 1;
            let adjusted_negative_counter = negative_counter as isize - 1;

            result_valid = positive_valid && negative_valid;
            if mode == FindDisorientationRayMode::Shallowest && positive_valid != negative_valid {
                if positive_counter as isize == adjusted_negative_counter {
                    result_valid = true;
                } else if adjusted_negative_counter < positive_counter as isize {
                    result_valid = negative_valid;
                }
            }
        }

        if !result_valid {
            disoriented.push(face_index);
        }
    }

    Ok(disoriented)
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

pub fn fix_self_intersections_relax(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    options: FixSelfIntersectionsRelaxOptions,
) -> Result<FixSelfIntersectionsRelaxResult, GeometryError> {
    validate_fix_self_intersections_relax_options(options)?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    let input_self_intersection_faces =
        crate::spatial::self_intersecting_faces_by_component_with_touch(
            vertices,
            faces_i64,
            options.epsilon,
            options.touch_is_intersection,
        )?;
    let mut output_vertices = vertices.to_vec();
    let mut relaxed_face_count = 0_usize;
    let mut moved_vertex_count = 0_usize;

    if !input_self_intersection_faces.is_empty() && options.relax_iterations > 0 {
        let relaxed_faces = expanded_self_intersection_face_region(
            &faces,
            &input_self_intersection_faces,
            options.max_expand,
        );
        relaxed_face_count = relaxed_faces.iter().filter(|selected| **selected).count();
        let relaxed_vertices =
            incident_vertices_for_face_region(vertices.len(), &faces, &relaxed_faces);
        let original_vertices = output_vertices.clone();
        relax_selected_vertices(
            &mut output_vertices,
            &faces,
            &relaxed_vertices,
            options.relax_iterations,
            options.force,
        );
        moved_vertex_count = output_vertices
            .iter()
            .zip(original_vertices.iter())
            .filter(|(after, before)| norm(sub(**after, **before)) > 1e-12)
            .count();
    } else if !input_self_intersection_faces.is_empty() {
        let relaxed_faces = expanded_self_intersection_face_region(
            &faces,
            &input_self_intersection_faces,
            options.max_expand,
        );
        relaxed_face_count = relaxed_faces.iter().filter(|selected| **selected).count();
    }

    let output_self_intersections =
        crate::spatial::self_intersecting_faces_by_component_with_touch(
            &output_vertices,
            faces_i64,
            options.epsilon,
            options.touch_is_intersection,
        )?
        .len();

    Ok(FixSelfIntersectionsRelaxResult {
        vertices: output_vertices,
        faces: faces_i64.to_vec(),
        report: FixSelfIntersectionsRelaxReport {
            input_vertex_count: vertices.len(),
            input_face_count: faces_i64.len(),
            output_vertex_count: vertices.len(),
            output_face_count: faces_i64.len(),
            input_self_intersections: input_self_intersection_faces.len(),
            output_self_intersections,
            relaxed_face_count,
            moved_vertex_count,
            relax_iterations: options.relax_iterations,
            max_expand: options.max_expand,
            force: options.force,
            method: "relax".to_string(),
            subdivide_edge_len_disabled: true,
            topology_changed: false,
        },
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
