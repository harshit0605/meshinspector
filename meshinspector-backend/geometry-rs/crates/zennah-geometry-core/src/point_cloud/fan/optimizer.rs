use super::{
    add, cross, dot, fan_boundary_neighbor, length, length_sq, normalized, squared_distance, sub,
    FanSortData,
};

#[derive(Debug, Clone, Copy)]
struct QueueElement {
    weight: f64,
    id: usize,
    prev_id: usize,
    next_id: usize,
}

pub(super) fn optimize_fan(
    points: &[[f64; 3]],
    center_index: usize,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
    neighbors: &mut Vec<usize>,
    mut sort_data: FanSortData,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
) -> (Option<usize>, usize) {
    let mut boundary = fan_boundary_neighbor(neighbors, &sort_data.angles, boundary_angle);
    if max_removes == 0 {
        return (boundary, 0);
    }

    let mut valid = vec![true; neighbors.len()];
    let mut current_size = neighbors.len();
    let mut queue = Vec::<QueueElement>::new();
    for index in 0..neighbors.len() {
        if points[neighbors[index]] == points[center_index] {
            valid[index] = false;
            current_size -= 1;
        } else if let Some(element) = fan_queue_element(
            points,
            center_index,
            normals,
            untrusted_indices,
            neighbors,
            &valid,
            &sort_data,
            boundary,
            index,
            crit_angle,
        ) {
            queue.push(element);
        }
    }
    if current_size < 2 {
        neighbors.clear();
        return (None, 0);
    }

    let mut removed_count = 0;
    while !queue.is_empty() {
        let top_index = queue
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.weight.total_cmp(&right.weight))
            .map(|(index, _)| index)
            .expect("queue is not empty");
        let element = queue.swap_remove(top_index);
        if !valid[element.id] || !valid[element.prev_id] || !valid[element.next_id] {
            continue;
        }

        let old_neighbor = neighbors[element.id];
        valid[element.id] = false;
        removed_count += 1;
        current_size -= 1;
        if removed_count >= max_removes {
            break;
        }
        if current_size < 2 {
            neighbors.clear();
            return (None, removed_count);
        }
        if boundary == Some(old_neighbor) {
            boundary = Some(neighbors[element.prev_id]);
        }

        for index in [element.next_id, element.prev_id] {
            if let Some(next_element) = fan_queue_element(
                points,
                center_index,
                normals,
                untrusted_indices,
                neighbors,
                &valid,
                &sort_data,
                boundary,
                index,
                crit_angle,
            ) {
                queue.push(next_element);
            }
        }
    }

    let mut compact_neighbors = Vec::with_capacity(current_size);
    let mut compact_angles = Vec::with_capacity(current_size);
    for ((neighbor, angle), is_valid) in neighbors
        .iter()
        .copied()
        .zip(sort_data.angles.iter().copied())
        .zip(valid)
    {
        if is_valid {
            compact_neighbors.push(neighbor);
            compact_angles.push(angle);
        }
    }
    *neighbors = compact_neighbors;
    sort_data.angles = compact_angles;
    if neighbors.len() < 2 {
        neighbors.clear();
        return (None, removed_count);
    }
    (boundary, removed_count)
}

