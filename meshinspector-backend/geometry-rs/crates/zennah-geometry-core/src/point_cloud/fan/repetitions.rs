use std::collections::HashMap;

use super::point_cloud_local_neighbor_fan;

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudLocalFanTriangles {
    pub triangles: Vec<[i64; 3]>,
    pub boundary_neighbor: i64,
    pub actual_radius: f64,
    pub removed_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudLocalTriangulationRepetitions {
    pub repetition_counts: [usize; 4],
    pub repeated_3: Vec<[i64; 3]>,
    pub repeated_2: Vec<[i64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudTriangulatedCandidateMesh {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub repetition_counts: [usize; 4],
    pub repeated_3_count: usize,
    pub repeated_2_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudTriangulatedCleanedCandidateMesh {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub repetition_counts: [usize; 4],
    pub repeated_3_count: usize,
    pub repeated_2_count: usize,
    pub input_face_count: usize,
    pub removed_hole_complicating_face_count: usize,
    pub output_repeated_boundary_vertex_count: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct Repetitions {
    same_oriented: usize,
    opposite_oriented: usize,
}

pub fn point_cloud_local_fan_triangles(
    points: &[[f64; 3]],
    center_index: usize,
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<PointCloudLocalFanTriangles, String> {
    let fan = point_cloud_local_neighbor_fan(
        points,
        center_index,
        radius,
        num_neighbors,
        boundary_angle,
        max_removes,
        crit_angle,
        normals,
        untrusted_indices,
    )?;
    let mut triangles = Vec::new();
    if fan.neighbors.len() >= 2 {
        for (offset, &curr) in fan.neighbors.iter().enumerate() {
            if curr == fan.boundary_neighbor {
                continue;
            }
            let next = fan.neighbors[(offset + 1) % fan.neighbors.len()];
            if next == curr {
                continue;
            }
            triangles.push([center_index as i64, next, curr]);
        }
    }

    Ok(PointCloudLocalFanTriangles {
        triangles,
        boundary_neighbor: fan.boundary_neighbor,
        actual_radius: fan.actual_radius,
        removed_count: fan.removed_count,
    })
}

pub fn point_cloud_local_triangulation_repetitions(
    points: &[[f64; 3]],
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<PointCloudLocalTriangulationRepetitions, String> {
    if points.is_empty() {
        return Err("point cloud must not be empty".to_string());
    }
    let mut map = HashMap::<[usize; 3], Repetitions>::new();
    for center_index in 0..points.len() {
        let fan = point_cloud_local_fan_triangles(
            points,
            center_index,
            radius,
            num_neighbors,
            boundary_angle,
            max_removes,
            crit_angle,
            normals,
            untrusted_indices,
        )?;
        for triangle in fan.triangles {
            let triangle = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            add_repetition(&mut map, triangle);
        }
    }

    let mut repetition_counts = [0usize; 4];
    let mut repeated_3 = Vec::new();
    let mut repeated_2 = Vec::new();
    let mut entries = map.into_iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (triangle, repetitions) in entries {
        let total = repetitions.same_oriented + repetitions.opposite_oriented;
        if total < repetition_counts.len() {
            repetition_counts[total] += 1;
        }
        if repetitions.same_oriented > 0 && repetitions.opposite_oriented > 0 {
            repetition_counts[0] += 1;
        }
        push_repeated_orientation(&mut repeated_3, triangle, repetitions, 3);
        push_repeated_orientation(&mut repeated_2, triangle, repetitions, 2);
    }

    Ok(PointCloudLocalTriangulationRepetitions {
        repetition_counts,
        repeated_3,
        repeated_2,
    })
}

pub fn point_cloud_triangulate_candidate_mesh(
    points: &[[f64; 3]],
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<PointCloudTriangulatedCandidateMesh, String> {
    let repetitions = point_cloud_local_triangulation_repetitions(
        points,
        radius,
        num_neighbors,
        boundary_angle,
        max_removes,
        crit_angle,
        normals,
        untrusted_indices,
    )?;
    let mut repeated_3 = repetitions.repeated_3;
    let mut repeated_2 = repetitions.repeated_2;
    sort_meshlib_triangles(&mut repeated_3);
    sort_meshlib_triangles(&mut repeated_2);
    let repeated_3_count = repeated_3.len();
    let repeated_2_count = repeated_2.len();
    repeated_3.extend(repeated_2);

    Ok(PointCloudTriangulatedCandidateMesh {
        vertices: points.to_vec(),
        faces: repeated_3,
        repetition_counts: repetitions.repetition_counts,
        repeated_3_count,
        repeated_2_count,
    })
}

pub fn point_cloud_triangulate_cleaned_candidate_mesh(
    points: &[[f64; 3]],
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
) -> Result<PointCloudTriangulatedCleanedCandidateMesh, String> {
    let candidate = point_cloud_triangulate_candidate_mesh(
        points,
        radius,
        num_neighbors,
        boundary_angle,
        max_removes,
        crit_angle,
        normals,
        untrusted_indices,
    )?;
    let input_face_count = candidate.faces.len();
    let cleaned = crate::remove_hole_complicating_faces(&candidate.vertices, &candidate.faces)
        .map_err(|error| error.to_string())?;

    Ok(PointCloudTriangulatedCleanedCandidateMesh {
        vertices: cleaned.vertices,
        faces: cleaned.faces,
        repetition_counts: candidate.repetition_counts,
        repeated_3_count: candidate.repeated_3_count,
        repeated_2_count: candidate.repeated_2_count,
        input_face_count,
        removed_hole_complicating_face_count: cleaned.report.removed_face_count,
        output_repeated_boundary_vertex_count: cleaned.report.output_repeated_vertex_count,
    })
}

fn add_repetition(map: &mut HashMap<[usize; 3], Repetitions>, triangle: [usize; 3]) {
    if triangle[0] == triangle[1] || triangle[0] == triangle[2] || triangle[1] == triangle[2] {
        return;
    }
    let (key, flipped) = unoriented_triangle_key(triangle);
    let repetitions = map.entry(key).or_default();
    if flipped {
        repetitions.opposite_oriented += 1;
    } else {
        repetitions.same_oriented += 1;
    }
}

fn unoriented_triangle_key(mut verts: [usize; 3]) -> ([usize; 3], bool) {
    let mut flipped = false;
    check_swap(&mut verts, &mut flipped, 0, 1);
    check_swap(&mut verts, &mut flipped, 0, 2);
    check_swap(&mut verts, &mut flipped, 1, 2);
    (verts, flipped)
}

fn check_swap(verts: &mut [usize; 3], flipped: &mut bool, left: usize, right: usize) {
    if verts[left] > verts[right] {
        *flipped = !*flipped;
        verts.swap(left, right);
    }
}

fn push_repeated_orientation(
    output: &mut Vec<[i64; 3]>,
    triangle: [usize; 3],
    repetitions: Repetitions,
    target: usize,
) {
    if repetitions.same_oriented == target {
        output.push(to_i64_triangle(triangle));
    }
    if repetitions.opposite_oriented == target {
        output.push([triangle[0] as i64, triangle[2] as i64, triangle[1] as i64]);
    }
}

fn to_i64_triangle(triangle: [usize; 3]) -> [i64; 3] {
    [triangle[0] as i64, triangle[1] as i64, triangle[2] as i64]
}

fn sort_meshlib_triangles(triangles: &mut [[i64; 3]]) {
    triangles.sort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_cloud_local_fan_triangles_uses_meshlib_next_curr_order() {
        let points = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let fan = point_cloud_local_fan_triangles(
            &points,
            0,
            1.1,
            0,
            3.2,
            0,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("fan triangles should build");

        assert_eq!(
            fan.triangles,
            vec![[0, 1, 2], [0, 4, 1], [0, 3, 4], [0, 2, 3]]
        );
        assert_eq!(fan.boundary_neighbor, -1);
        assert_eq!(fan.removed_count, 0);
    }

    #[test]
    fn point_cloud_local_fan_triangles_skips_boundary_neighbor_triangle() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let fan = point_cloud_local_fan_triangles(
            &points,
            0,
            1.1,
            0,
            3.0,
            0,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("fan triangles should build");

        assert_eq!(fan.triangles, vec![[0, 1, 2]]);
        assert_eq!(fan.boundary_neighbor, 1);
    }

    #[test]
    fn unoriented_triangle_key_matches_meshlib_swap_flipped_flag() {
        assert_eq!(unoriented_triangle_key([0, 1, 2]), ([0, 1, 2], false));
        assert_eq!(unoriented_triangle_key([0, 2, 1]), ([0, 1, 2], true));
        assert_eq!(unoriented_triangle_key([2, 0, 1]), ([0, 1, 2], false));
    }

    #[test]
    fn point_cloud_local_triangulation_repetitions_counts_same_and_opposite_like_meshlib() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let repetitions = point_cloud_local_triangulation_repetitions(
            &points,
            1.5,
            0,
            3.0,
            0,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("repetitions should compute");

        assert_eq!(repetitions.repetition_counts, [0, 0, 0, 1]);
        assert_eq!(repetitions.repeated_3, vec![[0, 1, 2]]);
        assert!(repetitions.repeated_2.is_empty());
    }

    #[test]
    fn point_cloud_triangulate_candidate_mesh_returns_sorted_rep3_then_rep2_faces() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let mesh = point_cloud_triangulate_candidate_mesh(
            &points,
            1.5,
            0,
            3.0,
            usize::MAX,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("candidate mesh should build");

        assert_eq!(mesh.vertices, points);
        assert_eq!(mesh.faces, vec![[0, 1, 2]]);
        assert_eq!(mesh.repetition_counts, [0, 0, 0, 1]);
        assert_eq!(mesh.repeated_3_count, 1);
        assert_eq!(mesh.repeated_2_count, 0);
    }

    #[test]
    fn point_cloud_triangulate_cleaned_candidate_mesh_reports_bad_triangle_removal_stage() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0]; points.len()];

        let mesh = point_cloud_triangulate_cleaned_candidate_mesh(
            &points,
            1.5,
            0,
            3.0,
            usize::MAX,
            std::f64::consts::TAU,
            Some(&normals),
            &[],
        )
        .expect("cleaned candidate mesh should build");

        assert_eq!(mesh.vertices, points);
        assert_eq!(mesh.faces, vec![[0, 1, 2]]);
        assert_eq!(mesh.input_face_count, 1);
        assert_eq!(mesh.removed_hole_complicating_face_count, 0);
        assert_eq!(mesh.output_repeated_boundary_vertex_count, 0);
    }

    #[test]
    fn cleaned_candidate_stage_removes_meshlib_hole_complicating_faces() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 1.0, 0.0],
        ];
        let faces = vec![[0, 1, 4], [1, 2, 5], [1, 3, 4]];
        let cleaned = crate::remove_hole_complicating_faces(&vertices, &faces)
            .expect("hole-complicating faces should be removed");

        assert_eq!(cleaned.faces, vec![[0, 1, 4], [1, 3, 4]]);
        assert_eq!(cleaned.report.removed_face_count, 1);
        assert_eq!(cleaned.report.output_repeated_vertex_count, 0);
    }

    #[test]
    fn sort_meshlib_triangles_matches_point_cloud_triangulator_lexicographic_order() {
        let mut faces = vec![[2, 0, 1], [0, 2, 1], [0, 1, 2], [0, 1, 1]];

        sort_meshlib_triangles(&mut faces);

        assert_eq!(faces, vec![[0, 1, 1], [0, 1, 2], [0, 2, 1], [2, 0, 1]]);
    }
}
