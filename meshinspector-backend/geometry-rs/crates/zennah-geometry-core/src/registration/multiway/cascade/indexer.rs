use std::cmp::Ordering;

use nalgebra::Vector3;

#[derive(Clone, Copy)]
pub(super) enum CascadeGrouping {
    Sequential,
    AabbTreeBased,
}

pub(super) trait CascadeIndexer {
    fn num_layers(&self) -> usize;
    fn num_elements(&self, layer: usize) -> usize;
    fn element_leaves(&self, layer: usize, element: usize) -> Vec<usize>;
    fn element_nodes(&self, layer: usize, element: usize) -> Vec<usize>;
}

pub(super) fn build_cascade_indexer(
    grouping: CascadeGrouping,
    objects: &[Vec<[f64; 3]>],
    max_group_size: usize,
) -> Result<Box<dyn CascadeIndexer>, String> {
    match grouping {
        CascadeGrouping::Sequential => Ok(Box::new(SequentialCascadeIndexer::new(
            objects.len(),
            max_group_size,
        ))),
        CascadeGrouping::AabbTreeBased => Ok(Box::new(AabbTreeCascadeIndexer::new(
            objects,
            max_group_size,
        )?)),
    }
}

struct SequentialCascadeIndexer {
    object_count: usize,
    max_group_size: usize,
}

impl SequentialCascadeIndexer {
    fn new(object_count: usize, max_group_size: usize) -> Self {
        Self {
            object_count,
            max_group_size,
        }
    }

    fn layer_leaf_count(&self, layer: usize) -> usize {
        let mut count = 1usize;
        for _ in 0..layer {
            count = count.saturating_mul(self.max_group_size);
        }
        count
    }
}

impl CascadeIndexer for SequentialCascadeIndexer {
    fn num_layers(&self) -> usize {
        let mut layers = 1usize;
        let mut elements = self.object_count;
        while elements > 1 {
            elements = elements.div_ceil(self.max_group_size);
            layers += 1;
        }
        layers
    }

    fn num_elements(&self, layer: usize) -> usize {
        self.object_count.div_ceil(self.layer_leaf_count(layer))
    }

    fn element_leaves(&self, layer: usize, element: usize) -> Vec<usize> {
        let leaf_count = self.layer_leaf_count(layer);
        let first = element * leaf_count;
        let last = ((element + 1) * leaf_count).min(self.object_count);
        (first..last).collect()
    }

    fn element_nodes(&self, layer: usize, element: usize) -> Vec<usize> {
        debug_assert!(layer > 0);
        let node_size = self.layer_leaf_count(layer - 1);
        let max_node = self.object_count.div_ceil(node_size);
        let first = element * self.max_group_size;
        let last = ((element + 1) * self.max_group_size).min(max_node);
        (first..last).collect()
    }
}

#[derive(Clone, Copy)]
struct ObjectBoundingBox {
    min: Vector3<f64>,
    max: Vector3<f64>,
}

impl ObjectBoundingBox {
    fn from_points(points: &[[f64; 3]]) -> Result<Self, String> {
        let Some(first) = points.first() else {
            return Err("AABB cascade object point cloud must not be empty".to_string());
        };
        let mut min = Vector3::new(first[0], first[1], first[2]);
        let mut max = min;
        for point in &points[1..] {
            let value = Vector3::new(point[0], point[1], point[2]);
            min.x = min.x.min(value.x);
            min.y = min.y.min(value.y);
            min.z = min.z.min(value.z);
            max.x = max.x.max(value.x);
            max.y = max.y.max(value.y);
            max.z = max.z.max(value.z);
        }
        Ok(Self { min, max })
    }

    fn union(boxes: &[BoxedObject]) -> Self {
        let mut min = boxes[0].bbox.min;
        let mut max = boxes[0].bbox.max;
        for boxed in &boxes[1..] {
            min.x = min.x.min(boxed.bbox.min.x);
            min.y = min.y.min(boxed.bbox.min.y);
            min.z = min.z.min(boxed.bbox.min.z);
            max.x = max.x.max(boxed.bbox.max.x);
            max.y = max.y.max(boxed.bbox.max.y);
            max.z = max.z.max(boxed.bbox.max.z);
        }
        Self { min, max }
    }

