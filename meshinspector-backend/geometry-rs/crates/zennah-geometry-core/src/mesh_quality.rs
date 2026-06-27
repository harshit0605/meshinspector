//! Output-quality verdicts for mesh operations.
//!
//! These encode the *acceptance policy* (thresholds + the geometry facts that
//! matter) for each operation's result. The geometry facts come from the same
//! Rust `mesh_stats`/`mesh_health` kernels the rest of the SDK uses, and the
//! threshold logic now lives here in Rust rather than in the Python service
//! layer. Each function returns the human-readable failure clauses; the caller
//! joins them into a final message and rejects the job when the list is
//! non-empty (an empty list means the output is acceptable).

use crate::mesh::{mesh_health, mesh_stats};
use crate::{GeometryError, MeshHealth, MeshStats};

fn stats_and_health(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
) -> Result<(MeshStats, MeshHealth), GeometryError> {
    let stats = mesh_stats(vertices, faces)?;
    let health = mesh_health(vertices, faces, false, None, 0.0)?;
    Ok((stats, health))
}

/// Failure clauses for a decimate result. Empty => acceptable.
pub fn decimate_output_failures(
    source_vertices: &[[f64; 3]],
    source_faces: &[[i64; 3]],
    output_vertices: &[[f64; 3]],
    output_faces: &[[i64; 3]],
) -> Result<Vec<String>, GeometryError> {
    let (source, source_health) = stats_and_health(source_vertices, source_faces)?;
    let (output, output_health) = stats_and_health(output_vertices, output_faces)?;
    let mut failures = Vec::new();

    if source.face_count > 0 && output.face_count == 0 {
        failures.push("removed all faces".to_string());
    }
    if source.vertex_count > 0 && output.vertex_count == 0 {
        failures.push("removed all vertices".to_string());
    }
    if source.boundary_edge_count == 0 && output.boundary_edge_count > 0 {
        failures.push(format!(
            "introduced {} boundary edge(s) into a closed source mesh",
            output.boundary_edge_count
        ));
    }
    if output_health.nonmanifold_edge_count > source_health.nonmanifold_edge_count {
        failures.push(format!(
            "introduced {} non-manifold edge(s); reduce the deletion amount or decimate a smaller region",
            output_health.nonmanifold_edge_count - source_health.nonmanifold_edge_count
        ));
    }
    let max_components = (source.connected_components * 2).max(source.connected_components + 2);
    if output.connected_components > max_components {
        failures.push(format!(
            "fragmented mesh components from {} to {}",
            source.connected_components, output.connected_components
        ));
    }
    Ok(failures)
}

/// Failure clauses for a hollow result. Empty => acceptable.
pub fn hollow_output_failures(
    source_vertices: &[[f64; 3]],
    source_faces: &[[i64; 3]],
    output_vertices: &[[f64; 3]],
    output_faces: &[[i64; 3]],
    wall_thickness_mm: f64,
) -> Result<Vec<String>, GeometryError> {
    let source = mesh_stats(source_vertices, source_faces)?;
    let output = mesh_stats(output_vertices, output_faces)?;
    let mut failures = Vec::new();

    if output.face_count == 0 || output.vertex_count == 0 {
        failures.push("produced an empty mesh".to_string());
        return Ok(failures);
    }

    // A shell keeps roughly wall x surface-area of material; far less means the
    // voxel difference crumbled into fragments.
    let expected_min_volume = (0.25 * wall_thickness_mm * source.surface_area_mm2).max(1e-6);
    if output.volume_mm3 < expected_min_volume.min(0.2 * source.volume_mm3) {
        failures.push(format!(
            "kept only {:.2} mm3 of {:.2} mm3",
            output.volume_mm3, source.volume_mm3
        ));
    }
    let max_components = (source.connected_components * 4).max(source.connected_components + 6);
    if output.connected_components > max_components {
        failures.push(format!(
            "fragmented into {} components",
            output.connected_components
        ));
    }
    Ok(failures)
}

/// Geometry failure clauses for an offset/shell result. Empty => acceptable.
/// Only applies when the source is watertight: the offset/shell of a closed
/// solid must stay closed and not fragment. (The trivial output/source
/// face-count ratio threshold is a scalar count policy applied by the caller.)
pub fn offset_shell_failures(
    source_vertices: &[[f64; 3]],
    source_faces: &[[i64; 3]],
    output_vertices: &[[f64; 3]],
    output_faces: &[[i64; 3]],
) -> Result<Vec<String>, GeometryError> {
    let source = mesh_stats(source_vertices, source_faces)?;
    let output = mesh_stats(output_vertices, output_faces)?;
    let mut failures = Vec::new();

    if output.face_count == 0 || output.vertex_count == 0 {
        return Ok(failures); // emptiness is handled by the caller's nonempty guard
    }

    if source.boundary_edge_count == 0 {
        if output.boundary_edge_count > 0 {
            failures.push(format!(
                "opened {} boundary edge(s) on a watertight source, so the shell is not manufacturable",
                output.boundary_edge_count
            ));
        }
        let max_components = (source.connected_components * 2).max(source.connected_components + 4);
        if output.connected_components > max_components {
            failures.push(format!(
                "fragmented a watertight source into {} components",
                output.connected_components
            ));
        }
    }
    Ok(failures)
}

/// Failure clauses for a boolean result. Empty => acceptable. A clean empty
/// result and a solid result are both fine; a non-empty near-zero-volume sliver
/// is degenerate residue.
pub fn boolean_output_failures(
    output_vertices: &[[f64; 3]],
    output_faces: &[[i64; 3]],
    operation: &str,
) -> Result<Vec<String>, GeometryError> {
    let output = mesh_stats(output_vertices, output_faces)?;
    let mut failures = Vec::new();
    if output.face_count > 0 && output.volume_mm3.abs() < 1e-6 {
        failures.push(format!(
            "Boolean {operation} produced {} degenerate, zero-volume face(s) instead of a solid result. The operands likely coincide or barely overlap; check their relative position and scale.",
            output.face_count
        ));
    }
    Ok(failures)
}
