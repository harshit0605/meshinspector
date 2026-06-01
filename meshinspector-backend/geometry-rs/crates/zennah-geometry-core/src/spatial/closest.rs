use super::bvh::{point_aabb_distance_sq, FlatBvh, FlatBvhNode};
use crate::math::{add, distance_sq, dot, scale, sub};

pub(super) struct ClosestPointHit {
    pub(super) point: [f64; 3],
    pub(super) distance_sq: f64,
    pub(super) face_index: i64,
}

pub(super) fn closest_point_with_bvh(
    point: [f64; 3],
    bvh: &FlatBvh,
    triangles: &[[[f64; 3]; 3]],
) -> ClosestPointHit {
    let mut hit = ClosestPointHit {
        point: triangles[0][0],
        distance_sq: f64::INFINITY,
        face_index: -1,
    };
    if bvh.nodes.is_empty() {
        return hit;
    }

    let mut stack = vec![0usize];
    while let Some(node_index) = stack.pop() {
        let node = &bvh.nodes[node_index];
        if point_aabb_distance_sq(point, node.bbox_min, node.bbox_max) > hit.distance_sq {
            continue;
        }

        if node.is_leaf() {
            let face_end = node.face_start + node.face_count;
            for face_index in &bvh.face_indices[node.face_start..face_end] {
                let candidate = closest_point_on_triangle(point, triangles[*face_index]);
                let distance_sq = distance_sq(point, candidate);
                if distance_sq < hit.distance_sq {
                    hit.point = candidate;
                    hit.distance_sq = distance_sq;
                    hit.face_index = *face_index as i64;
                }
            }
            continue;
        }

        push_closest_children(point, bvh, node, hit.distance_sq, &mut stack);
    }
    hit
}

fn push_closest_children(
    point: [f64; 3],
    bvh: &FlatBvh,
    node: &FlatBvhNode,
    best_distance_sq: f64,
    stack: &mut Vec<usize>,
) {
    match (node.first_child, node.second_child) {
        (Some(first), Some(second)) => {
            let first_node = &bvh.nodes[first];
            let second_node = &bvh.nodes[second];
            let first_distance =
                point_aabb_distance_sq(point, first_node.bbox_min, first_node.bbox_max);
            let second_distance =
                point_aabb_distance_sq(point, second_node.bbox_min, second_node.bbox_max);
            if first_distance <= second_distance {
                push_closest_child(second, second_distance, best_distance_sq, stack);
                push_closest_child(first, first_distance, best_distance_sq, stack);
            } else {
                push_closest_child(first, first_distance, best_distance_sq, stack);
                push_closest_child(second, second_distance, best_distance_sq, stack);
            }
        }
        (Some(child), None) | (None, Some(child)) => {
            let child_node = &bvh.nodes[child];
            let distance = point_aabb_distance_sq(point, child_node.bbox_min, child_node.bbox_max);
            push_closest_child(child, distance, best_distance_sq, stack);
        }
        (None, None) => {}
    }
}

fn push_closest_child(
    child_index: usize,
    distance_sq: f64,
    best_distance_sq: f64,
    stack: &mut Vec<usize>,
) {
    if distance_sq <= best_distance_sq {
        stack.push(child_index);
    }
}

pub(super) fn closest_point_on_triangle(point: [f64; 3], triangle: [[f64; 3]; 3]) -> [f64; 3] {
    let [a, b, c] = triangle;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(point, a);

    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = sub(point, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return add(a, scale(ab, v));
    }

    let cp = sub(point, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return add(a, scale(ac, w));
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add(b, scale(sub(c, b), w));
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    add(add(a, scale(ab, v)), scale(ac, w))
}
