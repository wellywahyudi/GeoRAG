use crate::output::OutputWriter;
use serde::Serialize;

/// Represents a planned action in dry-run mode
#[derive(Debug, Clone, Serialize)]
pub struct PlannedAction {
    pub action_type: ActionType,
    pub description: String,
    pub details: Vec<String>,
}

/// Types of actions that can be planned
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    CreateDirectory,
    CreateFile,
    WriteFile,
    CopyFile,
    ModifyFile,
}

impl PlannedAction {
    /// Create a new planned action
    pub fn new(action_type: ActionType, description: impl Into<String>) -> Self {
        Self {
            action_type,
            description: description.into(),
            details: Vec::new(),
        }
    }

    /// Add a detail to the planned action
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.details.push(detail.into());
        self
    }
}

/// Display planned actions in dry-run mode
pub fn display_planned_actions(output: &OutputWriter, actions: &[PlannedAction]) {
    if output.is_json() {
        // For JSON output, serialize the actions
        let _ = output.result(serde_json::json!({
            "dry_run": true,
            "planned_actions": actions,
        }));
    } else {
        // For human output, display each action
        output.section("Planned Actions (Dry Run)");
        for (i, action) in actions.iter().enumerate() {
            output.info(format!("{}. {:?}: {}", i + 1, action.action_type, action.description));
            for detail in &action.details {
                output.info(format!("   - {}", detail));
            }
        }
        output.info("\nNo changes were made. Run without --dry-run to execute these actions.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planned_action_creation() {
        let action = PlannedAction::new(ActionType::CreateFile, "Create config.toml")
            .with_detail("Path: .georag/config.toml")
            .with_detail("Content: workspace configuration");

        assert_eq!(action.description, "Create config.toml");
        assert_eq!(action.details.len(), 2);
    }

    #[test]
    fn test_action_type_serialization() {
        let action = PlannedAction::new(ActionType::WriteFile, "Write data");
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("write_file"));
    }
}
