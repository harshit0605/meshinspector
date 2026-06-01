use crate::math::{cross, norm, sub};

#[derive(Debug, Clone, PartialEq)]
pub struct ExactPlanarHoleFillPlan {
    pub boundary_loop: Vec<usize>,
    pub triangles: Vec<[usize; 3]>,
    pub num_tris: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactPlanarHoleFillExecution {
    pub faces: Vec<[i64; 3]>,
    pub source_face_for_faces: Vec<usize>,
}

pub fn exact_planar_hole_fill_plan(
    vertices: &[[f64; 3]],
    boundary_loop: &[usize],
    epsilon: f64,
) -> Option<ExactPlanarHoleFillPlan> {
    let boundary_loop = sanitize_boundary_loop(boundary_loop);
    if boundary_loop.len() < 3 || boundary_loop.iter().any(|vertex| *vertex >= vertices.len()) {
        return None;
    }
    let triangles = triangulate_planar_loop(vertices, &boundary_loop, effective_epsilon(epsilon))?;
    let num_tris = triangles.len();
    Some(ExactPlanarHoleFillPlan {
        boundary_loop,
        triangles,
        num_tris,
    })
}

pub fn execute_exact_planar_hole_fill_plan(
    plan: &ExactPlanarHoleFillPlan,
    source_face: usize,
) -> ExactPlanarHoleFillExecution {
    ExactPlanarHoleFillExecution {
        faces: plan
            .triangles
            .iter()
            .map(|face| [face[0] as i64, face[1] as i64, face[2] as i64])
            .collect(),
        source_face_for_faces: vec![source_face; plan.triangles.len()],
    }
}

fn sanitize_boundary_loop(boundary_loop: &[usize]) -> Vec<usize> {
    let mut output = Vec::with_capacity(boundary_loop.len());
    for vertex in boundary_loop {
        if output.last().copied() != Some(*vertex) {
            output.push(*vertex);
        }
    }
    if output.len() > 1 && output.first() == output.last() {
        output.pop();
    }
    output
}

fn triangulate_planar_loop(
    vertices: &[[f64; 3]],
    boundary_loop: &[usize],
    epsilon: f64,
) -> Option<Vec<[usize; 3]>> {
    let n = boundary_loop.len();
    if n == 3 {
        let face = [boundary_loop[0], boundary_loop[1], boundary_loop[2]];
        return (triangle_area(face, vertices) > epsilon * epsilon).then_some(vec![face]);
    }

    let points = boundary_loop
        .iter()
        .map(|vertex| vertices[*vertex])
        .collect::<Vec<_>>();
    let mut table = vec![
        vec![
            TriangulationCell {
                weight: 0.0,
                split: None,
            };
            n
        ];
        n
    ];

    for span in 2..n {
        for start in 0..(n - span) {
            let end = start + span;
            let mut best = TriangulationCell {
                weight: f64::INFINITY,
                split: None,
            };
            for split in (start + 1)..end {
                let face = [
                    boundary_loop[start],
                    boundary_loop[split],
                    boundary_loop[end],
                ];
                let area = triangle_area(face, vertices);
                if area <= epsilon * epsilon {
                    continue;
                }
                let weight = table[start][split].weight
                    + table[split][end].weight
                    + triangle_fill_metric(points[start], points[split], points[end]);
                if weight < best.weight {
                    best = TriangulationCell {
                        weight,
                        split: Some(split),
                    };
                }
            }
            table[start][end] = best;
        }
    }

    let mut triangles = Vec::with_capacity(n - 2);
    collect_triangulation_faces(&table, boundary_loop, 0, n - 1, &mut triangles);
    (triangles.len() == n - 2).then_some(triangles)
}

#[derive(Debug, Clone, Copy)]
struct TriangulationCell {
    weight: f64,
    split: Option<usize>,
}

fn collect_triangulation_faces(
    table: &[Vec<TriangulationCell>],
    boundary_loop: &[usize],
    start: usize,
    end: usize,
    faces: &mut Vec<[usize; 3]>,
) {
    if end <= start + 1 {
        return;
    }
    let Some(split) = table[start][end].split else {
        return;
    };
    faces.push([
        boundary_loop[start],
        boundary_loop[split],
        boundary_loop[end],
    ]);
    collect_triangulation_faces(table, boundary_loop, start, split, faces);
    collect_triangulation_faces(table, boundary_loop, split, end, faces);
}

fn triangle_fill_metric(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = norm(sub(b, a));
    let bc = norm(sub(c, b));
    let ca = norm(sub(a, c));
    let area = norm(cross(sub(b, a), sub(c, a))) * 0.5;
    if area <= 1e-12 {
        return f64::MAX / 4.0;
    }
    (ab + bc + ca) / area
}

fn triangle_area(face: [usize; 3], vertices: &[[f64; 3]]) -> f64 {
    let a = vertices[face[0]];
    let b = vertices[face[1]];
    let c = vertices[face[2]];
    0.5 * norm(cross(sub(b, a), sub(c, a)))
}

fn effective_epsilon(epsilon: f64) -> f64 {
    if epsilon.is_finite() && epsilon > 0.0 {
        epsilon
    } else {
        1e-9
    }
}
