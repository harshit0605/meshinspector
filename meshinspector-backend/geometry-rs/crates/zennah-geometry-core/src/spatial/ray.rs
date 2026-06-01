use super::bvh::{ray_aabb_entry_distance, ray_intersects_aabb, FlatBvh, FlatBvhNode};
use crate::math::{add, cross, dot, scale, sub};
use crate::RayHit;

pub(super) fn first_ray_hit_with_bvh(
    origin: [f64; 3],
    direction: [f64; 3],
    bvh: &FlatBvh,
    triangles: &[[[f64; 3]; 3]],
    epsilon: f64,
    ignored_faces: &[i64],
) -> Option<RayHit> {
    let mut hit = None;
    if bvh.nodes.is_empty() {
        return hit;
    }

    let mut stack = vec![0usize];
    while let Some(node_index) = stack.pop() {
        let node = &bvh.nodes[node_index];
        let best_distance = hit
            .as_ref()
            .map(|current: &RayHit| current.distance)
            .unwrap_or(f64::INFINITY);
        if !ray_intersects_aabb(
            origin,
            direction,
            node.bbox_min,
            node.bbox_max,
            best_distance,
        ) {
            continue;
        }

        if node.is_leaf() {
            let face_end = node.face_start + node.face_count;
            for face_index in &bvh.face_indices[node.face_start..face_end] {
                if ignored_faces.contains(&(*face_index as i64)) {
                    continue;
                }
                let Some(distance) =
                    ray_triangle_distance(origin, direction, triangles[*face_index], epsilon)
                else {
                    continue;
                };
                if hit
                    .as_ref()
                    .map(|current| distance < current.distance)
                    .unwrap_or(true)
                {
                    hit.replace(RayHit {
                        face_index: *face_index,
                        distance,
                        point: add(origin, scale(direction, distance)),
                    });
                }
            }
            continue;
        }

        push_ray_children(origin, direction, bvh, node, best_distance, &mut stack);
    }
    hit
}

pub(super) fn ray_hits_with_bvh(
    origin: [f64; 3],
    direction: [f64; 3],
    bvh: &FlatBvh,
    triangles: &[[[f64; 3]; 3]],
    epsilon: f64,
    ignored_faces: &[i64],
) -> Vec<RayHit> {
    if bvh.nodes.is_empty() {
        return Vec::new();
    }

    let mut hits = Vec::new();
    let mut stack = vec![0usize];
    while let Some(node_index) = stack.pop() {
        let node = &bvh.nodes[node_index];
        if !ray_intersects_aabb(
            origin,
            direction,
            node.bbox_min,
            node.bbox_max,
            f64::INFINITY,
        ) {
            continue;
        }

        if node.is_leaf() {
            let face_end = node.face_start + node.face_count;
            for face_index in &bvh.face_indices[node.face_start..face_end] {
                if ignored_faces.contains(&(*face_index as i64)) {
                    continue;
                }
                let Some(distance) =
                    ray_triangle_distance(origin, direction, triangles[*face_index], epsilon)
                else {
                    continue;
                };
                hits.push(RayHit {
                    face_index: *face_index,
                    distance,
                    point: add(origin, scale(direction, distance)),
                });
            }
            continue;
        }

        push_ray_children(origin, direction, bvh, node, f64::INFINITY, &mut stack);
    }

    hits.sort_by(|left, right| left.distance.total_cmp(&right.distance));
    hits
}

fn push_ray_children(
    origin: [f64; 3],
    direction: [f64; 3],
    bvh: &FlatBvh,
    node: &FlatBvhNode,
    best_distance: f64,
    stack: &mut Vec<usize>,
) {
    match (node.first_child, node.second_child) {
        (Some(first), Some(second)) => {
            let first_entry = flat_ray_child_entry(origin, direction, bvh, first, best_distance);
            let second_entry = flat_ray_child_entry(origin, direction, bvh, second, best_distance);
            if first_entry <= second_entry {
                push_ray_child(second, second_entry, stack);
                push_ray_child(first, first_entry, stack);
            } else {
                push_ray_child(first, first_entry, stack);
                push_ray_child(second, second_entry, stack);
            }
        }
        (Some(child), None) | (None, Some(child)) => {
            let entry = flat_ray_child_entry(origin, direction, bvh, child, best_distance);
            push_ray_child(child, entry, stack);
        }
        (None, None) => {}
    }
}

fn flat_ray_child_entry(
    origin: [f64; 3],
    direction: [f64; 3],
    bvh: &FlatBvh,
    child_index: usize,
    best_distance: f64,
) -> f64 {
    let child = &bvh.nodes[child_index];
    if ray_intersects_aabb(
        origin,
        direction,
        child.bbox_min,
        child.bbox_max,
        best_distance,
    ) {
        ray_aabb_entry_distance(origin, direction, child.bbox_min, child.bbox_max)
    } else {
        f64::INFINITY
    }
}

fn push_ray_child(child_index: usize, entry_distance: f64, stack: &mut Vec<usize>) {
    if entry_distance.is_finite() {
        stack.push(child_index);
    }
}

fn ray_triangle_distance(
    origin: [f64; 3],
    direction: [f64; 3],
    triangle: [[f64; 3]; 3],
    epsilon: f64,
) -> Option<f64> {
    let [a, b, c] = triangle;
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let h = cross(direction, edge2);
    let det = dot(edge1, h);
    if -epsilon < det && det < epsilon {
        return None;
    }

    let inv_det = 1.0 / det;
    let s = sub(origin, a);
    let u = inv_det * dot(s, h);
    if u < -epsilon || u > 1.0 + epsilon {
        return None;
    }

    let q = cross(s, edge1);
    let v = inv_det * dot(direction, q);
    if v < -epsilon || u + v > 1.0 + epsilon {
        return None;
    }

    let distance = inv_det * dot(edge2, q);
    if distance <= epsilon {
        return None;
    }
    Some(distance)
}
