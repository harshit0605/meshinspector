use crate::math::{add, cross, dot, norm, normalize_vector, scale, sub};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct SectionContourSegment {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub selected_region_hit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionContour {
    pub section_constant: f64,
    pub plane_axis: [f64; 3],
    pub plane_u_axis: [f64; 3],
    pub plane_v_axis: [f64; 3],
    pub plane_origin: [f64; 3],
    pub contour_count: usize,
    pub segment_count: usize,
    pub selected_region_segment_count: usize,
    pub perimeter_mm: Option<f64>,
    pub width_mm: Option<f64>,
    pub depth_mm: Option<f64>,
    pub projected_bounds_min: Option<[f64; 2]>,
    pub projected_bounds_max: Option<[f64; 2]>,
    pub bounds_min: Option<[f64; 3]>,
    pub bounds_max: Option<[f64; 3]>,
    pub segments: Vec<SectionContourSegment>,
}

pub fn section_contour(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    section_constant: f64,
    plane_axis: [f64; 3],
    selected_vertex_indices: &[i64],
    epsilon: f64,
) -> Result<SectionContour, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if !section_constant.is_finite() || !epsilon.is_finite() || epsilon <= 0.0 {
        return Err(GeometryError::InvalidThicknessInput {
            field: "section",
            value: section_constant,
        });
    }
    let normal = normalize_vector(plane_axis)?;
    let reference = if normal[1].abs() < 0.95 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u_axis = normalize_vector(cross(reference, normal))?;
    let v_axis = normalize_vector(cross(normal, u_axis))?;
    let plane_origin = scale(normal, section_constant);
    let selected_vertices = selected_vertex_set(selected_vertex_indices, vertices.len())?;

    let mut segments = Vec::new();
    let mut segment_keys = HashSet::new();
    let mut endpoint_keys = HashSet::new();
    let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
    let mut selected_region_segment_count = 0_usize;
    let mut perimeter_mm = 0.0_f64;
    let mut min_u = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    let mut bounds_min = [f64::INFINITY; 3];
    let mut bounds_max = [f64::NEG_INFINITY; 3];

    for face in &faces {
        let intersections = [
            edge_intersections(
                vertices[face[0]],
                vertices[face[1]],
                normal,
                section_constant,
                epsilon,
            ),
            edge_intersections(
                vertices[face[1]],
                vertices[face[2]],
                normal,
                section_constant,
                epsilon,
            ),
            edge_intersections(
                vertices[face[2]],
                vertices[face[0]],
                normal,
                section_constant,
                epsilon,
            ),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let mut unique_points: HashMap<String, [f64; 3]> = HashMap::new();
        for point in intersections {
            unique_points
                .entry(quantize_point(point, epsilon))
                .or_insert(point);
        }
        if unique_points.len() != 2 {
            continue;
        }
        let points = unique_points.into_values().collect::<Vec<_>>();
        let p0 = points[0];
        let p1 = points[1];
        let key0 = quantize_point(p0, epsilon);
        let key1 = quantize_point(p1, epsilon);
        let segment_key = sorted_segment_key(&key0, &key1);
        if !segment_keys.insert(segment_key) {
            continue;
        }

        endpoint_keys.insert(key0.clone());
        endpoint_keys.insert(key1.clone());
        adjacency
            .entry(key0.clone())
            .or_default()
            .insert(key1.clone());
        adjacency.entry(key1).or_default().insert(key0);

        perimeter_mm += norm(sub(p1, p0));
        for point in [p0, p1] {
            let u = dot(u_axis, point);
            let v = dot(v_axis, point);
            min_u = min_u.min(u);
            max_u = max_u.max(u);
            min_v = min_v.min(v);
            max_v = max_v.max(v);
            for axis in 0..3 {
                bounds_min[axis] = bounds_min[axis].min(point[axis]);
                bounds_max[axis] = bounds_max[axis].max(point[axis]);
            }
        }

        let selected_region_hit = face.iter().any(|vertex| selected_vertices.contains(vertex));
        if selected_region_hit {
            selected_region_segment_count += 1;
        }
        segments.push(SectionContourSegment {
            start: p0,
            end: p1,
            selected_region_hit,
        });
    }

    let segment_count = segments.len();
    let contour_count = contour_component_count(&endpoint_keys, &adjacency);
    Ok(SectionContour {
        section_constant,
        plane_axis: normal,
        plane_u_axis: u_axis,
        plane_v_axis: v_axis,
        plane_origin,
        contour_count,
        segment_count,
        selected_region_segment_count,
        perimeter_mm: (segment_count > 0).then_some(perimeter_mm),
        width_mm: (segment_count > 0).then_some(max_u - min_u),
        depth_mm: (segment_count > 0).then_some(max_v - min_v),
        projected_bounds_min: (segment_count > 0).then_some([min_u, min_v]),
        projected_bounds_max: (segment_count > 0).then_some([max_u, max_v]),
        bounds_min: (segment_count > 0).then_some(bounds_min),
        bounds_max: (segment_count > 0).then_some(bounds_max),
        segments,
    })
}

fn selected_vertex_set(
    indices: &[i64],
    vertex_count: usize,
) -> Result<HashSet<usize>, GeometryError> {
    let mut selected = HashSet::new();
    for index in indices {
        if *index < 0 {
            return Err(GeometryError::RegionVertexOutOfBounds {
                index: *index,
                vertex_count,
            });
        }
        let value = *index as usize;
        if value >= vertex_count {
            return Err(GeometryError::RegionVertexOutOfBounds {
                index: *index,
                vertex_count,
            });
        }
        selected.insert(value);
    }
    Ok(selected)
}

fn edge_intersections(
    a: [f64; 3],
    b: [f64; 3],
    normal: [f64; 3],
    section_constant: f64,
    epsilon: f64,
) -> Vec<[f64; 3]> {
    let da = dot(normal, a) - section_constant;
    let db = dot(normal, b) - section_constant;
    if da.abs() <= epsilon && db.abs() <= epsilon {
        return Vec::new();
    }
    if da.abs() <= epsilon {
        return vec![a];
    }
    if db.abs() <= epsilon {
        return vec![b];
    }
    if da * db > 0.0 {
        return Vec::new();
    }
    let t = da / (da - db);
    vec![add(a, scale(sub(b, a), t))]
}

fn quantize_point(point: [f64; 3], epsilon: f64) -> String {
    format!(
        "{}:{}:{}",
        (point[0] / epsilon).round() as i64,
        (point[1] / epsilon).round() as i64,
        (point[2] / epsilon).round() as i64
    )
}

fn sorted_segment_key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}|{b}")
    } else {
        format!("{b}|{a}")
    }
}

fn contour_component_count(
    endpoint_keys: &HashSet<String>,
    adjacency: &HashMap<String, HashSet<String>>,
) -> usize {
    let mut visited = HashSet::new();
    let mut components = 0_usize;
    for endpoint in endpoint_keys {
        if visited.contains(endpoint) {
            continue;
        }
        components += 1;
        let mut queue = VecDeque::from([endpoint.clone()]);
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            for next in adjacency.get(&current).into_iter().flatten() {
                if !visited.contains(next) {
                    queue.push_back(next.clone());
                }
            }
        }
    }
    components
}
