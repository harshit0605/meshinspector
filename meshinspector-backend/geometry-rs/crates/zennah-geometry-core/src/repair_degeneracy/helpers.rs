fn meshlib_triangle_aspect_ratio(vertices: &[[f64; 3]], face: [usize; 3]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    let bc = norm(sub(c, b));
    let ca = norm(sub(a, c));
    let ab = norm(sub(b, a));
    let half_perimeter = (bc + ca + ab) / 2.0;
    let denominator = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if denominator <= 0.0 {
        return f64::MAX;
    }
    bc * ca * ab / denominator
}

fn meshlib_like_topology_edge_count(counts: &DirectedEdgeCounts) -> usize {
    counts
        .forward
        .max(counts.reverse)
        .max(counts.total.div_ceil(2))
}

fn multiple_edge_split_operations(faces: &[[usize; 3]]) -> Vec<SplitOperation> {
    let mut by_edge: BTreeMap<(usize, usize), Vec<FaceEdgeOccurrence>> = BTreeMap::new();
    for (face_index, face) in faces.iter().enumerate() {
        for (edge_slot, (start, end)) in
            [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
                .into_iter()
                .enumerate()
        {
            if start == end {
                continue;
            }
            let (edge, forward) = ordered_directed_edge(start, end);
            by_edge.entry(edge).or_default().push(FaceEdgeOccurrence {
                face_index,
                edge_slot,
                forward,
            });
        }
    }

    let mut operations = Vec::new();
    for (edge, occurrences) in by_edge {
        let mut forward = occurrences
            .iter()
            .filter(|occurrence| occurrence.forward)
            .cloned()
            .collect::<Vec<_>>();
        let mut reverse = occurrences
            .iter()
            .filter(|occurrence| !occurrence.forward)
            .cloned()
            .collect::<Vec<_>>();
        if forward
            .len()
            .max(reverse.len())
            .max(occurrences.len().div_ceil(2))
            <= 1
        {
            continue;
        }
        if !forward.is_empty() {
            forward.remove(0);
        }
        if !reverse.is_empty() {
            reverse.remove(0);
        }
        while !forward.is_empty() || !reverse.is_empty() {
            let mut split_occurrences = Vec::new();
            if !forward.is_empty() {
                split_occurrences.push(forward.remove(0));
            }
            if !reverse.is_empty() {
                split_occurrences.push(reverse.remove(0));
            }
            operations.push(SplitOperation {
                edge,
                occurrences: split_occurrences,
            });
        }
    }
    operations
}

fn edge_midpoint(vertices: &[[f64; 3]], edge: (usize, usize)) -> [f64; 3] {
    let a = vertices[edge.0];
    let b = vertices[edge.1];
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

fn split_marked_faces(
    faces: &[[i64; 3]],
    split_map: &HashMap<usize, (usize, i64)>,
) -> Vec<[i64; 3]> {
    let mut output = Vec::with_capacity(faces.len() + split_map.len());
    for (face_index, face) in faces.iter().copied().enumerate() {
        let Some((edge_slot, midpoint)) = split_map.get(&face_index).copied() else {
            output.push(face);
            continue;
        };
        let start = edge_slot;
        let end = (edge_slot + 1) % 3;
        let opposite = (edge_slot + 2) % 3;
        output.push([face[start], midpoint, face[opposite]]);
        output.push([midpoint, face[end], face[opposite]]);
    }
    output
}

fn multi_hole_vertex_count(
    vertex_count: usize,
    faces_i64: &[[i64; 3]],
) -> Result<usize, GeometryError> {
    let faces = validate_faces(faces_i64, vertex_count)?;
    Ok(multi_hole_vertex_components(vertex_count, &faces).len())
}

fn multi_hole_vertex_components(
    vertex_count: usize,
    faces: &[[usize; 3]],
) -> Vec<MultiHoleVertexComponents> {
    let mut incident_faces = vec![Vec::new(); vertex_count];
    let mut edge_faces: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (face_index, face) in faces.iter().enumerate() {
        for vertex in unique_face_vertices(*face) {
            incident_faces[vertex].push(face_index);
        }
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            if a != b {
                edge_faces
                    .entry(ordered_edge(a, b))
                    .or_default()
                    .push(face_index);
            }
        }
    }

    let mut boundary_degrees = vec![0_usize; vertex_count];
    for ((a, b), face_ids) in &edge_faces {
        if face_ids.len() == 1 {
            boundary_degrees[*a] += 1;
            boundary_degrees[*b] += 1;
        }
    }

    let mut output = Vec::new();
    for vertex in 0..vertex_count {
        if boundary_degrees[vertex] <= 2 || incident_faces[vertex].len() <= 1 {
            continue;
        }
        let components = incident_face_components(vertex, &incident_faces[vertex], &edge_faces);
        if components.len() > 1 {
            output.push(MultiHoleVertexComponents { vertex, components });
        }
    }
    output
}

fn incident_face_components(
    vertex: usize,
    incident_faces: &[usize],
    edge_faces: &BTreeMap<(usize, usize), Vec<usize>>,
) -> Vec<Vec<usize>> {
    let incident_set = incident_faces.iter().copied().collect::<HashSet<_>>();
    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
    for ((a, b), face_ids) in edge_faces {
        if *a != vertex && *b != vertex {
            continue;
        }
        let connected = face_ids
            .iter()
            .copied()
            .filter(|face_id| incident_set.contains(face_id))
            .collect::<Vec<_>>();
        for first in &connected {
            for second in &connected {
                if first != second {
                    adjacency.entry(*first).or_default().push(*second);
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut components = Vec::new();
    for face_id in incident_faces {
        if !seen.insert(*face_id) {
            continue;
        }
        let mut stack = vec![*face_id];
        let mut component = Vec::new();
        while let Some(current) = stack.pop() {
            component.push(current);
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if seen.insert(*neighbor) {
                        stack.push(*neighbor);
                    }
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

fn unique_face_vertices(face: [usize; 3]) -> Vec<usize> {
    let mut vertices = vec![face[0], face[1], face[2]];
    vertices.sort_unstable();
    vertices.dedup();
    vertices
}

fn ordered_directed_edge(a: usize, b: usize) -> ((usize, usize), bool) {
    if a <= b {
        ((a, b), true)
    } else {
        ((b, a), false)
    }
}

fn ordered_edge(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
