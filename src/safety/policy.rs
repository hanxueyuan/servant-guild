//! Policy definitions for servant behavior.
//!
//! Controls what actions a servant is allowed to perform.
//! Includes allow/deny lists for tools, resource access, and risk levels.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

    pub fn check_tool(&self, tool_name: &str) -> bool {
        self.allowed_tools.contains(tool_name)
    }
}
