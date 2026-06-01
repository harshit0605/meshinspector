use super::exact_one_mesh::{
    exact_one_mesh_intersection_contours, ExactOneMeshContour, ExactOneMeshPrimitive,
};
use crate::math::{dot, norm, sub};
use crate::mesh::{edge_face_map, validate_faces};
use crate::GeometryError;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExactCutPrimitive {
    Vertex(usize),
    Edge([usize; 2]),
    Face(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactCutPoint {
    pub contour_index: usize,
    pub intersection_index: usize,
    pub primitive: ExactCutPrimitive,
    pub coordinate: [f64; 3],
    pub vertex_index: usize,
    pub inserted_vertex: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactEdgeSplitEvent {
    pub edge: [usize; 2],
    pub edge_parameter: f64,
    pub cut_point: usize,
    pub vertex_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactCutPathSegment {
    pub contour_index: usize,
    pub from_point: usize,
    pub to_point: usize,
    pub source_faces: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactCutPreplan {
    pub vertices_after_preplan: Vec<[f64; 3]>,
    pub cut_points: Vec<ExactCutPoint>,
    pub contour_points: Vec<Vec<usize>>,
    pub contour_closed: Vec<bool>,
    pub path_segments: Vec<ExactCutPathSegment>,
    pub edge_splits: Vec<ExactEdgeSplitEvent>,
    pub removed_face_candidates: Vec<usize>,
    pub bad_face_candidates: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactMeshPairCutPreplans {
    pub first: ExactCutPreplan,
    pub second: ExactCutPreplan,
}

pub fn exact_mesh_pair_cut_preplans(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<ExactMeshPairCutPreplans, GeometryError> {
    let contours = exact_one_mesh_intersection_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;

    Ok(ExactMeshPairCutPreplans {
        first: exact_cut_preplan(first_vertices, first_faces_i64, &contours.first, epsilon)?,
        second: exact_cut_preplan(second_vertices, second_faces_i64, &contours.second, epsilon)?,
    })
}

pub fn exact_cut_preplan(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    contours: &[ExactOneMeshContour],
    epsilon: f64,
) -> Result<ExactCutPreplan, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let topology = CutTopology::new(&faces);
    let tolerance_sq = effective_epsilon(epsilon).powi(2);
    let mut vertices_after_preplan = vertices.to_vec();
    let mut cut_points = Vec::new();
    let mut contour_points = Vec::with_capacity(contours.len());
    let contour_closed = contours
        .iter()
        .map(|contour| contour.closed)
        .collect::<Vec<_>>();
    let mut edge_splits_by_edge = BTreeMap::<[usize; 2], Vec<ExactEdgeSplitEvent>>::new();
    let mut reusable_cut_vertices = HashMap::<ExactCutPrimitive, Vec<ReusableCutVertex>>::new();

    for (contour_index, contour) in contours.iter().enumerate() {
        let mut contour_point_ids = Vec::with_capacity(contour.intersections.len());
        for (intersection_index, intersection) in contour.intersections.iter().enumerate() {
            let primitive = snap_primitive(
                &intersection.primitive,
                intersection.coordinate,
                vertices,
                &faces,
                tolerance_sq,
            );
            let inserted_vertex = !matches!(primitive, ExactCutPrimitive::Vertex(_));
            let vertex_index = match primitive {
                ExactCutPrimitive::Vertex(vertex_index) => vertex_index,
                _ => {
                    if let Some(vertex_index) = find_reusable_cut_vertex(
                        &reusable_cut_vertices,
                        primitive,
                        intersection.coordinate,
                        vertices,
                        epsilon,
                    ) {
                        vertex_index
                    } else {
                        let vertex_index = vertices_after_preplan.len();
                        vertices_after_preplan.push(intersection.coordinate);
                        reusable_cut_vertices.entry(primitive).or_default().push(
                            ReusableCutVertex {
                                coordinate: intersection.coordinate,
                                vertex_index,
                            },
                        );
                        vertex_index
                    }
                }
            };
            let cut_point_index = cut_points.len();
            if let ExactCutPrimitive::Edge(edge) = primitive {
                let edge_parameter = edge_parameter(
                    vertices[edge[0]],
                    vertices[edge[1]],
                    intersection.coordinate,
                );
                edge_splits_by_edge
                    .entry(ordered_edge(edge))
                    .or_default()
                    .push(ExactEdgeSplitEvent {
                        edge: ordered_edge(edge),
                        edge_parameter,
                        cut_point: cut_point_index,
                        vertex_index,
                    });
            }
            cut_points.push(ExactCutPoint {
                contour_index,
                intersection_index,
                primitive,
                coordinate: intersection.coordinate,
                vertex_index,
                inserted_vertex,
            });
            contour_point_ids.push(cut_point_index);
        }
        contour_points.push(contour_point_ids);
    }

    let mut path_segments = Vec::new();
    let mut removed_faces = BTreeSet::new();
    let mut face_segment_counts = HashMap::<usize, usize>::new();
    for (contour_index, contour) in contours.iter().enumerate() {
        let point_ids = &contour_points[contour_index];
        for (from_point, to_point) in contour_segment_pairs(point_ids, contour.closed) {
            if distance_sq(
                cut_points[from_point].coordinate,
                cut_points[to_point].coordinate,
            ) <= tolerance_sq
            {
                continue;
            }
            let source_faces = shared_source_faces(
                &cut_points[from_point].primitive,
                &cut_points[to_point].primitive,
                &topology,
            );
            for face in &source_faces {
                removed_faces.insert(*face);
                *face_segment_counts.entry(*face).or_insert(0) += 1;
            }
            path_segments.push(ExactCutPathSegment {
                contour_index,
                from_point,
                to_point,
                source_faces,
            });
        }
    }

    let mut edge_splits = Vec::new();
    for mut events in edge_splits_by_edge.into_values() {
        events.sort_by(|left, right| {
            left.edge_parameter
                .total_cmp(&right.edge_parameter)
                .then_with(|| left.cut_point.cmp(&right.cut_point))
        });
        edge_splits.extend(events);
    }

    let bad_face_candidates = face_segment_counts
        .into_iter()
        .filter_map(|(face, count)| (count > 1).then_some(face))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(ExactCutPreplan {
        vertices_after_preplan,
        cut_points,
        contour_points,
        contour_closed,
        path_segments,
        edge_splits,
        removed_face_candidates: removed_faces.into_iter().collect(),
        bad_face_candidates,
    })
}

#[derive(Debug)]
struct CutTopology {
    edge_faces: HashMap<(usize, usize), Vec<usize>>,
    vertex_faces: Vec<Vec<usize>>,
}

struct ReusableCutVertex {
    coordinate: [f64; 3],
    vertex_index: usize,
}

impl CutTopology {
    fn new(faces: &[[usize; 3]]) -> Self {
        let vertex_count = faces
            .iter()
            .flat_map(|face| face.iter().copied())
            .max()
            .map_or(0, |max_vertex| max_vertex + 1);
        let mut vertex_face_sets = vec![BTreeSet::new(); vertex_count];
        for (face_index, face) in faces.iter().enumerate() {
            for vertex in face {
                vertex_face_sets[*vertex].insert(face_index);
            }
        }
        Self {
            edge_faces: edge_face_map(faces),
            vertex_faces: vertex_face_sets
                .into_iter()
                .map(|faces| faces.into_iter().collect())
                .collect(),
        }
    }

    fn primitive_faces(&self, primitive: &ExactCutPrimitive) -> BTreeSet<usize> {
        match primitive {
            ExactCutPrimitive::Vertex(vertex) => self
                .vertex_faces
                .get(*vertex)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect(),
            ExactCutPrimitive::Edge(edge) => self
                .edge_faces
                .get(&edge_key(*edge))
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect(),
            ExactCutPrimitive::Face(face) => BTreeSet::from([*face]),
        }
    }
}

fn snap_primitive(
    primitive: &ExactOneMeshPrimitive,
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    tolerance_sq: f64,
) -> ExactCutPrimitive {
    if let Some(vertex) = snap_existing_vertex(coordinate, vertices, tolerance_sq) {
        return ExactCutPrimitive::Vertex(vertex);
    }
    match primitive {
        ExactOneMeshPrimitive::Edge(edge) => {
            snap_edge_primitive(*edge, coordinate, vertices, tolerance_sq)
        }
        ExactOneMeshPrimitive::Face(face) => {
            for vertex in faces[*face] {
                if distance_sq(vertices[vertex], coordinate) <= tolerance_sq {
                    return ExactCutPrimitive::Vertex(vertex);
                }
            }
            for edge in face_edges(faces[*face]) {
                if point_segment_distance_sq(coordinate, vertices[edge[0]], vertices[edge[1]])
                    <= tolerance_sq
                {
                    return snap_edge_primitive(edge, coordinate, vertices, tolerance_sq);
                }
            }
            ExactCutPrimitive::Face(*face)
        }
    }
}

fn snap_existing_vertex(
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
    tolerance_sq: f64,
) -> Option<usize> {
    let tolerance = tolerance_sq.sqrt() * 2.0;
    vertices
        .iter()
        .position(|vertex| distance_sq(*vertex, coordinate) <= tolerance.powi(2))
}

fn snap_edge_primitive(
    edge: [usize; 2],
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
    tolerance_sq: f64,
) -> ExactCutPrimitive {
    let edge = ordered_edge(edge);
    let start = vertices[edge[0]];
    let end = vertices[edge[1]];
    if distance_sq(start, coordinate) <= tolerance_sq {
        return ExactCutPrimitive::Vertex(edge[0]);
    }
    if distance_sq(end, coordinate) <= tolerance_sq {
        return ExactCutPrimitive::Vertex(edge[1]);
    }
    let edge_vector = sub(end, start);
    let edge_length = norm(edge_vector);
    let endpoint_tolerance = tolerance_sq.sqrt() * 2.0;
    if edge_length > f64::EPSILON
        && point_segment_distance_sq(coordinate, start, end) <= endpoint_tolerance.powi(2)
    {
        let parameter = dot(sub(coordinate, start), edge_vector) / (edge_length * edge_length);
        let parameter_tolerance = endpoint_tolerance / edge_length;
        if parameter <= parameter_tolerance {
            return ExactCutPrimitive::Vertex(edge[0]);
        }
        if parameter >= 1.0 - parameter_tolerance {
            return ExactCutPrimitive::Vertex(edge[1]);
        }
    }
    ExactCutPrimitive::Edge(ordered_edge(edge))
}

fn contour_segment_pairs(point_ids: &[usize], closed: bool) -> Vec<(usize, usize)> {
    if point_ids.len() < 2 {
        return Vec::new();
    }
    let mut pairs = point_ids
        .windows(2)
        .map(|window| (window[0], window[1]))
        .collect::<Vec<_>>();
    if closed && point_ids.len() > 2 {
        pairs.push((*point_ids.last().unwrap(), point_ids[0]));
    }
    pairs
}

fn shared_source_faces(
    from: &ExactCutPrimitive,
    to: &ExactCutPrimitive,
    topology: &CutTopology,
) -> Vec<usize> {
    let from_faces = topology.primitive_faces(from);
    let to_faces = topology.primitive_faces(to);
    let shared = from_faces
        .intersection(&to_faces)
        .copied()
        .collect::<Vec<_>>();
    if !shared.is_empty() {
        return shared;
    }
    from_faces.union(&to_faces).copied().collect()
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

fn edge_key(edge: [usize; 2]) -> (usize, usize) {
    let edge = ordered_edge(edge);
    (edge[0], edge[1])
}

fn find_reusable_cut_vertex(
    reusable_vertices: &HashMap<ExactCutPrimitive, Vec<ReusableCutVertex>>,
    primitive: ExactCutPrimitive,
    coordinate: [f64; 3],
    vertices: &[[f64; 3]],
    epsilon: f64,
) -> Option<usize> {
    let tolerance = effective_epsilon(epsilon) * 2.0;
    reusable_vertices.get(&primitive).and_then(|candidates| {
        candidates
            .iter()
            .find(|candidate| {
                reusable_cut_vertices_match(
                    primitive,
                    coordinate,
                    candidate.coordinate,
                    vertices,
                    tolerance,
                )
            })
            .map(|candidate| candidate.vertex_index)
    })
}

fn reusable_cut_vertices_match(
    primitive: ExactCutPrimitive,
    left: [f64; 3],
    right: [f64; 3],
    vertices: &[[f64; 3]],
    tolerance: f64,
) -> bool {
    match primitive {
        ExactCutPrimitive::Edge(edge) => {
            let start = vertices[edge[0]];
            let end = vertices[edge[1]];
            let edge_length = norm(sub(end, start)).max(f64::EPSILON);
            let left_parameter = edge_parameter(start, end, left);
            let right_parameter = edge_parameter(start, end, right);
            (left_parameter - right_parameter).abs() * edge_length <= tolerance
        }
        ExactCutPrimitive::Face(_) => norm(sub(left, right)) <= tolerance,
        ExactCutPrimitive::Vertex(_) => false,
    }
}

fn edge_parameter(start: [f64; 3], end: [f64; 3], point: [f64; 3]) -> f64 {
    let edge = sub(end, start);
    let length_sq = dot(edge, edge);
    if length_sq <= f64::EPSILON {
        return 0.0;
    }
    (dot(sub(point, start), edge) / length_sq).clamp(0.0, 1.0)
}

fn point_segment_distance_sq(point: [f64; 3], start: [f64; 3], end: [f64; 3]) -> f64 {
    let t = edge_parameter(start, end, point);
    let closest = [
        start[0] + t * (end[0] - start[0]),
        start[1] + t * (end[1] - start[1]),
        start[2] + t * (end[2] - start[2]),
    ];
    distance_sq(point, closest)
}

fn distance_sq(first: [f64; 3], second: [f64; 3]) -> f64 {
    let delta = sub(first, second);
    dot(delta, delta)
}

fn effective_epsilon(epsilon: f64) -> f64 {
    if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_cut_preplan_sorts_edge_splits_like_meshlib_precut() {
        let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let contours = vec![ExactOneMeshContour {
            intersections: vec![
                super::super::exact_one_mesh::ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [1.5, 0.0, 0.0],
                },
                super::super::exact_one_mesh::ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [0.5, 0.0, 0.0],
                },
            ],
            closed: false,
        }];

        let plan = exact_cut_preplan(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(plan.vertices_after_preplan.len(), vertices.len() + 2);
        assert_eq!(plan.edge_splits.len(), 2);
        assert!(plan.edge_splits[0].edge_parameter < plan.edge_splits[1].edge_parameter);
        assert_eq!(plan.path_segments.len(), 1);
        assert_eq!(plan.removed_face_candidates, vec![0]);
    }

    #[test]
    fn exact_cut_preplan_snaps_edge_endpoint_to_existing_vertex() {
        let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let contours = vec![ExactOneMeshContour {
            intersections: vec![super::super::exact_one_mesh::ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [0.0, 0.0, 0.0],
            }],
            closed: false,
        }];

        let plan = exact_cut_preplan(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(plan.vertices_after_preplan.len(), vertices.len());
        assert_eq!(plan.edge_splits.len(), 0);
        assert_eq!(plan.cut_points[0].primitive, ExactCutPrimitive::Vertex(0));
        assert!(!plan.cut_points[0].inserted_vertex);
    }

    #[test]
    fn exact_cut_preplan_snaps_near_edge_endpoint_by_parameter() {
        let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let contours = vec![ExactOneMeshContour {
            intersections: vec![super::super::exact_one_mesh::ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [2.0 + 5e-10, 0.0, 0.0],
            }],
            closed: false,
        }];

        let plan = exact_cut_preplan(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(plan.vertices_after_preplan.len(), vertices.len());
        assert_eq!(plan.cut_points[0].primitive, ExactCutPrimitive::Vertex(1));
    }

    #[test]
    fn exact_cut_preplan_prefers_any_existing_vertex_before_inserting_cut_vertex() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [3.0, 3.0, 0.0],
        ];
        let faces = vec![[0, 1, 2]];
        let contours = vec![ExactOneMeshContour {
            intersections: vec![super::super::exact_one_mesh::ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [5e-10, 0.0, 0.0],
            }],
            closed: false,
        }];

        let plan = exact_cut_preplan(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(plan.vertices_after_preplan.len(), vertices.len());
        assert_eq!(plan.cut_points[0].primitive, ExactCutPrimitive::Vertex(0));
        assert!(!plan.cut_points[0].inserted_vertex);
    }

    #[test]
    fn exact_cut_preplan_reuses_duplicate_edge_cut_vertices() {
        let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let intersection = super::super::exact_one_mesh::ExactOneMeshIntersection {
            primitive: ExactOneMeshPrimitive::Edge([0, 1]),
            coordinate: [1.0, 0.0, 0.0],
        };
        let contours = vec![
            ExactOneMeshContour {
                intersections: vec![
                    intersection.clone(),
                    super::super::exact_one_mesh::ExactOneMeshIntersection {
                        primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                        coordinate: [1.0, 1.0, 0.0],
                    },
                ],
                closed: false,
            },
            ExactOneMeshContour {
                intersections: vec![
                    super::super::exact_one_mesh::ExactOneMeshIntersection {
                        primitive: ExactOneMeshPrimitive::Edge([1, 0]),
                        coordinate: intersection.coordinate,
                    },
                    super::super::exact_one_mesh::ExactOneMeshIntersection {
                        primitive: ExactOneMeshPrimitive::Edge([2, 0]),
                        coordinate: [0.0, 1.0, 0.0],
                    },
                ],
                closed: false,
            },
        ];

        let plan = exact_cut_preplan(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(
            plan.cut_points[0].vertex_index,
            plan.cut_points[2].vertex_index
        );
    }

    #[test]
    fn closed_two_point_contour_keeps_single_precut_segment() {
        assert_eq!(contour_segment_pairs(&[2, 5], true), vec![(2, 5)]);
        assert_eq!(
            contour_segment_pairs(&[2, 5, 8], true),
            vec![(2, 5), (5, 8), (8, 2)]
        );
    }

    #[test]
    fn exact_cut_preplan_drops_degenerate_lone_segment() {
        let vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let contours = vec![ExactOneMeshContour {
            intersections: vec![
                super::super::exact_one_mesh::ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [1.0, 0.0, 0.0],
                },
                super::super::exact_one_mesh::ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [1.0, 0.0, 0.0],
                },
            ],
            closed: true,
        }];

        let plan = exact_cut_preplan(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(plan.path_segments.len(), 0);
        assert!(plan.removed_face_candidates.is_empty());
        assert!(plan.bad_face_candidates.is_empty());
    }

    #[test]
    fn exact_mesh_pair_cut_preplans_derive_precut_metadata() {
        let first_vertices = vec![[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]];
        let first_faces = vec![[0, 1, 2]];
        let second_vertices = vec![[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [3.0, 0.0, 0.0]];
        let second_faces = vec![[0, 1, 2]];

        let plans = exact_mesh_pair_cut_preplans(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert!(!plans.first.cut_points.is_empty());
        assert!(!plans.second.cut_points.is_empty());
        assert!(plans.first.vertices_after_preplan.len() >= first_vertices.len());
        assert!(plans.second.vertices_after_preplan.len() >= second_vertices.len());
    }
}
