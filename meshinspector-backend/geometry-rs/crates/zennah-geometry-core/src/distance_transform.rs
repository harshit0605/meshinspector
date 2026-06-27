use crate::math::scale;

pub(crate) fn axis_aligned_distance_map_model_transform(
    origin: [f64; 2],
    pixel_size: [f64; 2],
) -> [f64; 16] {
    [
        pixel_size[0],
        0.0,
        0.0,
        origin[0],
        0.0,
        pixel_size[1],
        0.0,
        origin[1],
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

pub(crate) fn distance_map_model_transform_from_frame(
    origin: [f64; 3],
    x_range: [f64; 3],
    y_range: [f64; 3],
    direction: [f64; 3],
    width: usize,
    height: usize,
) -> [f64; 16] {
    let pixel_x_vec = scale(x_range, 1.0 / width as f64);
    let pixel_y_vec = scale(y_range, 1.0 / height as f64);
    [
        pixel_x_vec[0],
        pixel_y_vec[0],
        direction[0],
        origin[0],
        pixel_x_vec[1],
        pixel_y_vec[1],
        direction[1],
        origin[1],
        pixel_x_vec[2],
        pixel_y_vec[2],
        direction[2],
        origin[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}
