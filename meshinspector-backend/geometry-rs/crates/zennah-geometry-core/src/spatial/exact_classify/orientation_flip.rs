use super::CLASSIFICATION_RAY_DIRECTION;
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Detect an operand-level orientation flip in the cut-side classification.
///
/// The cut-side classifier assigns each component to the include or exclude side
/// by the cut path's winding. That side is topological, not geometric: when a
/// coarse operand is cut across only one face, its entire body lands in a single
/// component whose winding can place it on the wrong side, inverting the whole
/// operand's keep/discard decision. We detect that with each component's
/// area-weighted centroid, but only let a component vote when its centroid sits
/// clearly OUTSIDE the contour's bounding box — there the centroid is a solid,
/// unambiguous sample of a body well clear of the intersection, whereas an
/// annular cap's centroid falls near the contour (inside the box) and is skipped.
/// A flip is reported only when every voting component agrees the side is
/// inverted, so coplanar/overlap cases (whose near-contour components do not vote,
/// and whose body components agree with the cut side) are left untouched.
pub(super) fn detect_operand_orientation_flip(
    faces: &[[usize; 3]],
    components: &[Vec<usize>],
    include_components: &BTreeSet<usize>,
    cut_edge_paths: &[Vec<[usize; 2]>],
    vertices: &[[f64; 3]],
    other_vertices: &[[f64; 3]],
    other_faces_i64: &[[i64; 3]],
    need_inside: bool,
    epsilon: f64,
) -> Result<bool, GeometryError> {
    let Some((min, max)) = cut_path_bbox(cut_edge_paths, vertices) else {
        return Ok(false);
    };
    let margin = 0.05
        * ((max[0] - min[0]).powi(2) + (max[1] - min[1]).powi(2) + (max[2] - min[2]).powi(2))
            .sqrt();
    let mut flip_votes = 0usize;
    let mut keep_votes = 0usize;
    for (component_index, component_faces) in components.iter().enumerate() {
        let centroid = super::component_sample_point(vertices, faces, component_faces);
        if !point_clear_of_bbox(centroid, min, max, margin) {
            continue;
        }
        let inside = super::super::point_inside_mesh(
            other_vertices,
            other_faces_i64,
            centroid,
            CLASSIFICATION_RAY_DIRECTION,
            epsilon,
        )?;
        if include_components.contains(&component_index) == (inside == need_inside) {
            keep_votes += 1;
        } else {
            flip_votes += 1;
        }
    }
    Ok(flip_votes > 0 && keep_votes == 0)
}

fn cut_path_bbox(
    cut_edge_paths: &[Vec<[usize; 2]>],
    vertices: &[[f64; 3]],
) -> Option<([f64; 3], [f64; 3])> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for path in cut_edge_paths {
        for edge in path {
            for vertex in edge {
                let point = vertices[*vertex];
                for axis in 0..3 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
                any = true;
            }
        }
    }
    any.then_some((min, max))
}

fn point_clear_of_bbox(point: [f64; 3], min: [f64; 3], max: [f64; 3], margin: f64) -> bool {
    (0..3).any(|axis| point[axis] < min[axis] - margin || point[axis] > max[axis] + margin)
}

pub(super) struct CutPathSideComponents {
    pub(super) left_components: BTreeSet<usize>,
    pub(super) right_components: BTreeSet<usize>,
}

impl CutPathSideComponents {
    pub(super) fn consistent(&self) -> bool {
        self.overlap_count() == 0
    }

    pub(super) fn left_root_count(&self) -> usize {
        usize::from(!self.left_components.is_empty())
    }

    pub(super) fn right_root_count(&self) -> usize {
        usize::from(!self.right_components.is_empty())
    }

    pub(super) fn overlap_count(&self) -> usize {
        self.left_components
            .intersection(&self.right_components)
            .count()
    }
}

pub(super) struct PathCutSideComponents {
    pub(super) left_components: BTreeSet<usize>,
    pub(super) right_components: BTreeSet<usize>,
}

pub(super) fn cut_path_side_components(
    cut_edge_paths: &[Vec<[usize; 2]>],
    directed_faces: &BTreeMap<[usize; 2], Vec<usize>>,
    face_to_component: &[Option<usize>],
) -> CutPathSideComponents {
    cut_path_side_components_impl(cut_edge_paths, directed_faces, face_to_component, true)
}

pub(super) fn cut_path_side_components_impl(
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

pub(super) fn path_cut_side_components(
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
    if let Some(faces) = directed_faces.get(&super::reverse_edge(edge)) {
        for face in faces {
            if let Some(component) = face_to_component.get(*face).copied().flatten() {
                right_components.insert(component);
            }
        }
    }
}

pub(super) fn merge_path_sides(path_sides: &[PathCutSideComponents]) -> CutPathSideComponents {
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

pub(super) fn orientation_normalized_path_sides(
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
