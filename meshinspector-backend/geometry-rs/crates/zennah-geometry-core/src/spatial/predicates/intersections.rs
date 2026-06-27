use crate::math::{add, cross, dot, norm, scale, sub};

pub(super) fn triangles_intersect(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    if !triangle_aabb_overlap(triangle_a, triangle_b, epsilon) {
        return false;
    }
    for index in 0..3 {
        if segment_intersects_triangle_touching(
            triangle_a[index],
            triangle_a[(index + 1) % 3],
            triangle_b,
            epsilon,
        ) {
            return true;
        }
        if segment_intersects_triangle_touching(
            triangle_b[index],
            triangle_b[(index + 1) % 3],
            triangle_a,
            epsilon,
        ) {
            return true;
        }
    }
    triangle_a
        .iter()
        .any(|point| point_in_triangle(*point, triangle_b, epsilon))
        || triangle_b
            .iter()
            .any(|point| point_in_triangle(*point, triangle_a, epsilon))
}

pub(super) fn triangles_intersect_no_touch(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    if !triangle_aabb_overlap(triangle_a, triangle_b, epsilon) {
        return false;
    }
    meshlib_triangles_intersect(
        triangle_a[0],
        triangle_a[1],
        triangle_a[2],
        triangle_b[0],
        triangle_b[1],
        triangle_b[2],
    ) || coplanar_triangles_overlap_with_area(triangle_a, triangle_b, epsilon)
}

pub(super) fn meshlib_distance_self_collision_no_shared_vertices(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    touch_is_intersection: bool,
) -> bool {
    let distance = find_tri_tri_distance_zero_limit(
        triangle_a,
        triangle_b,
        if touch_is_intersection {
            UpLimitCheck::Greater
        } else {
            UpLimitCheck::GreaterOrEqual
        },
    );
    distance.dist_sq <= 0.0 && (touch_is_intersection || distance.overlap)
}

pub(super) fn meshlib_self_collision(
    face_a: [usize; 3],
    face_b: [usize; 3],
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    touch_is_intersection: bool,
) -> bool {
    if let Some((a_indices, b_indices)) = shared_opposite_edge_triangles(face_a, face_b) {
        if !touch_is_intersection {
            return false;
        }
        let ap = [
            triangle_a[a_indices[0]],
            triangle_a[a_indices[1]],
            triangle_a[a_indices[2]],
        ];
        let bp = [
            triangle_b[b_indices[0]],
            triangle_b[b_indices[1]],
            triangle_b[b_indices[2]],
        ];
        let normal_a = triangle_unit_normal(ap);
        let normal_b = triangle_unit_normal(bp);
        const EPS_SQ: f64 = 1e-10;
        if dot(cross(normal_a, normal_b), cross(normal_a, normal_b)) > EPS_SQ {
            return false;
        }
        return dot(normal_a, normal_b) <= 0.0;
    }

    if let Some((a_shared, b_shared)) = shared_vertex(face_a, face_b) {
        if triangle_segment_intersect_meshlib(
            triangle_a[0],
            triangle_a[1],
            triangle_a[2],
            triangle_b[(b_shared + 1) % 3],
            triangle_b[(b_shared + 2) % 3],
        ) || triangle_segment_intersect_meshlib(
            triangle_b[0],
            triangle_b[1],
            triangle_b[2],
            triangle_a[(a_shared + 1) % 3],
            triangle_a[(a_shared + 2) % 3],
        ) {
            return true;
        }
        if !touch_is_intersection {
            return false;
        }
        point_in_triangle_meshlib(triangle_a[(a_shared + 1) % 3], triangle_b)
            || point_in_triangle_meshlib(triangle_a[(a_shared + 2) % 3], triangle_b)
            || point_in_triangle_meshlib(triangle_b[(b_shared + 1) % 3], triangle_a)
            || point_in_triangle_meshlib(triangle_b[(b_shared + 2) % 3], triangle_a)
    } else {
        meshlib_distance_self_collision_no_shared_vertices(
            triangle_a,
            triangle_b,
            touch_is_intersection,
        )
    }
}

