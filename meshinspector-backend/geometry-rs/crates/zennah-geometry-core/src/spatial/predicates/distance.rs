#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UpLimitCheck {
    Greater,
    GreaterOrEqual,
}

#[derive(Clone, Copy, Debug)]
struct TriTriDistance {
    a: [f64; 3],
    b: [f64; 3],
    dist_sq: f64,
    overlap: bool,
}

#[derive(Clone, Copy, Debug)]
struct TwoLineSegmentClosestPoints {
    a: [f64; 3],
    b: [f64; 3],
    dir: [f64; 3],
}

fn find_tri_tri_distance_zero_limit(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    up_limit_check: UpLimitCheck,
) -> TriTriDistance {
    let mut result = TriTriDistance {
        a: triangle_a[0],
        b: triangle_b[0],
        dist_sq: distance_sq(triangle_a[0], triangle_b[0]),
        overlap: true,
    };
    const PREV: [usize; 3] = [2, 0, 1];
    const NEXT: [usize; 3] = [1, 2, 0];

    for i in 0..3 {
        for j in 0..3 {
            let segment_distance = find_two_line_segment_closest_points(
                triangle_a[i],
                triangle_a[NEXT[i]],
                triangle_b[j],
                triangle_b[NEXT[j]],
            );
            let dd = distance_sq(segment_distance.a, segment_distance.b);
            if dd <= result.dist_sq {
                result.a = segment_distance.a;
                result.b = segment_distance.b;
                result.dist_sq = dd;

                let mut s = dot(sub(triangle_a[PREV[i]], result.a), segment_distance.dir);
                let mut t = dot(sub(triangle_b[PREV[j]], result.b), segment_distance.dir);
                if s <= 0.0 && t >= 0.0 {
                    result.overlap = false;
                    return result;
                }

                let p = dot(sub(result.b, result.a), segment_distance.dir);
                if s < 0.0 {
                    s = 0.0;
                }
                if t > 0.0 {
                    t = 0.0;
                }

                if p - s + t >= 0.0 {
                    result.overlap = false;
                    let plane_dist_sq = (p - s + t) * (p - s + t);
                    if can_exit_earlier(plane_dist_sq, up_limit_check) {
                        result.dist_sq = plane_dist_sq;
                        return result;
                    }
                }
            }
        }
    }

    if let Some(projected) =
        project_b_on_a(triangle_a, triangle_b, &mut result.overlap, up_limit_check)
    {
        return projected;
    }
    if let Some(mut projected) =
        project_b_on_a(triangle_b, triangle_a, &mut result.overlap, up_limit_check)
    {
        std::mem::swap(&mut projected.a, &mut projected.b);
        return projected;
    }

    if result.overlap {
        result.dist_sq = 0.0;
    }
    result
}

fn project_b_on_a(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    overlap: &mut bool,
    up_limit_check: UpLimitCheck,
) -> Option<TriTriDistance> {
    let edges = [
        sub(triangle_a[1], triangle_a[0]),
        sub(triangle_a[2], triangle_a[1]),
        sub(triangle_a[0], triangle_a[2]),
    ];
    let normal = normalized(cross(edges[0], edges[1]));
    if normal == [0.0, 0.0, 0.0] {
        return None;
    }

    let projections = [
        dot(sub(triangle_a[0], triangle_b[0]), normal),
        dot(sub(triangle_a[0], triangle_b[1]), normal),
        dot(sub(triangle_a[0], triangle_b[2]), normal),
    ];
    let point = if projections.iter().all(|value| *value >= 0.0) {
        let mut point = if projections[0] < projections[1] {
            0
        } else {
            1
        };
        if projections[2] < projections[point] {
            point = 2;
        }
        Some(point)
    } else if projections.iter().all(|value| *value <= 0.0) {
        let mut point = if projections[0] > projections[1] {
            0
        } else {
            1
        };
        if projections[2] > projections[point] {
            point = 2;
        }
        Some(point)
    } else {
        None
    }?;

    *overlap = false;
    let plane_dist_sq = projections[point] * projections[point];
    if can_exit_earlier(plane_dist_sq, up_limit_check) {
        return Some(TriTriDistance {
            a: triangle_a[0],
            b: triangle_b[point],
            dist_sq: plane_dist_sq,
            overlap: false,
        });
    }

    if mixed(sub(triangle_b[point], triangle_a[0]), normal, edges[0]) > 0.0
        && mixed(sub(triangle_b[point], triangle_a[1]), normal, edges[1]) > 0.0
        && mixed(sub(triangle_b[point], triangle_a[2]), normal, edges[2]) > 0.0
    {
        let a = add(triangle_b[point], scale(normal, projections[point]));
        let b = triangle_b[point];
        return Some(TriTriDistance {
            a,
            b,
            dist_sq: distance_sq(a, b),
            overlap: false,
        });
    }
    None
}

