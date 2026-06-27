use crate::math::{add, cross, norm, scale, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

const CLASSIFICATION_RAY_DIRECTION: [f64; 3] = [1.0, 0.371, 0.219];
#[derive(Debug, Clone, PartialEq)]
pub struct ExactComponentClassification {
    pub component_index: usize,
    pub face_indices: Vec<usize>,
    pub sample_point: [f64; 3],
    pub inside_other: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactMeshPartClassification {
    pub components: Vec<ExactComponentClassification>,
    pub selected_faces: Vec<usize>,
    pub used_cut_path_sides: bool,
    pub cut_paths_consistent: bool,
    pub cut_left_components: usize,
    pub cut_right_components: usize,
    pub cut_path_overlap_components: usize,
}

pub struct ExactCutPathClassificationInput<'a> {
    pub vertices: &'a [[f64; 3]],
    pub faces_i64: &'a [[i64; 3]],
    pub other_vertices: &'a [[f64; 3]],
    pub other_faces_i64: &'a [[i64; 3]],
    pub cut_edges: &'a [[usize; 2]],
    pub cut_edge_paths: &'a [Vec<[usize; 2]>],
    pub need_inside: bool,
    pub origin_is_first: bool,
    pub epsilon: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct ExactCutPathSideComponentDetails {
    pub component_faces: Vec<Vec<usize>>,
    pub left_component_indices: Vec<usize>,
    pub right_component_indices: Vec<usize>,
    pub overlap_component_indices: Vec<usize>,
    pub left_component_faces: Vec<Vec<usize>>,
    pub right_component_faces: Vec<Vec<usize>>,
    pub overlap_component_faces: Vec<Vec<usize>>,
}

pub fn exact_classify_components(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    other_vertices: &[[f64; 3]],
    other_faces_i64: &[[i64; 3]],
    cut_edges: &[[usize; 2]],
    need_inside: bool,
    epsilon: f64,
) -> Result<ExactMeshPartClassification, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    validate_faces(other_faces_i64, other_vertices.len())?;
    let components = connected_components_excluding_cut_edges(&faces, cut_edges);
    classify_components_by_sampling(
        vertices,
        &faces,
        other_vertices,
        other_faces_i64,
        &components,
        need_inside,
        epsilon,
    )
}

pub fn exact_classify_components_with_cut_paths(
    input: ExactCutPathClassificationInput<'_>,
) -> Result<ExactMeshPartClassification, GeometryError> {
    exact_classify_components_with_cut_paths_impl(input, &[], true)
}

pub(super) fn exact_classify_components_with_cut_paths_and_barriers(
    input: ExactCutPathClassificationInput<'_>,
    extra_barrier_edges: &[[usize; 2]],
) -> Result<ExactMeshPartClassification, GeometryError> {
    exact_classify_components_with_cut_paths_impl(input, extra_barrier_edges, true)
}

pub(super) fn exact_classify_components_with_cut_paths_and_barriers_without_orientation_normalization(
    input: ExactCutPathClassificationInput<'_>,
    extra_barrier_edges: &[[usize; 2]],
) -> Result<ExactMeshPartClassification, GeometryError> {
    exact_classify_components_with_cut_paths_impl(input, extra_barrier_edges, false)
}

fn exact_classify_components_with_cut_paths_impl(
    input: ExactCutPathClassificationInput<'_>,
    extra_barrier_edges: &[[usize; 2]],
    normalize_cut_path_orientation: bool,
) -> Result<ExactMeshPartClassification, GeometryError> {
    let faces = validate_faces(input.faces_i64, input.vertices.len())?;
    validate_faces(input.other_faces_i64, input.other_vertices.len())?;
    if input.cut_edge_paths.is_empty() {
        let component_cut_edges = component_cut_edges(input.cut_edges, extra_barrier_edges);
        let components = connected_components_excluding_cut_edges(&faces, &component_cut_edges);
        let mut classification = classify_components_by_sampling(
            input.vertices,
            &faces,
            input.other_vertices,
            input.other_faces_i64,
            &components,
            input.need_inside,
            input.epsilon,
        )?;
        classification.cut_paths_consistent = true;
        return Ok(classification);
    }

    let path_cut_edges =
        component_cut_edges(&cut_path_edges(input.cut_edge_paths), extra_barrier_edges);
    let components = connected_components_excluding_cut_edges(&faces, &path_cut_edges);
    let face_to_component = face_to_component_map(faces.len(), &components);
    let directed_faces = directed_face_map(&faces);
    let side_components = cut_path_side_components_impl(
        input.cut_edge_paths,
        &directed_faces,
        &face_to_component,
        normalize_cut_path_orientation,
    );
    if !side_components.consistent() {
        let mut classification = classify_components_by_sampling(
            input.vertices,
            &faces,
            input.other_vertices,
            input.other_faces_i64,
            &components,
            input.need_inside,
            input.epsilon,
        )?;
        classification.cut_paths_consistent = false;
        classification.cut_left_components = side_components.left_root_count();
        classification.cut_right_components = side_components.right_root_count();
        classification.cut_path_overlap_components = side_components.overlap_count();
        return Ok(classification);
    }

    let need_right_part = input.need_inside != input.origin_is_first;
    let include_components = if need_right_part {
        &side_components.right_components
    } else {
        &side_components.left_components
    };
    let exclude_components = if need_right_part {
        &side_components.left_components
    } else {
        &side_components.right_components
    };
    classify_components_by_cut_sides(CutSideClassificationInput {
        vertices: input.vertices,
        faces: &faces,
        other_vertices: input.other_vertices,
        other_faces_i64: input.other_faces_i64,
        components: &components,
        include_components,
        exclude_components,
        cut_left_components: side_components.left_root_count(),
        cut_right_components: side_components.right_root_count(),
        cut_path_overlap_components: side_components.overlap_count(),
        need_inside: input.need_inside,
        epsilon: input.epsilon,
    })
}

pub(super) fn exact_cut_path_side_component_details(
    vertex_count: usize,
    faces_i64: &[[i64; 3]],
    cut_edge_paths: &[Vec<[usize; 2]>],
) -> Result<ExactCutPathSideComponentDetails, GeometryError> {
    exact_cut_path_side_component_details_with_barriers(
        vertex_count,
        faces_i64,
        cut_edge_paths,
        &[],
    )
}

pub(super) fn exact_cut_path_side_component_details_with_barriers(
    vertex_count: usize,
    faces_i64: &[[i64; 3]],
    cut_edge_paths: &[Vec<[usize; 2]>],
    extra_barrier_edges: &[[usize; 2]],
) -> Result<ExactCutPathSideComponentDetails, GeometryError> {
    let faces = validate_faces(faces_i64, vertex_count)?;
    if cut_edge_paths.is_empty() {
        return Ok(ExactCutPathSideComponentDetails::default());
    }

    let path_cut_edges = component_cut_edges(&cut_path_edges(cut_edge_paths), extra_barrier_edges);
    let components = connected_components_excluding_cut_edges(&faces, &path_cut_edges);
    let face_to_component = face_to_component_map(faces.len(), &components);
    let directed_faces = directed_face_map(&faces);
    let side_components =
        cut_path_side_components(cut_edge_paths, &directed_faces, &face_to_component);
    let left_component_indices = side_components
        .left_components
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let right_component_indices = side_components
        .right_components
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let overlap_component_indices = side_components
        .left_components
        .intersection(&side_components.right_components)
        .copied()
        .collect::<Vec<_>>();
    let component_faces = components
        .iter()
        .map(|component| {
            let mut faces = component.clone();
            faces.sort_unstable();
            faces
        })
        .collect::<Vec<_>>();

    let left_component_faces =
        component_faces_for_indices(&component_faces, &left_component_indices);
    let right_component_faces =
        component_faces_for_indices(&component_faces, &right_component_indices);
    let overlap_component_faces =
        component_faces_for_indices(&component_faces, &overlap_component_indices);

    Ok(ExactCutPathSideComponentDetails {
        component_faces,
        left_component_indices,
        right_component_indices,
        overlap_component_indices,
        left_component_faces,
        right_component_faces,
        overlap_component_faces,
    })
}

fn component_cut_edges(
    base_edges: &[[usize; 2]],
    extra_barrier_edges: &[[usize; 2]],
) -> Vec<[usize; 2]> {
    base_edges
        .iter()
        .chain(extra_barrier_edges)
        .copied()
        .map(ordered_edge)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn classify_components_by_sampling(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    other_vertices: &[[f64; 3]],
    other_faces_i64: &[[i64; 3]],
    components: &[Vec<usize>],
    need_inside: bool,
    epsilon: f64,
) -> Result<ExactMeshPartClassification, GeometryError> {
    let mut output_components = Vec::with_capacity(components.len());
    let mut selected_faces = BTreeSet::new();

    for (component_index, component_faces) in components.iter().enumerate() {
        let sample_point = component_sample_point(vertices, faces, component_faces);
        let inside_other = super::point_inside_mesh(
            other_vertices,
            other_faces_i64,
            sample_point,
            CLASSIFICATION_RAY_DIRECTION,
            epsilon,
        )?;
        let selected = inside_other == need_inside;
        if selected {
            selected_faces.extend(component_faces.iter().copied());
        }
        output_components.push(ExactComponentClassification {
            component_index,
            face_indices: component_faces.clone(),
            sample_point,
            inside_other,
            selected,
        });
    }

    Ok(ExactMeshPartClassification {
        components: output_components,
        selected_faces: selected_faces.into_iter().collect(),
        used_cut_path_sides: false,
        cut_paths_consistent: true,
        cut_left_components: 0,
        cut_right_components: 0,
        cut_path_overlap_components: 0,
    })
}

struct CutSideClassificationInput<'a> {
    vertices: &'a [[f64; 3]],
    faces: &'a [[usize; 3]],
    other_vertices: &'a [[f64; 3]],
    other_faces_i64: &'a [[i64; 3]],
    components: &'a [Vec<usize>],
    include_components: &'a BTreeSet<usize>,
    exclude_components: &'a BTreeSet<usize>,
    cut_left_components: usize,
    cut_right_components: usize,
    cut_path_overlap_components: usize,
    need_inside: bool,
    epsilon: f64,
}

fn classify_components_by_cut_sides(
    input: CutSideClassificationInput<'_>,
) -> Result<ExactMeshPartClassification, GeometryError> {
    let mut output_components = Vec::with_capacity(input.components.len());
    let mut selected_faces = BTreeSet::new();

    for (component_index, component_faces) in input.components.iter().enumerate() {
        let sample_point = component_sample_point(input.vertices, input.faces, component_faces);
        let inside_other = super::point_inside_mesh(
            input.other_vertices,
            input.other_faces_i64,
            sample_point,
            CLASSIFICATION_RAY_DIRECTION,
            input.epsilon,
        )?;
        let selected = if input.include_components.contains(&component_index) {
            true
        } else if input.exclude_components.contains(&component_index) {
            false
        } else {
            inside_other == input.need_inside
        };
        if selected {
            selected_faces.extend(component_faces.iter().copied());
        }
        output_components.push(ExactComponentClassification {
            component_index,
            face_indices: component_faces.clone(),
            sample_point,
            inside_other,
            selected,
        });
    }

    Ok(ExactMeshPartClassification {
        components: output_components,
        selected_faces: selected_faces.into_iter().collect(),
        used_cut_path_sides: true,
        cut_paths_consistent: true,
        cut_left_components: input.cut_left_components,
        cut_right_components: input.cut_right_components,
        cut_path_overlap_components: input.cut_path_overlap_components,
    })
}

fn connected_components_excluding_cut_edges(
    faces: &[[usize; 3]],
    cut_edges: &[[usize; 2]],
) -> Vec<Vec<usize>> {
    if faces.is_empty() {
        return Vec::new();
    }
    let cut_edges = cut_edges
        .iter()
        .copied()
        .map(ordered_edge)
        .collect::<BTreeSet<_>>();
    let mut unmatched_edges = HashMap::<[usize; 2], Vec<([usize; 2], usize)>>::new();
    let mut adjacency = vec![Vec::<usize>::new(); faces.len()];
    for (face_index, face) in faces.iter().enumerate() {
        for edge in face_edges(*face) {
            let key = ordered_edge(edge);
            if cut_edges.contains(&key) {
                continue;
            }

            let candidates = unmatched_edges.entry(key).or_default();
            if let Some(match_index) = candidates
                .iter()
                .position(|(stored_edge, _)| *stored_edge == reverse_edge(edge))
            {
                let (_, adjacent_face) = candidates.swap_remove(match_index);
                adjacency[face_index].push(adjacent_face);
                adjacency[adjacent_face].push(face_index);
            } else {
                candidates.push((edge, face_index));
            }
        }
    }

    let mut seen = vec![false; faces.len()];
    let mut components = Vec::new();
    for start in 0..faces.len() {
        if seen[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut component = Vec::new();
        seen[start] = true;
        while let Some(face) = queue.pop_front() {
            component.push(face);
            for neighbor in &adjacency[face] {
                if !seen[*neighbor] {
                    seen[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
        components.push(component);
    }
    components
}

fn cut_path_edges(cut_edge_paths: &[Vec<[usize; 2]>]) -> Vec<[usize; 2]> {
    cut_edge_paths
        .iter()
        .flatten()
        .copied()
        .map(ordered_edge)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn face_to_component_map(face_count: usize, components: &[Vec<usize>]) -> Vec<Option<usize>> {
    let mut face_to_component = vec![None; face_count];
    for (component_index, component_faces) in components.iter().enumerate() {
        for face in component_faces {
            if let Some(slot) = face_to_component.get_mut(*face) {
                *slot = Some(component_index);
            }
        }
    }
    face_to_component
}

fn directed_face_map(faces: &[[usize; 3]]) -> BTreeMap<[usize; 2], Vec<usize>> {
    let mut directed_faces = BTreeMap::<[usize; 2], Vec<usize>>::new();
    for (face_index, face) in faces.iter().copied().enumerate() {
        for edge in face_edges(face) {
            directed_faces.entry(edge).or_default().push(face_index);
        }
    }
    directed_faces
}

struct CutPathSideComponents {
    left_components: BTreeSet<usize>,
    right_components: BTreeSet<usize>,
}

impl CutPathSideComponents {
    fn consistent(&self) -> bool {
        self.overlap_count() == 0
    }

    fn left_root_count(&self) -> usize {
        usize::from(!self.left_components.is_empty())
    }

    fn right_root_count(&self) -> usize {
        usize::from(!self.right_components.is_empty())
    }

    fn overlap_count(&self) -> usize {
        self.left_components
            .intersection(&self.right_components)
            .count()
    }
}

struct PathCutSideComponents {
    left_components: BTreeSet<usize>,
    right_components: BTreeSet<usize>,
}

fn cut_path_side_components(
    cut_edge_paths: &[Vec<[usize; 2]>],
    directed_faces: &BTreeMap<[usize; 2], Vec<usize>>,
    face_to_component: &[Option<usize>],
) -> CutPathSideComponents {
    cut_path_side_components_impl(cut_edge_paths, directed_faces, face_to_component, true)
}

fn cut_path_side_components_impl(
    cut_edge_paths: &[Vec<[usize; 2]>],
    directed_faces: &BTreeMap<[usize; 2], Vec<usize>>,
    face_to_component: &[Option<usize>],
    normalize_cut_path_orientation: bool,
) -> CutPathSideComponents {
    let path_sides = path_cut_side_components(cut_edge_paths, directed_faces, face_to_component);
    let fixed_sides = merge_path_sides(&path_sides);
    if fixed_sides.consistent() || !normalize_cut_path_orientation {
        return fixed_sides;
    }
    orientation_normalized_path_sides(&path_sides).unwrap_or(fixed_sides)
}

fn path_cut_side_components(
    cut_edge_paths: &[Vec<[usize; 2]>],
    directed_faces: &BTreeMap<[usize; 2], Vec<usize>>,
    face_to_component: &[Option<usize>],
) -> Vec<PathCutSideComponents> {
    cut_edge_paths
        .iter()
        .map(|path| {
            let mut left_components = BTreeSet::new();
            let mut right_components = BTreeSet::new();
            for edge in path.iter().copied() {
                collect_edge_side_components(
                    edge,
                    directed_faces,
                    face_to_component,
                    &mut left_components,
                    &mut right_components,
                );
            }
            PathCutSideComponents {
                left_components,
                right_components,
            }
        })
        .collect()
}

fn collect_edge_side_components(
    edge: [usize; 2],
    directed_faces: &BTreeMap<[usize; 2], Vec<usize>>,
    face_to_component: &[Option<usize>],
    left_components: &mut BTreeSet<usize>,
    right_components: &mut BTreeSet<usize>,
) {
    if let Some(faces) = directed_faces.get(&edge) {
        for face in faces {
            if let Some(component) = face_to_component.get(*face).copied().flatten() {
                left_components.insert(component);
            }
        }
    }
    if let Some(faces) = directed_faces.get(&reverse_edge(edge)) {
        for face in faces {
            if let Some(component) = face_to_component.get(*face).copied().flatten() {
                right_components.insert(component);
            }
        }
    }
}

fn merge_path_sides(path_sides: &[PathCutSideComponents]) -> CutPathSideComponents {
    let mut left_components = BTreeSet::new();
    let mut right_components = BTreeSet::new();
    for side in path_sides {
        left_components.extend(side.left_components.iter().copied());
        right_components.extend(side.right_components.iter().copied());
    }
    CutPathSideComponents {
        left_components,
        right_components,
    }
}

fn orientation_normalized_path_sides(
    path_sides: &[PathCutSideComponents],
) -> Option<CutPathSideComponents> {
    let mut graph = BTreeMap::<usize, Vec<(usize, bool)>>::new();
    for side in path_sides {
        add_same_side_constraints(&mut graph, &side.left_components);
        add_same_side_constraints(&mut graph, &side.right_components);
        for left in &side.left_components {
            for right in &side.right_components {
                add_component_constraint(&mut graph, *left, *right, true);
            }
        }
    }
    if graph.is_empty() {
        return None;
    }

    let mut colors = BTreeMap::<usize, bool>::new();
    for start in graph.keys().copied().collect::<Vec<_>>() {
        if colors.contains_key(&start) {
            continue;
        }
        colors.insert(start, false);
        let mut queue = VecDeque::from([start]);
        while let Some(component) = queue.pop_front() {
            let color = colors[&component];
            for (neighbor, opposite) in graph.get(&component).into_iter().flatten() {
                let neighbor_color = color ^ *opposite;
                match colors.get(neighbor).copied() {
                    Some(existing) if existing != neighbor_color => return None,
                    Some(_) => {}
                    None => {
                        colors.insert(*neighbor, neighbor_color);
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
    }

    let mut left_components = BTreeSet::new();
    let mut right_components = BTreeSet::new();
    for (component, color) in colors {
        if color {
            right_components.insert(component);
        } else {
            left_components.insert(component);
        }
    }
    Some(CutPathSideComponents {
        left_components,
        right_components,
    })
}

fn add_same_side_constraints(
    graph: &mut BTreeMap<usize, Vec<(usize, bool)>>,
    components: &BTreeSet<usize>,
) {
    let Some(first) = components.first().copied() else {
        return;
    };
    graph.entry(first).or_default();
    for component in components.iter().copied().skip(1) {
        add_component_constraint(graph, first, component, false);
    }
}

fn add_component_constraint(
    graph: &mut BTreeMap<usize, Vec<(usize, bool)>>,
    left: usize,
    right: usize,
    opposite: bool,
) {
    graph.entry(left).or_default().push((right, opposite));
    graph.entry(right).or_default().push((left, opposite));
}

fn component_sample_point(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    component_faces: &[usize],
) -> [f64; 3] {
    let mut weighted_sum = [0.0; 3];
    let mut total_area = 0.0;
    for face_index in component_faces {
        let face = faces[*face_index];
        let a = vertices[face[0]];
        let b = vertices[face[1]];
        let c = vertices[face[2]];
        let area = 0.5 * norm(cross(sub(b, a), sub(c, a)));
        let centroid = scale(add(add(a, b), c), 1.0 / 3.0);
        weighted_sum = add(weighted_sum, scale(centroid, area));
        total_area += area;
    }
    if total_area <= f64::EPSILON {
        let face = faces[component_faces[0]];
        return scale(
            add(add(vertices[face[0]], vertices[face[1]]), vertices[face[2]]),
            1.0 / 3.0,
        );
    }
    scale(weighted_sum, 1.0 / total_area)
}

fn face_edges(face: [usize; 3]) -> [[usize; 2]; 3] {
    [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
}

fn ordered_edge(edge: [usize; 2]) -> [usize; 2] {
    if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    }
}

fn reverse_edge(edge: [usize; 2]) -> [usize; 2] {
    [edge[1], edge[0]]
}

fn component_faces_for_indices(
    components: &[Vec<usize>],
    component_indices: &[usize],
) -> Vec<Vec<usize>> {
    component_indices
        .iter()
        .filter_map(|component_index| components.get(*component_index).cloned())
        .collect()
}

#[cfg(test)]
mod tests;
