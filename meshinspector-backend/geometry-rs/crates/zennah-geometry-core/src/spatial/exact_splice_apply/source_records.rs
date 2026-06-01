use super::super::exact_halfedge::ExactHalfEdgeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactMeshlibPreparedSourceRecord {
    pub next: ExactHalfEdgeId,
    pub left: Option<usize>,
    pub sym_prev: ExactHalfEdgeId,
}
