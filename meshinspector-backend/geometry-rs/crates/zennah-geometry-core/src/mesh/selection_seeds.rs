use super::base::validate_faces;
use crate::GeometryError;
use std::collections::BTreeSet;

fn linspace_positions(length: usize, count: usize) -> Vec<usize> {
    if length == 0 || count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![0];
    }
    let last = length - 1;
    (0..count)
        .map(|index| index.saturating_mul(last) / (count - 1))
        .collect()
}

pub fn bounded_seed_indices(vertices: &[[f64; 3]], indices: &[i64], max_count: usize) -> Vec<i64> {
    let unique_indices: Vec<usize> = indices
        .iter()
        .filter_map(|index| {
            if *index < 0 {
                None
            } else {
                let index = *index as usize;
                (index < vertices.len()).then_some(index)
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if max_count == 0 || unique_indices.len() <= max_count {
        return unique_indices
            .into_iter()
            .map(|index| index as i64)
            .collect();
    }

    let mut mins = [f64::INFINITY; 3];
    let mut maxs = [f64::NEG_INFINITY; 3];
    for index in &unique_indices {
        let point = vertices[*index];
        for axis in 0..3 {
            mins[axis] = mins[axis].min(point[axis]);
            maxs[axis] = maxs[axis].max(point[axis]);
        }
    }
    let spans = [maxs[0] - mins[0], maxs[1] - mins[1], maxs[2] - mins[2]];
    let max_span = spans
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));

    if max_span <= 1e-12 {
        return linspace_positions(unique_indices.len(), max_count)
            .into_iter()
            .map(|position| unique_indices[position] as i64)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }

    let divisions = ((max_count as f64).cbrt().ceil() as usize * 2).max(2);
    let mut first_by_cell = std::collections::BTreeMap::<usize, usize>::new();
    for (position, index) in unique_indices.iter().copied().enumerate() {
        let point = vertices[index];
        let mut grid = [0_usize; 3];
        for axis in 0..3 {
            let normalized = if spans[axis] > 1e-12 {
                (point[axis] - mins[axis]) / spans[axis]
            } else {
                0.0
            };
            let cell = (normalized * divisions as f64).floor() as usize;
            grid[axis] = cell.min(divisions - 1);
        }
        let key = grid[0] + divisions * grid[1] + divisions * divisions * grid[2];
        first_by_cell.entry(key).or_insert(position);
    }

    let mut representatives: Vec<usize> = first_by_cell.values().copied().collect::<Vec<_>>();
    representatives.sort_unstable();
    let mut representative_indices: Vec<usize> = representatives
        .into_iter()
        .map(|position| unique_indices[position])
        .collect();

    if representative_indices.len() > max_count {
        representative_indices = linspace_positions(representative_indices.len(), max_count)
            .into_iter()
            .map(|position| representative_indices[position])
            .collect();
    } else if representative_indices.len() < max_count {
        let current: BTreeSet<usize> = representative_indices.iter().copied().collect();
        let remaining: Vec<usize> = unique_indices
            .iter()
            .copied()
            .filter(|index| !current.contains(index))
            .collect();
        let needed = max_count - representative_indices.len();
        for position in linspace_positions(remaining.len(), needed.min(remaining.len())) {
            representative_indices.push(remaining[position]);
        }
    }

    representative_indices
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|index| index as i64)
        .collect()
}

pub fn selection_seed_indices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    vertex_ids: &[i64],
    face_ids: &[i64],
    region_vertex_indices: &[i64],
    brush_points_world: &[[f64; 3]],
) -> Result<Vec<i64>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    let mut seeds = BTreeSet::<usize>::new();

    for index in vertex_ids {
        insert_vertex_seed(&mut seeds, *index, vertices.len(), "selection.vertex_ids")?;
    }
    for face_id in face_ids {
        if *face_id < 0 {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "selection.face_ids",
                value: face_id.to_string(),
            });
        }
        let face_id = *face_id as usize;
        if face_id >= faces.len() {
            return Err(GeometryError::FaceRegionIndexOutOfBounds {
                index: face_id,
                face_count: faces.len(),
            });
        }
        for vertex in faces[face_id] {
            seeds.insert(vertex);
        }
    }
    for index in region_vertex_indices {
        insert_vertex_seed(&mut seeds, *index, vertices.len(), "region_vertex_indices")?;
    }
    if !brush_points_world.is_empty() {
        let closest =
            crate::spatial::closest_points_on_mesh(brush_points_world, vertices, faces_i64)?;
        for face_index in closest.face_indices {
            if face_index < 0 {
                continue;
            }
            let face_index = face_index as usize;
            if let Some(face) = faces.get(face_index) {
                for vertex in face {
                    seeds.insert(*vertex);
                }
            }
        }
    }

    if seeds.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "selection",
            value: "did not resolve to any mesh vertices".to_string(),
        });
    }

    Ok(seeds.into_iter().map(|index| index as i64).collect())
}

fn insert_vertex_seed(
    seeds: &mut BTreeSet<usize>,
    index: i64,
    vertex_count: usize,
    field: &'static str,
) -> Result<(), GeometryError> {
    if index < 0 || index as usize >= vertex_count {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("{index} is outside vertex count {vertex_count}"),
        });
    }
    seeds.insert(index as usize);
    Ok(())
}
