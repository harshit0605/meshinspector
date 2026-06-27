#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetContoursMode {
    Offset,
    Shell,
}

impl OffsetContoursMode {
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "offset" | "Offset" | "type::offset" | "Type::Offset" => Ok(Self::Offset),
            "shell" | "Shell" | "type::shell" | "Type::Shell" => Ok(Self::Shell),
            _ => Err("OffsetContours mode must be 'offset' or 'shell'".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetContoursEndType {
    Round,
    Cut,
}

impl OffsetContoursEndType {
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "round" | "Round" | "endtype::round" | "EndType::Round" => Ok(Self::Round),
            "cut" | "Cut" | "endtype::cut" | "EndType::Cut" => Ok(Self::Cut),
            _ => Err("OffsetContours end_type must be 'round' or 'cut'".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetContoursCornerType {
    Round,
    Sharp,
}

impl OffsetContoursCornerType {
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "round" | "Round" | "cornertype::round" | "CornerType::Round" => Ok(Self::Round),
            "sharp" | "Sharp" | "cornertype::sharp" | "CornerType::Sharp" => Ok(Self::Sharp),
            _ => Err("OffsetContours corner_type must be 'round' or 'sharp'".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetContoursOptions {
    pub mode: OffsetContoursMode,
    pub end_type: OffsetContoursEndType,
    pub corner_type: OffsetContoursCornerType,
    pub min_angle_precision: f64,
    pub max_sharp_angle: f64,
}

impl Default for OffsetContoursOptions {
    fn default() -> Self {
        Self {
            mode: OffsetContoursMode::Offset,
            end_type: OffsetContoursEndType::Round,
            corner_type: OffsetContoursCornerType::Round,
            min_angle_precision: std::f64::consts::PI / 9.0,
            max_sharp_angle: std::f64::consts::PI * 2.0 / 3.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OffsetContoursZRestoreMode {
    Default,
    Constant(f64),
    Custom(Vec<Vec<f64>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OffsetContoursZOptions {
    pub restore_mode: OffsetContoursZRestoreMode,
    pub relax_iterations: usize,
}

impl Default for OffsetContoursZOptions {
    fn default() -> Self {
        Self {
            restore_mode: OffsetContoursZRestoreMode::Default,
            relax_iterations: 1,
        }
    }
}

pub fn offset_contours(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    min_angle_precision: f64,
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    offset_contours_with_options(
        contours,
        offset,
        OffsetContoursOptions {
            mode: OffsetContoursMode::Offset,
            end_type: OffsetContoursEndType::Round,
            corner_type: OffsetContoursCornerType::Round,
            min_angle_precision,
            max_sharp_angle: OffsetContoursOptions::default().max_sharp_angle,
        },
    )
}

pub fn offset_contours_with_options(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    offset_contours_with_options_and_z_options(
        contours,
        offset,
        options,
        OffsetContoursZOptions::default(),
    )
}

pub fn offset_contours_with_options_and_z_options(
    contours: &[Vec<[f64; 3]>],
    offset: f64,
    options: OffsetContoursOptions,
    z_options: OffsetContoursZOptions,
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    if !offset.is_finite() {
        return Err("OffsetContours offset must be finite".to_string());
    }
    if !options.min_angle_precision.is_finite() || options.min_angle_precision <= 0.0 {
        return Err("OffsetContours min_angle_precision must be finite and positive".to_string());
    }
    if !options.max_sharp_angle.is_finite() {
        return Err("OffsetContours max_sharp_angle must be finite".to_string());
    }
    validate_z_options(contours, &z_options)?;

    if let Some(mut global_cut_outlines) =
        offset_open_cut_axis_aligned_collinear_overlapping_global_outlines(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut global_cut_outlines, &z_options)?;
        return Ok(global_cut_outlines);
    }

    if let Some(mut global_cut_outlines) =
        offset_open_cut_collinear_overlapping_global_outlines(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut global_cut_outlines, &z_options)?;
        return Ok(global_cut_outlines);
    }

    if let Some(mut global_cut_outlines) =
        offset_open_cut_collinear_touching_global_outlines(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut global_cut_outlines, &z_options)?;
        return Ok(global_cut_outlines);
    }

    if let Some(mut global_cut_outlines) =
        offset_open_cut_parallel_global_outlines(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut global_cut_outlines, &z_options)?;
        return Ok(global_cut_outlines);
    }

    if let Some(mut global_cut_outlines) =
        offset_open_cut_axis_aligned_global_outlines(contours, offset, options)?
    {
        apply_offset_contours_z_options(contours, &mut global_cut_outlines, &z_options)?;
        return Ok(global_cut_outlines);
    }

    let mut output = Vec::new();
    for contour in contours {
        if contour.is_empty() {
            continue;
        }
        if is_closed_contour(contour) {
            match options.mode {
                OffsetContoursMode::Offset => output.push(offset_closed_clockwise_contour(
                    contour,
                    offset,
                    options.min_angle_precision,
                    options.corner_type,
                    options.max_sharp_angle,
                )?),
                OffsetContoursMode::Shell => output.extend(offset_closed_clockwise_shell_contours(
                    contour,
                    offset,
                    options.min_angle_precision,
                    options.corner_type,
                    options.max_sharp_angle,
                )?),
            }
        } else {
            output.push(offset_open_round_contour(
                contour,
                offset,
                options.min_angle_precision,
                options.end_type,
                options.corner_type,
                options.max_sharp_angle,
            )?);
        }
    }
    apply_offset_contours_z_options(contours, &mut output, &z_options)?;
    Ok(output)
}
