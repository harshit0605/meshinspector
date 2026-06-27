#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibCopiedPrevNextEdgeUpdateDiagnostic {
    pub source_contour_edge_id: Option<usize>,
    pub target_contour_edge_id: Option<usize>,
    pub walked_source_edge_id: Option<usize>,
    pub update_kind: Option<&'static str>,
    pub previous_edge_id: usize,
    pub next_edge_id: usize,
    pub previous_origin: Option<usize>,
    pub next_origin: Option<usize>,
    pub previous_left: Option<usize>,
    pub next_right: Option<usize>,
    pub applied: bool,
    pub skipped_reason: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibCopiedFaceRecordCandidateDiagnostic {
    pub source_edge_id: usize,
    pub source_edge_vertices: Option<[usize; 2]>,
    pub source_edge_left: Option<usize>,
    pub source_edge_right: Option<usize>,
    pub source_next_edge_id: usize,
    pub source_prev_edge_id: usize,
    pub mapped_edge_id: Option<usize>,
    pub face_edge_id: Option<usize>,
    pub face_edge_origin: Option<usize>,
    pub face_edge_destination: Option<usize>,
    pub face_edge_left: Option<usize>,
    pub face_edge_right: Option<usize>,
    pub face_edge_next_edge_id: Option<usize>,
    pub face_edge_prev_edge_id: Option<usize>,
    pub face_edge_sym_next_edge_id: Option<usize>,
    pub face_edge_sym_prev_edge_id: Option<usize>,
    pub face_edge_left_ring_next_edge_id: Option<usize>,
    pub left_ring_valid: bool,
    pub left_ring_error: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibCopiedFaceRecordDiagnostic {
    pub output_face: usize,
    pub cut_face: usize,
    pub source_face: Option<usize>,
    pub selected_edge_id: usize,
    pub selected_source_edge_id: usize,
    pub selected_source_edge_vertices: Option<[usize; 2]>,
    pub selected_by_valid_left_ring: bool,
    pub selected_left_ring_valid: bool,
    pub selected_left_ring_error: Option<&'static str>,
    pub candidates: Vec<MeshlibCopiedFaceRecordCandidateDiagnostic>,
}
