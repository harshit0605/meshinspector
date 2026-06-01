use super::exact_coplanar::coplanar_overlap_contours;
use super::exact_one_mesh::{
    exact_lone_subdivision_mesh, exact_one_mesh_intersection_contours, ExactLoneSubdivisionEntry,
    ExactOneMeshContour, ExactOneMeshContours, ExactOneMeshPrimitive,
};
use crate::math::{cross, dot, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::BTreeSet;

const MAX_LONE_SUBDIVISION_ITERATIONS: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct ExactLoneSubdivisionPairPrepass {
    pub first_vertices: Vec<[f64; 3]>,
    pub first_faces: Vec<[i64; 3]>,
    pub first_source_face_for_faces: Vec<usize>,
    pub second_vertices: Vec<[f64; 3]>,
    pub second_faces: Vec<[i64; 3]>,
    pub second_source_face_for_faces: Vec<usize>,
    pub contours: ExactOneMeshContours,
    pub first_subdivisions: Vec<ExactLoneSubdivisionEntry>,
    pub second_subdivisions: Vec<ExactLoneSubdivisionEntry>,
    pub iterations: usize,
    pub hit_iteration_limit: bool,
    pub removed_lone_contours: usize,
}

pub fn exact_lone_subdivision_pair_prepass(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<ExactLoneSubdivisionPairPrepass, GeometryError> {
    validate_faces(first_faces_i64, first_vertices.len())?;
    validate_faces(second_faces_i64, second_vertices.len())?;

    let mut first_vertices = first_vertices.to_vec();
    let mut first_faces = first_faces_i64.to_vec();
    let mut first_source_face_for_faces = identity_source_map(first_faces.len());
    let mut second_vertices = second_vertices.to_vec();
    let mut second_faces = second_faces_i64.to_vec();
    let mut second_source_face_for_faces = identity_source_map(second_faces.len());
    let mut first_subdivisions = Vec::new();
    let mut second_subdivisions = Vec::new();
    let mut previous_lone_contours = Vec::new();

    for iteration in 0..MAX_LONE_SUBDIVISION_ITERATIONS {
        let mut contours = exact_one_mesh_intersection_contours(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            leaf_size,
            epsilon,
        )?;
        let lone_contours = lone_contour_indices(&contours);
        let first_lone = face_lone_contours(&contours.first, &contours.second, epsilon);
        let second_lone = face_lone_contours(&contours.second, &contours.first, epsilon);
        if lone_contours.is_empty() {
            return Ok(ExactLoneSubdivisionPairPrepass {
                first_vertices,
                first_faces,
                first_source_face_for_faces,
                second_vertices,
                second_faces,
                second_source_face_for_faces,
                contours,
                first_subdivisions,
                second_subdivisions,
                iterations: iteration,
                hit_iteration_limit: false,
                removed_lone_contours: 0,
            });
        }
        if lone_contours == previous_lone_contours
            || iteration + 1 == MAX_LONE_SUBDIVISION_ITERATIONS
        {
            let removed_lone_contours = remove_lone_contours(&mut contours, &lone_contours);
            return Ok(ExactLoneSubdivisionPairPrepass {
                first_vertices,
                first_faces,
                first_source_face_for_faces,
                second_vertices,
                second_faces,
                second_source_face_for_faces,
                contours,
                first_subdivisions,
                second_subdivisions,
                iterations: iteration + 1,
                hit_iteration_limit: iteration + 1 == MAX_LONE_SUBDIVISION_ITERATIONS,
                removed_lone_contours,
            });
        }
        previous_lone_contours = lone_contours;

        if !first_lone.is_empty() {
            let subdivided =
                exact_lone_subdivision_mesh(&first_vertices, &first_faces, &first_lone, epsilon)?;
            first_subdivisions.extend(remap_entries(
                subdivided.entries,
                &first_source_face_for_faces,
            ));
            first_source_face_for_faces = remap_sources(
                &subdivided.source_face_for_faces,
                &first_source_face_for_faces,
            );
            first_vertices = subdivided.vertices;
            first_faces = subdivided.faces;
        }
        if !second_lone.is_empty() {
            let subdivided = exact_lone_subdivision_mesh(
                &second_vertices,
                &second_faces,
                &second_lone,
                epsilon,
            )?;
            second_subdivisions.extend(remap_entries(
                subdivided.entries,
                &second_source_face_for_faces,
            ));
            second_source_face_for_faces = remap_sources(
                &subdivided.source_face_for_faces,
                &second_source_face_for_faces,
            );
            second_vertices = subdivided.vertices;
            second_faces = subdivided.faces;
        }
    }

    let contours = exact_one_mesh_intersection_contours(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        leaf_size,
        epsilon,
    )?;
    Ok(ExactLoneSubdivisionPairPrepass {
        first_vertices,
        first_faces,
        first_source_face_for_faces,
        second_vertices,
        second_faces,
        second_source_face_for_faces,
        contours,
        first_subdivisions,
        second_subdivisions,
        iterations: MAX_LONE_SUBDIVISION_ITERATIONS,
        hit_iteration_limit: true,
        removed_lone_contours: 0,
    })
}

pub(super) fn exact_pair_intersection_contours_with_coplanar(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<ExactOneMeshContours, GeometryError> {
    let mut contours = exact_one_mesh_intersection_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    let coplanar = coplanar_overlap_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        epsilon,
    )?;
    append_contours(&mut contours, coplanar.merged_contours);
    Ok(contours)
}

fn append_contours(target: &mut ExactOneMeshContours, mut source: ExactOneMeshContours) {
    target.first.append(&mut source.first);
    target.second.append(&mut source.second);
    target
        .coordinates_in_first_space
        .append(&mut source.coordinates_in_first_space);
}

fn lone_contour_indices(contours: &ExactOneMeshContours) -> Vec<usize> {
    contours
        .first
        .iter()
        .zip(&contours.second)
        .enumerate()
        .filter_map(|(index, (first, second))| {
            ((is_face_lone(first) && is_edge_lone(second))
                || (is_edge_lone(first) && is_face_lone(second)))
            .then_some(index)
        })
        .collect()
}

fn remove_lone_contours(contours: &mut ExactOneMeshContours, lone_indices: &[usize]) -> usize {
    let lone_indices = lone_indices.iter().copied().collect::<BTreeSet<_>>();
    if lone_indices.is_empty() {
        return 0;
    }
    let mut removed = 0;
    contours.first = contours
        .first
        .iter()
        .cloned()
        .enumerate()
        .filter_map(|(index, contour)| {
            if lone_indices.contains(&index) {
                removed += 1;
                None
            } else {
                Some(contour)
            }
        })
        .collect();
    contours.second = contours
        .second
        .iter()
        .cloned()
        .enumerate()
        .filter_map(|(index, contour)| (!lone_indices.contains(&index)).then_some(contour))
        .collect();
    contours.coordinates_in_first_space = contours
        .coordinates_in_first_space
        .iter()
        .cloned()
        .enumerate()
        .filter_map(|(index, contour)| (!lone_indices.contains(&index)).then_some(contour))
        .collect();
    removed
}

fn face_lone_contours(
    target: &[ExactOneMeshContour],
    paired: &[ExactOneMeshContour],
    epsilon: f64,
) -> Vec<ExactOneMeshContour> {
    target
        .iter()
        .zip(paired)
        .filter_map(|(target_contour, paired_contour)| {
            if !is_face_lone(target_contour)
                || !is_edge_lone(paired_contour)
                || is_degenerate_closed_lone_face_contour(target_contour, epsilon)
            {
                None
            } else {
                Some(target_contour.clone())
            }
        })
        .collect()
}

fn is_face_lone(contour: &ExactOneMeshContour) -> bool {
    !contour.intersections.is_empty()
        && contour
            .intersections
            .iter()
            .all(|point| matches!(point.primitive, ExactOneMeshPrimitive::Face(_)))
}

fn is_edge_lone(contour: &ExactOneMeshContour) -> bool {
    !contour.intersections.is_empty()
        && contour
            .intersections
            .iter()
            .all(|point| matches!(point.primitive, ExactOneMeshPrimitive::Edge(_)))
}

fn is_degenerate_closed_lone_face_contour(contour: &ExactOneMeshContour, epsilon: f64) -> bool {
    if !contour.closed {
        return false;
    }
    if contour.intersections.len() < 3 {
        return true;
    }
    face_contour_area_sq(contour) <= epsilon * epsilon * epsilon * epsilon
}

fn face_contour_area_sq(contour: &ExactOneMeshContour) -> f64 {
    let origin = contour.intersections[0].coordinate;
    let mut area = [0.0; 3];
    for index in 1..contour.intersections.len() - 1 {
        let left = sub(contour.intersections[index].coordinate, origin);
        let right = sub(contour.intersections[index + 1].coordinate, origin);
        let tri_area = cross(left, right);
        area[0] += tri_area[0];
        area[1] += tri_area[1];
        area[2] += tri_area[2];
    }
    dot(area, area) * 0.25
}

fn identity_source_map(face_count: usize) -> Vec<usize> {
    (0..face_count).collect()
}

fn remap_sources(source_faces: &[usize], previous_source_map: &[usize]) -> Vec<usize> {
    source_faces
        .iter()
        .map(|source| previous_source_map.get(*source).copied().unwrap_or(*source))
        .collect()
}

fn remap_entries(
    entries: Vec<ExactLoneSubdivisionEntry>,
    source_map: &[usize],
) -> Vec<ExactLoneSubdivisionEntry> {
    entries
        .into_iter()
        .map(|mut entry| {
            if let Some(source_face) = source_map.get(entry.source_face) {
                entry.source_face = *source_face;
            }
            entry
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::exact_one_mesh::ExactOneMeshIntersection;
    use super::*;

    #[test]
    fn face_lone_contours_select_closed_face_edge_pair() {
        let target = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [0.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [1.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [0.0, 1.0, 0.0],
                },
            ],
            closed: true,
        }];
        let paired = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [0.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                    coordinate: [1.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([2, 0]),
                    coordinate: [0.0, 1.0, 0.0],
                },
            ],
            closed: true,
        }];

        let selected = face_lone_contours(&target, &paired, 1e-9);

        assert_eq!(selected.len(), 1);
    }

    #[test]
    fn face_lone_contours_select_open_like_meshlib_default_detection() {
        let open_target = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [0.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [1.0, 0.0, 0.0],
                },
            ],
            closed: false,
        }];
        let open_paired = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [0.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                    coordinate: [1.0, 0.0, 0.0],
                },
            ],
            closed: false,
        }];

        assert_eq!(
            face_lone_contours(&open_target, &open_paired, 1e-9).len(),
            1
        );
    }

    #[test]
    fn face_lone_contours_ignore_closed_degenerate_lone_contour() {
        let degenerate_target = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [1.0, 1.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [1.0, 1.0, 0.0],
                },
            ],
            closed: true,
        }];
        let degenerate_paired = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [1.0, 1.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                    coordinate: [1.0, 1.0, 0.0],
                },
            ],
            closed: true,
        }];

        assert!(face_lone_contours(&degenerate_target, &degenerate_paired, 1e-9).is_empty());
    }

    #[test]
    fn repeated_lone_contour_removal_keeps_non_lone_contours_aligned() {
        let mut contours = ExactOneMeshContours {
            first: vec![
                ExactOneMeshContour {
                    intersections: vec![ExactOneMeshIntersection {
                        primitive: ExactOneMeshPrimitive::Face(0),
                        coordinate: [0.0, 0.0, 0.0],
                    }],
                    closed: false,
                },
                ExactOneMeshContour {
                    intersections: vec![ExactOneMeshIntersection {
                        primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                        coordinate: [1.0, 0.0, 0.0],
                    }],
                    closed: false,
                },
            ],
            second: vec![
                ExactOneMeshContour {
                    intersections: vec![ExactOneMeshIntersection {
                        primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                        coordinate: [0.0, 0.0, 0.0],
                    }],
                    closed: false,
                },
                ExactOneMeshContour {
                    intersections: vec![ExactOneMeshIntersection {
                        primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                        coordinate: [1.0, 0.0, 0.0],
                    }],
                    closed: false,
                },
            ],
            coordinates_in_first_space: vec![vec![[0.0, 0.0, 0.0]], vec![[1.0, 0.0, 0.0]]],
        };

        let lone_indices = lone_contour_indices(&contours);
        let removed = remove_lone_contours(&mut contours, &lone_indices);

        assert_eq!(removed, 1);
        assert_eq!(contours.first.len(), 1);
        assert_eq!(contours.second.len(), 1);
        assert_eq!(contours.coordinates_in_first_space.len(), 1);
        assert!(matches!(
            contours.first[0].intersections[0].primitive,
            ExactOneMeshPrimitive::Edge([0, 1])
        ));
    }
}
