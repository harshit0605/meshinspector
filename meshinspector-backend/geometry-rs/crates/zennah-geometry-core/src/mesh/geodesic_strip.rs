use crate::math::distance_sq;
use crate::types::GeometryError;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshPlanarTriangleStripPath {
    pub crossing_positions: Vec<f64>,
    pub crossing_points: Vec<[f64; 2]>,
    pub points: Vec<[f64; 2]>,
    pub segment_lengths: Vec<f64>,
    pub length_mm: f64,
    pub meshlib_reference: &'static str,
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct StripEdge {
    left: usize,
    right: usize,
}

#[derive(Debug, Default)]
struct PlanarTriangleStrip {
    points: Vec<[f64; 2]>,
    previous: Vec<Option<usize>>,
    next: Vec<Option<usize>>,
    edges: Vec<StripEdge>,
    apex: usize,
    left_after_apex: Option<usize>,
    right_after_apex: Option<usize>,
}

pub fn mesh_planar_triangle_strip_path(
    start: [f64; 2],
    portals: &[[[f64; 2]; 2]],
    end: [f64; 2],
) -> Result<MeshPlanarTriangleStripPath, GeometryError> {
    if portals.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "portals",
            value: "requires_at_least_one_portal".to_string(),
        });
    }
    validate_point("start", start)?;
    validate_point("end", end)?;
    for portal in portals {
        validate_point("portal_left", portal[0])?;
        validate_point("portal_right", portal[1])?;
        if same_point(portal[0], portal[1]) {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "portals",
                value: "portal_endpoints_must_be_distinct".to_string(),
            });
        }
    }

    let mut strip = PlanarTriangleStrip::default();
    strip.reset(start, portals[0][0], portals[0][1]);
    let mut previous_portal = portals[0];
    for portal in portals.iter().skip(1) {
        if same_point(previous_portal[1], portal[1]) {
            strip.next_edge_new_left(portal[0])?;
        } else if same_point(previous_portal[0], portal[0]) {
            strip.next_edge_new_right(portal[1])?;
        } else {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "portals",
                value: "each_next_portal_must_share_exactly_one_side".to_string(),
            });
        }
        previous_portal = *portal;
    }

    let mut crossing_positions = strip.find(end)?;
    crossing_positions.reverse();
    let crossing_points = crossing_positions
        .iter()
        .zip(portals.iter())
        .map(|(position, portal)| interpolate2(portal[0], portal[1], *position))
        .collect::<Vec<_>>();
    let mut points = Vec::with_capacity(crossing_points.len() + 2);
    points.push(start);
    points.extend(crossing_points.iter().copied());
    points.push(end);
    let segment_lengths = points
        .windows(2)
        .map(|window| distance2(window[0], window[1]))
        .collect::<Vec<_>>();
    let length_mm = segment_lengths.iter().sum();
    Ok(MeshPlanarTriangleStripPath {
        crossing_positions,
        crossing_points,
        points,
        segment_lengths,
        length_mm,
        meshlib_reference: "MR::PathInPlanarTriangleStrip / MR::reducePath",
    })
}

impl PlanarTriangleStrip {
    fn reset(&mut self, start: [f64; 2], edge0_left: [f64; 2], edge0_right: [f64; 2]) {
        self.points.clear();
        self.previous.clear();
        self.next.clear();
        self.edges.clear();
        self.apex = 0;

        self.points.push(start);
        self.previous.push(None);
        self.next.push(None);

        let left = self.points.len();
        self.points.push(edge0_left);
        self.previous.push(Some(self.apex));
        self.next.push(None);
        self.left_after_apex = Some(left);

        let right = self.points.len();
        self.points.push(edge0_right);
        self.previous.push(Some(self.apex));
        self.next.push(None);
        self.right_after_apex = Some(right);

        self.edges.push(StripEdge { left, right });
    }

    fn next_edge_new_left(&mut self, pos: [f64; 2]) -> Result<(), GeometryError> {
        let mut prev = self.last_edge()?.left;
        let curr = self.push_point(pos);
        let right = self.last_edge()?.right;
        self.edges.push(StripEdge { left: curr, right });

        while prev != self.apex {
            let before_prev =
                self.previous[prev].ok_or_else(|| invalid_strip("left_chain_broken"))?;
            if self.cross(before_prev, prev, curr) > 0.0 {
                self.previous[curr] = Some(prev);
                self.next[prev] = Some(curr);
                break;
            }
            prev = before_prev;
        }
        if prev == self.apex {
            while let Some(right_after_apex) = self.right_after_apex {
                if self.cross(curr, self.apex, right_after_apex) >= 0.0 {
                    break;
                }
                self.apex = right_after_apex;
                self.right_after_apex = self.next[right_after_apex];
            }
            self.left_after_apex = Some(curr);
            self.previous[curr] = Some(self.apex);
        }
        Ok(())
    }

