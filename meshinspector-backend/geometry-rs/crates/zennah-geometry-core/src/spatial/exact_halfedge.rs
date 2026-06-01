#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExactHalfEdgeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactHalfEdgeRecord {
    pub next: ExactHalfEdgeId,
    pub prev: ExactHalfEdgeId,
    pub origin: Option<usize>,
    pub left: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactHalfEdgeTopology {
    edges: Vec<ExactHalfEdgeRecord>,
}

impl ExactHalfEdgeTopology {
    pub(crate) fn new() -> Self {
        Self { edges: Vec::new() }
    }

    pub(crate) fn make_edge(
        &mut self,
        origin: Option<usize>,
        sym_origin: Option<usize>,
    ) -> ExactHalfEdgeId {
        let edge = ExactHalfEdgeId(self.edges.len());
        let sym = ExactHalfEdgeId(edge.0 + 1);
        self.edges.push(ExactHalfEdgeRecord {
            next: edge,
            prev: edge,
            origin,
            left: None,
        });
        self.edges.push(ExactHalfEdgeRecord {
            next: sym,
            prev: sym,
            origin: sym_origin,
            left: None,
        });
        edge
    }

    pub(crate) fn splice(
        &mut self,
        a: ExactHalfEdgeId,
        b: ExactHalfEdgeId,
    ) -> Result<(), &'static str> {
        self.validate_edge(a)?;
        self.validate_edge(b)?;
        if a == b {
            return Ok(());
        }

        let a_data = self.edges[a.0];
        let b_data = self.edges[b.0];
        let a_next = a_data.next;
        let b_next = b_data.next;
        self.validate_edge(a_next)?;
        self.validate_edge(b_next)?;

        let was_same_origin_id = a_data.origin == b_data.origin;
        if !was_same_origin_id && a_data.origin.is_some() && b_data.origin.is_some() {
            return Err("cannot splice rings with two different valid origins");
        }

        let was_same_left_id = a_data.left == b_data.left;
        if !was_same_left_id && a_data.left.is_some() && b_data.left.is_some() {
            return Err("cannot splice rings with two different valid left faces");
        }

        if !was_same_origin_id {
            if a_data.origin.is_some() {
                self.set_origin_ring(b, a_data.origin)?;
            } else if b_data.origin.is_some() {
                self.set_origin_ring(a, b_data.origin)?;
            }
        }

        if !was_same_left_id {
            if a_data.left.is_some() {
                self.set_left_ring(b, a_data.left)?;
            } else if b_data.left.is_some() {
                self.set_left_ring(a, b_data.left)?;
            }
        }

        self.edges[a.0].next = b_next;
        self.edges[b.0].next = a_next;
        self.edges[a_next.0].prev = b;
        self.edges[b_next.0].prev = a;

        if was_same_origin_id && b_data.origin.is_some() {
            self.set_origin_ring(b, None)?;
        }

        if was_same_left_id && b_data.left.is_some() {
            self.set_left_ring(b, None)?;
        }

