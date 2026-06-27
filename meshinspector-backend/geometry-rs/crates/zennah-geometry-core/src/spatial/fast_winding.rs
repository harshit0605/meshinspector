//! Fast hierarchical winding number (Barill et al., "Fast Winding Numbers for
//! Soups and Clouds", SIGGRAPH 2018).
//!
//! The exact winding number at a query point sums a solid angle over EVERY
//! triangle — O(points × faces), which dominates SDF-grid, signed-distance and
//! inside-tests on large meshes. This builds per-BVH-node aggregates (vector
//! area + area-weighted center + bounding radius) once, then evaluates each
//! query by descending the tree: a node far enough from the query (distance >
//! beta · radius) is approximated by a single dipole term; otherwise we recurse,
//! falling back to exact triangle solid angles at the leaves. Cost per query
//! drops to ~O(log faces).
//!
//! Accuracy is automatically highest exactly where it matters: near the surface
//! (winding ≈ 0.5, the inside/outside boundary) the query is close to the local
//! triangles, so the far-field test fails for those nodes and their contribution
//! is computed exactly. Only distant clusters — whose absolute winding
//! contribution is tiny — are approximated.

use super::bvh::FlatBvh;
use super::winding::triangle_solid_angle;
use crate::math::{add, cross, dot, scale, sub};

const FOUR_PI: f64 = 4.0 * std::f64::consts::PI;

/// Below this triangle count the exact brute-force sum is used instead (cheap,
/// bit-exact, and keeps small-mesh goldens unchanged). Tuned so production
/// meshes (100k–1M faces) take the fast path while fixtures stay exact.
pub(super) const FAST_WINDING_MIN_FACES: usize = 4096;

/// Barnes-Hut accuracy parameter. A node is treated as far-field when
/// distance(query, node_center) > BETA · node_radius. With only the first-order
/// (dipole) term, 2.5 keeps the field error well under ~0.02 while still pruning
/// the vast majority of distant clusters; near-surface queries recurse to exact
/// regardless, so the inside/outside sign is preserved.
pub(super) const FAST_WINDING_BETA: f64 = 2.5;

pub(super) struct WindingTree<'a> {
    bvh: &'a FlatBvh,
    triangles: &'a [[[f64; 3]; 3]],
    /// Σ vector-area (½ edge×edge) over the node's subtree.
    vector_area: Vec<[f64; 3]>,
    /// Area-weighted centroid of the node's subtree.
    center: Vec<[f64; 3]>,
    /// Max distance from `center` to any corner of the node's AABB (so it bounds
    /// every triangle point in the subtree).
    radius: Vec<f64>,
    beta_sq: f64,
}

