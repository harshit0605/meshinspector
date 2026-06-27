use nalgebra::{Matrix3, SMatrix, SVector, SymmetricEigen};

use super::cone_approx::{
    approximate_cone_meshlib, cone_angle_from_radius_height, project_cone_point,
};
use super::cylinder_approx::approximate_cylinder_meshlib;
use super::support::{arbitrary_perpendicular, cross};
use super::{
    add, dot, feature_to_primitive, length, normalize, scale, sub, FeaturePrimitive,
    FeaturePrimitiveKind, FeatureRefineOptions, FeatureRefinement, Primitive,
};
use crate::mesh::{face_normals_for_mesh, validate_faces, vertex_normals_from_faces};

const MESH_LIB_REFERENCE: &str = "MR::refineFeatureObject";

pub fn refine_feature_primitives_impl(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    features: &[FeaturePrimitive],
    options: FeatureRefineOptions,
) -> Result<Vec<FeatureRefinement>, String> {
    validate_refine_inputs(vertices, features, options)?;
    let faces = validate_faces(faces_i64, vertices.len()).map_err(|error| error.to_string())?;
    let face_normals =
        face_normals_for_mesh(vertices, faces_i64).map_err(|error| error.to_string())?;
    let vertex_normals = vertex_normals_from_faces(vertices, &faces);
    let incident_faces = incident_faces(vertices.len(), &faces);

    features
        .iter()
        .map(|feature| {
            refine_single_feature(
                vertices,
                &face_normals,
                &vertex_normals,
                &incident_faces,
                feature,
                options,
            )
        })
        .collect()
}

fn validate_refine_inputs(
    vertices: &[[f64; 3]],
    features: &[FeaturePrimitive],
    options: FeatureRefineOptions,
) -> Result<(), String> {
    if vertices.is_empty() {
        return Err("feature refinement requires at least one mesh vertex".to_string());
    }
    if features.is_empty() {
        return Err("feature refinement requires at least one feature primitive".to_string());
    }
    if !options.distance_limit.is_finite() || options.distance_limit < 0.0 {
        return Err(
            "feature refinement distance limit must be finite and non-negative".to_string(),
        );
    }
    if !options.normal_tolerance_degrees.is_finite()
        || !(0.0..=180.0).contains(&options.normal_tolerance_degrees)
    {
        return Err(
            "feature refinement normal tolerance must be finite and between 0 and 180 degrees"
                .to_string(),
        );
    }
    if options.max_iterations == 0 {
        return Err("feature refinement max_iterations must be positive".to_string());
    }
    Ok(())
}

fn refine_single_feature(
    vertices: &[[f64; 3]],
    face_normals: &[[f64; 3]],
    vertex_normals: &[[f64; 3]],
    incident_faces: &[Vec<usize>],
    feature: &FeaturePrimitive,
    options: FeatureRefineOptions,
) -> Result<FeatureRefinement, String> {
    let minimum = minimum_points(feature.kind);
    let mut current = feature.clone();
    let mut previous_selection: Option<Vec<usize>> = None;
    let mut selected = Vec::new();
    let mut iterations = 0usize;
    let mut converged = false;

    for iteration in 0..options.max_iterations {
        selected = select_refine_vertices(
            vertices,
            face_normals,
            vertex_normals,
            incident_faces,
            &current,
            options.distance_limit,
            options.normal_tolerance_degrees,
        )?;
        if selected.len() < minimum {
            return Err(format!(
                "Unable to refine. Number of selected verts ({}) less than minimal ({}) for this feature type",
                selected.len(),
                minimum
            ));
        }

        let selected_points = selected
            .iter()
            .copied()
            .map(|vertex_id| vertices[vertex_id])
            .collect::<Vec<_>>();
        current = fit_feature_from_points(feature, &selected_points)?;
        iterations = iteration + 1;

        if previous_selection.as_ref() == Some(&selected) {
            converged = true;
            break;
        }
        previous_selection = Some(selected.clone());
    }

    Ok(FeatureRefinement {
        feature_id: feature.feature_id.clone(),
        kind: feature.kind,
        primitive: current,
        selected_count: selected.len(),
        selected_vertex_indices: selected,
        iterations,
        converged,
        meshlib_reference: MESH_LIB_REFERENCE,
    })
}

