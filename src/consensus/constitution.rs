//! Constitution - Governance Rules for ServantGuild
//!
//! The constitution defines what types of actions require voting,
//! the quorum required, and any special conditions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The constitution defines governance rules for the guild
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constitution {
    /// Name of this constitution version
    pub name: String,
    /// Version number
    pub version: u32,
    /// Rules for different decision types
    pub rules: HashMap<DecisionType, GovernanceRule>,
    /// Whether owner veto is enabled
    pub owner_veto_enabled: bool,
    /// Maximum voting duration in seconds
    pub max_voting_duration_secs: u64,
}

impl Default for Constitution {
    fn default() -> Self {
        let mut rules = HashMap::new();

        // Code changes require normal quorum
        rules.insert(
            DecisionType::CodeChange,
            GovernanceRule {
                requires_vote: true,
                quorum_type: QuorumType::Normal,
                description: "Code modifications require guild approval".to_string(),
            },
        );

        // Configuration changes require normal quorum
        rules.insert(
            DecisionType::ConfigChange,
            GovernanceRule {
                requires_vote: true,
                quorum_type: QuorumType::Normal,
                description: "Configuration changes require guild approval".to_string(),
            },
        );

        // System updates require critical quorum
        rules.insert(
            DecisionType::SystemUpdate,
            GovernanceRule {
                requires_vote: true,
                quorum_type: QuorumType::Critical,
                description: "System updates require unanimous approval".to_string(),
            },
        );

        // Security changes require critical quorum
        rules.insert(
            DecisionType::SecurityChange,
            GovernanceRule {
                requires_vote: true,
                quorum_type: QuorumType::Critical,
                description: "Security policy changes require unanimous approval".to_string(),
            },
        );

        // Member addition requires critical quorum
        rules.insert(
            DecisionType::MemberAdd,
            GovernanceRule {
                requires_vote: true,
                quorum_type: QuorumType::Critical,
                description: "Adding new members requires unanimous approval".to_string(),
            },
        );

        // Member removal requires critical quorum
        rules.insert(
            DecisionType::MemberRemove,
            GovernanceRule {
                requires_vote: true,
                quorum_type: QuorumType::Critical,
                description: "Removing members requires unanimous approval".to_string(),
            },
        );

        // Resource allocation requires normal quorum
        rules.insert(
            DecisionType::ResourceAllocation,
            GovernanceRule {
                requires_vote: true,
                quorum_type: QuorumType::Normal,
                description: "Resource allocation decisions require guild approval".to_string(),
            },
        );

        // Emergency actions can be taken without vote but require audit
        rules.insert(
            DecisionType::EmergencyAction,
            GovernanceRule {
                requires_vote: false,
                quorum_type: QuorumType::Auto,
                description: "Emergency actions can be taken immediately but are audited"
                    .to_string(),
            },
        );

        // Routine operations don't require voting
        rules.insert(
            DecisionType::RoutineOperation,
            GovernanceRule {
                requires_vote: false,
                quorum_type: QuorumType::Auto,
                description: "Routine operations proceed automatically".to_string(),
            },
        );

        Self {
            name: "ServantGuild Default Constitution".to_string(),
            version: 1,
            rules,
            owner_veto_enabled: true,
            max_voting_duration_secs: 3600, // 1 hour
        }
    }
}

impl Constitution {
    /// Create a new constitution with custom rules
    pub fn new(name: String, version: u32) -> Self {
        Self {
            name,
            version,
            rules: HashMap::new(),
            owner_veto_enabled: true,
            max_voting_duration_secs: 3600,
        }
    }

    /// Check if a decision type requires voting
    pub fn requires_vote(&self, decision_type: &DecisionType) -> bool {
        self.rules
            .get(decision_type)
            .map(|r| r.requires_vote)
            .unwrap_or(true) // Default to requiring vote for unknown types
    }

    /// Get the quorum type for a decision
    pub fn get_quorum_type(&self, decision_type: &DecisionType) -> QuorumType {
        self.rules
            .get(decision_type)
            .map(|r| r.quorum_type.clone())
            .unwrap_or(QuorumType::Normal)
    }

    /// Add or update a governance rule
    pub fn set_rule(&mut self, decision_type: DecisionType, rule: GovernanceRule) {
        self.rules.insert(decision_type, rule);
    }
}

/// Types of decisions that require governance
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum DecisionType {
    /// Code modifications
    CodeChange,
    /// Configuration changes
    ConfigChange,
    /// System/infrastructure updates
    SystemUpdate,
    /// Deployment updates
    UpdateDeployment,
    /// Security policy changes
    SecurityChange,
    /// Adding new guild members
    MemberAdd,
    /// Removing guild members
    MemberRemove,
    /// Resource allocation decisions
    ResourceAllocation,
    /// Emergency actions (bypass voting)
    EmergencyAction,
    /// Routine operations (no voting needed)
    RoutineOperation,
    /// Custom decision type
    Custom(String),
}

/// Quorum requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuorumType {
    /// Simple majority (more than half)
    Normal,
    /// Unanimous approval required
    Critical,
    /// Automatic approval, no voting needed
    Auto,
    /// Custom threshold (percentage, 0-100)
    Custom(u8),
}

/// Governance rule for a specific decision type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRule {
    /// Whether this decision requires voting
    pub requires_vote: bool,
    /// Quorum type for this decision
    pub quorum_type: QuorumType,
    /// Human-readable description
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_constitution() {
        let constitution = Constitution::default();

        assert!(constitution.requires_vote(&DecisionType::CodeChange));
        assert!(constitution.requires_vote(&DecisionType::SecurityChange));
        assert!(!constitution.requires_vote(&DecisionType::RoutineOperation));

        assert_eq!(
            constitution.get_quorum_type(&DecisionType::CodeChange),
            QuorumType::Normal
        );
        assert_eq!(
            constitution.get_quorum_type(&DecisionType::SecurityChange),
            QuorumType::Critical
        );
    }

    #[test]
    fn test_custom_constitution() {
        let mut constitution = Constitution::new("Custom Rules".to_string(), 1);

        constitution.set_rule(
            DecisionType::CodeChange,
            GovernanceRule {
                requires_vote: false,
                quorum_type: QuorumType::Auto,
                description: "Auto-approve code changes".to_string(),
            },
        );

        assert!(!constitution.requires_vote(&DecisionType::CodeChange));
    }
}
