use super::exact_contours::{exact_intersection_contours, ExactContourIntersection};
use super::exact_kernel::{
    triangle_segment_intersection_point, ExactCoordinateConverter, ExactTriangleOwner,
};
use crate::math::{cross, dot, norm, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactOneMeshPrimitive {
    Face(usize),
    Edge([usize; 2]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactOneMeshIntersection {
    pub primitive: ExactOneMeshPrimitive,
    pub coordinate: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactOneMeshContour {
    pub intersections: Vec<ExactOneMeshIntersection>,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactOneMeshContours {
    pub first: Vec<ExactOneMeshContour>,
    pub second: Vec<ExactOneMeshContour>,
    pub coordinates_in_first_space: Vec<Vec<[f64; 3]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactLoneSubdivisionEntry {
    pub source_face: usize,
    pub contour_index: usize,
    pub inserted_vertex: usize,
    pub coordinate: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactLoneSubdivisionResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub source_face_for_faces: Vec<usize>,
    pub entries: Vec<ExactLoneSubdivisionEntry>,
}

pub fn exact_one_mesh_intersection_contours(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<ExactOneMeshContours, GeometryError> {
    let first_faces = validate_faces(first_faces_i64, first_vertices.len())?;
    let second_faces = validate_faces(second_faces_i64, second_vertices.len())?;
    let contours = exact_intersection_contours(
        first_vertices,
        first_faces_i64,
        second_vertices,
        second_faces_i64,
        leaf_size,
        epsilon,
    )?;
    if contours.is_empty() {
        return Ok(empty_output());
    }

    let Some(converter) = converter_for_mesh_pair(first_vertices, second_vertices) else {
        return Ok(empty_output());
    };

    let mut first_output = Vec::with_capacity(contours.len());
    let mut second_output = Vec::with_capacity(contours.len());
    let mut coordinate_output = Vec::with_capacity(contours.len());
    for contour in contours {
        let mut first_contour = ExactOneMeshContour {
            intersections: Vec::with_capacity(contour.intersections.len()),
            closed: contour.closed,
        };
        let mut second_contour = ExactOneMeshContour {
            intersections: Vec::with_capacity(contour.intersections.len()),
            closed: contour.closed,
        };
        let mut coordinates = Vec::with_capacity(contour.intersections.len());
        for record in contour.intersections {
            let coordinate = intersection_coordinate(
                &record,
                first_vertices,
                &first_faces,
                second_vertices,
                &second_faces,
                &converter,
            );
            coordinates.push(coordinate);
            first_contour
                .intersections
                .push(first_mesh_intersection(&record, coordinate));
            second_contour
                .intersections
                .push(second_mesh_intersection(&record, coordinate));
        }
        first_output.push(first_contour);
        second_output.push(second_contour);
        coordinate_output.push(coordinates);
    }

    merge_singleton_face_lone_contours(
        &mut first_output,
        &mut second_output,
        &mut coordinate_output,
        first_vertices,
        &first_faces,
    );
    merge_singleton_face_lone_contours(
        &mut second_output,
        &mut first_output,
        &mut coordinate_output,
        second_vertices,
        &second_faces,
    );

    Ok(ExactOneMeshContours {
        first: first_output,
        second: second_output,
        coordinates_in_first_space: coordinate_output,
    })
}

pub fn exact_lone_subdivision_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    contours: &[ExactOneMeshContour],
    epsilon: f64,
) -> Result<ExactLoneSubdivisionResult, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let face_contours = first_face_lone_contours(contours, faces.len());
    let mut output_vertices = vertices.to_vec();
    let mut output_faces = Vec::with_capacity(faces.len() + face_contours.len() * 2);
    let mut source_face_for_faces = Vec::with_capacity(output_faces.capacity());
    let mut entries = Vec::with_capacity(face_contours.len());

    for (face_index, face) in faces.iter().copied().enumerate() {
        let Some(contour_index) = face_contours.get(&face_index).copied() else {
            push_face(
                &mut output_faces,
                &mut source_face_for_faces,
                face,
                face_index,
            );
            continue;
        };
        let coordinate = contour_centroid(&contours[contour_index]);
        let inserted_vertex = output_vertices.len();
        output_vertices.push(coordinate);
        entries.push(ExactLoneSubdivisionEntry {
            source_face: face_index,
            contour_index,
            inserted_vertex,
            coordinate,
        });
        for split_face in [
            [face[0], face[1], inserted_vertex],
            [face[1], face[2], inserted_vertex],
            [face[2], face[0], inserted_vertex],
        ] {
            if triangle_area(split_face, &output_vertices) > epsilon * epsilon {
                push_face(
                    &mut output_faces,
                    &mut source_face_for_faces,
                    split_face,
                    face_index,
                );
            }
        }
    }

    Ok(ExactLoneSubdivisionResult {
        vertices: output_vertices,
        faces: output_faces,
        source_face_for_faces,
        entries,
    })
}

fn first_face_lone_contours(
    contours: &[ExactOneMeshContour],
    face_count: usize,
) -> BTreeMap<usize, usize> {
    let mut face_contours = BTreeMap::new();
    for (contour_index, contour) in contours.iter().enumerate() {
        let Some(face) = contour_face_primitive(contour) else {
            continue;
        };
        if face >= face_count {
            continue;
        }
        face_contours.entry(face).or_insert(contour_index);
    }
    face_contours
}

fn contour_face_primitive(contour: &ExactOneMeshContour) -> Option<usize> {
    let first = contour.intersections.first()?;
    let ExactOneMeshPrimitive::Face(face) = first.primitive.clone() else {
        return None;
    };
    contour
        .intersections
        .iter()
        .all(|point| point.primitive == ExactOneMeshPrimitive::Face(face))
        .then_some(face)
}

fn contour_centroid(contour: &ExactOneMeshContour) -> [f64; 3] {
    let mut centroid = [0.0; 3];
    for intersection in &contour.intersections {
        centroid[0] += intersection.coordinate[0];
        centroid[1] += intersection.coordinate[1];
        centroid[2] += intersection.coordinate[2];
    }
    let scale = 1.0 / contour.intersections.len() as f64;
    [
        centroid[0] * scale,
        centroid[1] * scale,
        centroid[2] * scale,
    ]
}

fn push_face(
    output_faces: &mut Vec<[i64; 3]>,
    source_face_for_faces: &mut Vec<usize>,
    face: [usize; 3],
    source_face: usize,
) {
    output_faces.push([face[0] as i64, face[1] as i64, face[2] as i64]);
    source_face_for_faces.push(source_face);
}

fn triangle_area(face: [usize; 3], vertices: &[[f64; 3]]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    0.5 * norm(cross(sub(b, a), sub(c, a)))
}

fn empty_output() -> ExactOneMeshContours {
    ExactOneMeshContours {
        first: Vec::new(),
        second: Vec::new(),
        coordinates_in_first_space: Vec::new(),
    }
}

fn intersection_coordinate(
    record: &ExactContourIntersection,
    first_vertices: &[[f64; 3]],
    first_faces: &[[usize; 3]],
    second_vertices: &[[f64; 3]],
    second_faces: &[[usize; 3]],
    converter: &ExactCoordinateConverter,
) -> [f64; 3] {
    let (triangle, segment) = match record.edge_owner {
        ExactTriangleOwner::First => (
            triangle_points(second_vertices, second_faces[record.triangle_face]),
            edge_points(first_vertices, record.edge),
        ),
        ExactTriangleOwner::Second => (
            triangle_points(first_vertices, first_faces[record.triangle_face]),
            edge_points(second_vertices, record.edge),
        ),
    };
    triangle_segment_intersection_point(triangle, segment, converter)
        .unwrap_or_else(|| midpoint(segment[0], segment[1]))
}

fn first_mesh_intersection(
    record: &ExactContourIntersection,
    coordinate: [f64; 3],
) -> ExactOneMeshIntersection {
    let primitive = match record.edge_owner {
        ExactTriangleOwner::First => ExactOneMeshPrimitive::Edge(record.edge),
        ExactTriangleOwner::Second => ExactOneMeshPrimitive::Face(record.triangle_face),
    };
    ExactOneMeshIntersection {
        primitive,
        coordinate,
    }
}

fn second_mesh_intersection(
    record: &ExactContourIntersection,
    coordinate: [f64; 3],
) -> ExactOneMeshIntersection {
    let primitive = match record.edge_owner {
        ExactTriangleOwner::First => ExactOneMeshPrimitive::Face(record.triangle_face),
        ExactTriangleOwner::Second => ExactOneMeshPrimitive::Edge([record.edge[1], record.edge[0]]),
    };
    ExactOneMeshIntersection {
        primitive,
        coordinate,
    }
}

fn converter_for_mesh_pair(
    first_vertices: &[[f64; 3]],
    second_vertices: &[[f64; 3]],
) -> Option<ExactCoordinateConverter> {
    let mut points = Vec::with_capacity(first_vertices.len() + second_vertices.len());
    points.extend_from_slice(first_vertices);
    points.extend_from_slice(second_vertices);
    ExactCoordinateConverter::from_points(&points)
}

fn triangle_points(vertices: &[[f64; 3]], face: [usize; 3]) -> [[f64; 3]; 3] {
    [vertices[face[0]], vertices[face[1]], vertices[face[2]]]
}

fn edge_points(vertices: &[[f64; 3]], edge: [usize; 2]) -> [[f64; 3]; 2] {
    [vertices[edge[0]], vertices[edge[1]]]
}

fn midpoint(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        0.5 * (first[0] + second[0]),
        0.5 * (first[1] + second[1]),
        0.5 * (first[2] + second[2]),
    ]
}

fn merge_singleton_face_lone_contours(
    primary: &mut Vec<ExactOneMeshContour>,
    secondary: &mut Vec<ExactOneMeshContour>,
    coordinates: &mut Vec<Vec<[f64; 3]>>,
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
) {
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for (index, contour) in primary.iter().enumerate() {
        if contour.intersections.len() != 1 || secondary[index].intersections.len() != 1 {
            continue;
        }
        if let ExactOneMeshPrimitive::Face(face) = contour.intersections[0].primitive {
            groups.entry(face).or_default().push(index);
        }
    }
    if groups.values().all(|indices| indices.len() < 3) {
        return;
    }

    let mut consumed = vec![false; primary.len()];
    let mut merged_primary = Vec::with_capacity(primary.len());
    let mut merged_secondary = Vec::with_capacity(secondary.len());
    let mut merged_coordinates = Vec::with_capacity(coordinates.len());

    for (face, mut indices) in groups {
        if indices.len() < 3 || face >= faces.len() {
            continue;
        }
        sort_lone_indices_around_face(&mut indices, primary, vertices, faces[face]);
        for index in &indices {
            consumed[*index] = true;
        }
        merged_primary.push(ExactOneMeshContour {
            intersections: indices
                .iter()
                .map(|index| primary[*index].intersections[0].clone())
                .collect(),
            closed: true,
        });
        merged_secondary.push(ExactOneMeshContour {
            intersections: indices
                .iter()
                .map(|index| secondary[*index].intersections[0].clone())
                .collect(),
            closed: true,
        });
        merged_coordinates.push(indices.iter().map(|index| coordinates[*index][0]).collect());
    }

    for index in 0..primary.len() {
        if consumed[index] {
            continue;
        }
        merged_primary.push(primary[index].clone());
        merged_secondary.push(secondary[index].clone());
        merged_coordinates.push(coordinates[index].clone());
    }

    *primary = merged_primary;
    *secondary = merged_secondary;
    *coordinates = merged_coordinates;
}

fn sort_lone_indices_around_face(
    indices: &mut [usize],
    contours: &[ExactOneMeshContour],
    vertices: &[[f64; 3]],
    face: [usize; 3],
) {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let normal = cross(sub(b, a), sub(c, a));
    let normal_len = norm(normal);
    let u = sub(b, a);
    let u_len = norm(u);
    if normal_len <= f64::EPSILON || u_len <= f64::EPSILON {
        return;
    }
    let normal_unit = [
        normal[0] / normal_len,
        normal[1] / normal_len,
        normal[2] / normal_len,
    ];
    let u_unit = [u[0] / u_len, u[1] / u_len, u[2] / u_len];
    let v_unit = cross(normal_unit, u_unit);
    let centroid = lone_centroid(indices, contours);

    indices.sort_by(|left, right| {
        let left_angle = face_angle(
            sub(contours[*left].intersections[0].coordinate, centroid),
            u_unit,
            v_unit,
        );
        let right_angle = face_angle(
            sub(contours[*right].intersections[0].coordinate, centroid),
            u_unit,
            v_unit,
        );
        left_angle
            .total_cmp(&right_angle)
            .then_with(|| left.cmp(right))
    });
}

fn lone_centroid(indices: &[usize], contours: &[ExactOneMeshContour]) -> [f64; 3] {
    let mut centroid = [0.0; 3];
    for index in indices {
        let point = contours[*index].intersections[0].coordinate;
        centroid[0] += point[0];
        centroid[1] += point[1];
        centroid[2] += point[2];
    }
    let scale = 1.0 / indices.len() as f64;
    [
        centroid[0] * scale,
        centroid[1] * scale,
        centroid[2] * scale,
    ]
}

fn face_angle(vector: [f64; 3], u: [f64; 3], v: [f64; 3]) -> f64 {
    dot(vector, v).atan2(dot(vector, u))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_one_mesh_contours_convert_edge_triangle_record_to_coordinates() {
        let first_vertices = vec![[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]];
        let first_faces = vec![[0, 1, 2]];
        let second_vertices = vec![[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [3.0, 0.0, 0.0]];
        let second_faces = vec![[0, 1, 2]];

        let contours = exact_one_mesh_intersection_contours(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert_eq!(contours.first.len(), contours.second.len());
        assert_eq!(
            contours.coordinates_in_first_space.len(),
            contours.first.len()
        );
        assert!(contours.coordinates_in_first_space[0]
            .iter()
            .any(|coordinate| coordinate[0].abs() < 1e-8
                && coordinate[1].abs() < 1e-8
                && coordinate[2].abs() < 1e-8));
    }

    #[test]
    fn exact_one_mesh_contours_skip_separated_inputs() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let first_faces = vec![[0, 1, 2]];
        let second_vertices = vec![[0.0, 0.0, 3.0], [2.0, 0.0, 3.0], [0.0, 2.0, 3.0]];
        let second_faces = vec![[0, 1, 2]];

        let contours = exact_one_mesh_intersection_contours(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert!(contours.first.is_empty());
        assert!(contours.second.is_empty());
        assert!(contours.coordinates_in_first_space.is_empty());
    }

    #[test]
    fn exact_lone_subdivision_mesh_splits_first_face_lone_contour_centroid() {
        let vertices = vec![[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [0.0, 3.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let contours = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [0.75, 0.75, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [1.25, 0.75, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Face(0),
                    coordinate: [0.75, 1.25, 0.0],
                },
            ],
            closed: true,
        }];

        let result = exact_lone_subdivision_mesh(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(result.vertices.len(), 4);
        assert_eq!(result.faces.len(), 3);
        assert_eq!(result.source_face_for_faces, vec![0, 0, 0]);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].source_face, 0);
        assert_eq!(result.entries[0].contour_index, 0);
        assert_eq!(result.entries[0].inserted_vertex, 3);
        assert_eq!(
            result.entries[0].coordinate,
            [0.9166666666666666, 0.9166666666666666, 0.0]
        );
    }

    #[test]
    fn exact_lone_subdivision_mesh_ignores_edge_contours() {
        let vertices = vec![[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [0.0, 3.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let contours = vec![ExactOneMeshContour {
            intersections: vec![
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                    coordinate: [1.0, 0.0, 0.0],
                },
                ExactOneMeshIntersection {
                    primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                    coordinate: [1.5, 1.5, 0.0],
                },
            ],
            closed: false,
        }];

        let result = exact_lone_subdivision_mesh(&vertices, &faces, &contours, 1e-9).unwrap();

        assert_eq!(result.vertices, vertices);
        assert_eq!(result.faces, faces);
        assert!(result.entries.is_empty());
    }
}
