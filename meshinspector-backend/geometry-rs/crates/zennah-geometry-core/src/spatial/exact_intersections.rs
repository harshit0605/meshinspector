use super::bvh::{build_flat_bvh, overlapping_face_pairs_between};
use super::exact_kernel::{
    triangle_triangle_intersections_sos, ExactCoordinateConverter, ExactVertCoords,
    TriangleEdgeIntersection,
};
use crate::mesh::validate_faces;
use crate::GeometryError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactMeshIntersection {
    pub first_face: usize,
    pub second_face: usize,
    pub intersections: Vec<TriangleEdgeIntersection>,
}

/// Finds exact edge/face intersections for candidate triangle pairs between two meshes.
pub fn exact_mesh_intersections(
    first_vertices: &[[f64; 3]],
    first_faces_i64: &[[i64; 3]],
    second_vertices: &[[f64; 3]],
    second_faces_i64: &[[i64; 3]],
    leaf_size: usize,
    epsilon: f64,
) -> Result<Vec<ExactMeshIntersection>, GeometryError> {
    let first_faces = validate_faces(first_faces_i64, first_vertices.len())?;
    let second_faces = validate_faces(second_faces_i64, second_vertices.len())?;
    if first_faces.is_empty() || second_faces.is_empty() {
        return Ok(Vec::new());
    }

    let first_triangles = triangles_from_faces(first_vertices, &first_faces);
    let second_triangles = triangles_from_faces(second_vertices, &second_faces);
    let first_bvh = build_flat_bvh(&first_triangles, leaf_size.max(1));
    let second_bvh = build_flat_bvh(&second_triangles, leaf_size.max(1));
    let candidate_pairs = overlapping_face_pairs_between(&first_bvh, &second_bvh, epsilon);
    if candidate_pairs.is_empty() {
        return Ok(Vec::new());
    }

    let Some(converter) = converter_for_mesh_pair(first_vertices, second_vertices) else {
        return Ok(Vec::new());
    };
    let Some(first_exact_vertices) = quantize_vertices(first_vertices, &converter, 0) else {
        return Ok(Vec::new());
    };
    let Some(second_exact_vertices) =
        quantize_vertices(second_vertices, &converter, first_vertices.len() as u64)
    else {
        return Ok(Vec::new());
    };

    let mut intersections = Vec::new();
    for (first_face, second_face) in candidate_pairs {
        let first_triangle = exact_triangle(first_faces[first_face], &first_exact_vertices);
        let second_triangle = exact_triangle(second_faces[second_face], &second_exact_vertices);
        let Some(pair_intersections) =
            triangle_triangle_intersections_sos(first_triangle, second_triangle)
        else {
            continue;
        };
        if !pair_intersections.is_empty() {
            intersections.push(ExactMeshIntersection {
                first_face,
                second_face,
                intersections: pair_intersections,
            });
        }
    }
    Ok(intersections)
}

fn triangles_from_faces(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> Vec<[[f64; 3]; 3]> {
    faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect()
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

fn quantize_vertices(
    vertices: &[[f64; 3]],
    converter: &ExactCoordinateConverter,
    id_offset: u64,
) -> Option<Vec<ExactVertCoords>> {
    vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            converter
                .quantize_point(*vertex)
                .map(|point| ExactVertCoords::new(id_offset + index as u64, point))
        })
        .collect()
}

fn exact_triangle(face: [usize; 3], vertices: &[ExactVertCoords]) -> [ExactVertCoords; 3] {
    [vertices[face[0]], vertices[face[1]], vertices[face[2]]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::ExactTriangleOwner;

    #[test]
    fn exact_mesh_intersections_reports_crossing_triangle_pair() {
        let first_vertices = vec![[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]];
        let first_faces = vec![[0, 1, 2]];
        let second_vertices = vec![[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [3.0, 0.0, 0.0]];
        let second_faces = vec![[0, 1, 2]];

        let intersections = exact_mesh_intersections(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert_eq!(intersections.len(), 1);
        assert_eq!(intersections[0].first_face, 0);
        assert_eq!(intersections[0].second_face, 0);
        assert!(intersections[0].intersections.iter().any(|intersection| {
            intersection.edge_owner == ExactTriangleOwner::Second && intersection.edge == [0, 1]
        }));
    }

    #[test]
    fn exact_mesh_intersections_skips_separated_bvh_pairs() {
        let first_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let first_faces = vec![[0, 1, 2]];
        let second_vertices = vec![[0.0, 0.0, 3.0], [2.0, 0.0, 3.0], [0.0, 2.0, 3.0]];
        let second_faces = vec![[0, 1, 2]];

        let intersections = exact_mesh_intersections(
            &first_vertices,
            &first_faces,
            &second_vertices,
            &second_faces,
            8,
            1e-9,
        )
        .unwrap();

        assert!(intersections.is_empty());
    }
}