    fn sorted_dims(&self) -> [usize; 3] {
        let diag = self.max - self.min;
        let mut dims = [(diag.x, 0usize), (diag.y, 1usize), (diag.z, 2usize)];
        dims.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        });
        [dims[0].1, dims[1].1, dims[2].1]
    }
}

#[derive(Clone)]
struct BoxedObject {
    bbox: ObjectBoundingBox,
    leaf: usize,
}

impl BoxedObject {
    fn center_sum(&self, dim: usize) -> f64 {
        self.bbox.min[dim] + self.bbox.max[dim]
    }
}

struct AabbTreeNode {
    left: Option<usize>,
    right: Option<usize>,
    leaf: Option<usize>,
}

impl AabbTreeNode {
    fn leaf(leaf: usize) -> Self {
        Self {
            left: None,
            right: None,
            leaf: Some(leaf),
        }
    }

    fn branch(left: usize, right: usize) -> Self {
        Self {
            left: Some(left),
            right: Some(right),
            leaf: None,
        }
    }

    fn is_leaf(&self) -> bool {
        self.leaf.is_some()
    }
}

struct ObjectAabbTree {
    nodes: Vec<AabbTreeNode>,
}

impl ObjectAabbTree {
    fn new(objects: &[Vec<[f64; 3]>]) -> Result<Self, String> {
        let mut boxed_objects = objects
            .iter()
            .enumerate()
            .map(|(leaf, points)| {
                Ok(BoxedObject {
                    bbox: ObjectBoundingBox::from_points(points)?,
                    leaf,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut nodes = Vec::new();
        Self::build_node(&mut nodes, &mut boxed_objects);
        Ok(Self { nodes })
    }

    fn build_node(nodes: &mut Vec<AabbTreeNode>, boxes: &mut [BoxedObject]) -> usize {
        let node_index = nodes.len();
        nodes.push(AabbTreeNode::leaf(0));
        if boxes.len() == 1 {
            nodes[node_index] = AabbTreeNode::leaf(boxes[0].leaf);
            return node_index;
        }

        let bbox = ObjectBoundingBox::union(boxes);
        let dims = bbox.sorted_dims();
        boxes.sort_by(|left, right| compare_boxed_objects(left, right, dims));
        let middle = boxes.len() / 2;
        let (left_boxes, right_boxes) = boxes.split_at_mut(middle);
        let left = Self::build_node(nodes, left_boxes);
        let right = Self::build_node(nodes, right_boxes);
        nodes[node_index] = AabbTreeNode::branch(left, right);
        node_index
    }

    fn get_subtrees(&self, min_count: usize) -> Vec<usize> {
        let mut result = vec![0usize];
        while result.len() < min_count {
            let mut next = Vec::with_capacity(result.len() * 2);
            for node_index in &result {
                let node = &self.nodes[*node_index];
                if node.is_leaf() {
                    next.push(*node_index);
                } else {
                    next.push(node.left.expect("AABB branch must have a left child"));
                    next.push(node.right.expect("AABB branch must have a right child"));
                }
            }
            if next.len() == result.len() {
                break;
            }
            result = next;
        }
        result
    }

    fn get_subtree_leaves(&self, root: usize) -> Vec<usize> {
        let mut leaves = Vec::new();
        let mut stack = vec![root];
        while let Some(node_index) = stack.pop() {
            let node = &self.nodes[node_index];
            if let Some(leaf) = node.leaf {
                leaves.push(leaf);
            } else {
                stack.push(node.right.expect("AABB branch must have a right child"));
                stack.push(node.left.expect("AABB branch must have a left child"));
            }
        }
        leaves.sort_unstable();
        leaves
    }
}

fn compare_boxed_objects(left: &BoxedObject, right: &BoxedObject, dims: [usize; 3]) -> Ordering {
    for dim in dims.iter().rev() {
        let ordering = left.center_sum(*dim).total_cmp(&right.center_sum(*dim));
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.leaf.cmp(&right.leaf)
}

struct AabbTreeCascadeIndexer {
    object_count: usize,
    layers: Vec<Vec<Vec<usize>>>,
    nodes_per_layer: Vec<Vec<Vec<usize>>>,
}

impl AabbTreeCascadeIndexer {
    fn new(objects: &[Vec<[f64; 3]>], max_group_size: usize) -> Result<Self, String> {
        let tree = ObjectAabbTree::new(objects)?;
        let mut layers = Vec::new();
        let mut leaf_count = objects.len();
        while leaf_count > max_group_size {
            let mut subtree_count = 1usize;
            while leaf_count > max_group_size {
                leaf_count = leaf_count.div_ceil(2);
                subtree_count <<= 1;
            }
            let layer = tree
                .get_subtrees(subtree_count)
                .into_iter()
                .map(|subtree| tree.get_subtree_leaves(subtree))
                .collect::<Vec<_>>();
            leaf_count = layer.len();
            layers.push(layer);
        }
        let nodes_per_layer = build_aabb_nodes_per_layer(&layers);
        Ok(Self {
            object_count: objects.len(),
            layers,
            nodes_per_layer,
        })
    }
}

impl CascadeIndexer for AabbTreeCascadeIndexer {
    fn num_layers(&self) -> usize {
        self.layers.len() + 1
    }

    fn num_elements(&self, layer: usize) -> usize {
        if layer == 0 {
            self.object_count
        } else if layer - 1 < self.layers.len() {
            self.layers[layer - 1].len()
        } else {
            1
        }
    }

    fn element_leaves(&self, layer: usize, element: usize) -> Vec<usize> {
        if layer == 0 {
            vec![element]
        } else {
            self.layers[layer - 1][element].clone()
        }
    }

    fn element_nodes(&self, layer: usize, element: usize) -> Vec<usize> {
        debug_assert!(layer > 0);
        if self.layers.is_empty() {
            return (0..self.object_count).collect();
        }
        if layer == 1 {
            self.layers[0][element].clone()
        } else if layer - 2 < self.nodes_per_layer.len() {
            self.nodes_per_layer[layer - 2][element].clone()
        } else if let Some(last_layer) = self.layers.last() {
            (0..last_layer.len()).collect()
        } else {
            (0..self.object_count).collect()
        }
    }
}

fn build_aabb_nodes_per_layer(layers: &[Vec<Vec<usize>>]) -> Vec<Vec<Vec<usize>>> {
    if layers.len() < 2 {
        return Vec::new();
    }
    (1..layers.len())
        .map(|layer_index| {
            layers[layer_index]
                .iter()
                .map(|current_leaves| {
                    layers[layer_index - 1]
                        .iter()
                        .enumerate()
                        .filter_map(|(previous_node, previous_leaves)| {
                            if sorted_lists_intersect(current_leaves, previous_leaves) {
                                Some(previous_node)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn sorted_lists_intersect(left: &[usize], right: &[usize]) -> bool {
    let mut left_index = 0usize;
    let mut right_index = 0usize;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Equal => return true,
            Ordering::Less => left_index += 1,
            Ordering::Greater => right_index += 1,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_tree_cascade_indexer_groups_spatially_interleaved_objects() {
        let objects = vec![
            vec![[-5.1, 0.0, 0.0], [-5.0, 0.1, 0.0]],
            vec![[0.2, 0.0, 0.0], [0.3, 0.1, 0.0]],
            vec![[-5.0, 0.0, 0.0], [-4.9, 0.1, 0.0]],
            vec![[0.0, 0.0, 0.0], [0.1, 0.1, 0.0]],
        ];

        let indexer = AabbTreeCascadeIndexer::new(&objects, 2)
            .expect("AABB cascade indexer should accept non-empty point clouds");

        assert_eq!(indexer.num_layers(), 2);
        assert_eq!(indexer.num_elements(1), 2);
        assert_eq!(indexer.element_nodes(1, 0), vec![0, 2]);
        assert_eq!(indexer.element_nodes(1, 1), vec![1, 3]);
        assert_eq!(indexer.element_leaves(1, 0), vec![0, 2]);
        assert_eq!(indexer.element_leaves(1, 1), vec![1, 3]);
        assert_eq!(indexer.element_nodes(2, 0), vec![0, 1]);
    }
}
