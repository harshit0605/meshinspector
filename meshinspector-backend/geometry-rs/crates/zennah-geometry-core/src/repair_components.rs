use crate::mesh::{connected_face_components_for_mesh, surface_area, validate_faces};
use crate::{GeometryError, MeshArrays};
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct WeldReport {
    pub input_vertex_count: usize,
    pub output_vertex_count: usize,
    pub merged_vertex_count: usize,
    pub input_face_count: usize,
    pub output_face_count: usize,
    pub removed_face_count: usize,
    pub eps_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeldResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: WeldReport,
}

/// Merge vertices that share a position to within `eps_mm` into a single vertex,
/// then drop the faces that collapse to a degenerate (repeated-index) triangle as
/// a result. Marching-cubes / voxel-boolean output can leave coincident vertex
/// records at pinch points (e.g. where a protected-hollow cavity tapers to
/// nothing), which read as zero-area boundary loops even though the surface has
/// no real opening. Welding those records closes the pseudo-hole and restores a
/// watertight surface. `eps_mm` is intentionally far below the voxel scale so
/// only (near-)coincident records merge — genuine geometry is untouched.
pub fn weld_coincident_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    eps_mm: f64,
) -> Result<WeldResult, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let eps = if eps_mm.is_finite() && eps_mm > 0.0 {
        eps_mm
    } else {
        1e-5
    };
    let inv = 1.0 / eps;
    let key = |p: &[f64; 3]| -> (i64, i64, i64) {
        (
            (p[0] * inv).round() as i64,
            (p[1] * inv).round() as i64,
            (p[2] * inv).round() as i64,
        )
    };

    // FxHashMap (not SipHash): this map is hot — one entry() per vertex, up to
    // ~1M, keyed by a 24-byte coordinate tuple. It is never iterated, so the
    // hasher choice cannot affect the welding result, only its speed.
    let mut bucket: FxHashMap<(i64, i64, i64), usize> =
        FxHashMap::with_capacity_and_hasher(vertices.len(), Default::default());
    let mut remap = vec![0_usize; vertices.len()];
    let mut welded: Vec<[f64; 3]> = Vec::with_capacity(vertices.len());
    for (old_index, position) in vertices.iter().enumerate() {
        let new_index = *bucket.entry(key(position)).or_insert_with(|| {
            welded.push(*position);
            welded.len() - 1
        });
        remap[old_index] = new_index;
    }

    let mut welded_faces: Vec<[i64; 3]> = Vec::with_capacity(faces_i64.len());
    for face in faces_i64 {
        let a = remap[face[0] as usize];
        let b = remap[face[1] as usize];
        let c = remap[face[2] as usize];
        // A face that lost a distinct corner to welding is degenerate; drop it.
        if a == b || b == c || a == c {
            continue;
        }
        welded_faces.push([a as i64, b as i64, c as i64]);
    }

    // Compact away any vertex left unreferenced after degenerate faces were dropped.
    let compact = compact_mesh(&welded, &welded_faces)?;
    Ok(WeldResult {
        report: WeldReport {
            input_vertex_count: vertices.len(),
            output_vertex_count: compact.vertices.len(),
            merged_vertex_count: vertices.len().saturating_sub(welded.len()),
            input_face_count: faces_i64.len(),
            output_face_count: compact.faces.len(),
            removed_face_count: faces_i64.len().saturating_sub(compact.faces.len()),
            eps_mm: eps,
        },
        vertices: compact.vertices,
        faces: compact.faces,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentPruneReport {
    pub input_component_count: usize,
    pub output_component_count: usize,
    pub removed_component_count: usize,
    pub input_face_count: usize,
    pub output_face_count: usize,
    pub removed_face_count: usize,
    pub input_vertex_count: usize,
    pub output_vertex_count: usize,
    pub removed_vertex_count: usize,
    pub retained_face_count: usize,
    pub min_area_mm2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentPruneResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: ComponentPruneReport,
}

pub fn prune_small_components(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    min_area_mm2: f64,
) -> Result<ComponentPruneResult, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let components = connected_face_components_for_mesh(vertices, faces_i64)?;
    let min_area = min_area_mm2.max(0.0);
    let mut kept_face_ids = BTreeSet::new();
    let mut removed_component_count = 0_usize;

    for component in &components {
        let component_faces: Vec<[usize; 3]> = component
            .iter()
            .map(|face_id| faces[*face_id as usize])
            .collect();
        let component_area = surface_area(vertices, &component_faces);
        if component_area >= min_area {
            kept_face_ids.extend(component.iter().map(|face_id| *face_id as usize));
        } else {
            removed_component_count += 1;
        }
    }

    let kept_faces: Vec<[i64; 3]> = faces_i64
        .iter()
        .enumerate()
        .filter_map(|(face_id, face)| kept_face_ids.contains(&face_id).then_some(*face))
        .collect();
    let compact = compact_mesh(vertices, &kept_faces)?;
    let output_components = connected_face_components_for_mesh(&compact.vertices, &compact.faces)?;

    Ok(ComponentPruneResult {
        report: ComponentPruneReport {
            input_component_count: components.len(),
            output_component_count: output_components.len(),
            removed_component_count,
            input_face_count: faces_i64.len(),
            output_face_count: compact.faces.len(),
            removed_face_count: faces_i64.len().saturating_sub(compact.faces.len()),
            input_vertex_count: vertices.len(),
            output_vertex_count: compact.vertices.len(),
            removed_vertex_count: vertices.len().saturating_sub(compact.vertices.len()),
            retained_face_count: compact.faces.len(),
            min_area_mm2: min_area,
        },
        vertices: compact.vertices,
        faces: compact.faces,
    })
}

fn compact_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<MeshArrays, GeometryError> {
    validate_faces(faces_i64, vertices.len())?;
    let used: BTreeSet<usize> = faces_i64
        .iter()
        .flat_map(|face| face.iter().map(|vertex_id| *vertex_id as usize))
        .collect();
    let mut remap = vec![-1_i64; vertices.len()];
    let mut compact_vertices = Vec::with_capacity(used.len());
    for old_vertex in used {
        remap[old_vertex] = compact_vertices.len() as i64;
        compact_vertices.push(vertices[old_vertex]);
    }
    let compact_faces = faces_i64
        .iter()
        .map(|face| {
            [
                remap[face[0] as usize],
                remap[face[1] as usize],
                remap[face[2] as usize],
            ]
        })
        .collect();
    Ok(MeshArrays {
        vertices: compact_vertices,
        faces: compact_faces,
    })
}