fn triangle_aabb_overlap(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    let (a_min, a_max) = triangle_bounds(triangle_a);
    let (b_min, b_max) = triangle_bounds(triangle_b);
    aabb_bounds_overlap(a_min, a_max, b_min, b_max, epsilon)
}

pub(super) fn aabb_bounds_overlap(
    a_min: [f64; 3],
    a_max: [f64; 3],
    b_min: [f64; 3],
    b_max: [f64; 3],
    epsilon: f64,
) -> bool {
    (0..3).all(|axis| a_min[axis] <= b_max[axis] + epsilon && b_min[axis] <= a_max[axis] + epsilon)
}

fn triangle_bounds(triangle: [[f64; 3]; 3]) -> ([f64; 3], [f64; 3]) {
    let mut bbox_min = triangle[0];
    let mut bbox_max = triangle[0];
    for vertex in triangle.iter().skip(1) {
        for axis in 0..3 {
            bbox_min[axis] = bbox_min[axis].min(vertex[axis]);
            bbox_max[axis] = bbox_max[axis].max(vertex[axis]);
        }
    }
    (bbox_min, bbox_max)
}

fn meshlib_triangles_intersect(
    mut a: [f64; 3],
    mut b: [f64; 3],
    mut c: [f64; 3],
    mut d: [f64; 3],
    mut e: [f64; 3],
    mut f: [f64; 3],
) -> bool {
    if dir_dbl_area(a, b, c) == [0.0, 0.0, 0.0] {
        let rotated = rotate_to_longest_edge(a, b, c);
        a = rotated.0;
        b = rotated.1;
        return segment_intersects_triangle_strict(a, b, [d, e, f]);
    }
    if dir_dbl_area(d, e, f) == [0.0, 0.0, 0.0] {
        let rotated = rotate_to_longest_edge(d, e, f);
        d = rotated.0;
        e = rotated.1;
        return segment_intersects_triangle_strict(d, e, [a, b, c]);
    }

    let abcd = mixed(sub(a, d), sub(b, d), sub(c, d));
    let abce = mixed(sub(a, e), sub(b, e), sub(c, e));
    let abcf = mixed(sub(a, f), sub(b, f), sub(c, f));
    let abc_de = abcd * abce >= 0.0;
    let abc_fd = abcf * abcd >= 0.0;
    if abc_de && abc_fd && abce * abcf >= 0.0 {
        return false;
    }

    let defa = mixed(sub(d, a), sub(e, a), sub(f, a));
    let defb = mixed(sub(d, b), sub(e, b), sub(f, b));
    let defc = mixed(sub(d, c), sub(e, c), sub(f, c));
    let def_ab = defa * defb >= 0.0;
    let def_ca = defc * defa >= 0.0;
    if def_ab && def_ca && defb * defc >= 0.0 {
        return false;
    }

    if abc_de {
        std::mem::swap(&mut d, &mut f);
    } else if abc_fd {
        std::mem::swap(&mut d, &mut e);
    }

    if def_ab {
        std::mem::swap(&mut a, &mut c);
    } else if def_ca {
        std::mem::swap(&mut a, &mut b);
    }

    let abde = mixed(sub(a, e), sub(b, e), sub(d, e));
    let abdf = mixed(sub(a, f), sub(b, f), sub(d, f));
    if abde * abdf < 0.0 {
        return true;
    }

    let acde = mixed(sub(a, e), sub(c, e), sub(d, e));
    if abde * acde < 0.0 {
        return true;
    }
    if abdf == 0.0 && acde == 0.0 {
        return true;
    }

    let acdf = mixed(sub(a, f), sub(c, f), sub(d, f));
    if acde * acdf < 0.0 {
        return true;
    }
    if abdf * acdf < 0.0 {
        return true;
    }
    abde == 0.0 && acdf == 0.0
}

fn dir_dbl_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [f64; 3] {
    cross(sub(b, a), sub(c, a))
}

fn mixed(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    dot(a, cross(b, c))
}

