use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

const MESH_LIB_INT_RANGE: f64 = 0.99 * i32::MAX as f64;

/// Integer point used by the exact-predicate boolean path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactPoint3 {
    pub x: i128,
    pub y: i128,
    pub z: i128,
}

impl ExactPoint3 {
    #[must_use]
    pub const fn new(x: i128, y: i128, z: i128) -> Self {
        Self { x, y, z }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

/// Vertex id plus quantized coordinates for simulation-of-simplicity predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactVertCoords {
    pub id: u64,
    pub point: ExactPoint3,
}

impl ExactVertCoords {
    #[must_use]
    pub const fn new(id: u64, point: ExactPoint3) -> Self {
        Self { id, point }
    }
}

/// Sign of the exact tetrahedron volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactSign {
    Negative,
    Zero,
    Positive,
}

/// MeshLib-style centered float-to-int converter for exact predicates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExactCoordinateConverter {
    center: [f64; 3],
    inv_range: f64,
    range: f64,
}

impl ExactCoordinateConverter {
    #[must_use]
    pub fn from_points(points: &[[f64; 3]]) -> Option<Self> {
        let first = *points.first()?;
        if !is_finite_point(first) {
            return None;
        }
        let mut min = first;
        let mut max = first;
        for point in points.iter().skip(1).copied() {
            if !is_finite_point(point) {
                return None;
            }
            for axis in 0..3 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
        Self::from_bounds(min, max)
    }

    #[must_use]
    pub fn from_bounds(min: [f64; 3], max: [f64; 3]) -> Option<Self> {
        if !is_finite_point(min) || !is_finite_point(max) {
            return None;
        }
        let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        if size.iter().any(|value| *value < 0.0) {
            return None;
        }
        let max_dim = size.into_iter().fold(0.0_f64, f64::max);
        if max_dim <= 0.0 {
            return None;
        }
        let center = [
            0.5 * (min[0] + max[0]),
            0.5 * (min[1] + max[1]),
            0.5 * (min[2] + max[2]),
        ];
        let inv_range = MESH_LIB_INT_RANGE / max_dim;
        Some(Self {
            center,
            inv_range,
            range: max_dim / MESH_LIB_INT_RANGE,
        })
    }

    #[must_use]
    pub fn quantize_point(&self, point: [f64; 3]) -> Option<ExactPoint3> {
        if !is_finite_point(point) {
            return None;
        }
        Some(ExactPoint3::new(
            ((point[0] - self.center[0]) * self.inv_range).round() as i128,
            ((point[1] - self.center[1]) * self.inv_range).round() as i128,
            ((point[2] - self.center[2]) * self.inv_range).round() as i128,
        ))
    }

    #[must_use]
    pub fn restore_point(&self, point: ExactPoint3) -> [f64; 3] {
        [
            point.x as f64 * self.range + self.center[0],
            point.y as f64 * self.range + self.center[1],
            point.z as f64 * self.range + self.center[2],
        ]
    }
}

/// Exact signed tetrahedron volume.
#[must_use]
pub fn orient3d_volume(a: ExactPoint3, b: ExactPoint3, c: ExactPoint3, d: ExactPoint3) -> i128 {
    let x = a.sub(d);
    let y = b.sub(d);
    let z = c.sub(d);

    x.x * (y.y * z.z - y.z * z.y) - x.y * (y.x * z.z - y.z * z.x) + x.z * (y.x * z.y - y.y * z.x)
}

#[must_use]
pub fn orient3d_sign(a: ExactPoint3, b: ExactPoint3, c: ExactPoint3, d: ExactPoint3) -> ExactSign {
    match orient3d_volume(a, b, c, d).cmp(&0) {
        std::cmp::Ordering::Less => ExactSign::Negative,
        std::cmp::Ordering::Equal => ExactSign::Zero,
        std::cmp::Ordering::Greater => ExactSign::Positive,
    }
}

/// Deterministic MeshLib-style orientation side with symbolic perturbation fallback.
#[must_use]
pub fn orient3d_sos_positive(
    a: ExactPoint3,
    b: ExactPoint3,
    c: ExactPoint3,
    d: ExactPoint3,
) -> bool {
    orient3d_from_origin_sos_positive(a.sub(d), b.sub(d), c.sub(d))
}

/// Sorts vertex ids before applying symbolic orientation, matching MeshLib's contract.
pub fn orient3d_precise_sos_positive(vertices: [ExactVertCoords; 4]) -> Option<bool> {
    let mut odd = false;
    let mut order = [0_usize, 1, 2, 3];
    for i in 0..3 {
        for j in (i + 1)..4 {
            if vertices[order[i]].id == vertices[order[j]].id {
                return None;
            }
            if vertices[order[i]].id > vertices[order[j]].id {
                odd = !odd;
                order.swap(i, j);
            }
        }
    }

    let positive = orient3d_sos_positive(
        vertices[order[0]].point,
        vertices[order[1]].point,
        vertices[order[2]].point,
        vertices[order[3]].point,
    );
    Some(odd != positive)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriangleSegmentIntersection {
    pub do_intersect: bool,
    pub d_is_left_from_abc: bool,
}

/// Triangle owner for exact triangle-triangle edge/face intersection records.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ExactTriangleOwner {
    First,
    Second,
}

/// Local edge/triangle intersection produced by the exact boolean predicate stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriangleEdgeIntersection {
    pub edge_owner: ExactTriangleOwner,
    pub edge: [usize; 2],
    pub triangle_owner: ExactTriangleOwner,
    pub triangle: [usize; 3],
    pub d_is_left_from_triangle: bool,
}

