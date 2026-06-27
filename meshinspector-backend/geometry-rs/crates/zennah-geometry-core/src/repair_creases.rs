use crate::mesh::validate_faces;
use crate::repair::{orient_faces_outward, triangulate_hole_loop};
use crate::repair_smoothness::{crease_edge_diagnostics, crease_repair_plan_diagnostics};
use crate::GeometryError;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct FixMeshCreasesReport {
    pub input_face_count: usize,
    pub output_face_count: usize,
    pub input_crease_edge_count: usize,
    pub output_crease_edge_count: usize,
    pub repaired_region_count: usize,
    pub removed_face_count: usize,
    pub added_face_count: usize,
    pub filled_hole_count: usize,
    pub skipped_hole_count: usize,
    pub iteration_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixMeshCreasesResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: FixMeshCreasesReport,
}

pub fn fix_mesh_creases(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    angle_from_planar_radians: f64,
    critical_tri_aspect_ratio: f64,
    max_iters: usize,
) -> Result<FixMeshCreasesResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let input_crease_edge_count =
        crease_edge_diagnostics(vertices, faces_i64, angle_from_planar_radians)?.crease_edge_count;
    let mut faces = faces_i64.to_vec();
    let mut repaired_region_count = 0_usize;
    let mut removed_face_count = 0_usize;
    let mut added_face_count = 0_usize;
    let mut filled_hole_count = 0_usize;
    let mut skipped_hole_count = 0_usize;
    let mut iteration_count = 0_usize;

    for _ in 0..max_iters {
        let plan = crease_repair_plan_diagnostics(
            vertices,
            &faces,
            angle_from_planar_radians,
            critical_tri_aspect_ratio,
        )?;
        if plan.planned_region_count == 0 {
            break;
        }

        let selected_face_indices = plan
            .regions
            .iter()
            .flat_map(|region| region.selected_face_indices.iter().copied())
            .collect::<BTreeSet<_>>();
        if selected_face_indices.is_empty() {
            break;
        }

        let selected_faces = faces
            .iter()
            .enumerate()
            .filter_map(|(face_index, face)| {
                selected_face_indices.contains(&face_index).then_some(*face)
            })
            .collect::<Vec<_>>();
        let region_loops = crate::ordered_boundary_loops(vertices, &selected_faces)?;
        let mut next_faces = faces
            .iter()
            .enumerate()
            .filter_map(|(face_index, face)| {
                (!selected_face_indices.contains(&face_index)).then_some(*face)
            })
            .collect::<Vec<_>>();

        let mut iteration_added_faces = 0_usize;
        let mut iteration_filled_holes = 0_usize;
        for boundary_loop in region_loops {
            if boundary_loop.len() < 3 {
                skipped_hole_count += 1;
                continue;
            }
            let new_faces = triangulate_hole_loop(vertices, &boundary_loop);
            if new_faces.is_empty() {
                skipped_hole_count += 1;
                continue;
            }
            iteration_added_faces += new_faces.len();
            iteration_filled_holes += 1;
            next_faces.extend(new_faces);
        }
        if iteration_filled_holes == 0 {
            break;
        }

        faces = orient_faces_outward(vertices, &next_faces)?;
        repaired_region_count += plan.planned_region_count;
        removed_face_count += selected_face_indices.len();
        added_face_count += iteration_added_faces;
        filled_hole_count += iteration_filled_holes;
        iteration_count += 1;
    }

    let output_crease_edge_count =
        crease_edge_diagnostics(vertices, &faces, angle_from_planar_radians)?.crease_edge_count;
    let output_face_count = faces.len();
    Ok(FixMeshCreasesResult {
        vertices: vertices.to_vec(),
        faces,
        report: FixMeshCreasesReport {
            input_face_count: faces_i64.len(),
            output_face_count,
            input_crease_edge_count,
            output_crease_edge_count,
            repaired_region_count,
            removed_face_count,
            added_face_count,
            filled_hole_count,
            skipped_hole_count,
            iteration_count,
        },
    })
}
