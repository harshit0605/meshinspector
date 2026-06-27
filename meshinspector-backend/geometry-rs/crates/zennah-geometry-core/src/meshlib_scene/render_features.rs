use super::*;
use crate::{subdivide_mesh, SubdivideMeshOptions};

const FEATURE_RENDER_CIRCLE_SEGMENTS: usize = 128;
const FEATURE_RENDER_SPHERE_VERTICES: usize = 2048;

pub fn meshlib_scene_feature_object_render_payload(
    input: &MeshlibSceneFeatureRenderInput,
) -> Result<MeshlibSceneFeatureRenderPayload, String> {
    let viewport_mask = if input.viewport_mask == 0 {
        VIEWPORT_MASK_ALL
    } else {
        input.viewport_mask
    };
    let mut objects = Vec::new();
    for object in &input.feature_objects {
        if object.visibility_mask & viewport_mask == 0 {
            continue;
        }
        objects.push(meshlib_scene_feature_render_object(object, viewport_mask)?);
    }
    Ok(MeshlibSceneFeatureRenderPayload { objects })
}

fn meshlib_scene_feature_render_object(
    object: &MeshlibSceneFeatureObject,
    viewport_mask: u32,
) -> Result<MeshlibSceneFeatureRenderObject, String> {
    let mut render = MeshlibSceneFeatureRenderObject {
        object_key: object.object_key.clone(),
        object_name: object.object_name.clone(),
        feature_type: object.feature_type.clone(),
        selected: object.selected,
        label: meshlib_scene_feature_label(object, viewport_mask),
        primary_points: Vec::new(),
        primary_polylines: Vec::new(),
        primary_mesh_vertices: Vec::new(),
        primary_mesh_faces: Vec::new(),
        subfeature_points: Vec::new(),
        subfeature_polylines: Vec::new(),
        dimensions: Vec::new(),
    };

    match object.feature_type.as_str() {
        "PointObject" => {
            render
                .primary_points
                .push(object.xf.transform_point([0.0, 0.0, 0.0]));
        }
        "LineObject" => {
            render.primary_polylines.push(meshlib_render_polyline(
                object,
                &[[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                false,
            ));
        }
        "PlaneObject" => {
            const CORNERS: [[f64; 3]; 4] = [
                [1.0, 1.0, 0.0],
                [1.0, -1.0, 0.0],
                [-1.0, -1.0, 0.0],
                [-1.0, 1.0, 0.0],
            ];
            render.primary_mesh_vertices = CORNERS
                .iter()
                .map(|point| object.xf.transform_point(*point))
                .collect();
            render.primary_mesh_faces = vec![[0, 2, 1], [0, 3, 2]];
        }
        "CircleObject" => {
            render
                .primary_polylines
                .push(meshlib_render_circle_polyline(object));
        }
        "CylinderObject" => {
            let (vertices, faces) = meshlib_render_open_cylinder_mesh(object);
            render.primary_mesh_vertices = vertices;
            render.primary_mesh_faces = faces;
        }
        "ConeObject" => {
            let (vertices, faces) = meshlib_render_open_cone_mesh(object);
            render.primary_mesh_vertices = vertices;
            render.primary_mesh_faces = faces;
        }
        "SphereObject" => {
            let (vertices, faces) = meshlib_render_sphere_mesh(object)?;
            render.primary_mesh_vertices = vertices;
            render.primary_mesh_faces = faces;
        }
        unsupported => {
            return Err(format!(
                "MRU scene FeatureObject {} has unsupported render type {}",
                object.object_key, unsupported
            ));
        }
    }

    meshlib_scene_feature_subfeatures(object, viewport_mask, &mut render);
    meshlib_scene_feature_dimensions(object, viewport_mask, &mut render);
    Ok(render)
}

fn meshlib_render_polyline(
    object: &MeshlibSceneFeatureObject,
    points: &[[f64; 3]],
    closed: bool,
) -> MeshlibSceneFeatureRenderPolyline {
    MeshlibSceneFeatureRenderPolyline {
        points: points
            .iter()
            .map(|point| object.xf.transform_point(*point))
            .collect(),
        closed,
    }
}

fn meshlib_render_circle_polyline(
    object: &MeshlibSceneFeatureObject,
) -> MeshlibSceneFeatureRenderPolyline {
    meshlib_render_circle_polyline_at(object, [0.0, 0.0, 0.0])
}

fn meshlib_render_circle_polyline_at(
    object: &MeshlibSceneFeatureObject,
    center: [f64; 3],
) -> MeshlibSceneFeatureRenderPolyline {
    let mut points = Vec::with_capacity(FEATURE_RENDER_CIRCLE_SEGMENTS);
    for index in 0..FEATURE_RENDER_CIRCLE_SEGMENTS {
        let angle = index as f64 * std::f64::consts::TAU / FEATURE_RENDER_CIRCLE_SEGMENTS as f64;
        points.push(object.xf.transform_point([
            center[0] + angle.cos(),
            center[1] + angle.sin(),
            center[2],
        ]));
    }
    MeshlibSceneFeatureRenderPolyline {
        points,
        closed: true,
    }
}

fn meshlib_render_open_cylinder_mesh(
    object: &MeshlibSceneFeatureObject,
) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let mut vertices = Vec::with_capacity(2 * FEATURE_RENDER_CIRCLE_SEGMENTS);
    for side in 0..2 {
        let z = if side == 0 { -0.5 } else { 0.5 };
        for index in 0..FEATURE_RENDER_CIRCLE_SEGMENTS {
            let angle =
                index as f64 * std::f64::consts::TAU / FEATURE_RENDER_CIRCLE_SEGMENTS as f64;
            vertices.push(object.xf.transform_point([angle.cos(), angle.sin(), z]));
        }
    }

    let mut faces = Vec::with_capacity(2 * FEATURE_RENDER_CIRCLE_SEGMENTS);
    for index in 0..FEATURE_RENDER_CIRCLE_SEGMENTS {
        let a = index as i64;
        let b = ((index + 1) % FEATURE_RENDER_CIRCLE_SEGMENTS) as i64;
        let c = (index + FEATURE_RENDER_CIRCLE_SEGMENTS) as i64;
        let d =
            ((index + 1) % FEATURE_RENDER_CIRCLE_SEGMENTS + FEATURE_RENDER_CIRCLE_SEGMENTS) as i64;
        faces.push([a, b, c]);
        faces.push([b, d, c]);
    }
    (vertices, faces)
}

fn meshlib_render_open_cone_mesh(
    object: &MeshlibSceneFeatureObject,
) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let mut vertices = Vec::with_capacity(FEATURE_RENDER_CIRCLE_SEGMENTS + 1);
    for index in 0..FEATURE_RENDER_CIRCLE_SEGMENTS {
        let angle = index as f64 * std::f64::consts::TAU / FEATURE_RENDER_CIRCLE_SEGMENTS as f64;
        vertices.push(object.xf.transform_point([angle.cos(), angle.sin(), 1.0]));
    }
    vertices.push(object.xf.transform_point([0.0, 0.0, 0.0]));

    let apex = FEATURE_RENDER_CIRCLE_SEGMENTS as i64;
    let mut faces = Vec::with_capacity(FEATURE_RENDER_CIRCLE_SEGMENTS);
    for index in 0..FEATURE_RENDER_CIRCLE_SEGMENTS {
        let a = index as i64;
        let b = ((index + 1) % FEATURE_RENDER_CIRCLE_SEGMENTS) as i64;
        faces.push([b, a, apex]);
    }
    (vertices, faces)
}

