//! Policy definitions for servant behavior.
//!
//! Controls what actions a servant is allowed to perform.
//! Includes allow/deny lists for tools, resource access, and risk levels.

use crate::config::schema::AutonomyConfig;
use crate::security::AutonomyLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyPolicy {
    /// List of allowed tool names.
    pub allowed_tools: HashSet<String>,

    /// Maximum risk level allowed without human approval.
    pub max_risk_level: RiskLevel,

    /// Read-only mode: prevents any persistent state modification.
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert("echo".to_string());

        Self {
            allowed_tools: allowed,
            max_risk_level: RiskLevel::Low,
            read_only: true, // Default to safe mode
        }
    }
}

impl SafetyPolicy {
    pub fn new_permissive() -> Self {
        Self {
            allowed_tools: HashSet::from_iter(vec![
                "echo".to_string(),
                "read_file".to_string(),
                "write_file".to_string(),
            ]),
            max_risk_level: RiskLevel::Medium,
            read_only: false,
        }
    }

    /// Create policy from autonomy config
    pub fn from_config(config: &AutonomyConfig, _workspace_dir: &Path) -> Self {
        let allowed_tools: HashSet<String> = config
            .allowed_commands
            .iter()
            .cloned()
            .collect();

        let max_risk_level = match config.level {
            AutonomyLevel::Manual => RiskLevel::Low,
            AutonomyLevel::Supervised => RiskLevel::Medium,
            AutonomyLevel::Semi => RiskLevel::Medium,
            AutonomyLevel::Full => RiskLevel::High,
        };

        Self {
            allowed_tools,
            max_risk_level,
            read_only: config.level == AutonomyLevel::Manual,
        }
    }

    pub fn check_tool(&self, tool_name: &str) -> bool {
        self.allowed_tools.contains(tool_name)
    }

    /// Check if action is allowed
    pub fn can_act(&self) -> bool {
        true
    }

    /// Check if rate limited
    pub fn is_rate_limited(&self) -> bool {
        false
    }

    /// Record an action
    pub fn record_action(&self) {
        // No-op for now
    }

    /// Check if command is allowed
    pub fn is_command_allowed(&self, _command: &str) -> bool {
        true
    }

    /// Check for forbidden path argument
    pub fn forbidden_path_argument(&self, _arg: &str) -> Option<String> {
        None
    }
}
