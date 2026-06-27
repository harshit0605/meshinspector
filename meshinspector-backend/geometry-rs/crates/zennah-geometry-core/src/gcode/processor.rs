use super::arc::{arc_points_from_center, arc_points_from_radius};
use super::parser::Command;
use super::{
    GcodeMachineSettings, GcodePathResult, GcodePathSegment, WorkPlane, DEFAULT_FEEDRATE,
    INCH_TO_MM, ROTATION_SAMPLE_POINTS,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum MoveMode {
    Rapid,
    Linear,
    ClockwiseArc,
    CounterClockwiseArc,
}

impl MoveMode {
    fn is_idle(self) -> bool {
        matches!(self, Self::Rapid)
    }

    fn is_arc(self) -> bool {
        matches!(self, Self::ClockwiseArc | Self::CounterClockwiseArc)
    }

    fn clockwise(self) -> bool {
        matches!(self, Self::ClockwiseArc)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CoordType {
    Absolute,
    Relative,
}

#[derive(Debug, Clone)]
struct FrameInput {
    move_mode: Option<MoveMode>,
    target_position: [Option<f64>; 3],
    target_rotation: [Option<f64>; 3],
    center_offset: [Option<f64>; 3],
    radius: Option<f64>,
    feedrate: Option<f64>,
    go_home: bool,
    update_scale: bool,
    next_scale: [Option<f64>; 3],
    reset_scale: bool,
}

impl Default for FrameInput {
    fn default() -> Self {
        Self {
            move_mode: None,
            target_position: [None, None, None],
            target_rotation: [None, None, None],
            center_offset: [None, None, None],
            radius: None,
            feedrate: None,
            go_home: false,
            update_scale: false,
            next_scale: [None, None, None],
            reset_scale: false,
        }
    }
}

pub(super) struct GcodeProcessor {
    settings: GcodeMachineSettings,
    position: [f64; 3],
    rotation: [f64; 3],
    scale: [f64; 3],
    unit_scale: f64,
    coord_type: CoordType,
    move_mode: MoveMode,
    work_plane: WorkPlane,
    feedrate: f64,
    pub(super) max_feedrate: f64,
}

impl GcodeProcessor {
    pub(super) fn new_with_settings(settings: GcodeMachineSettings) -> Self {
        let settings = settings.sanitized();
        Self {
            position: settings.home_position,
            settings,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            unit_scale: 1.0,
            coord_type: CoordType::Absolute,
            move_mode: MoveMode::Rapid,
            work_plane: WorkPlane::Xy,
            feedrate: DEFAULT_FEEDRATE,
            max_feedrate: 0.0,
        }
    }

    pub(super) fn process_frame(
        &mut self,
        commands: &[Command],
        frame_index: usize,
        output: &mut GcodePathResult,
    ) {
        let input = self.frame_input(commands);
        if input.reset_scale {
            self.scale = [1.0, 1.0, 1.0];
        }
        if input.update_scale {
            self.apply_scale_update(input.next_scale);
            return;
        }

        if let Some(move_mode) = input.move_mode {
            self.move_mode = move_mode;
        }
        if let Some(feedrate) = input.feedrate {
            self.feedrate = feedrate;
            if !self.move_mode.is_idle() {
                self.max_feedrate = self.max_feedrate.max(feedrate);
            }
        }

        if input.go_home {
            let start_position = self.position;
            let start_rotation = self.rotation;
            self.position = self.settings.home_position;
            self.push_segment(
                start_position,
                start_rotation,
                self.position,
                self.rotation,
                true,
                self.settings.feedrate_idle,
                frame_index,
                output,
            );
            return;
        }

        let start_position = self.position;
        let start_rotation = self.rotation;
        let end_position = self.target_position(&input);
        let end_rotation = self.target_rotation(&input);

        if let Some(warning) = self
            .settings
            .rotation_limit_warning(start_rotation, end_rotation)
        {
            output
                .warnings
                .push(format!("frame {frame_index}: {warning}"));
        }

        let has_position_command = input.target_position.iter().any(Option::is_some);
        let has_rotation_command = input.target_rotation.iter().any(Option::is_some);

        if self.move_mode.is_arc() {
            self.process_arc(
                start_position,
                start_rotation,
                end_position,
                end_rotation,
                &input,
                frame_index,
                output,
            );
        } else if has_position_command || has_rotation_command {
            self.process_linear(
                start_position,
                start_rotation,
                end_position,
                end_rotation,
                frame_index,
                output,
            );
        }

        self.position = end_position;
        self.rotation = end_rotation;
        if !self.move_mode.is_idle() {
            self.max_feedrate = self.max_feedrate.max(self.feedrate);
        }
    }

    fn frame_input(&mut self, commands: &[Command]) -> FrameInput {
        let mut input = FrameInput::default();
        for command in commands {
            match command.key {
                b'G' => self.apply_g_command(command.value, &mut input),
                b'X' | b'Y' | b'Z' => {
                    let axis = usize::from(command.key - b'X');
                    if input.update_scale {
                        if command.value != 0.0 {
                            input.next_scale[axis] = Some(command.value);
                        }
                    } else {
                        input.target_position[axis] =
                            Some(command.value * self.unit_scale * self.scale[axis]);
                    }
                }
                b'A' | b'B' | b'C' => {
                    let axis = usize::from(command.key - b'A');
                    input.target_rotation[axis] = Some(command.value);
                }
                b'I' | b'J' | b'K' => {
                    let axis = usize::from(command.key - b'I');
                    input.center_offset[axis] =
                        Some(command.value * self.unit_scale * self.scale[axis]);
                }
                b'R' => input.radius = Some(command.value * self.unit_scale),
                b'F' => input.feedrate = Some(command.value),
                _ => {}
            }
        }
        input
    }

    fn apply_g_command(&mut self, value: f64, input: &mut FrameInput) {
        let code = value.trunc() as i32;
        match code {
            0 => input.move_mode = Some(MoveMode::Rapid),
            1 => input.move_mode = Some(MoveMode::Linear),
            2 => input.move_mode = Some(MoveMode::ClockwiseArc),
            3 => input.move_mode = Some(MoveMode::CounterClockwiseArc),
            17 => self.work_plane = WorkPlane::Xy,
            18 => self.work_plane = WorkPlane::Zx,
            19 => self.work_plane = WorkPlane::Yz,
            20 => self.unit_scale = INCH_TO_MM,
            21 => self.unit_scale = 1.0,
            28 => input.go_home = true,
            50 => input.reset_scale = true,
            51 => input.update_scale = true,
            90 => self.coord_type = CoordType::Absolute,
            91 => self.coord_type = CoordType::Relative,
            _ => {}
        }
    }

    fn apply_scale_update(&mut self, update: [Option<f64>; 3]) {
        for (axis, value) in update.into_iter().enumerate() {
            if let Some(value) = value {
                self.scale[axis] = value;
            }
        }
    }

    fn target_position(&self, input: &FrameInput) -> [f64; 3] {
        let mut target = self.position;
        for (axis, value) in input.target_position.iter().enumerate() {
            let Some(value) = value else {
                continue;
            };
            target[axis] = match self.coord_type {
                CoordType::Absolute => *value,
                CoordType::Relative => target[axis] + *value,
            };
        }
        target
    }

    fn target_rotation(&self, input: &FrameInput) -> [f64; 3] {
        let mut target = self.rotation;
        for (axis, value) in input.target_rotation.iter().enumerate() {
            let Some(value) = value else {
                continue;
            };
            target[axis] = match self.coord_type {
                CoordType::Absolute => *value,
                CoordType::Relative => target[axis] + *value,
            };
        }
        target
    }

    fn process_linear(
        &mut self,
        start_position: [f64; 3],
        start_rotation: [f64; 3],
        end_position: [f64; 3],
        end_rotation: [f64; 3],
        frame_index: usize,
        output: &mut GcodePathResult,
    ) {
        if start_rotation == end_rotation {
            self.push_segment(
                start_position,
                start_rotation,
                end_position,
                end_rotation,
                self.move_mode.is_idle(),
                self.segment_feedrate(),
                frame_index,
                output,
            );
            return;
        }

        let mut previous_position = start_position;
        let mut previous_rotation = start_rotation;
        let segment_count = ROTATION_SAMPLE_POINTS - 1;
        for step in 1..=segment_count {
            let t = step as f64 / segment_count as f64;
            let next_position = interpolate3(start_position, end_position, t);
            let next_rotation = interpolate3(start_rotation, end_rotation, t);
            self.push_segment(
                previous_position,
                previous_rotation,
                next_position,
                next_rotation,
                self.move_mode.is_idle(),
                self.segment_feedrate(),
                frame_index,
                output,
            );
            previous_position = next_position;
            previous_rotation = next_rotation;
        }
    }

    fn process_arc(
        &mut self,
        start_position: [f64; 3],
        start_rotation: [f64; 3],
        end_position: [f64; 3],
        end_rotation: [f64; 3],
        input: &FrameInput,
        frame_index: usize,
        output: &mut GcodePathResult,
    ) {
        if start_position == end_position && start_rotation == end_rotation {
            return;
        }

        let mut center = start_position;
        let has_center = input.center_offset.iter().any(Option::is_some);
        let action = if has_center {
            for (axis, value) in input.center_offset.iter().enumerate() {
                if let Some(value) = value {
                    center[axis] += *value;
                }
            }
            arc_points_from_center(
                center,
                start_position,
                end_position,
                self.move_mode.clockwise(),
                self.work_plane,
            )
        } else if let Some(radius) = input.radius {
            arc_points_from_radius(
                start_position,
                end_position,
                radius,
                self.move_mode.clockwise(),
                self.work_plane,
            )
        } else {
            return;
        };

        if let Some(warning) = action.warning {
            output
                .warnings
                .push(format!("frame {frame_index}: {warning}"));
        }

        let segment_count = action.points.len().saturating_sub(1);
        if segment_count == 0 {
            return;
        }

        for (index, points) in action.points.windows(2).enumerate() {
            let start_t = index as f64 / segment_count as f64;
            let end_t = (index + 1) as f64 / segment_count as f64;
            self.push_segment(
                points[0],
                interpolate3(start_rotation, end_rotation, start_t),
                points[1],
                interpolate3(start_rotation, end_rotation, end_t),
                false,
                self.segment_feedrate(),
                frame_index,
                output,
            );
        }
    }

    fn push_segment(
        &self,
        start_position: [f64; 3],
        start_rotation: [f64; 3],
        end_position: [f64; 3],
        end_rotation: [f64; 3],
        idle: bool,
        feedrate: f64,
        source_frame_index: usize,
        output: &mut GcodePathResult,
    ) {
        output.segments.push(GcodePathSegment {
            start: self.transformed_position(start_position, start_rotation),
            end: self.transformed_position(end_position, end_rotation),
            tool_direction_start: self.tool_direction(start_rotation),
            tool_direction_end: self.tool_direction(end_rotation),
            source_frame_index,
            idle,
            feedrate,
        });
    }

    fn segment_feedrate(&self) -> f64 {
        if self.move_mode.is_idle() {
            self.settings.feedrate_idle
        } else {
            self.feedrate
        }
    }

    fn transformed_position(&self, position: [f64; 3], rotation: [f64; 3]) -> [f64; 3] {
        self.apply_rotation_order(position, rotation)
    }

    fn tool_direction(&self, rotation: [f64; 3]) -> [f64; 3] {
        self.apply_rotation_order([0.0, 0.0, 1.0], rotation)
    }

    fn apply_rotation_order(&self, mut value: [f64; 3], rotation: [f64; 3]) -> [f64; 3] {
        for axis in &self.settings.rotation_order {
            let axis_vector = self.settings.rotation_axes[*axis];
            value = rotate_around_axis(value, axis_vector, rotation[*axis]);
        }
        value
    }
}

fn interpolate3(start: [f64; 3], end: [f64; 3], t: f64) -> [f64; 3] {
    [
        start[0] + (end[0] - start[0]) * t,
        start[1] + (end[1] - start[1]) * t,
        start[2] + (end[2] - start[2]) * t,
    ]
}

fn rotate_around_axis(point: [f64; 3], axis: [f64; 3], angle_degrees: f64) -> [f64; 3] {
    if angle_degrees == 0.0 {
        return point;
    }
    let angle = angle_degrees.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    let dot = point[0] * axis[0] + point[1] * axis[1] + point[2] * axis[2];
    let cross = [
        axis[1] * point[2] - axis[2] * point[1],
        axis[2] * point[0] - axis[0] * point[2],
        axis[0] * point[1] - axis[1] * point[0],
    ];

    [
        point[0] * cos + cross[0] * sin + axis[0] * dot * (1.0 - cos),
        point[1] * cos + cross[1] * sin + axis[1] * dot * (1.0 - cos),
        point[2] * cos + cross[2] * sin + axis[2] * dot * (1.0 - cos),
    ]
}