        Ok(())
    }

    pub(crate) fn stitch_contours(
        &mut self,
        first_contour: &[ExactHalfEdgeId],
        second_contour: &[ExactHalfEdgeId],
    ) -> Result<(), &'static str> {
        if first_contour.len() != second_contour.len() {
            return Err("contours must have the same length");
        }

        for (e0, e1) in first_contour
            .iter()
            .copied()
            .zip(second_contour.iter().copied())
        {
            self.validate_stitch_pair(e0, e1)?;

            if self.origin(e0) != self.origin(e1) {
                self.set_origin(e1, None)?;
                let prev_e1 = self.prev(e1);
                self.splice(e0, prev_e1)?;
            }

            let e0_sym = Self::sym(e0);
            let e1_sym = Self::sym(e1);
            if self.origin(e0_sym) != self.origin(e1_sym) {
                self.set_origin(e1_sym, None)?;
                let prev_e0_sym = self.prev(e0_sym);
                self.splice(prev_e0_sym, e1_sym)?;
            }

            if self.next(e0) != e1 || self.next(e1_sym) != e0_sym {
                return Err("stitch pair did not become adjacent");
            }
        }

        for (e0, e1) in first_contour
            .iter()
            .copied()
            .zip(second_contour.iter().copied())
        {
            if self.next(e0) == e1 {
                self.splice(e0, e1)?;
            }
            let e0_sym = Self::sym(e0);
            let e1_sym = Self::sym(e1);
            if self.next(e1_sym) == e0_sym {
                let prev_e1_sym = self.prev(e1_sym);
                self.splice(prev_e1_sym, e1_sym)?;
            }
            if !self.is_lone_edge(e1)? {
                return Err("second contour edge was not deleted");
            }
        }

        Ok(())
    }

    pub(crate) fn next(&self, edge: ExactHalfEdgeId) -> ExactHalfEdgeId {
        self.edges[edge.0].next
    }

    pub(crate) fn prev(&self, edge: ExactHalfEdgeId) -> ExactHalfEdgeId {
        self.edges[edge.0].prev
    }

    pub(crate) fn origin(&self, edge: ExactHalfEdgeId) -> Option<usize> {
        self.edges[edge.0].origin
    }

    pub(crate) fn left(&self, edge: ExactHalfEdgeId) -> Option<usize> {
        self.edges[edge.0].left
    }

    pub(crate) fn right(&self, edge: ExactHalfEdgeId) -> Option<usize> {
        self.left(Self::sym(edge))
    }

    pub(crate) fn edge_ids(&self) -> impl Iterator<Item = ExactHalfEdgeId> + '_ {
        (0..self.edges.len()).map(ExactHalfEdgeId)
    }

    pub(crate) fn set_origin(
        &mut self,
        edge: ExactHalfEdgeId,
        origin: Option<usize>,
    ) -> Result<(), &'static str> {
        self.set_origin_ring(edge, origin)
    }

    pub(crate) fn set_left(
        &mut self,
        edge: ExactHalfEdgeId,
        left: Option<usize>,
    ) -> Result<(), &'static str> {
        self.set_left_ring(edge, left)
    }

    pub(crate) fn set_left_direct(
        &mut self,
        edge: ExactHalfEdgeId,
        left: Option<usize>,
    ) -> Result<(), &'static str> {
        self.validate_edge(edge)?;
        self.edges[edge.0].left = left;
        Ok(())
    }

    pub(crate) fn apply_meshlib_stitched_edge_record_rewrite(
        &mut self,
        target: ExactHalfEdgeId,
        mapped_from_next: ExactHalfEdgeId,
        mapped_from_left: Option<usize>,
        mapped_from_sym_prev: ExactHalfEdgeId,
        patch_reciprocals: bool,
    ) -> Result<(), &'static str> {
        self.validate_edge(target)?;
        self.validate_edge(mapped_from_next)?;
        self.validate_edge(mapped_from_sym_prev)?;
        if self.left(target).is_some() {
            return Err("target contour edge must not have a left face");
        }

        let target_sym = Self::sym(target);
        self.edges[target.0].next = mapped_from_next;
        if patch_reciprocals {
            self.edges[mapped_from_next.0].prev = target;
        }
        self.edges[target.0].left = mapped_from_left;
        self.edges[target_sym.0].prev = mapped_from_sym_prev;
        if patch_reciprocals {
            self.edges[mapped_from_sym_prev.0].next = target_sym;
        }
        Ok(())
    }

    pub(crate) fn apply_meshlib_near_stitch_edge_update(
        &mut self,
        previous: ExactHalfEdgeId,
        next: ExactHalfEdgeId,
    ) -> Result<(), &'static str> {
        self.validate_meshlib_near_stitch_edge_update(previous, next)?;

        self.edges[previous.0].next = next;
        self.edges[next.0].prev = previous;
        Ok(())
    }

    pub(crate) fn validate_meshlib_near_stitch_edge_update(
        &self,
        previous: ExactHalfEdgeId,
        next: ExactHalfEdgeId,
    ) -> Result<(), &'static str> {
        self.validate_edge(previous)?;
        self.validate_edge(next)?;
        if self.origin(previous) != self.origin(next) {
            return Err("near stitch edges must share origin");
        }
        if self.left(previous).is_some() {
            return Err("previous near stitch edge must not have a left face");
        }
        if self.right(next).is_some() {
            return Err("next near stitch edge must not have a right face");
        }

        Ok(())
    }

    pub(crate) fn set_meshlib_translated_record(
        &mut self,
        edge: ExactHalfEdgeId,
        next: ExactHalfEdgeId,
        prev: ExactHalfEdgeId,
        origin: Option<usize>,
        left: Option<usize>,
    ) -> Result<(), &'static str> {
        self.validate_edge(edge)?;
        self.validate_edge(next)?;
        self.validate_edge(prev)?;
        self.edges[edge.0] = ExactHalfEdgeRecord {
            next,
            prev,
            origin,
            left,
        };
        Ok(())
    }

    pub(crate) fn is_lone_edge(&self, edge: ExactHalfEdgeId) -> Result<bool, &'static str> {
        self.validate_edge(edge)?;
        let sym = Self::sym(edge);
        self.validate_edge(sym)?;
        let edge_data = self.edges[edge.0];
        let sym_data = self.edges[sym.0];
        Ok(edge_data.origin.is_none()
            && edge_data.left.is_none()
            && edge_data.next == edge
            && edge_data.prev == edge
            && sym_data.origin.is_none()
            && sym_data.left.is_none()
            && sym_data.next == sym
            && sym_data.prev == sym)
    }

    pub(crate) fn not_lone_undirected_edge_count(&self) -> Result<usize, &'static str> {
        if self.edges.len() % 2 != 0 {
            return Err("half-edge storage has an odd edge count");
        }
        let mut count = 0;
        for edge in (0..self.edges.len()).step_by(2) {
            if !self.is_lone_edge(ExactHalfEdgeId(edge))? {
                count += 1;
            }
        }
        Ok(count)
    }

    pub(crate) fn shares_origin_ring(
        &self,
        a: ExactHalfEdgeId,
        b: ExactHalfEdgeId,
    ) -> Result<bool, &'static str> {
        Ok(self.origin_ring(a)?.contains(&b))
    }

    pub(crate) fn left_ring_origins(
        &self,
        start: ExactHalfEdgeId,
    ) -> Result<Vec<usize>, &'static str> {
        self.left_ring(start)?
            .into_iter()
            .map(|edge| self.origin(edge).ok_or("left ring edge missing origin"))
            .collect()
    }

    pub(crate) fn left_ring_edges(
        &self,
        start: ExactHalfEdgeId,
    ) -> Result<Vec<ExactHalfEdgeId>, &'static str> {
        self.left_ring(start)
    }

    pub(crate) fn validate_meshlib_face_left_ring(
        &self,
        start: ExactHalfEdgeId,
        face: usize,
    ) -> Result<(), &'static str> {
        self.validate_edge(start)?;
        if self.left(start) != Some(face) {
            return Err("MeshLib face record edge must have face on left");
        }
        for edge in self.left_ring(start)? {
            if self.left(edge) != Some(face) {
                return Err("MeshLib face left ring must keep same face");
            }
        }
        Ok(())
    }

    pub(crate) fn sym(edge: ExactHalfEdgeId) -> ExactHalfEdgeId {
        ExactHalfEdgeId(edge.0 ^ 1)
    }

    fn validate_stitch_pair(
        &self,
        first: ExactHalfEdgeId,
        second: ExactHalfEdgeId,
    ) -> Result<(), &'static str> {
        self.validate_edge(first)?;
        self.validate_edge(second)?;
        if first == second {
            return Err("cannot stitch an edge to itself");
        }
        if self.left(first).is_some() {
            return Err("first contour edge must not have a left face");
        }
        if self.right(second).is_some() {
            return Err("second contour edge must not have a right face");
        }
        Ok(())
    }

    fn set_origin_ring(
        &mut self,
        start: ExactHalfEdgeId,
        origin: Option<usize>,
    ) -> Result<(), &'static str> {
        for edge in self.origin_ring(start)? {
            self.edges[edge.0].origin = origin;
        }
        Ok(())
    }

    fn set_left_ring(
        &mut self,
        start: ExactHalfEdgeId,
        left: Option<usize>,
    ) -> Result<(), &'static str> {
        for edge in self.left_ring(start)? {
            self.edges[edge.0].left = left;
        }
        Ok(())
    }

    fn origin_ring(&self, start: ExactHalfEdgeId) -> Result<Vec<ExactHalfEdgeId>, &'static str> {
        self.validate_edge(start)?;
        let mut ring = Vec::new();
        let mut current = start;
        for _ in 0..=self.edges.len() {
            ring.push(current);
            current = self.next(current);
            self.validate_edge(current)?;
            if current == start {
                return Ok(ring);
            }
        }
        Err("origin ring did not close")
    }

    fn left_ring(&self, start: ExactHalfEdgeId) -> Result<Vec<ExactHalfEdgeId>, &'static str> {
        self.validate_edge(start)?;
        let mut ring = Vec::new();
        let mut current = start;
        for _ in 0..=self.edges.len() {
            ring.push(current);
            current = self.prev(Self::sym(current));
            self.validate_edge(current)?;
            if current == start {
                return Ok(ring);
            }
        }
        Err("left ring did not close")
    }

    fn validate_edge(&self, edge: ExactHalfEdgeId) -> Result<(), &'static str> {
        if edge.0 < self.edges.len() {
            Ok(())
        } else {
            Err("half-edge id out of range")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halfedge_splice_merges_origin_ring_when_one_origin_is_invalid() {
        let mut topology = ExactHalfEdgeTopology::new();
        let first = topology.make_edge(Some(7), Some(8));
        let second = topology.make_edge(None, Some(9));

        topology.splice(first, second).unwrap();

        assert!(topology.shares_origin_ring(first, second).unwrap());
        assert_eq!(topology.next(first), second);
        assert_eq!(topology.next(second), first);
        assert_eq!(topology.prev(first), second);
        assert_eq!(topology.prev(second), first);
        assert_eq!(topology.origin(first), Some(7));
        assert_eq!(topology.origin(second), Some(7));
    }

    #[test]
    fn halfedge_splice_splits_origin_ring_when_ids_match() {
        let mut topology = ExactHalfEdgeTopology::new();
        let first = topology.make_edge(Some(7), Some(8));
        let second = topology.make_edge(None, Some(9));
        topology.splice(first, second).unwrap();

        topology.splice(first, second).unwrap();

        assert!(!topology.shares_origin_ring(first, second).unwrap());
        assert_eq!(topology.next(first), first);
        assert_eq!(topology.next(second), second);
        assert_eq!(topology.origin(first), Some(7));
        assert_eq!(topology.origin(second), None);
    }

    #[test]
    fn halfedge_splice_rejects_conflicting_valid_origin_ids() {
        let mut topology = ExactHalfEdgeTopology::new();
        let first = topology.make_edge(Some(7), Some(8));
        let second = topology.make_edge(Some(70), Some(9));

        let error = topology.splice(first, second).unwrap_err();

        assert_eq!(
            error,
            "cannot splice rings with two different valid origins"
        );
    }

    #[test]
    fn halfedge_stitch_contours_deletes_second_single_edge_contour() {
        let mut topology = ExactHalfEdgeTopology::new();
        let first = topology.make_edge(Some(1), Some(2));
        let second = topology.make_edge(Some(3), Some(4));
        assert_eq!(topology.not_lone_undirected_edge_count().unwrap(), 2);

        topology.stitch_contours(&[first], &[second]).unwrap();

        assert!(topology.is_lone_edge(second).unwrap());
        assert_eq!(topology.not_lone_undirected_edge_count().unwrap(), 1);
        assert!(!topology.shares_origin_ring(first, second).unwrap());
        assert_eq!(topology.origin(first), Some(1));
        assert_eq!(topology.origin(ExactHalfEdgeTopology::sym(first)), Some(2));
    }

    #[test]
    fn halfedge_stitch_contours_rejects_mismatched_lengths() {
        let mut topology = ExactHalfEdgeTopology::new();
        let first = topology.make_edge(Some(1), Some(2));
        let second = topology.make_edge(Some(3), Some(4));

        let error = topology
            .stitch_contours(&[first], &[second, first])
            .unwrap_err();

        assert_eq!(error, "contours must have the same length");
    }

    #[test]
    fn halfedge_stitch_contours_requires_open_left_and_right_sides() {
        let mut topology = ExactHalfEdgeTopology::new();
        let first = topology.make_edge(Some(1), Some(2));
        let second = topology.make_edge(Some(3), Some(4));
        topology.set_left(first, Some(10)).unwrap();

        let error = topology.stitch_contours(&[first], &[second]).unwrap_err();

        assert_eq!(error, "first contour edge must not have a left face");
    }

    #[test]
    fn halfedge_applies_meshlib_stitched_edge_record_rewrite() {
        let mut topology = ExactHalfEdgeTopology::new();
        let target = topology.make_edge(Some(1), Some(2));
        let mapped_from_next = topology.make_edge(Some(2), Some(3));
        let mapped_from_sym_prev = topology.make_edge(Some(4), Some(1));

        topology
            .apply_meshlib_stitched_edge_record_rewrite(
                target,
                mapped_from_next,
                Some(42),
                mapped_from_sym_prev,
                true,
            )
            .unwrap();

        assert_eq!(topology.next(target), mapped_from_next);
        assert_eq!(topology.prev(mapped_from_next), target);
        assert_eq!(topology.left(target), Some(42));
        assert_eq!(
            topology.prev(ExactHalfEdgeTopology::sym(target)),
            mapped_from_sym_prev
        );
        assert_eq!(
            topology.next(mapped_from_sym_prev),
            ExactHalfEdgeTopology::sym(target)
        );
    }

    #[test]
    fn halfedge_applies_meshlib_direct_stitched_edge_record_rewrite() {
        let mut topology = ExactHalfEdgeTopology::new();
        let target = topology.make_edge(Some(1), Some(2));
        let mapped_from_next = topology.make_edge(Some(2), Some(3));
        let mapped_from_sym_prev = topology.make_edge(Some(4), Some(1));

        topology
            .apply_meshlib_stitched_edge_record_rewrite(
                target,
                mapped_from_next,
                Some(42),
                mapped_from_sym_prev,
                false,
            )
            .unwrap();

        assert_eq!(topology.next(target), mapped_from_next);
        assert_ne!(topology.prev(mapped_from_next), target);
        assert_eq!(topology.left(target), Some(42));
        assert_eq!(
            topology.prev(ExactHalfEdgeTopology::sym(target)),
            mapped_from_sym_prev
        );
        assert_ne!(
            topology.next(mapped_from_sym_prev),
            ExactHalfEdgeTopology::sym(target)
        );
    }

    #[test]
    fn halfedge_rejects_meshlib_rewrite_on_closed_target_contour_edge() {
        let mut topology = ExactHalfEdgeTopology::new();
        let target = topology.make_edge(Some(1), Some(2));
        let mapped_from_next = topology.make_edge(Some(2), Some(3));
        let mapped_from_sym_prev = topology.make_edge(Some(4), Some(1));
        topology.set_left_direct(target, Some(7)).unwrap();

        let error = topology
            .apply_meshlib_stitched_edge_record_rewrite(
                target,
                mapped_from_next,
                Some(42),
                mapped_from_sym_prev,
                true,
            )
            .unwrap_err();

        assert_eq!(error, "target contour edge must not have a left face");
    }

    #[test]
    fn halfedge_applies_meshlib_open_contour_near_stitch_update() {
        let mut topology = ExactHalfEdgeTopology::new();
        let previous = topology.make_edge(Some(7), Some(8));
        let next = topology.make_edge(Some(7), Some(9));

        topology
            .apply_meshlib_near_stitch_edge_update(previous, next)
            .unwrap();

        assert_eq!(topology.next(previous), next);
        assert_eq!(topology.prev(next), previous);
    }

    #[test]
    fn halfedge_rejects_near_stitch_update_with_mismatched_origins() {
        let mut topology = ExactHalfEdgeTopology::new();
        let previous = topology.make_edge(Some(7), Some(8));
        let next = topology.make_edge(Some(70), Some(9));

        let error = topology
            .apply_meshlib_near_stitch_edge_update(previous, next)
            .unwrap_err();

        assert_eq!(error, "near stitch edges must share origin");
    }

    #[test]
    fn halfedge_rejects_near_stitch_update_when_previous_has_left_face() {
        let mut topology = ExactHalfEdgeTopology::new();
        let previous = topology.make_edge(Some(7), Some(8));
        let next = topology.make_edge(Some(7), Some(9));
        topology.set_left_direct(previous, Some(42)).unwrap();

        let error = topology
            .apply_meshlib_near_stitch_edge_update(previous, next)
            .unwrap_err();

        assert_eq!(error, "previous near stitch edge must not have a left face");
    }

    #[test]
    fn halfedge_rejects_near_stitch_update_when_next_has_right_face() {
        let mut topology = ExactHalfEdgeTopology::new();
        let previous = topology.make_edge(Some(7), Some(8));
        let next = topology.make_edge(Some(7), Some(9));
        topology
            .set_left_direct(ExactHalfEdgeTopology::sym(next), Some(42))
            .unwrap();

        let error = topology
            .apply_meshlib_near_stitch_edge_update(previous, next)
            .unwrap_err();

        assert_eq!(error, "next near stitch edge must not have a right face");
    }
}
