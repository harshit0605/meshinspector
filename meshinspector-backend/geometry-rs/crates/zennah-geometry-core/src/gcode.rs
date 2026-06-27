const INCH_TO_MM: f64 = 25.4;
const DEFAULT_FEEDRATE: f64 = 100.0;
const DEFAULT_IDLE_FEEDRATE: f64 = 10_000.0;
const ACCURACY: f64 = 1.0e-3;
const ARC_STEP_RADIANS: f64 = std::f64::consts::PI / 30.0;
const ROTATION_SAMPLE_POINTS: usize = 21;

mod arc;
mod parser;
mod processor;
mod settings;
mod source;

use processor::GcodeProcessor;
pub use settings::GcodeMachineSettings;
pub use source::{
    load_gcode_source, parse_gcode_file_paths, parse_gcode_file_paths_with_settings,
    write_gcode_source,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcodePathSegment {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub tool_direction_start: [f64; 3],
    pub tool_direction_end: [f64; 3],
    pub source_frame_index: usize,
    pub idle: bool,
    pub feedrate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GcodePathResult {
    pub segments: Vec<GcodePathSegment>,
    pub frame_count: usize,
    pub command_count: usize,
    pub max_feedrate: f64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum WorkPlane {
    Xy,
    Zx,
    Yz,
}

impl WorkPlane {
    fn to_work(self, point: [f64; 3]) -> [f64; 3] {
        match self {
            Self::Xy => point,
            Self::Zx => [point[2], point[0], point[1]],
            Self::Yz => [point[1], point[2], point[0]],
        }
    }

    fn from_work(self, point: [f64; 3]) -> [f64; 3] {
        match self {
            Self::Xy => point,
            Self::Zx => [point[1], point[2], point[0]],
            Self::Yz => [point[2], point[0], point[1]],
        }
    }
}

pub fn parse_gcode_paths(source: &str) -> Result<GcodePathResult, String> {
    parse_gcode_paths_with_settings(source, &GcodeMachineSettings::default())
}

pub fn parse_gcode_paths_with_settings(
    source: &str,
    settings: &GcodeMachineSettings,
) -> Result<GcodePathResult, String> {
    let frames = split_gcode_source_frames(source);
    parse_gcode_source_frames_with_settings(&frames, settings)
}

pub(super) fn split_gcode_source_frames(source: &str) -> Vec<&str> {
    source.split('\n').filter(|line| !line.is_empty()).collect()
}

pub fn parse_gcode_source_frames_with_settings<S: AsRef<str>>(
    frames: &[S],
    settings: &GcodeMachineSettings,
) -> Result<GcodePathResult, String> {
    let mut output = GcodePathResult {
        segments: Vec::new(),
        frame_count: frames.len(),
        command_count: 0,
        max_feedrate: 0.0,
        warnings: Vec::new(),
    };
    let mut processor = GcodeProcessor::new_with_settings(settings.clone());
    for (frame_index, frame) in frames.iter().enumerate() {
        let commands = parser::parse_frame(frame.as_ref());
        output.command_count += commands.len();
        processor.process_frame(&commands, frame_index, &mut output);
    }
    output.max_feedrate = processor.max_feedrate;
    for segment in &mut output.segments {
        if segment.idle && segment.feedrate == 0.0 {
            segment.feedrate = processor.max_feedrate;
        }
    }
    Ok(output)
}