fn meshlib_render_sphere_mesh(
    object: &MeshlibSceneFeatureObject,
) -> Result<(Vec<[f64; 3]>, Vec<[i64; 3]>), String> {
    let vertices = vec![
        normalize3([-0.5, -0.5, -0.5]),
        normalize3([-0.5, 0.5, -0.5]),
        normalize3([0.5, 0.5, -0.5]),
        normalize3([0.5, -0.5, -0.5]),
        normalize3([-0.5, -0.5, 0.5]),
        normalize3([-0.5, 0.5, 0.5]),
        normalize3([0.5, 0.5, 0.5]),
        normalize3([0.5, -0.5, 0.5]),
    ];
    let faces = vec![
        [0_i64, 1, 2],
        [2, 3, 0],
        [0, 4, 5],
        [5, 1, 0],
        [0, 3, 7],
        [7, 4, 0],
        [6, 5, 4],
        [4, 7, 6],
        [1, 5, 6],
        [6, 2, 1],
        [6, 7, 3],
        [3, 2, 6],
    ];
    let sphere = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: FEATURE_RENDER_SPHERE_VERTICES - vertices.len(),
            max_deviation_after_flip: Some(1.0),
            project_new_vertices_to_unit_sphere: true,
            ..SubdivideMeshOptions::default()
        },
    )
    .map_err(|error| format!("MR::makeSphere-style FeatureObject render failed: {error}"))?;

    Ok((
        sphere
            .mesh
            .vertices
            .into_iter()
            .map(|point| object.xf.transform_point(point))
            .collect(),
        sphere.mesh.faces,
    ))
}