fn shared_opposite_edge_triangles(
    face_a: [usize; 3],
    face_b: [usize; 3],
) -> Option<([usize; 3], [usize; 3])> {
    for i in 0..3 {
        let a0 = face_a[i];
        let a1 = face_a[(i + 1) % 3];
        for j in 0..3 {
            if face_b[j] == a1 && face_b[(j + 1) % 3] == a0 {
                return Some(([i, (i + 1) % 3, (i + 2) % 3], [j, (j + 1) % 3, (j + 2) % 3]));
            }
        }
    }
    None
}

fn shared_vertex(face_a: [usize; 3], face_b: [usize; 3]) -> Option<(usize, usize)> {
    for (i, vertex_a) in face_a.iter().copied().enumerate() {
        for (j, vertex_b) in face_b.iter().copied().enumerate() {
            if vertex_a == vertex_b {
                return Some((i, j));
            }
        }
    }
    None
}

fn triangle_unit_normal(triangle: [[f64; 3]; 3]) -> [f64; 3] {
    normalized(cross(
        sub(triangle[1], triangle[0]),
        sub(triangle[2], triangle[0]),
    ))
}

fn triangle_segment_intersect_meshlib(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
    e: [f64; 3],
) -> bool {
    let abcd = mixed(sub(a, d), sub(b, d), sub(c, d));
    let abce = mixed(sub(a, e), sub(b, e), sub(c, e));
    if abcd * abce >= 0.0 {
        return false;
    }
    triangle_line_intersect_meshlib(a, b, c, d, e)
}

fn triangle_line_intersect_meshlib(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
    e: [f64; 3],
) -> bool {
    let dabe = mixed(sub(d, e), sub(a, e), sub(b, e));
    let dbce = mixed(sub(d, e), sub(b, e), sub(c, e));
    if dabe * dbce <= 0.0 {
        return false;
    }
    let dcae = mixed(sub(d, e), sub(c, e), sub(a, e));
    if dbce * dcae <= 0.0 {
        return false;
    }
    dcae * dabe > 0.0
}

fn point_in_triangle_meshlib(point: [f64; 3], triangle: [[f64; 3]; 3]) -> bool {
    let [a, b, c] = triangle;
    if mixed(sub(point, a), sub(point, b), sub(point, c)) != 0.0 {
        return false;
    }
    let norm_dir = cross(sub(b, a), sub(c, a));
    if dot(norm_dir, cross(sub(b, a), sub(point, a))) < 0.0 {
        return false;
    }
    if dot(norm_dir, cross(sub(c, b), sub(point, b))) < 0.0 {
        return false;
    }
    if dot(norm_dir, cross(sub(a, c), sub(point, c))) < 0.0 {
        return false;
    }
    if dot(norm_dir, norm_dir) == 0.0 {
        if a == b && b == c && point != a {
            return false;
        }
        if dot(sub(b, a), sub(c, a)) <= 0.0 {
            return point_in_segment_meshlib(point, b, c);
        }
        if distance_sq(a, b) > distance_sq(a, c) {
            return point_in_segment_meshlib(point, a, b);
        }
        return point_in_segment_meshlib(point, a, c);
    }
    true
}

fn point_in_segment_meshlib(point: [f64; 3], a: [f64; 3], b: [f64; 3]) -> bool {
    if dot(
        cross(sub(point, a), sub(point, b)),
        cross(sub(point, a), sub(point, b)),
    ) != 0.0
    {
        return false;
    }
    dot(sub(point, a), sub(b, a)) >= 0.0 && dot(sub(point, b), sub(a, b)) >= 0.0
}

fn rotate_to_longest_edge(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let ab_sq = distance_sq(a, b);
    let bc_sq = distance_sq(b, c);
    let ca_sq = distance_sq(c, a);
    if ab_sq >= bc_sq && ab_sq >= ca_sq {
        return (a, b);
    }
    if bc_sq >= ca_sq {
        return (b, c);
    }
    (c, a)
}

fn distance_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(sub(b, a), sub(b, a))
}

