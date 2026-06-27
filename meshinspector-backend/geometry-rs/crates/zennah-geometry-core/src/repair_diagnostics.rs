use crate::mesh::mesh_health;
use crate::{
    basic_repair, crease_edge_diagnostics, duplicate_multi_hole_vertices,
    duplicate_nonmanifold_vertices, hole_complicating_faces_diagnostics, multiple_edge_diagnostics,
    not_smooth_face_diagnostics, repeated_hole_boundary_vertices_diagnostics, GeometryError,
};
use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshHealerIssue {
    pub issue_id: String,
    pub label: String,
    pub count: usize,
    pub severity: String,
    pub rust_repair_available: bool,
    pub repair_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshHealerReport {
    pub input_vertex_count: usize,
    pub input_face_count: usize,
    pub holes_count: usize,
    pub boundary_edge_count: usize,
    pub nonmanifold_edge_count: usize,
    pub self_intersections: Option<usize>,
    pub self_intersections_available: bool,
    pub total_issue_count: usize,
    pub issue_category_count: usize,
    pub fixable_issue_count: usize,
    pub auto_repair_ready: bool,
    pub issues: Vec<MeshHealerIssue>,
}

pub fn mesh_healer_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    merge_tolerance: f64,
    area_epsilon: f64,
    detect_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> Result<MeshHealerReport, GeometryError> {
    let low_risk_repair = basic_repair(vertices, faces_i64, merge_tolerance, area_epsilon)?;
    let health = mesh_health(
        vertices,
        faces_i64,
        detect_self_intersections,
        max_self_intersection_faces,
        epsilon,
    )?;
    let multiple_edges = multiple_edge_diagnostics(vertices, faces_i64)?;
    let multi_hole_vertices = duplicate_multi_hole_vertices(vertices, faces_i64)?;
    let nonmanifold_vertices = duplicate_nonmanifold_vertices(vertices, faces_i64)?;
    let repeated_hole_boundary_vertices =
        repeated_hole_boundary_vertices_diagnostics(vertices, faces_i64)?;
    let hole_complicating_faces = hole_complicating_faces_diagnostics(vertices, faces_i64)?;
    let not_smooth_faces = not_smooth_face_diagnostics(vertices, faces_i64, 0.3)?;
    let crease_edges = crease_edge_diagnostics(vertices, faces_i64, PI * 175.0 / 180.0)?;
    let mut issues = Vec::new();

    push_issue(
        &mut issues,
        "duplicate_vertices",
        "Duplicate / close vertices",
        low_risk_repair.report.merged_vertices,
        "warning",
        true,
        Some("unite_close_vertices"),
    );
    push_issue(
        &mut issues,
        "degenerate_faces",
        "Degenerate faces",
        low_risk_repair.report.removed_degenerate_faces,
        "error",
        true,
        Some("basic_repair"),
    );
    push_issue(
        &mut issues,
        "unreferenced_vertices",
        "Unreferenced vertices",
        low_risk_repair.report.removed_unreferenced_vertices,
        "info",
        true,
        Some("basic_repair"),
    );
    push_issue(
        &mut issues,
        "holes",
        "Open holes",
        health.holes_count,
        "warning",
        true,
        Some("service_fill_holes"),
    );
    push_issue(
        &mut issues,
        "multiple_edges",
        "Multiple edges",
        multiple_edges.multiple_edge_count,
        "error",
        true,
        Some("repair_multiple_edges"),
    );
    push_issue(
        &mut issues,
        "multi_hole_vertices",
        "Multi-hole vertices",
        multi_hole_vertices.report.input_multi_hole_vertex_count,
        "warning",
        true,
        Some("duplicate_multi_hole_vertices"),
    );
    push_issue(
        &mut issues,
        "nonmanifold_vertices",
        "Non-manifold vertices",
        nonmanifold_vertices.report.input_nonmanifold_vertex_count,
        "error",
        true,
        Some("duplicate_nonmanifold_vertices"),
    );
    push_issue(
        &mut issues,
        "repeated_hole_boundary_vertices",
        "Repeated hole-boundary vertices",
        repeated_hole_boundary_vertices.repeated_vertex_count,
        "warning",
        false,
        None,
    );
    push_issue(
        &mut issues,
        "hole_complicating_faces",
        "Hole-complicating faces",
        hole_complicating_faces.complicating_face_count,
        "warning",
        true,
        Some("remove_hole_complicating_faces"),
    );
    push_issue(
        &mut issues,
        "not_smooth_faces",
        "Not-smooth faces",
        not_smooth_faces.not_smooth_face_count,
        "warning",
        false,
        None,
    );
    push_issue(
        &mut issues,
        "crease_edges",
        "Crease edges",
        crease_edges.crease_edge_count,
        "warning",
        true,
        Some("fix_mesh_creases"),
    );
    push_issue(
        &mut issues,
        "nonmanifold_edges",
        "Non-manifold edges",
        health.nonmanifold_edge_count,
        "error",
        true,
        Some("repair_nonmanifold_edges"),
    );
    if let Some(self_intersections) = health.self_intersections {
        push_issue(
            &mut issues,
            "self_intersections",
            "Self-intersections",
            self_intersections,
            "error",
            true,
            Some("rebuild_via_sdf"),
        );
    }

    let total_issue_count = issues.iter().map(|issue| issue.count).sum();
    let fixable_issue_count = issues
        .iter()
        .filter(|issue| issue.rust_repair_available)
        .map(|issue| issue.count)
        .sum();
    let auto_repair_ready = issues.iter().all(|issue| issue.rust_repair_available);
    Ok(MeshHealerReport {
        input_vertex_count: vertices.len(),
        input_face_count: faces_i64.len(),
        holes_count: health.holes_count,
        boundary_edge_count: health.boundary_edge_count,
        nonmanifold_edge_count: health.nonmanifold_edge_count,
        self_intersections: health.self_intersections,
        self_intersections_available: health.self_intersections_available,
        total_issue_count,
        issue_category_count: issues.len(),
        fixable_issue_count,
        auto_repair_ready,
        issues,
    })
}

fn push_issue(
    issues: &mut Vec<MeshHealerIssue>,
    issue_id: &str,
    label: &str,
    count: usize,
    severity: &str,
    rust_repair_available: bool,
    repair_command: Option<&str>,
) {
    if count == 0 {
        return;
    }
    issues.push(MeshHealerIssue {
        issue_id: issue_id.to_string(),
        label: label.to_string(),
        count,
        severity: severity.to_string(),
        rust_repair_available,
        repair_command: repair_command.map(str::to_string),
    });
}