fn meshlib_scene_feature_subfeatures(
    object: &MeshlibSceneFeatureObject,
    viewport_mask: u32,
    render: &mut MeshlibSceneFeatureRenderObject,
) {
    if object.subfeature_visibility & viewport_mask == 0 {
        return;
    }

    match object.feature_type.as_str() {
        "PlaneObject" => {
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, 0.0]));
            render.subfeature_polylines.push(meshlib_render_polyline(
                object,
                &[
                    [1.0, 1.0, 0.0],
                    [1.0, -1.0, 0.0],
                    [-1.0, -1.0, 0.0],
                    [-1.0, 1.0, 0.0],
                ],
                true,
            ));
        }
        "CircleObject" | "SphereObject" => {
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, 0.0]));
        }
        "CylinderObject" => {
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, 0.0]));
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, 0.5]));
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, -0.5]));
            render.subfeature_polylines.push(meshlib_render_polyline(
                object,
                &[[0.0, 0.0, -0.5], [0.0, 0.0, 0.5]],
                false,
            ));
            render
                .subfeature_polylines
                .push(meshlib_render_circle_polyline_at(object, [0.0, 0.0, 0.5]));
            render
                .subfeature_polylines
                .push(meshlib_render_circle_polyline_at(object, [0.0, 0.0, -0.5]));
        }
        "ConeObject" => {
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, 0.5]));
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, 0.0]));
            render
                .subfeature_points
                .push(object.xf.transform_point([0.0, 0.0, 1.0]));
            render.subfeature_polylines.push(meshlib_render_polyline(
                object,
                &[[0.0, 0.0, 1.0], [0.0, 0.0, 0.0]],
                false,
            ));
            render
                .subfeature_polylines
                .push(meshlib_render_circle_polyline_at(object, [0.0, 0.0, 1.0]));
        }
        _ => {}
    }
}

fn meshlib_scene_feature_label(object: &MeshlibSceneFeatureObject, viewport_mask: u32) -> String {
    if object.details_on_name_tag & viewport_mask == 0 {
        return object.object_name.clone();
    }
    match object.feature_type.as_str() {
        "PointObject" => {
            let point = object.xf.transform_point([0.0, 0.0, 0.0]);
            format!(
                "{}  |  {:.2}; {:.2}; {:.2}",
                object.object_name, point[0], point[1], point[2]
            )
        }
        "LineObject" => {
            let direction = normalize3(object.xf.row_x);
            format!(
                "{}  |  dir {:.2}, {:.2}, {:.2}",
                object.object_name, direction[0], direction[1], direction[2]
            )
        }
        "PlaneObject" => {
            let normal = normalize3(object.xf.row_z);
            format!(
                "{}  |  N {:.2}, {:.2}, {:.2}",
                object.object_name, normal[0], normal[1], normal[2]
            )
        }
        _ => object.object_name.clone(),
    }
}

fn meshlib_scene_feature_dimensions(
    object: &MeshlibSceneFeatureObject,
    viewport_mask: u32,
    render: &mut MeshlibSceneFeatureRenderObject,
) {
    let mut kinds: Vec<_> = object
        .dimension_visibility
        .iter()
        .filter_map(|(kind, mask)| {
            if mask & viewport_mask == 0 {
                None
            } else {
                Some(kind.as_str())
            }
        })
        .collect();
    kinds.sort_by(|left, right| {
        meshlib_dimension_order(left)
            .cmp(&meshlib_dimension_order(right))
            .then_with(|| left.cmp(right))
    });

    for kind in kinds {
        match (object.feature_type.as_str(), kind) {
            ("CircleObject" | "SphereObject" | "CylinderObject", "Diameter") => {
                render.dimensions.push(MeshlibSceneFeatureRenderDimension {
                    kind: kind.to_string(),
                    points: vec![
                        object.xf.transform_point([-1.0, 0.0, 0.0]),
                        object.xf.transform_point([1.0, 0.0, 0.0]),
                    ],
                })
            }
            ("ConeObject", "Diameter") => {
                render.dimensions.push(MeshlibSceneFeatureRenderDimension {
                    kind: kind.to_string(),
                    points: vec![
                        object.xf.transform_point([-1.0, 0.0, 1.0]),
                        object.xf.transform_point([1.0, 0.0, 1.0]),
                    ],
                })
            }
            ("ConeObject", "Angle") => render.dimensions.push(MeshlibSceneFeatureRenderDimension {
                kind: kind.to_string(),
                points: vec![
                    object.xf.transform_point([0.5, 0.0, 0.5]),
                    object.xf.transform_point([-0.5, 0.0, 0.5]),
                ],
            }),
            ("CylinderObject", "Length") => {
                render.dimensions.push(MeshlibSceneFeatureRenderDimension {
                    kind: kind.to_string(),
                    points: vec![
                        object.xf.transform_point([0.0, 0.0, -0.5]),
                        object.xf.transform_point([0.0, 0.0, 0.5]),
                    ],
                })
            }
            ("ConeObject", "Length") => {
                render.dimensions.push(MeshlibSceneFeatureRenderDimension {
                    kind: kind.to_string(),
                    points: vec![
                        object.xf.transform_point([0.0, 0.0, 0.0]),
                        object.xf.transform_point([0.0, 0.0, 1.0]),
                    ],
                })
            }
            _ => {}
        }
    }
}

fn meshlib_dimension_order(kind: &str) -> usize {
    match kind {
        "Diameter" => 0,
        "Angle" => 1,
        "Length" => 2,
        _ => usize::MAX,
    }
}

fn normalize3(vector: [f64; 3]) -> [f64; 3] {
    let len = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if len <= f64::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    [vector[0] / len, vector[1] / len, vector[2] / len]
}