fn select_refine_vertices(
    vertices: &[[f64; 3]],
    face_normals: &[[f64; 3]],
    vertex_normals: &[[f64; 3]],
    incident_faces: &[Vec<usize>],
    feature: &FeaturePrimitive,
    distance_limit: f64,
    normal_tolerance_degrees: f64,
) -> Result<Vec<usize>, String> {
    let distance_limit_sq = distance_limit * distance_limit;
    let cos_normal_tolerance = normal_tolerance_degrees.to_radians().cos();
    let mut selected = Vec::new();

    for (vertex_id, point) in vertices.iter().copied().enumerate() {
        let projection = project_feature_point(feature, point)?;
        if length_sq(sub(projection.point, point)) >= distance_limit_sq {
            continue;
        }
        let Some(world_normal) = projection.normal else {
            selected.push(vertex_id);
            continue;
        };
        if normal_matches(
            world_normal,
            vertex_normals[vertex_id],
            cos_normal_tolerance,
        ) || incident_faces[vertex_id].iter().copied().any(|face_id| {
            normal_matches(world_normal, face_normals[face_id], cos_normal_tolerance)
        }) {
            selected.push(vertex_id);
        }
    }

    Ok(selected)
}

fn normal_matches(a: [f64; 3], b: [f64; 3], threshold: f64) -> bool {
    let Some(a) = normalize(a) else {
        return false;
    };
    let Some(b) = normalize(b) else {
        return false;
    };
    dot(a, b).abs() >= threshold
}

#[derive(Debug, Clone, Copy)]
struct FeatureProjection {
    point: [f64; 3],
    normal: Option<[f64; 3]>,
}

fn project_feature_point(
    feature: &FeaturePrimitive,
    point: [f64; 3],
) -> Result<FeatureProjection, String> {
    match (feature.kind, feature_to_primitive(feature)?) {
        (FeaturePrimitiveKind::Point, Primitive::Sphere { center, .. }) => Ok(FeatureProjection {
            point: center,
            normal: None,
        }),
        (FeaturePrimitiveKind::Sphere, Primitive::Sphere { center, radius }) => {
            let normal = normalize(sub(point, center)).unwrap_or([1.0, 0.0, 0.0]);
            Ok(FeatureProjection {
                point: add(center, scale(normal, radius)),
                normal: Some(normal),
            })
        }
        (
            FeaturePrimitiveKind::Line,
            Primitive::ConeSegment {
                reference_point,
                dir,
                ..
            },
        ) => {
            let projected = add(
                reference_point,
                scale(dir, dot(sub(point, reference_point), dir)),
            );
            Ok(FeatureProjection {
                point: projected,
                normal: None,
            })
        }
        (FeaturePrimitiveKind::Plane, Primitive::Plane { center, normal }) => {
            let signed_distance = dot(sub(point, center), normal);
            Ok(FeatureProjection {
                point: sub(point, scale(normal, signed_distance)),
                normal: Some(normal),
            })
        }
        (
            FeaturePrimitiveKind::Circle,
            Primitive::ConeSegment {
                reference_point,
                dir,
                positive_side_radius,
                ..
            },
        ) => {
            let plane_point = sub(point, scale(dir, dot(sub(point, reference_point), dir)));
            let radial_dir = normalize(sub(plane_point, reference_point))
                .unwrap_or_else(|| arbitrary_perpendicular(dir));
            Ok(FeatureProjection {
                point: add(reference_point, scale(radial_dir, positive_side_radius)),
                normal: None,
            })
        }
        (
            FeaturePrimitiveKind::Cylinder,
            Primitive::ConeSegment {
                reference_point,
                dir,
                positive_side_radius,
                ..
            },
        ) => {
            let axis_offset = scale(dir, dot(sub(point, reference_point), dir));
            let axis_point = add(reference_point, axis_offset);
            let normal =
                normalize(sub(point, axis_point)).unwrap_or_else(|| arbitrary_perpendicular(dir));
            Ok(FeatureProjection {
                point: add(axis_point, scale(normal, positive_side_radius)),
                normal: Some(normal),
            })
        }
        (
            FeaturePrimitiveKind::Cone,
            Primitive::ConeSegment {
                reference_point,
                dir,
                positive_side_radius,
                positive_length,
                ..
            },
        ) => {
            let angle = cone_angle_from_radius_height(positive_side_radius, positive_length)?;
            let (projection, normal) = project_cone_point(reference_point, dir, angle, point)?;
            Ok(FeatureProjection {
                point: projection,
                normal: Some(normal),
            })
        }
        _ => Err(format!(
            "feature {} has incompatible primitive payload for {:?}",
            feature.feature_id, feature.kind
        )),
    }
}

