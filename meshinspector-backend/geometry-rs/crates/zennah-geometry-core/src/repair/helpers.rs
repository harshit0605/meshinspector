fn validate_fix_self_intersections_relax_options(
    options: FixSelfIntersectionsRelaxOptions,
) -> Result<(), GeometryError> {
    if !options.force.is_finite() || options.force < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "force",
            value: options.force.to_string(),
        });
    }
    if !options.epsilon.is_finite() || options.epsilon <= 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "epsilon",
            value: options.epsilon.to_string(),
        });
    }
    Ok(())
}

fn expanded_self_intersection_face_region(
    faces: &[[usize; 3]],
    seed_face_ids: &[usize],
    max_expand: usize,
) -> Vec<bool> {
    let mut adjacency = vec![BTreeSet::<usize>::new(); faces.len()];
    for face_ids in edge_face_map(faces).into_values() {
        if face_ids.len() < 2 {
            continue;
        }
        for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                adjacency[face_ids[i]].insert(face_ids[j]);
                adjacency[face_ids[j]].insert(face_ids[i]);
            }
        }
    }

    let mut selected = vec![false; faces.len()];
    let mut queue = VecDeque::<(usize, usize)>::new();
    for face_id in seed_face_ids {
        if *face_id < faces.len() && !selected[*face_id] {
            selected[*face_id] = true;
            queue.push_back((*face_id, 0));
        }
    }
    while let Some((face_id, depth)) = queue.pop_front() {
        if depth >= max_expand {
            continue;
        }
        for neighbor in &adjacency[face_id] {
            if !selected[*neighbor] {
                selected[*neighbor] = true;
                queue.push_back((*neighbor, depth + 1));
            }
        }
    }
    selected
}

fn incident_vertices_for_face_region(
    vertex_count: usize,
    faces: &[[usize; 3]],
    face_region: &[bool],
) -> Vec<bool> {
    let mut vertices = vec![false; vertex_count];
    for (face_id, selected) in face_region.iter().enumerate() {
        if !*selected {
            continue;
        }
        for vertex_id in faces[face_id] {
            vertices[vertex_id] = true;
        }
    }
    vertices
}

fn relax_selected_vertices(
    vertices: &mut [[f64; 3]],
    faces: &[[usize; 3]],
    selected_vertices: &[bool],
    iterations: usize,
    force: f64,
) {
    let neighbors = meshlib_relax_neighbor_rings(vertices.len(), faces);
    for _ in 0..iterations {
        let previous = vertices.to_vec();
        for (vertex_id, selected) in selected_vertices.iter().enumerate() {
            if !*selected || neighbors[vertex_id].is_empty() {
                continue;
            }
            let mut sum = [0.0; 3];
            for neighbor in &neighbors[vertex_id] {
                for axis in 0..3 {
                    sum[axis] += previous[*neighbor][axis];
                }
            }
            let inv_count = 1.0 / neighbors[vertex_id].len() as f64;
            for axis in 0..3 {
                let average = sum[axis] * inv_count;
                vertices[vertex_id][axis] =
                    previous[vertex_id][axis] + force * (average - previous[vertex_id][axis]);
            }
        }
    }
}

fn meshlib_relax_neighbor_rings(vertex_count: usize, faces: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let mut directed_counts = HashMap::<(usize, usize), (usize, usize)>::new();
    for face in faces {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let (lo, hi, is_forward) = if a < b { (a, b, true) } else { (b, a, false) };
            let entry = directed_counts.entry((lo, hi)).or_insert((0, 0));
            if is_forward {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }

    let mut rings = vec![Vec::new(); vertex_count];
    for ((lo, hi), (lo_to_hi, hi_to_lo)) in directed_counts {
        let edge_multiplicity = lo_to_hi.max(hi_to_lo).max(1);
        for _ in 0..edge_multiplicity {
            rings[lo].push(hi);
            rings[hi].push(lo);
        }
    }
    rings
}

fn face_area(vertices: &[[f64; 3]], face: [usize; 3]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    norm(cross(sub(b, a), sub(c, a))) * 0.5
}

fn boundary_edges_in_python_order(faces: &[[usize; 3]]) -> Vec<(usize, usize)> {
    let mut counts: HashMap<(usize, usize), usize> = HashMap::new();
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    for face in faces {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let edge = ordered_edge(a, b);
            if seen.insert(edge) {
                order.push(edge);
            }
            *counts.entry(edge).or_default() += 1;
        }
    }
    order
        .into_iter()
        .filter(|edge| counts.get(edge) == Some(&1))
        .collect()
}

fn ordered_edge(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    if points.is_empty() {
        return [0.0; 3];
    }
    let mut total = [0.0; 3];
    for point in points {
        for axis in 0..3 {
            total[axis] += point[axis];
        }
    }
    scale(total, 1.0 / points.len() as f64)
}

fn loop_normal(points: &[[f64; 3]]) -> [f64; 3] {
    let mut normal = [0.0; 3];
    if points.is_empty() {
        return normal;
    }
    for (index, point) in points.iter().enumerate() {
        let next = points[(index + 1) % points.len()];
        normal[0] += (point[1] - next[1]) * (point[2] + next[2]);
        normal[1] += (point[2] - next[2]) * (point[0] + next[0]);
        normal[2] += (point[0] - next[0]) * (point[1] + next[1]);
    }
    let magnitude = norm(normal).max(1e-12);
    scale(normal, 1.0 / magnitude)
}

fn merge_key(vertex: [f64; 3], tolerance: f64) -> MergeKey {
    if tolerance == 0.0 {
        return MergeKey::Exact(vertex.map(exact_float_key));
    }
    MergeKey::Quantized(vertex.map(|value| (value / tolerance).round_ties_even() as i64))
}

fn boundary_vertex_set(faces: &[[usize; 3]]) -> HashSet<usize> {
    edge_face_map(faces)
        .into_iter()
        .filter(|(_, face_ids)| face_ids.len() == 1)
        .flat_map(|((a, b), _)| [a, b])
        .collect()
}

fn exact_float_key(value: f64) -> u64 {
    if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    }
}
