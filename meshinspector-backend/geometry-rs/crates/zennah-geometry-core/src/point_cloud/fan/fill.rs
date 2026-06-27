use super::point_cloud_triangulate_topology_candidate_mesh;

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudTriangulatedFilledCandidateMesh {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub repetition_counts: [usize; 4],
    pub repeated_3_count: usize,
    pub repeated_2_count: usize,
    pub candidate_face_count: usize,
    pub topology_skipped_face_count: usize,
    pub topology_degenerate_face_count: usize,
    pub topology_nonmanifold_edge_face_count: usize,
    pub topology_nonmanifold_vertex_face_count: usize,
    pub topology_unsafe_retry_face_count: usize,
    pub removed_hole_complicating_face_count: usize,
    pub input_hole_count: usize,
    pub filled_hole_count: usize,
    pub skipped_hole_count: usize,
    pub added_fill_face_count: usize,
    pub max_hole_perimeter: f64,
}

pub fn point_cloud_triangulate_filled_candidate_mesh(
    points: &[[f64; 3]],
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    crit_hole_length: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<PointCloudTriangulatedFilledCandidateMesh, String> {
    let topology = point_cloud_triangulate_topology_candidate_mesh(
        points,
        radius,
        num_neighbors,
        boundary_angle,
        max_removes,
        crit_angle,
        normals,
        untrusted_indices,
    )?;
    let max_hole_perimeter = if crit_hole_length >= 0.0 {
        crit_hole_length
    } else {
        bounding_box_diagonal(&topology.vertices) * 0.1
    };
    let fill = fill_holes_by_perimeter(&topology.vertices, &topology.faces, max_hole_perimeter)?;

    Ok(PointCloudTriangulatedFilledCandidateMesh {
        vertices: topology.vertices,
        faces: fill.faces,
        repetition_counts: topology.repetition_counts,
        repeated_3_count: topology.repeated_3_count,
        repeated_2_count: topology.repeated_2_count,
        candidate_face_count: topology.candidate_face_count,
        topology_skipped_face_count: topology.topology_skipped_face_count,
        topology_degenerate_face_count: topology.topology_degenerate_face_count,
        topology_nonmanifold_edge_face_count: topology.topology_nonmanifold_edge_face_count,
        topology_nonmanifold_vertex_face_count: topology.topology_nonmanifold_vertex_face_count,
        topology_unsafe_retry_face_count: topology.topology_unsafe_retry_face_count,
        removed_hole_complicating_face_count: topology.removed_hole_complicating_face_count,
        input_hole_count: fill.input_hole_count,
        filled_hole_count: fill.filled_hole_count,
        skipped_hole_count: fill.skipped_hole_count,
        added_fill_face_count: fill.added_fill_face_count,
        max_hole_perimeter,
    })
}

#[derive(Debug, Clone, PartialEq)]
struct PerimeterFillResult {
    faces: Vec<[i64; 3]>,
    input_hole_count: usize,
    filled_hole_count: usize,
    skipped_hole_count: usize,
    added_fill_face_count: usize,
}

fn fill_holes_by_perimeter(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    max_hole_perimeter: f64,
) -> Result<PerimeterFillResult, String> {
    if !max_hole_perimeter.is_finite() || max_hole_perimeter < 0.0 {
        return Err("max_hole_perimeter must be finite and non-negative".to_string());
    }
    let loops =
        crate::ordered_boundary_loops(vertices, faces).map_err(|error| error.to_string())?;
    let mut output_faces = faces.to_vec();
    let mut filled_hole_count = 0;
    let mut skipped_hole_count = 0;
    let mut added_fill_face_count = 0;

    for boundary_loop in &loops {
        let perimeter = boundary_loop_perimeter(vertices, boundary_loop);
        if perimeter > max_hole_perimeter {
            skipped_hole_count += 1;
            continue;
        }
        let new_faces = crate::repair::fill::triangulate_hole_loop_strong(
            vertices,
            &output_faces,
            boundary_loop,
        );
        added_fill_face_count += new_faces.len();
        output_faces.extend(new_faces);
        filled_hole_count += 1;
    }

    let output_faces = if filled_hole_count > 0 {
        crate::orient_faces_outward(vertices, &output_faces).map_err(|error| error.to_string())?
    } else {
        output_faces
    };

    Ok(PerimeterFillResult {
        faces: output_faces,
        input_hole_count: loops.len(),
        filled_hole_count,
        skipped_hole_count,
        added_fill_face_count,
    })
}

fn boundary_loop_perimeter(vertices: &[[f64; 3]], boundary_loop: &[usize]) -> f64 {
    if boundary_loop.len() < 2 {
        return 0.0;
    }
    boundary_loop
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            let next = boundary_loop[(index + 1) % boundary_loop.len()];
            distance(vertices[*vertex], vertices[next])
        })
        .sum()
}

fn bounding_box_diagonal(vertices: &[[f64; 3]]) -> f64 {
    if vertices.is_empty() {
        return 0.0;
    }
    let mut min = vertices[0];
    let mut max = vertices[0];
    for vertex in vertices.iter().skip(1) {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    distance(min, max)
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_holes_by_perimeter_fills_only_holes_under_meshlib_threshold() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let faces = vec![[0, 1, 2]];

        let skipped = fill_holes_by_perimeter(&vertices, &faces, 2.0).unwrap();
        let filled = fill_holes_by_perimeter(&vertices, &faces, 4.0).unwrap();

        assert_eq!(skipped.input_hole_count, 1);
        assert_eq!(skipped.filled_hole_count, 0);
        assert_eq!(skipped.skipped_hole_count, 1);
        assert_eq!(filled.filled_hole_count, 1);
        assert_eq!(filled.added_fill_face_count, 1);
        assert_eq!(filled.faces.len(), 2);
    }

    #[test]
    fn point_cloud_triangulate_filled_candidate_mesh_uses_default_bbox_threshold() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let mesh = point_cloud_triangulate_filled_candidate_mesh(
            &points,
            1.5,
            0,
            3.0,
            usize::MAX,
            std::f64::consts::TAU,
            -1.0,
            Some(&normals),
            &[],
        )
        .expect("filled candidate mesh should build");

        assert_eq!(mesh.faces, vec![[0, 1, 2]]);
        assert_eq!(mesh.input_hole_count, 1);
        assert_eq!(mesh.filled_hole_count, 0);
        assert_eq!(mesh.skipped_hole_count, 1);
        assert!(mesh.max_hole_perimeter > 0.0);
    }
}
