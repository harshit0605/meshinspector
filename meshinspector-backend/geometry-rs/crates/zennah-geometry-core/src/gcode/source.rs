use super::{parse_gcode_source_frames_with_settings, GcodeMachineSettings, GcodePathResult};
use std::path::Path;

pub fn load_gcode_source(path: impl AsRef<Path>) -> Result<Vec<String>, String> {
    let path = path.as_ref();
    validate_gcode_path(path)?;
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("Cannot read G-code file: {}: {error}", path.display()))?;
    Ok(super::split_gcode_source_frames(&source)
        .into_iter()
        .map(str::to_string)
        .collect())
}

pub fn write_gcode_source(source: &[String], path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    validate_gcode_path(path)?;
    let mut output = String::new();
    for frame in source {
        output.push_str(frame);
        output.push('\n');
    }
    std::fs::write(path, output)
        .map_err(|error| format!("Cannot write G-code file: {}: {error}", path.display()))
}

pub fn parse_gcode_file_paths(path: impl AsRef<Path>) -> Result<GcodePathResult, String> {
    let frames = load_gcode_source(path)?;
    parse_gcode_source_frames_with_settings(&frames, &GcodeMachineSettings::default())
}

pub fn parse_gcode_file_paths_with_settings(
    path: impl AsRef<Path>,
    settings: &GcodeMachineSettings,
) -> Result<GcodePathResult, String> {
    let frames = load_gcode_source(path)?;
    parse_gcode_source_frames_with_settings(&frames, settings)
}

fn validate_gcode_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("Path is empty".to_string());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "gcode" | "nc" | "txt") {
        Ok(())
    } else {
        Err(format!(
            "Unsupported G-code file extension: {}",
            path.display()
        ))
    }
}