impl<'a> WindingTree<'a> {
    pub(super) fn build(triangles: &'a [[[f64; 3]; 3]], bvh: &'a FlatBvh, beta: f64) -> Self {
        let n = bvh.nodes.len();
        let mut vector_area = vec![[0.0; 3]; n];
        let mut center = vec![[0.0; 3]; n];
        let mut radius = vec![0.0; n];
        let mut total_area = vec![0.0; n];

        // Children are always pushed after their parent (pre-order), so a child's
        // node index exceeds its parent's. Reverse order therefore visits every
        // child before its parent — a single O(nodes) bottom-up pass.
        for i in (0..n).rev() {
            let node = &bvh.nodes[i];
            let mut va = [0.0; 3];
            let mut area_sum = 0.0;
            let mut weighted_centroid = [0.0; 3];

            if node.is_leaf() {
                let faces = &bvh.face_indices[node.face_start..node.face_start + node.face_count];
                for &face in faces {
                    let tri = triangles[face];
                    // vector area = ½ (v1 - v0) × (v2 - v0); |vector area| = area
                    let area_vec = scale(cross(sub(tri[1], tri[0]), sub(tri[2], tri[0])), 0.5);
                    let area = norm3(area_vec);
                    let centroid = [
                        (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0,
                        (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0,
                        (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0,
                    ];
                    va = add(va, area_vec);
                    area_sum += area;
                    weighted_centroid = add(weighted_centroid, scale(centroid, area));
                }
            } else {
                for child in [node.first_child, node.second_child].into_iter().flatten() {
                    va = add(va, vector_area[child]);
                    area_sum += total_area[child];
                    weighted_centroid =
                        add(weighted_centroid, scale(center[child], total_area[child]));
                }
            }

            vector_area[i] = va;
            total_area[i] = area_sum;
            center[i] = if area_sum > 0.0 {
                scale(weighted_centroid, 1.0 / area_sum)
            } else {
                // Degenerate (zero-area) subtree: vector_area is 0 so the dipole
                // vanishes regardless; use the AABB center for the radius bound.
                [
                    0.5 * (node.bbox_min[0] + node.bbox_max[0]),
                    0.5 * (node.bbox_min[1] + node.bbox_max[1]),
                    0.5 * (node.bbox_min[2] + node.bbox_max[2]),
                ]
            };
            radius[i] = aabb_max_corner_distance(center[i], node.bbox_min, node.bbox_max);
        }

        WindingTree {
            bvh,
            triangles,
            vector_area,
            center,
            radius,
            beta_sq: beta * beta,
        }
    }

    /// Winding number at `query` (≈ 1 inside a closed outward-oriented mesh, ≈ 0
    /// outside), matching `winding_numbers` / `triangle_solid_angle` convention.
    pub(super) fn winding_at(&self, query: [f64; 3]) -> f64 {
        if self.bvh.nodes.is_empty() {
            return 0.0;
        }
        // Fixed-size traversal stack — zero allocation per query (called for
        // millions of points). BVH height is O(log faces); 128 is far above any
        // realistic depth, and we guard the push.
        let mut stack = [0usize; 128];
        let mut top = 1usize;
        stack[0] = 0;
        let mut omega = 0.0; // accumulated solid angle (divided by 4π at the end)

        while top > 0 {
            top -= 1;
            let i = stack[top];
            let r = sub(self.center[i], query);
            let dist_sq = dot(r, r);

            // Far-field: distance > beta · radius  ⇔  dist² > beta² · radius²
            if dist_sq > self.beta_sq * self.radius[i] * self.radius[i] {
                let dist = dist_sq.sqrt();
                // dipole: Ω ≈ (center - query) · vector_area / |center - query|³
                omega += dot(r, self.vector_area[i]) / (dist_sq * dist);
                continue;
            }

            let node = &self.bvh.nodes[i];
            if node.is_leaf() {
                let faces = &self.bvh.face_indices
                    [node.face_start..node.face_start + node.face_count];
                for &face in faces {
                    omega += triangle_solid_angle(query, self.triangles[face]);
                }
                continue;
            }

            for child in [node.first_child, node.second_child].into_iter().flatten() {
                if top < stack.len() {
                    stack[top] = child;
                    top += 1;
                }
            }
        }

        omega / FOUR_PI
    }
}

/// Per-point winding evaluator shared across the parallel point loop: the fast
/// hierarchical tree at/above [`FAST_WINDING_MIN_FACES`], else the exact
/// brute-force sum (so small meshes and fixtures stay bit-identical).
pub(super) enum WindingEvaluator<'a> {
    Exact(&'a [[[f64; 3]; 3]]),
    Fast(WindingTree<'a>),
}

impl<'a> WindingEvaluator<'a> {
    pub(super) fn new(triangles: &'a [[[f64; 3]; 3]], bvh: &'a FlatBvh) -> Self {
        if triangles.len() >= FAST_WINDING_MIN_FACES {
            WindingEvaluator::Fast(WindingTree::build(triangles, bvh, FAST_WINDING_BETA))
        } else {
            WindingEvaluator::Exact(triangles)
        }
    }

    pub(super) fn winding_at(&self, query: [f64; 3]) -> f64 {
        match self {
            WindingEvaluator::Exact(triangles) => {
                triangles
                    .iter()
                    .map(|triangle| triangle_solid_angle(query, *triangle))
                    .sum::<f64>()
                    / FOUR_PI
            }
            WindingEvaluator::Fast(tree) => tree.winding_at(query),
        }
    }
}

fn norm3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn aabb_max_corner_distance(center: [f64; 3], bbox_min: [f64; 3], bbox_max: [f64; 3]) -> f64 {
    let mut sum = 0.0;
    for axis in 0..3 {
        let delta = (center[axis] - bbox_min[axis])
            .abs()
            .max((center[axis] - bbox_max[axis]).abs());
        sum += delta * delta;
    }
    sum.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::bvh::build_flat_bvh;

    /// Tessellated UV sphere (outward-oriented), enough triangles to build a real
    /// multi-level BVH and exercise the far-field pruning.
    fn uv_sphere(stacks: usize, slices: usize, radius: f64) -> Vec<[[f64; 3]; 3]> {
        let mut verts = Vec::new();
        for i in 0..=stacks {
            let phi = std::f64::consts::PI * i as f64 / stacks as f64;
            for j in 0..=slices {
                let theta = 2.0 * std::f64::consts::PI * j as f64 / slices as f64;
                verts.push([
                    radius * phi.sin() * theta.cos(),
                    radius * phi.cos(),
                    radius * phi.sin() * theta.sin(),
                ]);
            }
        }
        let idx = |i: usize, j: usize| i * (slices + 1) + j;
        let mut tris = Vec::new();
        for i in 0..stacks {
            for j in 0..slices {
                let a = verts[idx(i, j)];
                let b = verts[idx(i + 1, j)];
                let c = verts[idx(i + 1, j + 1)];
                let d = verts[idx(i, j + 1)];
                // outward orientation
                tris.push([a, b, c]);
                tris.push([a, c, d]);
            }
        }
        tris
    }

    fn exact_winding(query: [f64; 3], tris: &[[[f64; 3]; 3]]) -> f64 {
        tris.iter()
            .map(|t| triangle_solid_angle(query, *t))
            .sum::<f64>()
            / FOUR_PI
    }

    #[test]
    fn fast_winding_matches_exact_on_sphere() {
        let tris = uv_sphere(40, 40, 1.0); // 3200 triangles, real BVH depth
        let bvh = build_flat_bvh(&tris, 16);
        let tree = WindingTree::build(&tris, &bvh, FAST_WINDING_BETA);

        // Sample points spanning inside, near-surface, and outside.
        let mut max_err: f64 = 0.0;
        let mut sign_mismatches = 0;
        let mut checked = 0;
        for gx in -3..=3 {
            for gy in -3..=3 {
                for gz in -3..=3 {
                    let q = [gx as f64 * 0.45, gy as f64 * 0.45, gz as f64 * 0.45];
                    let exact = exact_winding(q, &tris);
                    let fast = tree.winding_at(q);
                    max_err = max_err.max((exact - fast).abs());
                    // Inside/outside sign agreement at the 0.5 threshold, except
                    // for points right on the surface (|exact - 0.5| tiny).
                    if (exact - 0.5).abs() > 0.1 {
                        let inside_exact = exact >= 0.5;
                        let inside_fast = fast >= 0.5;
                        if inside_exact != inside_fast {
                            sign_mismatches += 1;
                        }
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked > 300);
        assert_eq!(sign_mismatches, 0, "inside/outside classification must agree");
        // Dipole + beta=2 keeps the field error small; tighten if this regresses.
        assert!(max_err < 0.02, "max winding error {max_err} too large");
    }
}