fn fit_feature_from_points(
    template: &FeaturePrimitive,
    points: &[[f64; 3]],
) -> Result<FeaturePrimitive, String> {
    match template.kind {
        FeaturePrimitiveKind::Point => fit_point(template, points),
        FeaturePrimitiveKind::Sphere => fit_sphere(template, points),
        FeaturePrimitiveKind::Line => fit_line(template, points),
        FeaturePrimitiveKind::Plane => fit_plane(template, points),
        FeaturePrimitiveKind::Circle => fit_circle(template, points),
        FeaturePrimitiveKind::Cylinder => fit_cylinder(template, points),
        FeaturePrimitiveKind::Cone => fit_cone(template, points),
    }
}

fn fit_point(template: &FeaturePrimitive, points: &[[f64; 3]]) -> Result<FeaturePrimitive, String> {
    Ok(FeaturePrimitive {
        center: centroid(points),
        radius: 0.0,
        length: 0.0,
        ..template.clone()
    })
}

fn fit_sphere(
    template: &FeaturePrimitive,
    points: &[[f64; 3]],
) -> Result<FeaturePrimitive, String> {
    let mut accum_a = SMatrix::<f64, 4, 4>::zeros();
    let mut accum_b = SVector::<f64, 4>::zeros();
    for point in points {
        let rhs = dot(*point, *point);
        let vec = SVector::<f64, 4>::new(2.0 * point[0], 2.0 * point[1], 2.0 * point[2], -1.0);
        accum_a += vec * vec.transpose();
        accum_b += vec * rhs;
    }
    let result = accum_a
        .qr()
        .solve(&accum_b)
        .ok_or_else(|| format!("Unable to refine feature {} as sphere", template.feature_id))?;
    let center = [result[0], result[1], result[2]];
    let radius_sq = dot(center, center) - result[3];
    Ok(FeaturePrimitive {
        center,
        radius: radius_sq.max(0.0).sqrt(),
        length: 0.0,
        direction: None,
        ..template.clone()
    })
}

fn fit_line(template: &FeaturePrimitive, points: &[[f64; 3]]) -> Result<FeaturePrimitive, String> {
    let center = centroid(points);
    let mut direction = principal_axis(points, center, true)?;
    let bounds = point_bounds(points);
    let bbox_center = bounds.center();
    let bbox_center_proj = add(
        center,
        scale(direction, dot(sub(bbox_center, center), direction)),
    );
    if length_sq(add(bbox_center_proj, direction)) < length_sq(bbox_center_proj) {
        direction = scale(direction, -1.0);
    }
    Ok(FeaturePrimitive {
        center: bbox_center,
        direction: Some(direction),
        radius: 0.0,
        length: bounds.diagonal(),
        ..template.clone()
    })
}

fn fit_plane(template: &FeaturePrimitive, points: &[[f64; 3]]) -> Result<FeaturePrimitive, String> {
    let center = centroid(points);
    let mut normal = principal_axis(points, center, false)?;
    let distance = dot(normal, center);
    if distance < 0.0 {
        normal = scale(normal, -1.0);
    }
    let bounds = point_bounds(points);
    let bbox_center = bounds.center();
    let signed_distance = dot(sub(bbox_center, center), normal);
    Ok(FeaturePrimitive {
        center: sub(bbox_center, scale(normal, signed_distance)),
        direction: Some(normal),
        radius: 0.0,
        length: bounds.diagonal(),
        ..template.clone()
    })
}

fn fit_circle(
    template: &FeaturePrimitive,
    points: &[[f64; 3]],
) -> Result<FeaturePrimitive, String> {
    let plane = fit_plane(template, points)?;
    let normal = plane.direction.ok_or_else(|| {
        format!(
            "Unable to refine feature {} circle normal",
            template.feature_id
        )
    })?;
    let basis_x = arbitrary_perpendicular(normal);
    let basis_y = normalize(cross(normal, basis_x)).ok_or_else(|| {
        format!(
            "Unable to refine feature {} circle basis",
            template.feature_id
        )
    })?;
    let origin = plane.center;

    let mut accum_a = Matrix3::<f64>::zeros();
    let mut accum_b = SVector::<f64, 3>::zeros();
    for point in points {
        let local = sub(*point, origin);
        let x = dot(local, basis_x);
        let y = dot(local, basis_y);
        let vec = SVector::<f64, 3>::new(2.0 * x, 2.0 * y, -1.0);
        accum_a += vec * vec.transpose();
        accum_b += vec * (x * x + y * y);
    }
    let result = accum_a
        .qr()
        .solve(&accum_b)
        .ok_or_else(|| format!("Unable to refine feature {} as circle", template.feature_id))?;
    let radius_sq = result[0] * result[0] + result[1] * result[1] - result[2];
    let center = add(
        add(origin, scale(basis_x, result[0])),
        scale(basis_y, result[1]),
    );
    Ok(FeaturePrimitive {
        center,
        direction: Some(normal),
        radius: radius_sq.max(0.0).sqrt(),
        length: 0.0,
        ..template.clone()
    })
}

