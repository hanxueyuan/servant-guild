use crate::runtime::state::HostState;
use crate::runtime::bindings::zeroclaw::host::safety::{Host, Severity};
use crate::safety::audit::{AuditEvent, AuditEventType};
use wasmtime::Result;

#[async_trait::async_trait]
impl Host for HostState {
    async fn audit_log(
        &mut self,
        action: String,
        target_resource: String,
        outcome: String,
        severity: Severity,
    ) {
        let risk_level = match severity {
            Severity::Info => "low",
            Severity::Warning => "medium",
            Severity::Critical => "high",
        };

        let event = AuditEvent::new(AuditEventType::ServantAction)
            .with_agent(self.servant_id.clone())
            .with_resource_action(action, target_resource)
            .with_result(true, None, 0, Some(outcome))
            .finalize();

        // Log to disk via AuditLogger
        if let Err(e) = self.audit_logger.log(&event) {
            eprintln!("Failed to write audit log: {}", e);
        }
    }

    async fn request_permission(
        &mut self,
        action: String,
        target_resource: String,
    ) -> Result<bool, String> {
        // Check if this is a high-risk action
        let is_high_risk = self.is_high_risk_action(&action, &target_resource);

        if is_high_risk {
            if let Some(ref consensus_engine) = self.consensus_engine {
                let servant_id = self.servant_id.clone();
                consensus_engine.register_servant(servant_id.clone());

                let decision_type = if action.to_lowercase().contains("secret")
                    || target_resource.to_lowercase().contains("secret")
                    || target_resource.to_lowercase().contains("credential")
                    || target_resource.to_lowercase().contains("key")
                {
                    crate::consensus::DecisionType::SecurityChange
                } else {
                    crate::consensus::DecisionType::SystemUpdate
                };

                let proposal = consensus_engine
                    .create_proposal(
                        format!("Permission: {action}"),
                        format!("Target: {target_resource}"),
                        servant_id.clone(),
                        decision_type,
                        None,
                    )
                    .map_err(|e| format!("Failed to create proposal: {e}"))?;

                consensus_engine
                    .cast_vote(
                        &proposal.id,
                        servant_id,
                        crate::consensus::Vote::Approve,
                        "requester approval".to_string(),
                    )
                    .map_err(|e| format!("Failed to cast vote: {e}"))?;

                let tally = consensus_engine
                    .evaluate_proposal(&proposal.id)
                    .map_err(|e| format!("Failed to evaluate proposal: {e}"))?;

                Ok(matches!(tally.result, crate::consensus::ConsensusResult::Passed))
            } else {
                Ok(false)
            }
        } else {
            // Low-risk actions are auto-approved
            Ok(true)
        }
    }
}

impl HostState {
    /// Determine if an action is high-risk
    fn is_high_risk_action(&self, action: &str, target: &str) -> bool {
        // High-risk patterns
        let high_risk_actions = [
            "delete", "remove", "drop", "truncate", "destroy",
            "execute", "eval", "spawn", "fork",
            "chmod", "chown", "setuid", "setgid",
            "network", "connect", "bind", "listen",
            "config", "settings", "credentials", "secrets",
        ];

        // High-risk targets
        let high_risk_targets = [
            ".env", ".git", "credentials", "secrets", "keys",
            "/etc", "/root", "/home", "/var", "/usr",
            "database", "db", "postgres", "mysql", "redis",
        ];

        let action_lower = action.to_lowercase();
        let target_lower = target.to_lowercase();

        // Check if action is high-risk
        let action_is_risky = high_risk_actions
            .iter()
            .any(|ra| action_lower.contains(ra));

        // Check if target is sensitive
        let target_is_sensitive = high_risk_targets
            .iter()
            .any(|rt| target_lower.contains(rt));

        action_is_risky || target_is_sensitive
    }
}
