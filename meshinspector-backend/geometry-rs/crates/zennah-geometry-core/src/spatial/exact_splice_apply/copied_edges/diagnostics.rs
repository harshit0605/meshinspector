#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExactMeshlibCopiedSourceEdgeStatus {
    MappedContour,
    Copied,
    MissingOutputVertices,
    NotPreparedSourceEdge,
}

impl ExactMeshlibCopiedSourceEdgeStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::MappedContour => "mapped-contour",
            Self::Copied => "copied",
            Self::MissingOutputVertices => "missing-output-vertices",
            Self::NotPreparedSourceEdge => "not-prepared-source-edge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibCopiedSourceEdgeDiagnostic {
    pub(crate) status: ExactMeshlibCopiedSourceEdgeStatus,
    pub(crate) source_halfedge: Option<usize>,
    pub(crate) source_origin: Option<usize>,
    pub(crate) source_left: Option<usize>,
    pub(crate) source_right: Option<usize>,
    pub(crate) source_left_mapped_face: Option<usize>,
    pub(crate) source_right_mapped_face: Option<usize>,
    pub(crate) source_next_halfedge: Option<usize>,
    pub(crate) source_prev_halfedge: Option<usize>,
    pub(crate) output_edge_id: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibCopiedSourceEdgeLookupDiagnostic {
    pub(crate) status: ExactMeshlibCopiedSourceEdgeStatus,
    pub(crate) matched_source_edge: Option<[usize; 2]>,
    pub(crate) source_halfedge: Option<usize>,
    pub(crate) source_origin: Option<usize>,
    pub(crate) source_left: Option<usize>,
    pub(crate) source_right: Option<usize>,
    pub(crate) source_left_mapped_face: Option<usize>,
    pub(crate) source_right_mapped_face: Option<usize>,
    pub(crate) source_next_halfedge: Option<usize>,
    pub(crate) source_prev_halfedge: Option<usize>,
    pub(crate) output_edge_id: Option<usize>,
    pub(crate) output_origin: Option<usize>,
    pub(crate) output_left: Option<usize>,
    pub(crate) output_right: Option<usize>,
    pub(crate) output_next_edge_id: Option<usize>,
    pub(crate) output_prev_edge_id: Option<usize>,
    pub(crate) matching_statuses: usize,
}
