use super::predicates::aabb_bounds_overlap;

#[derive(Debug, Clone)]
struct AabbNode {
    pub(super) bbox_min: [f64; 3],
    pub(super) bbox_max: [f64; 3],
    pub(super) face_indices: Vec<usize>,
    pub(super) face_count: usize,
    left: Option<Box<AabbNode>>,
    right: Option<Box<AabbNode>>,
}

impl AabbNode {
    fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }
}

#[derive(Debug, Clone)]
pub(super) struct FlatBvhNode {
    pub(super) bbox_min: [f64; 3],
    pub(super) bbox_max: [f64; 3],
    pub(super) first_child: Option<usize>,
    pub(super) second_child: Option<usize>,
    pub(super) face_start: usize,
    pub(super) face_count: usize,
}

impl FlatBvhNode {
    pub(super) fn is_leaf(&self) -> bool {
        self.first_child.is_none() && self.second_child.is_none()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FlatBvh {
    pub(super) nodes: Vec<FlatBvhNode>,
    pub(super) face_indices: Vec<usize>,
}

fn build_aabb_node(
    triangles: &[[[f64; 3]; 3]],
    face_indices: &[usize],
    leaf_size: usize,
) -> AabbNode {
    let (bbox_min, bbox_max) = indexed_triangle_bounds(triangles, face_indices);
    if face_indices.len() <= leaf_size {
        return AabbNode {
            bbox_min,
            bbox_max,
            face_indices: face_indices.to_vec(),
            face_count: face_indices.len(),
            left: None,
            right: None,
        };
    }

    let axis = longest_centroid_axis(triangles, face_indices);
    let mut ordered = face_indices.to_vec();
    ordered.sort_by(|left, right| {
        triangle_centroid_axis(triangles[*left], axis)
            .total_cmp(&triangle_centroid_axis(triangles[*right], axis))
    });

    let midpoint = usize::max(1, ordered.len() / 2);
    let (left_indices, right_indices) = ordered.split_at(midpoint);
    if left_indices.is_empty() || right_indices.is_empty() {
        return AabbNode {
            bbox_min,
            bbox_max,
            face_indices: face_indices.to_vec(),
            face_count: face_indices.len(),
            left: None,
            right: None,
        };
    }

    AabbNode {
        bbox_min,
        bbox_max,
        face_indices: Vec::new(),
        face_count: face_indices.len(),
        left: Some(Box::new(build_aabb_node(
            triangles,
            left_indices,
            leaf_size,
        ))),
        right: Some(Box::new(build_aabb_node(
            triangles,
            right_indices,
            leaf_size,
        ))),
    }
}

fn flatten_aabb_tree(root: &AabbNode) -> FlatBvh {
    let mut bvh = FlatBvh {
        nodes: Vec::with_capacity(root.face_count.saturating_mul(2).max(1)),
        face_indices: Vec::with_capacity(root.face_count),
    };
    push_flat_bvh_node(root, &mut bvh);
    bvh
}

fn push_flat_bvh_node(node: &AabbNode, bvh: &mut FlatBvh) -> usize {
    let node_index = bvh.nodes.len();
    bvh.nodes.push(FlatBvhNode {
        bbox_min: node.bbox_min,
        bbox_max: node.bbox_max,
        first_child: None,
        second_child: None,
        face_start: 0,
        face_count: node.face_count,
    });

    if node.is_leaf() {
        let face_start = bvh.face_indices.len();
        bvh.face_indices.extend_from_slice(&node.face_indices);
        bvh.nodes[node_index].face_start = face_start;
        bvh.nodes[node_index].face_count = node.face_indices.len();
        return node_index;
    }

    if let Some(left) = node.left.as_ref() {
        bvh.nodes[node_index].first_child = Some(push_flat_bvh_node(left, bvh));
    }
    if let Some(right) = node.right.as_ref() {
        bvh.nodes[node_index].second_child = Some(push_flat_bvh_node(right, bvh));
    }
    node_index
}

fn indexed_triangle_bounds(
    triangles: &[[[f64; 3]; 3]],
    face_indices: &[usize],
) -> ([f64; 3], [f64; 3]) {
    let mut bbox_min = [f64::INFINITY; 3];
    let mut bbox_max = [f64::NEG_INFINITY; 3];
    for face_index in face_indices {
        for vertex in triangles[*face_index] {
            for axis in 0..3 {
                bbox_min[axis] = bbox_min[axis].min(vertex[axis]);
                bbox_max[axis] = bbox_max[axis].max(vertex[axis]);
            }
        }
    }
    (bbox_min, bbox_max)
}

fn longest_centroid_axis(triangles: &[[[f64; 3]; 3]], face_indices: &[usize]) -> usize {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for face_index in face_indices {
        let centroid = triangle_centroid(triangles[*face_index]);
        for axis in 0..3 {
            min[axis] = min[axis].min(centroid[axis]);
            max[axis] = max[axis].max(centroid[axis]);
        }
    }
    let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if extent[1] > extent[0] && extent[1] >= extent[2] {
        1
    } else if extent[2] > extent[0] && extent[2] > extent[1] {
        2
    } else {
        0
    }
}

fn triangle_centroid(triangle: [[f64; 3]; 3]) -> [f64; 3] {
    [
        (triangle[0][0] + triangle[1][0] + triangle[2][0]) / 3.0,
        (triangle[0][1] + triangle[1][1] + triangle[2][1]) / 3.0,
        (triangle[0][2] + triangle[1][2] + triangle[2][2]) / 3.0,
    ]
}

fn triangle_centroid_axis(triangle: [[f64; 3]; 3], axis: usize) -> f64 {
    triangle_centroid(triangle)[axis]
}

pub(super) fn overlapping_face_pairs(bvh: &FlatBvh, epsilon: f64) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    if bvh.nodes.is_empty() {
        return pairs;
    }

    let mut stack = vec![(0usize, 0usize, true)];
    while let Some((node_a_index, node_b_index, same_node)) = stack.pop() {
        let node_a = &bvh.nodes[node_a_index];
        let node_b = &bvh.nodes[node_b_index];
        if !aabb_bounds_overlap(
            node_a.bbox_min,
            node_a.bbox_max,
            node_b.bbox_min,
            node_b.bbox_max,
            epsilon,
        ) {
            continue;
        }

        if node_a.is_leaf() && node_b.is_leaf() {
            collect_leaf_overlapping_pairs(bvh, node_a, node_b, same_node, &mut pairs);
            continue;
        }

        if same_node {
            if let Some(left) = node_a.first_child {
                stack.push((left, left, true));
            }
            if let Some(right) = node_a.second_child {
                stack.push((right, right, true));
            }
            if let (Some(left), Some(right)) = (node_a.first_child, node_a.second_child) {
                stack.push((left, right, false));
            }
            continue;
        }

        if node_b.is_leaf() || (!node_a.is_leaf() && node_a.face_count >= node_b.face_count) {
            if let Some(left) = node_a.first_child {
                stack.push((left, node_b_index, false));
            }
            if let Some(right) = node_a.second_child {
                stack.push((right, node_b_index, false));
            }
        } else {
            if let Some(left) = node_b.first_child {
                stack.push((node_a_index, left, false));
            }
            if let Some(right) = node_b.second_child {
                stack.push((node_a_index, right, false));
            }
        }
    }

    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

pub(super) fn overlapping_face_pairs_between(
    left: &FlatBvh,
    right: &FlatBvh,
    epsilon: f64,
) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    if left.nodes.is_empty() || right.nodes.is_empty() {
        return pairs;
    }

    let mut stack = vec![(0usize, 0usize)];
    while let Some((left_index, right_index)) = stack.pop() {
        let left_node = &left.nodes[left_index];
        let right_node = &right.nodes[right_index];
        if !aabb_bounds_overlap(
            left_node.bbox_min,
            left_node.bbox_max,
            right_node.bbox_min,
            right_node.bbox_max,
            epsilon,
        ) {
            continue;
        }

        if left_node.is_leaf() && right_node.is_leaf() {
            collect_cross_leaf_overlapping_pairs(left, right, left_node, right_node, &mut pairs);
            continue;
        }

        if right_node.is_leaf()
            || (!left_node.is_leaf() && left_node.face_count >= right_node.face_count)
        {
            if let Some(child) = left_node.first_child {
                stack.push((child, right_index));
            }
            if let Some(child) = left_node.second_child {
                stack.push((child, right_index));
            }
        } else {
            if let Some(child) = right_node.first_child {
                stack.push((left_index, child));
            }
            if let Some(child) = right_node.second_child {
                stack.push((left_index, child));
            }
        }
    }

    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

fn collect_leaf_overlapping_pairs(
    bvh: &FlatBvh,
    node_a: &FlatBvhNode,
    node_b: &FlatBvhNode,
    same_node: bool,
    pairs: &mut Vec<(usize, usize)>,
) {
    let faces_a = &bvh.face_indices[node_a.face_start..(node_a.face_start + node_a.face_count)];
    if same_node {
        for index_a in 0..faces_a.len() {
            let face_a = faces_a[index_a];
            for face_b in &faces_a[(index_a + 1)..] {
                pairs.push((face_a, *face_b));
            }
        }
        return;
    }

    let faces_b = &bvh.face_indices[node_b.face_start..(node_b.face_start + node_b.face_count)];
    for face_a in faces_a {
        for face_b in faces_b {
            if face_a == face_b {
                continue;
            }
            pairs.push(if face_a < face_b {
                (*face_a, *face_b)
            } else {
                (*face_b, *face_a)
            });
        }
    }
}

fn collect_cross_leaf_overlapping_pairs(
    left: &FlatBvh,
    right: &FlatBvh,
    left_node: &FlatBvhNode,
    right_node: &FlatBvhNode,
    pairs: &mut Vec<(usize, usize)>,
) {
    let left_faces =
        &left.face_indices[left_node.face_start..(left_node.face_start + left_node.face_count)];
    let right_faces =
        &right.face_indices[right_node.face_start..(right_node.face_start + right_node.face_count)];
    for left_face in left_faces {
        for right_face in right_faces {
            pairs.push((*left_face, *right_face));
        }
    }
}

pub(super) fn ray_candidate_faces(
    bvh: &FlatBvh,
    origin: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
) -> Vec<usize> {
    let mut candidates = Vec::new();
    if bvh.nodes.is_empty() {
        return candidates;
    }

    let mut stack = vec![0usize];
    while let Some(node_index) = stack.pop() {
        let node = &bvh.nodes[node_index];
        if !ray_intersects_aabb(
            origin,
            direction,
            node.bbox_min,
            node.bbox_max,
            max_distance,
        ) {
            continue;
        }

        if node.is_leaf() {
            let face_end = node.face_start + node.face_count;
            candidates.extend_from_slice(&bvh.face_indices[node.face_start..face_end]);
            continue;
        }
        if let Some(left) = node.first_child {
            stack.push(left);
        }
        if let Some(right) = node.second_child {
            stack.push(right);
        }
    }
    candidates
}

pub(super) fn closest_candidate_faces(
    bvh: &FlatBvh,
    point: [f64; 3],
    current_best_sq: f64,
) -> Vec<usize> {
    let mut candidates = Vec::new();
    if bvh.nodes.is_empty() {
        return candidates;
    }

    let mut stack = vec![0usize];
    while let Some(node_index) = stack.pop() {
        let node = &bvh.nodes[node_index];
        if point_aabb_distance_sq(point, node.bbox_min, node.bbox_max) > current_best_sq {
            continue;
        }

        if node.is_leaf() {
            let face_end = node.face_start + node.face_count;
            candidates.extend_from_slice(&bvh.face_indices[node.face_start..face_end]);
            continue;
        }

        let mut children: Vec<(usize, f64)> = [node.first_child, node.second_child]
            .into_iter()
            .flatten()
            .map(|child| {
                let child_node = &bvh.nodes[child];
                (
                    child,
                    point_aabb_distance_sq(point, child_node.bbox_min, child_node.bbox_max),
                )
            })
            .collect();
        children.sort_by(|left, right| left.1.total_cmp(&right.1).reverse());
        for (child, distance_sq) in children {
            if distance_sq <= current_best_sq {
                stack.push(child);
            }
        }
    }
    candidates
}

pub(super) fn point_aabb_distance_sq(
    point: [f64; 3],
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
) -> f64 {
    let mut total = 0.0;
    for axis in 0..3 {
        let delta = f64::max(
            f64::max(bbox_min[axis] - point[axis], 0.0),
            point[axis] - bbox_max[axis],
        );
        total += delta * delta;
    }
    total
}

pub(super) fn ray_intersects_aabb(
    origin: [f64; 3],
    direction: [f64; 3],
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
    max_distance: f64,
) -> bool {
    ray_aabb_interval(origin, direction, bbox_min, bbox_max)
        .map(|(tmin, tmax)| tmax >= tmin && tmin <= max_distance && tmax >= 0.0)
        .unwrap_or(false)
}

pub(super) fn ray_aabb_entry_distance(
    origin: [f64; 3],
    direction: [f64; 3],
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
) -> f64 {
    ray_aabb_interval(origin, direction, bbox_min, bbox_max)
        .map(|(tmin, _)| tmin.max(0.0))
        .unwrap_or(f64::INFINITY)
}

fn ray_aabb_interval(
    origin: [f64; 3],
    direction: [f64; 3],
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
) -> Option<(f64, f64)> {
    let mut tmin = 0.0;
    let mut tmax = f64::INFINITY;
    for axis in 0..3 {
        if direction[axis].abs() < 1e-12 {
            if origin[axis] < bbox_min[axis] || origin[axis] > bbox_max[axis] {
                return None;
            }
            continue;
        }
        let inv = 1.0 / direction[axis];
        let mut near = (bbox_min[axis] - origin[axis]) * inv;
        let mut far = (bbox_max[axis] - origin[axis]) * inv;
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }
        if near > tmin {
            tmin = near;
        }
        if far < tmax {
            tmax = far;
        }
        if tmax < tmin {
            return None;
        }
    }
    Some((tmin, tmax))
}

pub(crate) fn build_flat_bvh(triangles: &[[[f64; 3]; 3]], leaf_size: usize) -> FlatBvh {
    let face_indices: Vec<usize> = (0..triangles.len()).collect();
    let root = build_aabb_node(triangles, &face_indices, leaf_size);
    flatten_aabb_tree(&root)
}