fn fit_cylinder(
    template: &FeaturePrimitive,
    points: &[[f64; 3]],
) -> Result<FeaturePrimitive, String> {
    let cylinder = approximate_cylinder_meshlib(points).map_err(|error| {
        format!(
            "Unable to refine feature {} as cylinder: {error}",
            template.feature_id
        )
    })?;
    Ok(FeaturePrimitive {
        center: cylinder.center,
        direction: Some(cylinder.direction),
        radius: cylinder.radius,
        length: cylinder.length,
        ..template.clone()
    })
}

fn fit_cone(template: &FeaturePrimitive, points: &[[f64; 3]]) -> Result<FeaturePrimitive, String> {
    let cone = approximate_cone_meshlib(points).map_err(|error| {
        format!(
            "Unable to refine feature {} as cone: {error}",
            template.feature_id
        )
    })?;
    Ok(FeaturePrimitive {
        center: cone.apex,
        direction: Some(cone.direction),
        radius: cone.base_radius,
        length: cone.height,
        ..template.clone()
    })
}

fn principal_axis(
    points: &[[f64; 3]],
    center: [f64; 3],
    largest: bool,
) -> Result<[f64; 3], String> {
    let mut covariance = Matrix3::<f64>::zeros();
    for point in points {
        let centered = sub(*point, center);
        for row in 0..3 {
            for column in 0..3 {
                covariance[(row, column)] += centered[row] * centered[column];
            }
        }
    }
    covariance /= points.len().max(1) as f64;

    let eigen = SymmetricEigen::new(covariance);
    let axis_index = (0..3)
        .min_by(|left, right| {
            let ordering = eigen.eigenvalues[*left]
                .partial_cmp(&eigen.eigenvalues[*right])
                .unwrap_or(std::cmp::Ordering::Equal);
            if largest {
                ordering.reverse()
            } else {
                ordering
            }
        })
        .unwrap_or(0);
    normalize([
        eigen.eigenvectors[(0, axis_index)],
        eigen.eigenvectors[(1, axis_index)],
        eigen.eigenvectors[(2, axis_index)],
    ])
    .ok_or_else(|| "Unable to refine feature from degenerate point covariance".to_string())
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for point in points {
        sum = add(sum, *point);
    }
    scale(sum, 1.0 / points.len() as f64)
}

#[derive(Debug, Clone, Copy)]
struct PointBounds {
    min: [f64; 3],
    max: [f64; 3],
}

impl PointBounds {
    fn center(self) -> [f64; 3] {
        scale(add(self.min, self.max), 0.5)
    }

    fn diagonal(self) -> f64 {
        length(sub(self.max, self.min))
    }
}

fn point_bounds(points: &[[f64; 3]]) -> PointBounds {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for point in points {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    PointBounds { min, max }
}

fn incident_faces(vertex_count: usize, faces: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let mut incident = vec![Vec::new(); vertex_count];
    for (face_index, face) in faces.iter().copied().enumerate() {
        for vertex_id in face {
            incident[vertex_id].push(face_index);
        }
    }
    incident
}

fn minimum_points(kind: FeaturePrimitiveKind) -> usize {
    match kind {
        FeaturePrimitiveKind::Point => 1,
        FeaturePrimitiveKind::Line => 2,
        FeaturePrimitiveKind::Plane | FeaturePrimitiveKind::Circle => 3,
        FeaturePrimitiveKind::Sphere => 4,
        FeaturePrimitiveKind::Cylinder => 6,
        FeaturePrimitiveKind::Cone => 7,
    }
}

fn length_sq(vector: [f64; 3]) -> f64 {
    dot(vector, vector)
}