fn can_exit_earlier(dist_sq_lower_bound: f64, up_limit_check: UpLimitCheck) -> bool {
    dist_sq_lower_bound > 0.0
        || (up_limit_check == UpLimitCheck::GreaterOrEqual && dist_sq_lower_bound == 0.0)
}

fn find_two_line_segment_closest_points(
    a0: [f64; 3],
    a1: [f64; 3],
    b0: [f64; 3],
    b1: [f64; 3],
) -> TwoLineSegmentClosestPoints {
    let adir = sub(a1, a0);
    let bdir = sub(b1, b0);
    let aa = dot(adir, adir);
    let bb = dot(bdir, bdir);
    let ab = dot(adir, bdir);
    let denom = aa * bb - ab * ab;

    let mut d = sub(b0, a0);
    let ad = dot(adir, d);
    let bd = dot(bdir, d);
    let mut t = (ad * bb - bd * ab) / denom;
    if t < 0.0 || t.is_nan() {
        t = 0.0;
    } else if t > 1.0 {
        t = 1.0;
    }

    let u = (t * ab - bd) / bb;
    if u <= 0.0 || u.is_nan() {
        let b = b0;
        t = ad / aa;
        if t <= 0.0 || t.is_nan() {
            return TwoLineSegmentClosestPoints {
                a: a0,
                b,
                dir: sub(b0, a0),
            };
        }
        if t >= 1.0 {
            let a = add(a0, adir);
            return TwoLineSegmentClosestPoints {
                a,
                b,
                dir: sub(b0, a),
            };
        }
        let a = add(a0, scale(adir, t));
        let tmp = cross(d, adir);
        return TwoLineSegmentClosestPoints {
            a,
            b,
            dir: cross(adir, tmp),
        };
    }

    if u >= 1.0 {
        let b = add(b0, bdir);
        t = (ab + ad) / aa;
        if t <= 0.0 || t.is_nan() {
            return TwoLineSegmentClosestPoints {
                a: a0,
                b,
                dir: sub(b, a0),
            };
        }
        if t >= 1.0 {
            let a = add(a0, adir);
            return TwoLineSegmentClosestPoints {
                a,
                b,
                dir: sub(b, a),
            };
        }
        let a = add(a0, scale(adir, t));
        d = sub(b, a0);
        let tmp = cross(d, adir);
        return TwoLineSegmentClosestPoints {
            a,
            b,
            dir: cross(adir, tmp),
        };
    }

    let b = add(b0, scale(bdir, u));
    if t <= 0.0 || t.is_nan() {
        let tmp = cross(d, bdir);
        return TwoLineSegmentClosestPoints {
            a: a0,
            b,
            dir: cross(bdir, tmp),
        };
    }
    if t >= 1.0 {
        let a = add(a0, adir);
        d = sub(b0, a);
        let tmp = cross(d, bdir);
        return TwoLineSegmentClosestPoints {
            a,
            b,
            dir: cross(bdir, tmp),
        };
    }

    let a = add(a0, scale(adir, t));
    let mut dir = cross(adir, bdir);
    if dot(dir, d) < 0.0 {
        dir = scale(dir, -1.0);
    }
    TwoLineSegmentClosestPoints { a, b, dir }
}

fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let magnitude = norm(vector);
    if magnitude == 0.0 {
        [0.0, 0.0, 0.0]
    } else {
        scale(vector, 1.0 / magnitude)
    }
}