/// Exact triangle-segment predicate for triangle ABC and segment DE.
pub fn triangle_segment_intersection_sos(
    vertices: [ExactVertCoords; 5],
) -> Option<TriangleSegmentIntersection> {
    let abcd = orient4(vertices, [0, 1, 2, 3])?;
    let abce = orient4(vertices, [0, 1, 2, 4])?;
    if abcd == abce {
        return Some(TriangleSegmentIntersection {
            do_intersect: false,
            d_is_left_from_abc: abcd,
        });
    }

    let dabe = orient4(vertices, [0, 1, 3, 4])?;
    let dbce = orient4(vertices, [1, 2, 3, 4])?;
    if dabe != dbce {
        return Some(TriangleSegmentIntersection {
            do_intersect: false,
            d_is_left_from_abc: abcd,
        });
    }

    let dcae = !orient4(vertices, [0, 2, 3, 4])?;
    Some(TriangleSegmentIntersection {
        do_intersect: dbce == dcae,
        d_is_left_from_abc: abcd,
    })
}

/// Extracts the local edge/face intersections between two triangles.
pub fn triangle_triangle_intersections_sos(
    first: [ExactVertCoords; 3],
    second: [ExactVertCoords; 3],
) -> Option<Vec<TriangleEdgeIntersection>> {
    let mut intersections = Vec::new();
    collect_triangle_edge_hits(
        second,
        first,
        ExactTriangleOwner::First,
        ExactTriangleOwner::Second,
        &mut intersections,
    )?;
    collect_triangle_edge_hits(
        first,
        second,
        ExactTriangleOwner::Second,
        ExactTriangleOwner::First,
        &mut intersections,
    )?;
    Some(intersections)
}

/// Orders two non-degenerate triangle intersections along a segment.
///
/// Returns `Some(true)` when `first_triangle` intersects the segment before
/// `second_triangle`. Returns `None` for coplanar, non-crossing, or exactly
/// equal cases that need the later symbolic ordering stage.
pub fn segment_intersection_order_rational(
    segment: [ExactVertCoords; 2],
    first_triangle: [ExactVertCoords; 3],
    second_triangle: [ExactVertCoords; 3],
) -> Option<bool> {
    let first = segment_plane_parameter(first_triangle, segment[0].point, segment[1].point)?;
    let second = segment_plane_parameter(second_triangle, segment[0].point, segment[1].point)?;
    compare_rationals(first, second)
}