fn fan_queue_element(
    points: &[[f64; 3]],
    center_index: usize,
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
    neighbors: &[usize],
    valid: &[bool],
    sort_data: &FanSortData,
    boundary: Option<usize>,
    id: usize,
    crit_angle: f64,
) -> Option<QueueElement> {
    if !valid[id] {
        return None;
    }
    let next_id = cycle_next(valid, id);
    let prev_id = cycle_prev(valid, id);
    if boundary == Some(neighbors[id]) {
        return boundary_queue_element(
            points,
            center_index,
            neighbors,
            id,
            prev_id,
            next_id,
            false,
        );
    }
    if boundary == Some(neighbors[prev_id]) {
        return boundary_queue_element(points, center_index, neighbors, id, prev_id, next_id, true);
    }

    let mut angle_diff = sort_data.angles[next_id] - sort_data.angles[prev_id];
    if angle_diff < 0.0 {
        angle_diff += std::f64::consts::TAU;
    }
    if angle_diff > std::f64::consts::PI {
        return None;
    }

    let a = points[center_index];
    let b = points[neighbors[next_id]];
    let c = points[neighbors[id]];
    let d = points[neighbors[prev_id]];
    let ac_length_sq = squared_distance(a, c);
    if (ac_length_sq > squared_distance(b, a) && triangle_aspect_ratio(a, b, c) > 1e3)
        || (ac_length_sq > squared_distance(d, a) && triangle_aspect_ratio(a, c, d) > 1e3)
    {
        return Some(QueueElement {
            weight: f64::MAX,
            id,
            prev_id,
            next_id,
        });
    }

    let flip_possible = if trust_normal_at(normals, untrusted_indices, center_index)
        && trust_normal_at(normals, untrusted_indices, neighbors[id])
        && dot(
            normals.expect("trusted normals exist")[center_index],
            normals.expect("trusted normals exist")[neighbors[id]],
        ) < 0.0
    {
        true
    } else {
        is_unfold_quadrangle_convex(a, b, c, d)
    };
    if !flip_possible {
        return None;
    }

    let mut delone_profit = delone_flip_profit_sq(a, b, c, d);
    if delone_profit == 0.0
        && center_index.min(neighbors[id]) > neighbors[next_id].min(neighbors[prev_id])
    {
        delone_profit = -1.0;
    }
    let angle_profit = tris_angle_profit(a, b, c, d, crit_angle);
    if delone_profit < 0.0 && angle_profit <= 0.0 {
        return None;
    }

    let mut weight = 0.0;
    if delone_profit > 0.0 {
        weight += delone_profit / sort_data.normalizer_sq;
    }
    if angle_profit > 0.0 {
        weight += angle_profit;
    }
    let norm_val = length(sub(c, a));
    if norm_val == 0.0 {
        return Some(QueueElement {
            weight: f64::MAX,
            id,
            prev_id,
            next_id,
        });
    }
    weight += dot(sub(c, a), sort_data.normal).abs() / norm_val;

    if let Some(normals) = normals {
        if trust_normal_at(Some(normals), untrusted_indices, center_index)
            && trust_normal_at(Some(normals), untrusted_indices, neighbors[id])
        {
            let c_normal = normals[neighbors[id]];
            weight += 5.0 * (1.0 - dot(normals[center_index], c_normal));
            let abc_normal = cross(sub(b, a), sub(c, a));
            let acd_normal = cross(sub(c, a), sub(d, a));
            let tri_normal_weight = dot(normalized(add(abc_normal, acd_normal)), c_normal);
            if tri_normal_weight < 0.0 {
                weight = f64::MAX;
            } else {
                weight += 5.0 * (1.0 - tri_normal_weight);
            }
        }
    }

    Some(QueueElement {
        weight,
        id,
        prev_id,
        next_id,
    })
}

fn boundary_queue_element(
    points: &[[f64; 3]],
    center_index: usize,
    neighbors: &[usize],
    id: usize,
    prev_id: usize,
    next_id: usize,
    next_el: bool,
) -> Option<QueueElement> {
    let prev_ind = if next_el { id } else { prev_id };
    let next_ind = if next_el { next_id } else { id };
    let other_id = if next_el { next_id } else { prev_id };
    let center = points[center_index];
    let length_sq = squared_distance(center, points[neighbors[id]]);
    let other_length_sq = squared_distance(center, points[neighbors[other_id]]);
    if length_sq < other_length_sq {
        return None;
    }
    if triangle_aspect_ratio(
        center,
        points[neighbors[prev_ind]],
        points[neighbors[next_ind]],
    ) <= 1e3
    {
        return None;
    }
    Some(QueueElement {
        weight: f64::MAX,
        id,
        prev_id,
        next_id,
    })
}