    fn next_edge_new_right(&mut self, pos: [f64; 2]) -> Result<(), GeometryError> {
        let mut prev = self.last_edge()?.right;
        let curr = self.push_point(pos);
        let left = self.last_edge()?.left;
        self.edges.push(StripEdge { left, right: curr });

        while prev != self.apex {
            let before_prev =
                self.previous[prev].ok_or_else(|| invalid_strip("right_chain_broken"))?;
            if self.cross(before_prev, prev, curr) < 0.0 {
                self.previous[curr] = Some(prev);
                self.next[prev] = Some(curr);
                break;
            }
            prev = before_prev;
        }
        if prev == self.apex {
            while let Some(left_after_apex) = self.left_after_apex {
                if self.cross(curr, self.apex, left_after_apex) <= 0.0 {
                    break;
                }
                self.apex = left_after_apex;
                self.left_after_apex = self.next[left_after_apex];
            }
            self.right_after_apex = Some(curr);
            self.previous[curr] = Some(self.apex);
        }
        Ok(())
    }

    fn find(&mut self, end: [f64; 2]) -> Result<Vec<f64>, GeometryError> {
        self.next_edge_new_left(end)?;
        let mut crossings = Vec::with_capacity(self.edges.len().saturating_sub(1));
        let mut curr = self.last_edge()?.left;
        let mut prev = self.previous[curr].ok_or_else(|| invalid_strip("end_chain_broken"))?;

        for edge_index in (0..self.edges.len().saturating_sub(1)).rev() {
            let edge = self.edges[edge_index];
            if edge.left == prev {
                crossings.push(0.0);
                curr = prev;
                prev = self.previous[prev].ok_or_else(|| invalid_strip("left_previous_missing"))?;
            } else if edge.right == prev {
                crossings.push(1.0);
                curr = prev;
                prev =
                    self.previous[prev].ok_or_else(|| invalid_strip("right_previous_missing"))?;
            } else if edge.left == curr {
                crossings.push(0.0);
            } else if edge.right == curr {
                crossings.push(1.0);
            } else {
                let left_cross = self.cross(prev, edge.left, curr);
                let right_cross = self.cross(prev, edge.right, curr);
                let denominator = left_cross - right_cross;
                let crossing = if denominator != 0.0 {
                    (left_cross / denominator).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                crossings.push(crossing);
            }
        }
        Ok(crossings)
    }

    fn last_edge(&self) -> Result<StripEdge, GeometryError> {
        self.edges
            .last()
            .copied()
            .ok_or_else(|| invalid_strip("empty_strip"))
    }

    fn push_point(&mut self, pos: [f64; 2]) -> usize {
        let curr = self.points.len();
        self.points.push(pos);
        self.previous.push(None);
        self.next.push(None);
        curr
    }

    fn cross(&self, p: usize, q: usize, r: usize) -> f64 {
        cross2(
            sub2(self.points[r], self.points[q]),
            sub2(self.points[p], self.points[q]),
        )
    }
}

fn validate_point(field: &'static str, point: [f64; 2]) -> Result<(), GeometryError> {
    if point.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(GeometryError::InvalidSelectionParameter {
            field,
            value: "coordinates_must_be_finite".to_string(),
        })
    }
}

fn invalid_strip(value: &str) -> GeometryError {
    GeometryError::InvalidSelectionParameter {
        field: "triangle_strip",
        value: value.to_string(),
    }
}

fn same_point(left: [f64; 2], right: [f64; 2]) -> bool {
    distance_sq([left[0], left[1], 0.0], [right[0], right[1], 0.0]) <= 1e-18
}

fn interpolate2(left: [f64; 2], right: [f64; 2], t: f64) -> [f64; 2] {
    [
        left[0] * (1.0 - t) + right[0] * t,
        left[1] * (1.0 - t) + right[1] * t,
    ]
}

fn distance2(left: [f64; 2], right: [f64; 2]) -> f64 {
    distance_sq([left[0], left[1], 0.0], [right[0], right[1], 0.0]).sqrt()
}

fn sub2(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] - right[0], left[1] - right[1]]
}

fn cross2(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[1] - left[1] * right[0]
}
