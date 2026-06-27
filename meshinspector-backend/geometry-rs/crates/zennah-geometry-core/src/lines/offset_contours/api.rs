use super::validate_contour;

mod closed_variable;
mod math;
mod origins;
mod sharp;

use closed_variable::offset_closed_clockwise_signed_inward_contour;
use math::{
    add2, contour_normal, find_angle, insert_round_corner, is_closed_contour, line_intersection_xy,
    restore_adjacent_edge_average_z, rotate_around, same_xy, scale2, signed_area_xy,
    RoundCornerParams,
};
use sharp::{insert_sharp_corner, SharpCornerParams};

pub use origins::{
    offset_contours_with_options_and_origins,
    offset_contours_with_options_and_origins_and_z_callback,
    offset_contours_with_options_and_z_callback,
    offset_contours_with_options_and_origins_and_z_options,
    offset_contours_with_variable_offsets_and_origins,
    offset_contours_with_variable_offsets_and_origins_and_z_callback,
    offset_contours_with_variable_offsets_and_origins_and_z_options,
    offset_contours_with_variable_offsets_and_z_callback, OffsetContourIndex, OffsetContoursOrigin,
    OffsetContoursResult,
};

include!("api/public.rs");
include!("api/global_axis.rs");
include!("api/global_non_axis.rs");
include!("api/local_geometry.rs");
include!("api/axis_aligned_union.rs");
include!("api/z_restore.rs");
