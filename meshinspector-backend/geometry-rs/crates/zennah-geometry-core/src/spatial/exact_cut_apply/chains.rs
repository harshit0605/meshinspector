use super::super::exact_cut::ExactCutPreplan;
use super::helpers::{face_point_position, FacePointPosition};
use super::polygon::ear_clip_planar_polygon;
use crate::math::{cross, sub};
use std::collections::{HashMap, HashSet};

impl super::CutMeshBuilder {
    /// Split a triangle the contour crosses as one or more open chains, each
    /// running from a boundary point through strictly-interior points to a second
    /// boundary point. A fine operand's cross-section makes these chains on a
    /// coarse operand's large face; several non-crossing chains can share one
    /// face (e.g. concentric contour loops crossing the face's diagonal) — the
    /// case the other strategies (all-interior cycle, boundary-only segments,
    /// single-vertex spokes) do not cover, which left the coarse face unsplit and
    /// forced the boolean onto the downstream planar cap. The face is subdivided
    /// by iteratively splitting along each chain, then each region is ear
    /// clipped. Boolean intersection contours are non-self-intersecting, so the
    /// chains never cross. Runs only as a fallback after the existing strategies,
    /// so cases they already handle keep their exact output (parity preserved).
    pub(super) fn try_split_boundary_chain(
        &mut self,
        face: [usize; 3],
        face_index: usize,
        segment_indices: &[usize],
        preplan: &ExactCutPreplan,
    ) -> Option<()> {
        // Build cut-point adjacency for this face's segments; every chain is a
        // path between two degree-one endpoints.
        let mut point_segments: HashMap<usize, Vec<usize>> = HashMap::new();
        for &segment_index in segment_indices {
            let segment = &preplan.path_segments[segment_index];
            point_segments
                .entry(segment.from_point)
                .or_default()
                .push(segment_index);
            point_segments
                .entry(segment.to_point)
                .or_default()
                .push(segment_index);
        }
        let endpoints: Vec<usize> = point_segments
            .iter()
            .filter_map(|(point, segments)| (segments.len() == 1).then_some(*point))
            .collect();
        if endpoints.is_empty() {
            return None;
        }

        // Walk every chain once. A segment left unvisited afterwards means an
        // interior cycle is present, which this strategy leaves to the fallback.
        let mut visited: HashSet<usize> = HashSet::new();
        let mut chains: Vec<Vec<usize>> = Vec::new();
        for &start in &endpoints {
            if point_segments[&start]
                .iter()
                .all(|segment| visited.contains(segment))
            {
                continue;
            }
            let mut chain = vec![start];
            let mut current = start;
            while let Some(next_segment) = point_segments
                .get(&current)
                .and_then(|segments| segments.iter().copied().find(|s| !visited.contains(s)))
            {
                visited.insert(next_segment);
                let segment = &preplan.path_segments[next_segment];
                current = if segment.from_point == current {
                    segment.to_point
                } else {
                    segment.from_point
                };
                chain.push(current);
            }
            chains.push(chain);
        }
        if visited.len() != segment_indices.len() {
            return None;
        }

        // Resolve each chain: two boundary ends, strictly-interior middles. The
        // chain endpoints (plus the corners) seed the triangle's boundary loop.
        let mut boundary_nodes: Vec<(f64, usize)> =
            vec![(0.0, face[0]), (1.0, face[1]), (2.0, face[2])];
        let mut chain_vertices: Vec<Vec<usize>> = Vec::with_capacity(chains.len());
        for chain in &chains {
            if chain.len() < 2 {
                return None;
            }
            let last = chain.len() - 1;
            let mut vertices = Vec::with_capacity(chain.len());
            for (index, &point) in chain.iter().enumerate() {
                let cut_point = &preplan.cut_points[point];
                let position = face_point_position(
                    face,
                    face_index,
                    cut_point.primitive,
                    cut_point.coordinate,
                    &self.vertices,
                )?;
                if index == 0 || index == last {
                    match position {
                        FacePointPosition::Boundary(value) => {
                            boundary_nodes.push((value, cut_point.vertex_index))
                        }
                        FacePointPosition::Interior => return None,
                    }
                } else if !matches!(position, FacePointPosition::Interior) {
                    return None;
                }
                vertices.push(cut_point.vertex_index);
            }
            if vertices[0] == vertices[last] {
                return None;
            }
            chain_vertices.push(vertices);
        }

        // Triangle boundary loop with chain endpoints inserted in perimeter order
        // (corners at 0/1/2, edge crossings in between), deduplicating coincidents.
        boundary_nodes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut boundary_loop: Vec<usize> = Vec::with_capacity(boundary_nodes.len());
        for (_, vertex) in boundary_nodes {
            if boundary_loop.last().copied() != Some(vertex) {
                boundary_loop.push(vertex);
            }
        }
        if boundary_loop.len() > 1 && boundary_loop.first() == boundary_loop.last() {
            boundary_loop.pop();
        }
        if boundary_loop.len() < 3 {
            return None;
        }

        // Subdivide by iteratively splitting along each chain; the contour is
        // non-self-intersecting, so each chain lies on one sub-polygon's loop.
        let mut polygons: Vec<Vec<usize>> = vec![boundary_loop];
        for chain in &chain_vertices {
            if !split_polygon_by_chain(&mut polygons, chain) {
                return None;
            }
        }

        // Triangulate each region; bail to the cap fallback (leaving self
        // unmutated) if any region cannot be ear clipped.
        let face_normal = cross(
            sub(self.vertices[face[1]], self.vertices[face[0]]),
            sub(self.vertices[face[2]], self.vertices[face[0]]),
        );
        let mut output_faces: Vec<[usize; 3]> = Vec::new();
        for polygon in &polygons {
            output_faces.extend(ear_clip_planar_polygon(
                polygon,
                &self.vertices,
                face_normal,
                self.epsilon,
            )?);
        }

        for chain in &chain_vertices {
            for window in chain.windows(2) {
                if window[0] != window[1] {
                    self.push_cut_edge([window[0], window[1]]);
                }
            }
        }
        for triangle in output_faces {
            self.push_face(triangle, face_index);
        }
        Some(())
    }
}

/// Split each polygon carrying a chain's two boundary endpoints into the two
/// regions on either side of the chain. Returns false if no current polygon
/// holds both endpoints (e.g. crossing chains), letting the caller bail safely.
fn split_polygon_by_chain(polygons: &mut Vec<Vec<usize>>, chain: &[usize]) -> bool {
    let start = chain[0];
    let end = chain[chain.len() - 1];
    let interior = &chain[1..chain.len() - 1];
    for index in 0..polygons.len() {
        let polygon = &polygons[index];
        let (Some(start_pos), Some(end_pos)) = (
            polygon.iter().position(|vertex| *vertex == start),
            polygon.iter().position(|vertex| *vertex == end),
        ) else {
            continue;
        };
        let mut region_a = cyclic_arc(polygon, start_pos, end_pos);
        region_a.extend(interior.iter().rev().copied());
        let mut region_b = cyclic_arc(polygon, end_pos, start_pos);
        region_b.extend(interior.iter().copied());
        polygons.remove(index);
        polygons.push(region_a);
        polygons.push(region_b);
        return true;
    }
    false
}

/// Polygon vertices walked forward cyclically from `from` to `to` inclusive.
fn cyclic_arc(polygon: &[usize], from: usize, to: usize) -> Vec<usize> {
    let mut arc = Vec::new();
    let mut index = from;
    loop {
        arc.push(polygon[index]);
        if index == to {
            break;
        }
        index = (index + 1) % polygon.len();
    }
    arc
}
