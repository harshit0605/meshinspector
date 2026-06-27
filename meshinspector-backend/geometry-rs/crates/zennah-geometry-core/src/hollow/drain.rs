use super::{clamp, region_index_by_id, validate_region_offsets, validate_region_vertex_index};
use crate::math::{add, cross, dot, norm, normalize_vector, scale, sub};
use crate::mesh::bounds;
use crate::{DrainHolePlan, GeometryError, MeshArrays};

pub fn plan_drain_holes(
    vertices: &[[f64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    ring_axis: [f64; 3],
    wall_thickness_mm: f64,
    hole_diameter_mm: f64,
) -> Result<Vec<DrainHolePlan>, GeometryError> {
    let ranges = validate_region_offsets(vertex_offsets, vertex_indices.len(), region_ids.len())?;
    let region_by_id = region_index_by_id(region_ids);
    let inner_range = region_by_id
        .get("inner_band")
        .map(|region_index| ranges[*region_index].clone())
        .ok_or(GeometryError::MissingInnerBandRegion)?;
    if inner_range.is_empty() {
        return Err(GeometryError::MissingInnerBandRegion);
    }

    let center = centroid(vertices);
    let axis = normalize_vector(ring_axis)?;
    let mut inner_vertices = Vec::with_capacity(inner_range.len());
    for index in &vertex_indices[inner_range] {
        inner_vertices.push(vertices[validate_region_vertex_index(*index, vertices.len())?]);
    }

    let mut valid_dirs = Vec::new();
    let mut valid_vertices = Vec::new();
    for vertex in &inner_vertices {
        let centered = sub(*vertex, center);
        let radial = sub(centered, scale(axis, dot(centered, axis)));
        let radial_norm = norm(radial);
        if radial_norm > 1e-6 {
            valid_dirs.push(scale(radial, 1.0 / radial_norm));
            valid_vertices.push(*vertex);
        }
    }
    if valid_dirs.is_empty() {
        return Err(GeometryError::DrainHoleDirectionsUnavailable);
    }

    let mut radial_basis = scale(
        valid_dirs.iter().copied().fold([0.0_f64; 3], add),
        1.0 / valid_dirs.len() as f64,
    );
    if norm(radial_basis) < 1e-6 {
        radial_basis = valid_dirs[0];
    }
    radial_basis = normalize_vector(radial_basis)?;

    let (bbox_min, bbox_max) = bounds(vertices);
    let bbox_size = [
        bbox_max[0] - bbox_min[0],
        bbox_max[1] - bbox_min[1],
        bbox_max[2] - bbox_min[2],
    ];
    let max_bbox_size = bbox_size.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let length = clamp(
        max_bbox_size * 0.18,
        (wall_thickness_mm * 5.0).max(3.0),
        8.0,
    );

    let mut plans = Vec::with_capacity(2);
    for basis in [radial_basis, scale(radial_basis, -1.0)] {
        let (anchor, direction) =
            pick_drain_anchor(&valid_vertices, &valid_dirs, center, axis, basis)?;
        let center_point = add(anchor, scale(direction, wall_thickness_mm * 0.55));
        plans.push(DrainHolePlan {
            center_mm: center_point,
            direction,
            radius_mm: hole_diameter_mm / 2.0,
            length_mm: length,
        });
    }
    Ok(plans)
}

pub fn drain_hole_cutter_mesh(
    plan: DrainHolePlan,
    sections: usize,
) -> Result<MeshArrays, GeometryError> {
    if sections < 8 {
        return Err(GeometryError::InvalidDrainHoleSections { sections });
    }

    let direction = normalize_vector(plan.direction)?;
    let mut helper = [0.0, 1.0, 0.0];
    if dot(direction, helper).abs() > 0.92 {
        helper = [1.0, 0.0, 0.0];
    }
    let tangent_u = normalize_vector(cross(direction, helper))?;
    let tangent_v = normalize_vector(cross(direction, tangent_u))?;
    let half = scale(direction, plan.length_mm / 2.0);
    let start = sub(plan.center_mm, half);
    let end = add(plan.center_mm, half);

    let mut vertices = Vec::with_capacity(sections * 2 + 2);
    for base in [start, end] {
        for index in 0..sections {
            let theta = 2.0 * std::f64::consts::PI * index as f64 / sections as f64;
            vertices.push(add(
                base,
                add(
                    scale(tangent_u, plan.radius_mm * theta.cos()),
                    scale(tangent_v, plan.radius_mm * theta.sin()),
                ),
            ));
        }
    }
    let start_center = vertices.len() as i64;
    vertices.push(start);
    let end_center = vertices.len() as i64;
    vertices.push(end);

    let mut faces = Vec::with_capacity(sections * 4);
    for index in 0..sections {
        let next = (index + 1) % sections;
        let a = index as i64;
        let b = next as i64;
        let c = (sections + next) as i64;
        let d = (sections + index) as i64;
        faces.push([a, b, c]);
        faces.push([a, c, d]);
        faces.push([start_center, b, a]);
        faces.push([end_center, d, c]);
    }

    Ok(MeshArrays { vertices, faces })
}

pub fn drain_hole_cutters_mesh(
    plans: &[DrainHolePlan],
    sections: usize,
) -> Result<MeshArrays, GeometryError> {
    if plans.is_empty() {
        return Ok(MeshArrays {
            vertices: Vec::new(),
            faces: Vec::new(),
        });
    }

    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut offset = 0_i64;
    for plan in plans {
        let cutter = drain_hole_cutter_mesh(plan.clone(), sections)?;
        vertices.extend(cutter.vertices);
        faces.extend(
            cutter
                .faces
                .into_iter()
                .map(|face| [face[0] + offset, face[1] + offset, face[2] + offset]),
        );
        offset = vertices.len() as i64;
    }
    Ok(MeshArrays { vertices, faces })
}

fn centroid(vertices: &[[f64; 3]]) -> [f64; 3] {
    if vertices.is_empty() {
        return [0.0; 3];
    }
    scale(
        vertices.iter().copied().fold([0.0_f64; 3], add),
        1.0 / vertices.len() as f64,
    )
}

fn pick_drain_anchor(
    valid_vertices: &[[f64; 3]],
    valid_dirs: &[[f64; 3]],
    center: [f64; 3],
    axis: [f64; 3],
    direction: [f64; 3],
) -> Result<([f64; 3], [f64; 3]), GeometryError> {
    let mut best_index = 0;
    let mut best_score = f64::NEG_INFINITY;
    for (index, valid_dir) in valid_dirs.iter().enumerate() {
        let score = dot(*valid_dir, direction);
        if score > best_score {
            best_score = score;
            best_index = index;
        }
    }
    let anchor = valid_vertices[best_index];
    let radial = sub(
        sub(anchor, center),
        scale(axis, dot(sub(anchor, center), axis)),
    );
    Ok((anchor, normalize_vector(radial)?))
}
