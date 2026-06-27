#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetContourIndex {
    pub contour_id: i32,
    pub vert_id: i32,
}

impl OffsetContourIndex {
    pub fn unknown() -> Self {
        Self {
            contour_id: -1,
            vert_id: -1,
        }
    }

    pub fn valid(&self) -> bool {
        self.contour_id >= 0 && self.vert_id >= 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetContoursOrigin {
    pub l_org: OffsetContourIndex,
    pub l_dest: OffsetContourIndex,
    pub u_org: OffsetContourIndex,
    pub u_dest: OffsetContourIndex,
    pub l_ratio: f64,
    pub u_ratio: f64,
}

impl OffsetContoursOrigin {
    pub fn source_vertex(contour_id: usize, vert_id: usize) -> Self {
        Self {
            l_org: OffsetContourIndex {
                contour_id: contour_id as i32,
                vert_id: vert_id as i32,
            },
            l_dest: OffsetContourIndex::unknown(),
            u_org: OffsetContourIndex::unknown(),
            u_dest: OffsetContourIndex::unknown(),
            l_ratio: 0.0,
            u_ratio: 0.0,
        }
    }

    pub fn is_intersection(&self) -> bool {
        self.l_dest.valid()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OffsetContoursResult {
    pub contours: Vec<Vec<[f64; 3]>>,
    pub origins: Vec<Vec<OffsetContoursOrigin>>,
}

fn identity_contour_with_origins(
    contour: &[[f64; 3]],
    contour_id: usize,
) -> (Vec<[f64; 3]>, Vec<OffsetContoursOrigin>) {
    let origins = (0..contour.len())
        .map(|index| {
            OffsetContoursOrigin::source_vertex(
                contour_id,
                if index + 1 == contour.len() { 0 } else { index },
            )
        })
        .collect();
    (contour.to_vec(), origins)
}

pub fn offset_contours_with_options_and_z_callback<F>(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
    relax_iterations: usize,
    callback: F,
) -> Result<Vec<Vec<[f64; 3]>>, String>
where
    F: Fn(&[Vec<[f64; 3]>], OffsetContourIndex, &OffsetContoursOrigin) -> Result<f64, String>,
{
    Ok(offset_contours_with_options_and_origins_and_z_callback(
        contours,
        offset,
        options,
        relax_iterations,
        callback,
    )?
    .contours)
}

pub fn offset_contours_with_options_and_origins_and_z_callback<F>(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
    relax_iterations: usize,
    callback: F,
) -> Result<OffsetContoursResult, String>
where
    F: Fn(&[Vec<[f64; 3]>], OffsetContourIndex, &OffsetContoursOrigin) -> Result<f64, String>,
{
    let mut result = offset_contours_with_options_and_origins_and_z_options(
        contours,
        offset,
        options,
        OffsetContoursZOptions {
            restore_mode: super::OffsetContoursZRestoreMode::Default,
            relax_iterations: 0,
        },
    )?;
    apply_z_callback(&mut result, relax_iterations, callback)?;
    Ok(result)
}

pub fn offset_contours_with_variable_offsets_and_z_callback<F>(
    contours: &[Vec<[f64; 3]>],
    offsets: &[Vec<f64>],
    options: OffsetContoursOptions,
    relax_iterations: usize,
    callback: F,
) -> Result<Vec<Vec<[f64; 3]>>, String>
where
    F: Fn(&[Vec<[f64; 3]>], OffsetContourIndex, &OffsetContoursOrigin) -> Result<f64, String>,
{
    Ok(offset_contours_with_variable_offsets_and_origins_and_z_callback(
        contours,
        offsets,
        options,
        relax_iterations,
        callback,
    )?
    .contours)
}

pub fn offset_contours_with_variable_offsets_and_origins_and_z_callback<F>(
    contours: &[Vec<[f64; 3]>],
    offsets: &[Vec<f64>],
    options: OffsetContoursOptions,
    relax_iterations: usize,
    callback: F,
) -> Result<OffsetContoursResult, String>
where
    F: Fn(&[Vec<[f64; 3]>], OffsetContourIndex, &OffsetContoursOrigin) -> Result<f64, String>,
{
    let mut result = offset_contours_with_variable_offsets_and_origins_and_z_options(
        contours,
        offsets,
        options,
        OffsetContoursZOptions {
            restore_mode: super::OffsetContoursZRestoreMode::Default,
            relax_iterations: 0,
        },
    )?;
    apply_z_callback(&mut result, relax_iterations, callback)?;
    Ok(result)
}

fn apply_z_callback<F>(
    result: &mut OffsetContoursResult,
    relax_iterations: usize,
    callback: F,
) -> Result<(), String>
where
    F: Fn(&[Vec<[f64; 3]>], OffsetContourIndex, &OffsetContoursOrigin) -> Result<f64, String>,
{
    let offset_contours = result.contours.clone();
    for (contour_id, (contour, origins)) in result.contours.iter_mut().zip(&result.origins).enumerate() {
        if contour.len() != origins.len() {
            return Err("OffsetContours zCallback requires origin map length parity".to_string());
        }
        for (vert_id, (point, origin)) in contour.iter_mut().zip(origins).enumerate() {
            point[2] = callback(
                &offset_contours,
                OffsetContourIndex {
                    contour_id: contour_id as i32,
                    vert_id: vert_id as i32,
                },
                origin,
            )?;
            if !point[2].is_finite() {
                return Err("OffsetContours zCallback must return finite values".to_string());
            }
        }
    }
    super::relax_offset_contours_z(&mut result.contours, relax_iterations);
    Ok(())
}

pub fn offset_contours_with_options_and_origins(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
) -> Result<OffsetContoursResult, String> {
    offset_contours_with_options_and_origins_and_z_options(
        contours,
        offset,
        options,
        OffsetContoursZOptions::default(),
    )
}

pub fn offset_contours_with_options_and_origins_and_z_options(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
    z_options: OffsetContoursZOptions,
) -> Result<OffsetContoursResult, String> {
    if !offset.is_finite() {
        return Err("OffsetContours offset must be finite".to_string());
    }
    if !options.min_angle_precision.is_finite() || options.min_angle_precision <= 0.0 {
        return Err("OffsetContours min_angle_precision must be finite and positive".to_string());
    }
    if !options.max_sharp_angle.is_finite() {
        return Err("OffsetContours max_sharp_angle must be finite".to_string());
    }
    if offset == 0.0 && options.mode == OffsetContoursMode::Offset {
        let mut output = OffsetContoursResult {
            contours: Vec::new(),
            origins: Vec::new(),
        };
        for (contour_id, contour) in contours.iter().enumerate() {
            if contour.is_empty() {
                continue;
            }
            if !is_closed_contour(contour) {
                return Err(
                    "OffsetContours origins are currently supported for non-zero closed contours"
                        .to_string(),
                );
            }
            let (contour_points, contour_origins) =
                identity_contour_with_origins(contour, contour_id);
            output.contours.push(contour_points);
            output.origins.push(contour_origins);
        }
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }
    if offset == 0.0 {
        return Err(
            "OffsetContours origins are currently supported for non-zero closed contours"
                .to_string(),
        );
    }
    if let Some(mut output) =
        offset_open_cut_axis_aligned_crossing_origins(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }
    if let Some(mut output) =
        offset_open_cut_horizontal_collinear_overlapping_origins(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }
    if let Some(mut output) =
        offset_open_cut_vertical_collinear_overlapping_origins(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }
    if let Some(mut output) =
        offset_open_cut_collinear_touching_origins(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }
    if let Some(mut output) =
        offset_open_cut_non_axis_collinear_overlapping_origins(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }
    if let Some(mut output) =
        offset_open_cut_horizontal_overlapping_parallel_origins(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }
    if let Some(mut output) =
        offset_open_cut_non_axis_overlapping_parallel_origins(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
        return Ok(output);
    }

    let mut output = OffsetContoursResult {
        contours: Vec::new(),
        origins: Vec::new(),
    };
    for (contour_id, contour) in contours.iter().enumerate() {
        if contour.is_empty() {
            continue;
        }
        if !is_closed_contour(contour) {
            let (contour_points, contour_origins) = offset_open_contour_with_origins(
                contour,
                contour_id,
                &vec![offset.abs(); contour.len()],
                options.min_angle_precision,
                options.end_type,
                options.corner_type,
                options.max_sharp_angle,
            )?;
            output.contours.push(contour_points);
            output.origins.push(contour_origins);
            continue;
        }
        let contour_results = match options.mode {
            OffsetContoursMode::Offset => {
                let result = if offset > 0.0 {
                    offset_closed_clockwise_contour_with_origins(
                        contour,
                        contour_id,
                        offset,
                        options.min_angle_precision,
                        options.corner_type,
                        options.max_sharp_angle,
                    )?
                } else {
                    offset_closed_clockwise_negative_contour_with_origins(
                        contour, contour_id, offset,
                    )?
                };
                vec![result]
            }
            OffsetContoursMode::Shell => offset_closed_clockwise_shell_contours_with_origins(
                contour,
                contour_id,
                offset,
                options.min_angle_precision,
                options.corner_type,
                options.max_sharp_angle,
            )?,
        };
        for (contour_points, contour_origins) in contour_results {
            output.contours.push(contour_points);
            output.origins.push(contour_origins);
        }
    }
    apply_offset_contours_z_options(contours, &mut output.contours, &z_options)?;
    Ok(output)
}
