#[derive(Debug, Clone)]
pub(super) struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl DisjointSet {
    pub(super) fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            let root = self.find(self.parent[value]);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    pub(super) fn unite(&mut self, first: usize, second: usize) -> bool {
        let mut first_root = self.find(first);
        let mut second_root = self.find(second);
        if first_root == second_root {
            return false;
        }
        if self.rank[first_root] < self.rank[second_root] {
            std::mem::swap(&mut first_root, &mut second_root);
        }
        self.parent[second_root] = first_root;
        if self.rank[first_root] == self.rank[second_root] {
            self.rank[first_root] += 1;
        }
        true
    }
}
