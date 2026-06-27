use crate::GeometryError;
use std::collections::BTreeSet;

pub fn apply_meshlib_selection_modifier(
    current_ids: &[usize],
    incoming_ids: &[usize],
    mode: &str,
    item_count: Option<usize>,
) -> Result<Vec<usize>, GeometryError> {
    validate_selection_ids(current_ids, item_count, "current_ids")?;
    validate_selection_ids(incoming_ids, item_count, "incoming_ids")?;

    let current: BTreeSet<usize> = current_ids.iter().copied().collect();
    let incoming: BTreeSet<usize> = incoming_ids.iter().copied().collect();
    let normalized = mode.trim().to_ascii_lowercase().replace(['-', ' '], "_");

    let output: BTreeSet<usize> = match normalized.as_str() {
        "" | "replace" | "select" | "select_one" | "primary_click" => incoming,
        "add" | "append" | "union" | "extend" => current.union(&incoming).copied().collect(),
        "subtract" | "remove" | "erase" | "difference" => {
            current.difference(&incoming).copied().collect()
        }
        "toggle" | "xor" | "primary_ctrl" | "primary_control" | "ctrl" | "control" | "cmd"
        | "command" => current.symmetric_difference(&incoming).copied().collect(),
        _ => {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "selection_modifier",
                value: format!("unsupported MeshInspector selection modifier: {mode}"),
            });
        }
    };
    Ok(output.into_iter().collect())
}

fn validate_selection_ids(
    ids: &[usize],
    item_count: Option<usize>,
    field: &'static str,
) -> Result<(), GeometryError> {
    let Some(item_count) = item_count else {
        return Ok(());
    };
    for id in ids {
        if *id >= item_count {
            return Err(GeometryError::InvalidSelectionParameter {
                field,
                value: format!("id {id} is outside item count {item_count}"),
            });
        }
    }
    Ok(())
}