fn cycle_next(valid: &[bool], mut index: usize) -> usize {
    loop {
        index += 1;
        if index == valid.len() {
            index = 0;
        }
        if valid[index] {
            return index;
        }
    }
}

fn cycle_prev(valid: &[bool], mut index: usize) -> usize {
    loop {
        index = if index == 0 {
            valid.len() - 1
        } else {
            index - 1
        };
        if valid[index] {
            return index;
        }
    }
}

fn trust_normal_at(
    normals: Option<&[[f64; 3]]>,
    untrusted_indices: &[usize],
    index: usize,
) -> bool {
    normals.is_some() && !untrusted_indices.contains(&index)
}

fn triangle_aspect_ratio(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let bc = length(sub(c, b));
    let ca = length(sub(a, c));
    let ab = length(sub(b, a));
    let half_perimeter = (bc + ca + ab) / 2.0;
    let den = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if den <= 0.0 {
        return f64::MAX;
    }
    bc * ca * ab / den
}

fn delone_flip_profit_sq(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let metric_ac = circumcircle_diameter_sq(a, c, d).max(circumcircle_diameter_sq(c, a, b));
    let metric_bd = circumcircle_diameter_sq(b, d, a).max(circumcircle_diameter_sq(d, b, c));
    metric_ac - metric_bd
}

fn circumcircle_diameter_sq(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = squared_distance(b, a);
    let ca = squared_distance(a, c);
    let bc = squared_distance(c, b);
    if ab <= 0.0 {
        return ca;
    }
    if ca <= 0.0 {
        return bc;
    }
    if bc <= 0.0 {
        return ab;
    }
    let f = length_sq(cross(sub(b, a), sub(c, a)));
    if f <= 0.0 {
        return f64::INFINITY;
    }
    ab * ca * bc / f
}

fn tris_angle_profit(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3], crit_angle: f64) -> f64 {
    let ac = sub(c, a);
    let ab = sub(b, a);
    let ad = sub(d, a);
    angle(cross(ab, ac), cross(ac, ad)) - crit_angle
}

fn is_unfold_quadrangle_convex(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> bool {
    let x = shortest_path_in_quadrangle(a, b, c, d);
    x > 0.0 && x < 1.0
}

fn shortest_path_in_quadrangle(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let vec_b = sub(b, a);
    let vec_c = sub(c, a);
    let vec_d = sub(d, a);
    let unfold_b = [length(vec_b), 0.0];
    let unfold_c = unfold_on_plane(vec_b, vec_c, unfold_b, true);
    let unfold_d = unfold_on_plane(vec_c, vec_d, unfold_c, true);
    line_intersection(unfold_c, unfold_b, unfold_d).clamp(0.0, 1.0)
}

fn unfold_on_plane(b: [f64; 3], c: [f64; 3], d: [f64; 2], to_left_from_zero_d: bool) -> [f64; 2] {
    let dot_bc = dot(b, c);
    let cross_bc = length(cross(b, c));
    let dd = dot2(d, d);
    if dd <= 0.0 {
        return [0.0, 0.0];
    }
    let orthogonal = if to_left_from_zero_d {
        [-d[1], d[0]]
    } else {
        [d[1], -d[0]]
    };
    [
        (dot_bc * d[0] + cross_bc * orthogonal[0]) / dd,
        (dot_bc * d[1] + cross_bc * orthogonal[1]) / dd,
    ]
}

fn line_intersection(b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> f64 {
    let c1 = cross2(d, c);
    let c2 = cross2(sub2(c, b), sub2(d, b));
    let cc = c1 + c2;
    if cc == 0.0 {
        return 0.0;
    }
    c1 / cc
}

fn sub2(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] - right[0], left[1] - right[1]]
}

fn dot2(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[0] + left[1] * right[1]
}

fn cross2(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[1] - left[1] * right[0]
}

fn angle(left: [f64; 3], right: [f64; 3]) -> f64 {
    let denominator = length(left) * length(right);
    if denominator <= 0.0 {
        return 0.0;
    }
    (dot(left, right) / denominator).clamp(-1.0, 1.0).acos()
}
