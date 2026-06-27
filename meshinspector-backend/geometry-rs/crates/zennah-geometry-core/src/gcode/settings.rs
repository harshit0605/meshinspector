use super::DEFAULT_IDLE_FEEDRATE;

#[derive(Debug, Clone, PartialEq)]
pub struct GcodeMachineSettings {
    pub home_position: [f64; 3],
    pub feedrate_idle: f64,
    pub rotation_axes: [[f64; 3]; 3],
    pub rotation_order: Vec<usize>,
    pub rotation_limits: [Option<[f64; 2]>; 3],
}

impl Default for GcodeMachineSettings {
    fn default() -> Self {
        Self {
            home_position: [0.0, 0.0, 0.0],
            feedrate_idle: DEFAULT_IDLE_FEEDRATE,
            rotation_axes: [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
            rotation_order: vec![0, 1, 2],
            rotation_limits: [None, None, None],
        }
    }
}

impl GcodeMachineSettings {
    pub fn sanitized(&self) -> Self {
        let defaults = Self::default();
        let mut rotation_axes = defaults.rotation_axes;
        for (index, axis) in self.rotation_axes.iter().enumerate() {
            if let Some(normalized) = normalize(*axis) {
                rotation_axes[index] = normalized;
            }
        }

        let mut rotation_order = Vec::with_capacity(self.rotation_order.len());
        for axis in &self.rotation_order {
            if *axis < 3 && !rotation_order.contains(axis) {
                rotation_order.push(*axis);
            }
        }

        Self {
            home_position: self.home_position,
            feedrate_idle: self.feedrate_idle.clamp(0.0, 100_000.0),
            rotation_axes,
            rotation_order,
            rotation_limits: self.rotation_limits.map(sanitize_rotation_limit),
        }
    }

    pub(super) fn rotation_limit_warning(&self, start: [f64; 3], end: [f64; 3]) -> Option<String> {
        for axis in &self.rotation_order {
            let Some([min, max]) = self.rotation_limits[*axis] else {
                continue;
            };
            let start_angle = start[*axis];
            let end_angle = end[*axis];
            if start_angle < min || start_angle > max || end_angle < min || end_angle > max {
                return Some("Error input angle: Going beyond the limits.".to_string());
            }
        }
        None
    }
}

fn sanitize_rotation_limit(limit: Option<[f64; 2]>) -> Option<[f64; 2]> {
    let [min, max] = limit?;
    if min > max {
        return None;
    }
    Some([min.max(-180.0), max.min(180.0)])
}

fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length_sq = vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2];
    if length_sq == 0.0 {
        return None;
    }
    let length = length_sq.sqrt();
    Some([vector[0] / length, vector[1] / length, vector[2] / length])
}
