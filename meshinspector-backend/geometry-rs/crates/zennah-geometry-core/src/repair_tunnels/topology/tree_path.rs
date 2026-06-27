use super::TunnelTopology;
use std::collections::BTreeSet;

impl TunnelTopology {
    pub(super) fn build_tree_loop(
        &self,
        join_edge: usize,
        primary_tree: &BTreeSet<usize>,
    ) -> Option<Vec<usize>> {
        let distances = self.tree_vertex_distances(primary_tree);
        let start = self.half_edges[join_edge].dest;
        let target = self.half_edges[join_edge].org;
        let mut path = self.tree_path_between(start, target, primary_tree, &distances)?;
        path.push(join_edge);
        self.is_edge_loop(&path).then_some(path)
    }

    fn tree_vertex_distances(&self, primary_tree: &BTreeSet<usize>) -> Vec<Option<usize>> {
        let mut distances = vec![None; self.outgoing_edges.len()];
        for root in 0..self.outgoing_edges.len() {
            if distances[root].is_some() || self.outgoing_edges[root].is_empty() {
                continue;
            }
            distances[root] = Some(0);
            let mut active = vec![root];
            while let Some(vertex) = active.pop() {
                let Some(distance) = distances[vertex] else {
                    continue;
                };
                for edge in self.org_ring_for_vertex(vertex) {
                    if !primary_tree.contains(&self.half_edges[edge].undirected) {
                        continue;
                    }
                    let next_vertex = self.half_edges[edge].dest;
                    if distances[next_vertex].is_some() {
                        continue;
                    }
                    distances[next_vertex] = Some(distance + 1);
                    active.push(next_vertex);
                }
            }
        }
        distances
    }

    fn tree_edge_back(
        &self,
        vertex: usize,
        primary_tree: &BTreeSet<usize>,
        distances: &[Option<usize>],
    ) -> Option<usize> {
        let distance = distances.get(vertex).copied().flatten()?;
        if distance == 0 {
            return None;
        }
        for edge in self.org_ring_for_vertex(vertex) {
            if !primary_tree.contains(&self.half_edges[edge].undirected) {
                continue;
            }
            let next_vertex = self.half_edges[edge].dest;
            if distances.get(next_vertex).copied().flatten()? + 1 == distance {
                return Some(edge);
            }
        }
        None
    }

    fn tree_path_between(
        &self,
        mut start: usize,
        mut finish: usize,
        primary_tree: &BTreeSet<usize>,
        distances: &[Option<usize>],
    ) -> Option<Vec<usize>> {
        let mut start_distance = distances.get(start).copied().flatten()?;
        let mut finish_distance = distances.get(finish).copied().flatten()?;
        let mut start_to_branch = Vec::new();
        let mut finish_to_branch = Vec::new();

        while start_distance > finish_distance {
            let edge = self.tree_edge_back(start, primary_tree, distances)?;
            start_to_branch.push(edge);
            start = self.half_edges[edge].dest;
            start_distance -= 1;
        }
        while finish_distance > start_distance {
            let edge = self.tree_edge_back(finish, primary_tree, distances)?;
            finish_to_branch.push(edge);
            finish = self.half_edges[edge].dest;
            finish_distance -= 1;
        }
        while start != finish {
            if start_distance == 0 {
                return None;
            }
            let start_edge = self.tree_edge_back(start, primary_tree, distances)?;
            start_to_branch.push(start_edge);
            start = self.half_edges[start_edge].dest;
            start_distance -= 1;

            let finish_edge = self.tree_edge_back(finish, primary_tree, distances)?;
            finish_to_branch.push(finish_edge);
            finish = self.half_edges[finish_edge].dest;
        }

        let mut path = start_to_branch;
        path.extend(
            finish_to_branch
                .into_iter()
                .rev()
                .filter_map(|edge| self.half_edges[edge].twin),
        );
        Some(path)
    }

    fn org_ring_for_vertex(&self, vertex: usize) -> Vec<usize> {
        self.edge_with_org
            .get(vertex)
            .copied()
            .flatten()
            .map(|edge| self.org_ring(edge))
            .unwrap_or_default()
    }
}