/// Computes the precise point where a triangle intersects a segment.
///
/// The calculation follows MeshLib's direction: quantize all input coordinates
/// with the shared converter, use integer tetrahedron volumes as interpolation
/// weights, round in integer space, then restore to floating coordinates.
pub fn triangle_segment_intersection_point(
    triangle: [[f64; 3]; 3],
    segment: [[f64; 3]; 2],
    converter: &ExactCoordinateConverter,
) -> Option<[f64; 3]> {
    let triangle = [
        converter.quantize_point(triangle[0])?,
        converter.quantize_point(triangle[1])?,
        converter.quantize_point(triangle[2])?,
    ];
    let segment = [
        converter.quantize_point(segment[0])?,
        converter.quantize_point(segment[1])?,
    ];
    let start_volume = orient3d_volume(triangle[0], triangle[1], triangle[2], segment[0]).abs();
    let end_volume = orient3d_volume(triangle[0], triangle[1], triangle[2], segment[1]).abs();
    let sum = start_volume + end_volume;
    if sum == 0 {
        return Some(average_points(segment[0], segment[1], converter));
    }

    let point = ExactPoint3::new(
        weighted_coord(start_volume, segment[1].x, end_volume, segment[0].x, sum)?,
        weighted_coord(start_volume, segment[1].y, end_volume, segment[0].y, sum)?,
        weighted_coord(start_volume, segment[1].z, end_volume, segment[0].z, sum)?,
    );
    Some(converter.restore_point(point))
}

fn orient4(vertices: [ExactVertCoords; 5], indices: [usize; 4]) -> Option<bool> {
    orient3d_precise_sos_positive([
        vertices[indices[0]],
        vertices[indices[1]],
        vertices[indices[2]],
        vertices[indices[3]],
    ])
}

fn collect_triangle_edge_hits(
    triangle: [ExactVertCoords; 3],
    edge_vertices: [ExactVertCoords; 3],
    edge_owner: ExactTriangleOwner,
    triangle_owner: ExactTriangleOwner,
    output: &mut Vec<TriangleEdgeIntersection>,
) -> Option<()> {
    for edge_index in 0..3 {
        let edge = [edge_index, (edge_index + 1) % 3];
        let intersection = triangle_segment_intersection_sos([
            triangle[0],
            triangle[1],
            triangle[2],
            edge_vertices[edge[0]],
            edge_vertices[edge[1]],
        ])?;
        if intersection.do_intersect {
            output.push(TriangleEdgeIntersection {
                edge_owner,
                edge,
                triangle_owner,
                triangle: [0, 1, 2],
                d_is_left_from_triangle: intersection.d_is_left_from_abc,
            });
        }
    }
    Some(())
}

fn segment_plane_parameter(
    triangle: [ExactVertCoords; 3],
    segment_start: ExactPoint3,
    segment_end: ExactPoint3,
) -> Option<(i128, i128)> {
    let start_volume = orient3d_volume(
        triangle[0].point,
        triangle[1].point,
        triangle[2].point,
        segment_start,
    );
    let end_volume = orient3d_volume(
        triangle[0].point,
        triangle[1].point,
        triangle[2].point,
        segment_end,
    );
    if start_volume == 0 && end_volume == 0 {
        return None;
    }
    if start_volume.signum() == end_volume.signum() && start_volume != 0 {
        return None;
    }

    let mut numerator = start_volume;
    let mut denominator = start_volume - end_volume;
    if denominator == 0 {
        return None;
    }
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    if numerator < 0 || numerator > denominator {
        return None;
    }
    Some((numerator, denominator))
}

fn compare_rationals(left: (i128, i128), right: (i128, i128)) -> Option<bool> {
    let left_scaled = BigInt::from(left.0) * BigInt::from(right.1);
    let right_scaled = BigInt::from(right.0) * BigInt::from(left.1);
    match left_scaled.cmp(&right_scaled) {
        std::cmp::Ordering::Less => Some(true),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(false),
    }
}

fn weighted_coord(
    start_volume: i128,
    end_coord: i128,
    end_volume: i128,
    start_coord: i128,
    denominator: i128,
) -> Option<i128> {
    let numerator = BigInt::from(start_volume) * BigInt::from(end_coord)
        + BigInt::from(end_volume) * BigInt::from(start_coord);
    div_round_bigint(numerator, BigInt::from(denominator))
}

