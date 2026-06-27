use crate::mesh::{edge_face_map, validate_faces};
use crate::{service_fill_holes, GeometryError};
use std::collections::{BTreeSet, VecDeque};

mod topology;

#[derive(Debug, Clone, PartialEq)]
pub struct TunnelDiagnostics {
    pub vertex_count: usize,
    pub face_count: usize,
    pub edge_count: usize,
    pub connected_component_count: usize,
    pub boundary_edge_count: usize,
    pub nonmanifold_edge_count: usize,
    pub euler_characteristic: i64,
    pub genus: Option<i64>,
    pub tunnel_count: usize,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TunnelEliminationReport {
    pub input_face_count: usize,
    pub detected_tunnel_face_count: usize,
    pub removed_face_count: usize,
    pub filled_holes: usize,
    pub added_faces: usize,
    pub output_face_count: usize,
    pub output_boundary_edge_count: usize,
    pub output_tunnel_count: usize,
    pub tunnel_face_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TunnelEliminationResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: TunnelEliminationReport,
}

pub fn tunnel_diagnostics(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<TunnelDiagnostics, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let edge_map = edge_face_map(&faces);
    let boundary_edge_count = edge_map
        .values()
        .filter(|face_ids| face_ids.len() == 1)
        .count();
    let nonmanifold_edge_count = edge_map
        .values()
        .filter(|face_ids| face_ids.len() > 2)
        .count();
    let used_vertices = faces
        .iter()
        .flat_map(|face| face.iter().copied())
        .collect::<BTreeSet<_>>();
    let connected_component_count = connected_components(&faces, used_vertices.len());
    let euler_characteristic =
        used_vertices.len() as i64 - edge_map.len() as i64 + faces.len() as i64;
    let closed = boundary_edge_count == 0 && nonmanifold_edge_count == 0;
    let genus = if closed && connected_component_count > 0 {
        let numerator = 2_i64 * connected_component_count as i64 - euler_characteristic;
        if numerator >= 0 && numerator % 2 == 0 {
            Some(numerator / 2)
        } else {
            None
        }
    } else {
        None
    };

    Ok(TunnelDiagnostics {
        vertex_count: used_vertices.len(),
        face_count: faces.len(),
        edge_count: edge_map.len(),
        connected_component_count,
        boundary_edge_count,
        nonmanifold_edge_count,
        euler_characteristic,
        genus,
        tunnel_count: genus.unwrap_or(0).max(0) as usize,
        closed,
    })
}

pub fn detect_tunnel_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<usize>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    Ok(topology::detect_tunnel_face_band(vertices, &faces))
}

pub fn eliminate_tunnels(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<TunnelEliminationResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let tunnel_face_indices = detect_tunnel_faces(vertices, faces_i64)?;
    let tunnel_faces = tunnel_face_indices.iter().copied().collect::<BTreeSet<_>>();
    let remaining_faces = faces_i64
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| (!tunnel_faces.contains(&face_index)).then_some(*face))
        .collect::<Vec<_>>();
    let removed_face_count = faces_i64.len().saturating_sub(remaining_faces.len());
    let fill_result = if removed_face_count > 0 {
        service_fill_holes(vertices, &remaining_faces, None)?
    } else {
        crate::HoleFillResult {
            vertices: vertices.to_vec(),
            faces: faces_i64.to_vec(),
            report: crate::HoleFillReport {
                input_holes: 0,
                filled_holes: 0,
                added_vertices: 0,
                added_faces: 0,
                new_face_indices: Vec::new(),
                skipped_holes: 0,
            },
        }
    };
    let output_diagnostics = tunnel_diagnostics(&fill_result.vertices, &fill_result.faces)?;

    Ok(TunnelEliminationResult {
        vertices: fill_result.vertices,
        faces: fill_result.faces,
        report: TunnelEliminationReport {
            input_face_count: faces_i64.len(),
            detected_tunnel_face_count: tunnel_face_indices.len(),
            removed_face_count,
            filled_holes: fill_result.report.filled_holes,
            added_faces: fill_result.report.added_faces,
            output_face_count: output_diagnostics.face_count,
            output_boundary_edge_count: output_diagnostics.boundary_edge_count,
            output_tunnel_count: output_diagnostics.tunnel_count,
            tunnel_face_indices,
        },
    })
}

