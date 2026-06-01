use crate::{FaceOrientationResult, GeometryError};
use std::collections::{HashMap, VecDeque};

pub fn orient_faces_consistently(
    faces: &[[i64; 3]],
) -> Result<FaceOrientationResult, GeometryError> {
    let mut converted_faces = Vec::with_capacity(faces.len());
    for (face_index, face) in faces.iter().enumerate() {
        let mut converted = [0_usize; 3];
        for (corner, value) in face.iter().enumerate() {
            if *value < 0 {
                return Err(GeometryError::NegativeFaceIndex {
                    face: face_index,
                    vertex: *value,
                });
            }
            converted[corner] = *value as usize;
        }
        converted_faces.push(converted);
    }

    let mut edge_lookup: HashMap<(usize, usize), usize> = HashMap::new();
    let mut edge_occurrences: Vec<Vec<(usize, usize, usize)>> = Vec::new();
    for (face_index, face) in converted_faces.iter().enumerate() {
        for (u, v) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let key = if u <= v { (u, v) } else { (v, u) };
            if let Some(edge_index) = edge_lookup.get(&key) {
                edge_occurrences[*edge_index].push((face_index, u, v));
            } else {
                edge_lookup.insert(key, edge_occurrences.len());
                edge_occurrences.push(vec![(face_index, u, v)]);
            }
        }
    }

    let mut adjacency = vec![Vec::<(usize, bool)>::new(); faces.len()];
    for occurrences in edge_occurrences {
        if occurrences.len() < 2 {
            continue;
        }
        for left_index in 0..occurrences.len() {
            let (face_a, a_u, a_v) = occurrences[left_index];
            for (face_b, b_u, b_v) in occurrences.iter().skip(left_index + 1) {
                let same_direction = a_u == *b_u && a_v == *b_v;
                adjacency[face_a].push((*face_b, same_direction));
                adjacency[*face_b].push((face_a, same_direction));
            }
        }
    }

    let mut flip_state = vec![-1_i8; faces.len()];
    let mut component_offsets = vec![0_usize];
    let mut component_faces = Vec::<usize>::new();
    for start in 0..faces.len() {
        if flip_state[start] != -1 {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        flip_state[start] = 0;
        while let Some(face_index) = queue.pop_front() {
            component_faces.push(face_index);
            for (neighbor, same_direction) in &adjacency[face_index] {
                let desired = flip_state[face_index] ^ i8::from(*same_direction);
                if flip_state[*neighbor] == -1 {
                    flip_state[*neighbor] = desired;
                    queue.push_back(*neighbor);
                }
            }
        }
        component_offsets.push(component_faces.len());
    }

    let mut oriented_faces = faces.to_vec();
    for (face_index, should_flip) in flip_state.iter().enumerate() {
        if *should_flip == 1 {
            oriented_faces[face_index] = [
                oriented_faces[face_index][0],
                oriented_faces[face_index][2],
                oriented_faces[face_index][1],
            ];
        }
    }
    Ok(FaceOrientationResult {
        faces: oriented_faces,
        component_offsets,
        component_faces,
    })
}
