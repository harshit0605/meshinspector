use super::TunnelTopology;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

impl TunnelTopology {
    #[allow(dead_code)]
    pub(super) fn find_smallest_metric_co_loop(&self, loop_edges: &[usize]) -> Option<Vec<usize>> {
        if !self.is_edge_loop(loop_edges) {
            return None;
        }
        let loop_info = LoopMetricInfo::new(self, loop_edges)?;
        let mut to_left = BTreeSet::new();
        for (index, edge) in loop_edges.iter().enumerate() {
            let previous = if index > 0 {
                loop_edges[index - 1]
            } else {
                *loop_edges.last()?
            };
            let stop = self.half_edges[previous].twin?;
            for ring_edge in self.org_ring_after(*edge) {
                if ring_edge == stop {
                    break;
                }
                to_left.insert(ring_edge);
            }
        }

        let vertex_count = self.outgoing_edges.len();
        let mut distances = vec![f64::INFINITY; vertex_count];
        let mut roots = vec![None; vertex_count];
        let mut back_edges: Vec<Option<usize>> = vec![None; vertex_count];
        let mut next_steps = BinaryHeap::new();
        let mut sequence = 0_usize;
        for edge in loop_edges {
            let vertex = self.half_edges[*edge].org;
            distances[vertex] = 0.0;
            roots[vertex] = Some(vertex);
            next_steps.push(PathCandidate {
                vertex,
                metric: 0.0,
                sequence,
            });
            sequence += 1;
        }

        let mut best_metric = f64::INFINITY;
        let mut best_first_edge = None;

        while next_steps
            .peek()
            .is_some_and(|candidate| candidate.metric < best_metric)
        {
            let Some(candidate) = next_steps.pop() else {
                break;
            };
            let vertex = candidate.vertex;
            if distances[vertex] < candidate.metric {
                continue;
            }
            let ring_start = back_edges[vertex]
                .and_then(|edge| self.half_edges[edge].twin)
                .or(self.edge_with_org[vertex]);
            let ring_edges = ring_start
                .map(|edge| self.org_ring(edge))
                .unwrap_or_default();
            for edge in ring_edges {
                if to_left.contains(&edge) {
                    continue;
                }
                let Some(sym) = self.half_edges[edge].twin else {
                    continue;
                };
                let metric = self.edge_length(edge);
                let dest = self.half_edges[edge].dest;
                if to_left.contains(&sym) {
                    let Some(loop_origin) = roots[dest] else {
                        continue;
                    };
                    let along_loop = loop_info.distance(loop_origin, dest)?;
                    let candidate_metric = distances[vertex] + metric + along_loop;
                    if candidate_metric < best_metric {
                        best_metric = candidate_metric;
                        best_first_edge = Some(sym);
                    }
                }

                let next_distance = distances[vertex] + metric;
                if next_distance < distances[dest] {
                    distances[dest] = next_distance;
                    roots[dest] = roots[vertex];
                    back_edges[dest] = Some(edge);
                    next_steps.push(PathCandidate {
                        vertex: dest,
                        metric: next_distance,
                        sequence,
                    });
                    sequence += 1;
                }
            }
        }

        let best_first_edge = best_first_edge?;
        let loop_destination = self.half_edges[best_first_edge].org;
        let mut result = vec![best_first_edge];
        let mut current = self.half_edges[best_first_edge].dest;
        let loop_origin = roots[current]?;
        while current != loop_origin {
            let back_edge = back_edges[current]?;
            result.push(self.half_edges[back_edge].twin?);
            current = self.half_edges[back_edge].org;
        }
        loop_info.append_path(loop_origin, loop_destination, &mut result)?;
        self.is_edge_loop(&result).then_some(result)
    }

    fn org_ring_after(&self, start: usize) -> Vec<usize> {
        self.org_ring(start).into_iter().skip(1).collect()
    }
}

#[derive(Debug, Clone)]
struct LoopVertexInfo {
    index: usize,
    len_from_start: f64,
}

#[derive(Debug, Clone)]
struct LoopMetricInfo<'a> {
    topology: &'a TunnelTopology,
    loop_edges: &'a [usize],
    by_vertex: BTreeMap<usize, LoopVertexInfo>,
    total_length: f64,
}

impl<'a> LoopMetricInfo<'a> {
    fn new(topology: &'a TunnelTopology, loop_edges: &'a [usize]) -> Option<Self> {
        let mut by_vertex = BTreeMap::new();
        let mut total_length = 0.0;
        for (index, edge) in loop_edges.iter().enumerate() {
            let vertex = topology.half_edges[*edge].org;
            if by_vertex
                .insert(
                    vertex,
                    LoopVertexInfo {
                        index,
                        len_from_start: total_length,
                    },
                )
                .is_some()
            {
                return None;
            }
            total_length += topology.edge_length(*edge);
        }
        Some(Self {
            topology,
            loop_edges,
            by_vertex,
            total_length,
        })
    }

    fn distance(&self, first_vertex: usize, second_vertex: usize) -> Option<f64> {
        self.path(first_vertex, second_vertex, None)
    }

    fn append_path(
        &self,
        first_vertex: usize,
        second_vertex: usize,
        output: &mut Vec<usize>,
    ) -> Option<f64> {
        self.path(first_vertex, second_vertex, Some(output))
    }

    fn path(
        &self,
        first_vertex: usize,
        second_vertex: usize,
        mut output: Option<&mut Vec<usize>>,
    ) -> Option<f64> {
        if first_vertex == second_vertex {
            return Some(0.0);
        }
        let first = self.by_vertex.get(&first_vertex)?;
        let second = self.by_vertex.get(&second_vertex)?;
        if first.index == second.index {
            return None;
        }

        let mut forward = second.index > first.index;
        let mut distance = if forward {
            second.len_from_start - first.len_from_start
        } else {
            first.len_from_start - second.len_from_start
        };
        if 2.0 * distance > self.total_length {
            forward = !forward;
            distance = self.total_length - distance;
        }

        if let Some(output) = output.as_deref_mut() {
            if forward {
                self.append_forward(first.index, second.index, output);
            } else {
                self.append_backward(first.index, second.index, output);
            }
        }
        Some(distance)
    }

    fn append_forward(&self, first_index: usize, second_index: usize, output: &mut Vec<usize>) {
        if second_index > first_index {
            output.extend(self.loop_edges[first_index..second_index].iter().copied());
        } else {
            output.extend(self.loop_edges[first_index..].iter().copied());
            output.extend(self.loop_edges[..second_index].iter().copied());
        }
    }

    fn append_backward(&self, first_index: usize, second_index: usize, output: &mut Vec<usize>) {
        if first_index > second_index {
            for index in ((second_index + 1)..=first_index).rev() {
                if let Some(twin) = self.topology.half_edges[self.loop_edges[index - 1]].twin {
                    output.push(twin);
                }
            }
        } else {
            for index in (1..=first_index).rev() {
                if let Some(twin) = self.topology.half_edges[self.loop_edges[index - 1]].twin {
                    output.push(twin);
                }
            }
            for index in ((second_index + 1)..=self.loop_edges.len()).rev() {
                if let Some(twin) = self.topology.half_edges[self.loop_edges[index - 1]].twin {
                    output.push(twin);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PathCandidate {
    vertex: usize,
    metric: f64,
    sequence: usize,
}

impl PartialEq for PathCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.vertex == other.vertex
            && self.metric.to_bits() == other.metric.to_bits()
            && self.sequence == other.sequence
    }
}

impl Eq for PathCandidate {}

impl PartialOrd for PathCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .metric
            .partial_cmp(&self.metric)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}
