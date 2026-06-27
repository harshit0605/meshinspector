mod negative;
mod open;
mod shell;

use super::math::{
    add2, contour_normal, find_angle, insert_round_corner, is_closed_contour, rotate_around,
    same_xy, scale2, signed_area_xy, RoundCornerParams,
};
use super::sharp::{insert_sharp_corner, SharpCornerParams};
use super::{
    apply_offset_contours_z_options,
    closed_variable::offset_closed_clockwise_signed_inward_contour, OffsetContoursCornerType,
    OffsetContoursEndType, OffsetContoursMode, OffsetContoursOptions, OffsetContoursZOptions,
};
use crate::lines::validate_contour;
use negative::{
    canonical_ascending_edge, negative_intersection_origin, negative_variable_intersection_origin,
    source_edge_angle, source_edge_ratio, source_index, SourceEdge,
};
use open::offset_open_contour_with_origins;
use shell::{
    offset_closed_clockwise_shell_contours_with_origins,
    offset_closed_clockwise_variable_shell_contours_with_origins,
};

include!("origins/api.rs");
include!("origins/open_axis.rs");
include!("origins/open_collinear.rs");
include!("origins/open_non_axis.rs");
include!("origins/open_parallel.rs");
include!("origins/closed.rs");
include!("origins/outline.rs");
include!("origins/negative_offsets.rs");