fn connected_components(faces: &[[usize; 3]], referenced_vertex_count: usize) -> usize {
    if faces.is_empty() || referenced_vertex_count == 0 {
        return 0;
    }

    let mut vertex_faces = Vec::<Vec<usize>>::new();
    let max_vertex = faces
        .iter()
        .flat_map(|face| face.iter().copied())
        .max()
        .unwrap_or(0);
    vertex_faces.resize(max_vertex + 1, Vec::new());
    for (face_index, face) in faces.iter().enumerate() {
        for vertex in face {
            vertex_faces[*vertex].push(face_index);
        }
    }

    let mut seen_faces = vec![false; faces.len()];
    let mut components = 0_usize;
    for start in 0..faces.len() {
        if seen_faces[start] {
            continue;
        }
        components += 1;
        let mut queue = VecDeque::from([start]);
        seen_faces[start] = true;
        while let Some(face_index) = queue.pop_front() {
            for vertex in faces[face_index] {
                for adjacent_face in &vertex_faces[vertex] {
                    if !seen_faces[*adjacent_face] {
                        seen_faces[*adjacent_face] = true;
                        queue.push_back(*adjacent_face);
                    }
                }
            }
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_reports_no_tunnels() {
        let vertices = vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        let faces = vec![
            [0, 3, 2],
            [0, 2, 1],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];

        let report = tunnel_diagnostics(&vertices, &faces).unwrap();

        assert_eq!(report.euler_characteristic, 2);
        assert_eq!(report.genus, Some(0));
        assert_eq!(report.tunnel_count, 0);
        assert!(report.closed);
    }

    #[test]
    fn detect_tunnel_faces_matches_meshlib_torus_face_band() {
        let (vertices, faces) = torus(24, 8);
        let face_band = detect_tunnel_faces(&vertices, &faces).unwrap();

        assert_eq!(
            face_band,
            vec![
                10, 11, 26, 27, 42, 43, 58, 59, 74, 75, 90, 91, 106, 107, 122, 123, 138, 139, 154,
                155, 170, 171, 186, 187, 202, 203, 218, 219, 234, 235, 250, 251, 266, 267, 282,
                283, 298, 299, 314, 315, 330, 331, 346, 347, 362, 363, 378, 379,
            ]
        );
    }

    #[test]
    fn detect_tunnel_faces_matches_meshlib_torus_24x12_face_band() {
        let (vertices, faces) = torus(24, 12);
        let face_band = detect_tunnel_faces(&vertices, &faces).unwrap();
        let expected = (0..24)
            .flat_map(|radial_index| {
                let base = 2 * (radial_index * 12 + 2);
                [base, base + 1]
            })
            .collect::<Vec<_>>();

        assert_eq!(face_band, expected);
    }

    #[test]
    fn detect_tunnel_faces_matches_meshlib_torus_24x10_face_band() {
        let (vertices, faces) = torus(24, 10);
        let face_band = detect_tunnel_faces(&vertices, &faces).unwrap();
        let expected = (0..24)
            .flat_map(|radial_index| {
                let base = 2 * (radial_index * 10 + 1);
                [base, base + 1]
            })
            .collect::<Vec<_>>();

        assert_eq!(face_band, expected);
    }

    #[test]
    fn eliminate_tunnels_matches_meshlib_torus_delete_and_fill_counts() {
        let (vertices, faces) = torus(24, 8);

        let result = eliminate_tunnels(&vertices, &faces).unwrap();
        let diagnostics = tunnel_diagnostics(&result.vertices, &result.faces).unwrap();

        assert_eq!(result.vertices.len(), 192);
        assert_eq!(result.faces.len(), 380);
        assert_eq!(result.report.input_face_count, 384);
        assert_eq!(result.report.detected_tunnel_face_count, 48);
        assert_eq!(result.report.removed_face_count, 48);
        assert_eq!(result.report.filled_holes, 2);
        assert_eq!(result.report.added_faces, 44);
        assert_eq!(result.report.output_face_count, 380);
        assert_eq!(result.report.output_boundary_edge_count, 0);
        assert_eq!(result.report.output_tunnel_count, 0);
        assert_eq!(diagnostics.genus, Some(0));
        assert_eq!(diagnostics.tunnel_count, 0);
        assert!(diagnostics.closed);
    }

    fn torus(radial_segments: usize, tube_segments: usize) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
        let major_radius = 9.0;
        let minor_radius = 1.2;
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        for i in 0..radial_segments {
            let theta = std::f64::consts::TAU * i as f64 / radial_segments as f64;
            let radial = [theta.cos(), 0.0, theta.sin()];
            let center = [
                radial[0] * major_radius,
                radial[1] * major_radius,
                radial[2] * major_radius,
            ];
            for j in 0..tube_segments {
                let phi = std::f64::consts::TAU * j as f64 / tube_segments as f64;
                vertices.push([
                    center[0] + radial[0] * minor_radius * phi.cos(),
                    minor_radius * phi.sin(),
                    center[2] + radial[2] * minor_radius * phi.cos(),
                ]);
            }
        }
        for i in 0..radial_segments {
            let ni = (i + 1) % radial_segments;
            for j in 0..tube_segments {
                let nj = (j + 1) % tube_segments;
                let a = i * tube_segments + j;
                let b = ni * tube_segments + j;
                let c = ni * tube_segments + nj;
                let d = i * tube_segments + nj;
                faces.push([a as i64, b as i64, c as i64]);
                faces.push([a as i64, c as i64, d as i64]);
            }
        }
        (vertices, faces)
    }
}