fn div_round_bigint(numerator: BigInt, denominator: BigInt) -> Option<i128> {
    if denominator.is_zero() {
        return None;
    }
    let half = &denominator / 2;
    let rounded = if numerator.is_negative() {
        numerator - half
    } else {
        numerator + half
    };
    let quotient: BigInt = rounded / denominator;
    quotient.to_i128()
}

fn average_points(
    first: ExactPoint3,
    second: ExactPoint3,
    converter: &ExactCoordinateConverter,
) -> [f64; 3] {
    converter.restore_point(ExactPoint3::new(
        (first.x + second.x) / 2,
        (first.y + second.y) / 2,
        (first.z + second.z) / 2,
    ))
}

fn orient3d_from_origin_sos_positive(a: ExactPoint3, b: ExactPoint3, c: ExactPoint3) -> bool {
    let volume = orient3d_volume(a, b, c, ExactPoint3::new(0, 0, 0));
    if volume != 0 {
        return volume > 0;
    }

    let mut area = cross2(b.x, b.y, c.x, c.y);
    if area != 0 {
        return area > 0;
    }
    area = -cross2(b.x, b.z, c.x, c.z);
    if area != 0 {
        return area > 0;
    }
    area = cross2(b.y, b.z, c.y, c.z);
    if area != 0 {
        return area > 0;
    }
    area = -cross2(a.x, a.y, c.x, c.y);
    if area != 0 {
        return area > 0;
    }
    if c.x != 0 {
        return c.x > 0;
    }
    if c.y != 0 {
        return c.y < 0;
    }
    area = cross2(a.x, a.z, c.x, c.z);
    if area != 0 {
        return area > 0;
    }
    if c.z != 0 {
        return c.z > 0;
    }
    area = cross2(a.x, a.y, b.x, b.y);
    if area != 0 {
        return area > 0;
    }
    if b.x != 0 {
        return b.x < 0;
    }
    if b.y != 0 {
        return b.y > 0;
    }
    if a.x != 0 {
        return a.x > 0;
    }
    true
}

fn cross2(ax: i128, ay: i128, bx: i128, by: i128) -> i128 {
    ax * by - ay * bx
}

