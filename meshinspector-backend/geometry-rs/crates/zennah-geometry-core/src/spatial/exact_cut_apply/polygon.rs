pub(super) fn boundary_path(
    face: [usize; 3],
    start_vertex: usize,
    start_pos: f64,
    end_vertex: usize,
    end_pos: f64,
) -> Vec<usize> {
    let mut path = vec![start_vertex];
    let end_unwrapped = if start_pos < end_pos {
        end_pos
    } else {
        end_pos + 3.0
    };
    let mut position = start_pos.floor() as i32 + 1;
    while (position as f64) < end_unwrapped {
        let vertex = face[position.rem_euclid(3) as usize];
        path.push(vertex);
        position += 1;
    }
    path.push(end_vertex);
    dedupe_adjacent(path)
}

pub(super) fn dedupe_closed_polygon(vertices: &[usize]) -> Vec<usize> {
    let mut output = dedupe_adjacent(vertices.to_vec());
    if output.len() > 1 && output.first() == output.last() {
        output.pop();
    }
    output
}

fn dedupe_adjacent(vertices: Vec<usize>) -> Vec<usize> {
    let mut output = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        if output.last().copied() != Some(vertex) {
            output.push(vertex);
        }
    }
    output
}
