use super::bvh::{build_flat_bvh, overlapping_face_pairs};
use super::predicates::{
    faces_share_vertex, meshlib_self_collision,
    triangles_intersect as triangles_intersect_predicate,
    triangles_intersect_no_touch as triangles_intersect_no_touch_predicate,
};
use crate::mesh::validate_faces;
use crate::GeometryError;
use std::collections::{BTreeSet, HashMap, VecDeque};

pub fn self_intersecting_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    epsilon: f64,
) -> Result<Vec<usize>, GeometryError> {
    self_intersecting_faces_with_touch(vertices, faces_i64, epsilon, true)
}

pub fn self_intersecting_faces_with_touch(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    epsilon: f64,
    touch_is_intersection: bool,
) -> Result<Vec<usize>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.len() < 2 {
        return Ok(Vec::new());
    }
    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let candidate_pairs = overlapping_face_pairs(&bvh, epsilon);
    let mut intersecting = BTreeSet::new();
    for (face_a, face_b) in candidate_pairs {
        if faces_share_vertex(faces[face_a], faces[face_b]) {
            continue;
        }
        let intersects = if touch_is_intersection {
            triangles_intersect_predicate(triangles[face_a], triangles[face_b], epsilon)
        } else {
            triangles_intersect_no_touch_predicate(triangles[face_a], triangles[face_b], epsilon)
        };
        if intersects {
            intersecting.insert(face_a);
            intersecting.insert(face_b);
        }
    }

    Ok(intersecting.into_iter().collect())
}

pub fn self_intersecting_faces_by_component_with_touch(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    epsilon: f64,
    touch_is_intersection: bool,
) -> Result<Vec<usize>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.len() < 2 {
        return Ok(Vec::new());
    }
    let component_ids = face_component_ids(&faces);
    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let candidate_pairs = overlapping_face_pairs(&bvh, epsilon);
    let mut intersecting = BTreeSet::new();
    for (face_a, face_b) in candidate_pairs {
        if component_ids[face_a] != component_ids[face_b] {
            continue;
        }
        let intersects = meshlib_self_collision(
            faces[face_a],
            faces[face_b],
            triangles[face_a],
            triangles[face_b],
            touch_is_intersection,
        );
        if intersects {
            intersecting.insert(face_a);
            intersecting.insert(face_b);
        }
    }

    Ok(intersecting.into_iter().collect())
}

fn face_component_ids(faces: &[[usize; 3]]) -> Vec<usize> {
    let mut adjacency = vec![Vec::<usize>::new(); faces.len()];
    let mut directed_edges: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (face_index, face) in faces.iter().copied().enumerate() {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            directed_edges.entry((a, b)).or_default().push(face_index);
        }
    }
    for ((a, b), left_face_ids) in &directed_edges {
        if let Some(right_face_ids) = directed_edges.get(&(*b, *a)) {
            for left_face_id in left_face_ids {
                for right_face_id in right_face_ids {
                    if left_face_id != right_face_id {
                        adjacency[*left_face_id].push(*right_face_id);
                    }
                }
            }
        }
    }

    let mut component_ids = vec![usize::MAX; faces.len()];
    let mut next_component = 0_usize;
    for start in 0..faces.len() {
        if component_ids[start] != usize::MAX {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        component_ids[start] = next_component;
        while let Some(face_id) = queue.pop_front() {
            for neighbor in &adjacency[face_id] {
                if component_ids[*neighbor] == usize::MAX {
                    component_ids[*neighbor] = next_component;
                    queue.push_back(*neighbor);
                }
            }
        }
        next_component += 1;
    }
    component_ids
}