fn is_finite_point(point: [f64; 3]) -> bool {
    point.into_iter().all(f64::is_finite)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(id: u64, x: i128, y: i128, z: i128) -> ExactVertCoords {
        ExactVertCoords::new(id, ExactPoint3::new(x, y, z))
    }

    #[test]
    fn coordinate_converter_matches_meshlib_int_range_contract() {
        let converter =
            ExactCoordinateConverter::from_bounds([0.0, 0.0, -1.0], [0.0, 0.0, 1.0]).unwrap();

        let low = converter.quantize_point([0.0, 0.0, -1.0]).unwrap();
        let high = converter.quantize_point([0.0, 0.0, 1.0]).unwrap();

        assert!(-low.z <= i128::from(i32::MAX / 2));
        assert!(high.z <= i128::from(i32::MAX / 2));
        assert!(high.z - low.z > 0);
        assert!(converter.restore_point(low)[2] < -0.98);
    }

    #[test]
    fn orient3d_sign_uses_exact_integer_volume() {
        let a = ExactPoint3::new(0, 0, 0);
        let b = ExactPoint3::new(1, 0, 0);
        let c = ExactPoint3::new(0, 1, 0);

        assert_eq!(
            orient3d_sign(a, b, c, ExactPoint3::new(0, 0, -1)),
            ExactSign::Positive
        );
        assert_eq!(
            orient3d_sign(a, b, c, ExactPoint3::new(0, 0, 1)),
            ExactSign::Negative
        );
        assert_eq!(
            orient3d_sign(a, b, c, ExactPoint3::new(1, 1, 0)),
            ExactSign::Zero
        );
    }

    #[test]
    fn triangle_segment_intersection_matches_meshlib_precise_fixture() {
        let result = triangle_segment_intersection_sos([
            vertex(0, 2, 1, 0),
            vertex(1, -2, 1, 0),
            vertex(2, 0, -2, 0),
            vertex(3, 0, 0, -1),
            vertex(4, 0, 0, 1),
        ])
        .unwrap();

        assert!(result.do_intersect);
        assert!(result.d_is_left_from_abc);
    }

    #[test]
    fn triangle_segment_intersection_rejects_one_sided_segment() {
        let result = triangle_segment_intersection_sos([
            vertex(0, 2, 1, 0),
            vertex(1, -2, 1, 0),
            vertex(2, 0, -2, 0),
            vertex(3, 0, 0, 1),
            vertex(4, 0, 0, 2),
        ])
        .unwrap();

        assert!(!result.do_intersect);
    }

    #[test]
    fn triangle_segment_intersection_point_matches_plane_crossing() {
        let converter =
            ExactCoordinateConverter::from_bounds([-2.0, -2.0, -1.0], [2.0, 1.0, 1.0]).unwrap();

        let point = triangle_segment_intersection_point(
            [[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]],
            [[0.0, 0.0, -1.0], [0.0, 0.0, 1.0]],
            &converter,
        )
        .unwrap();

        assert!(point[0].abs() < 1e-8);
        assert!(point[1].abs() < 1e-8);
        assert!(point[2].abs() < 1e-8);
    }

    #[test]
    fn triangle_triangle_intersections_report_crossing_edges() {
        let first = [vertex(0, 2, 1, 0), vertex(1, -2, 1, 0), vertex(2, 0, -2, 0)];
        let second = [vertex(3, 0, 0, -1), vertex(4, 0, 0, 1), vertex(5, 3, 0, 0)];

        let intersections = triangle_triangle_intersections_sos(first, second).unwrap();

        assert!(intersections.iter().any(|intersection| {
            intersection.edge_owner == ExactTriangleOwner::Second
                && intersection.edge == [0, 1]
                && intersection.triangle_owner == ExactTriangleOwner::First
        }));
    }

    #[test]
    fn triangle_triangle_intersections_reject_separated_triangles() {
        let first = [vertex(0, 0, 0, 0), vertex(1, 2, 0, 0), vertex(2, 0, 2, 0)];
        let second = [vertex(3, 0, 0, 2), vertex(4, 2, 0, 2), vertex(5, 0, 2, 2)];

        let intersections = triangle_triangle_intersections_sos(first, second).unwrap();

        assert!(intersections.is_empty());
    }

    #[test]
    fn segment_intersection_order_rational_orders_plane_crossings() {
        let segment = [vertex(0, 0, 0, -2), vertex(1, 0, 0, 2)];
        let lower_triangle = [
            vertex(2, -1, -1, -1),
            vertex(3, 1, -1, -1),
            vertex(4, 0, 1, -1),
        ];
        let upper_triangle = [
            vertex(5, -1, -1, 1),
            vertex(6, 1, -1, 1),
            vertex(7, 0, 1, 1),
        ];

        assert_eq!(
            segment_intersection_order_rational(segment, lower_triangle, upper_triangle),
            Some(true)
        );
        assert_eq!(
            segment_intersection_order_rational(
                [segment[1], segment[0]],
                lower_triangle,
                upper_triangle,
            ),
            Some(false)
        );
    }

    #[test]
    fn segment_intersection_order_rational_defers_equal_crossings() {
        let segment = [vertex(0, 0, 0, -2), vertex(1, 0, 0, 2)];
        let triangle = [
            vertex(2, -1, -1, 0),
            vertex(3, 1, -1, 0),
            vertex(4, 0, 1, 0),
        ];

        assert_eq!(
            segment_intersection_order_rational(segment, triangle, triangle),
            None
        );
    }

    #[test]
    fn precise_orientation_rejects_duplicate_symbolic_ids() {
        assert_eq!(
            orient3d_precise_sos_positive([
                vertex(0, 0, 0, 0),
                vertex(1, 1, 0, 0),
                vertex(1, 0, 1, 0),
                vertex(2, 0, 0, 1),
            ]),
            None
        );
    }
}
