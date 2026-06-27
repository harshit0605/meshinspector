#[derive(Debug, Clone, PartialEq)]
pub struct MeshPlyDocument {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub vertex_colors: Vec<[u8; 4]>,
    pub face_colors: Vec<[u8; 4]>,
    pub vertex_uvs: Vec<[f64; 2]>,
    pub vertex_normals: Vec<[f64; 3]>,
    pub tri_corner_uvs: Vec<[[f64; 2]; 3]>,
    pub edges: Vec<[i64; 2]>,
    pub texture_files: Vec<String>,
    pub texture_images: Vec<MeshPlyTextureImage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPlyTextureImage {
    pub file: String,
    pub resolved_path: String,
    pub width: u32,
    pub height: u32,
    pub pixels_rgba: Vec<[u8; 4]>,
    pub filter: String,
    pub wrap: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlyScalarType {
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Float,
    Double,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PlyProperty {
    Scalar {
        ty: PlyScalarType,
        name: String,
    },
    List {
        count_ty: PlyScalarType,
        item_ty: PlyScalarType,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlyElement {
    pub(super) name: String,
    pub(super) count: usize,
    pub(super) properties: Vec<PlyProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlyHeader {
    pub(super) format: PlyFormat,
    pub(super) elements: Vec<PlyElement>,
    pub(super) comments: Vec<String>,
    pub(super) data_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum PlyCell {
    Scalar(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlyVertexMeshData {
    pub(super) vertices: Vec<[f64; 3]>,
    pub(super) colors: Vec<[u8; 4]>,
    pub(super) uvs: Vec<[f64; 2]>,
    pub(super) normals: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlyFaceMeshData {
    pub(super) faces: Vec<[i64; 3]>,
    pub(super) colors: Vec<[u8; 4]>,
    pub(super) tri_corner_uvs: Vec<[[f64; 2]; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlyEdgeMeshData {
    pub(super) edges: Vec<[i64; 2]>,
}
