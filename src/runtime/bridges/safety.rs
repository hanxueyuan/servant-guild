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
            .with_actor("servant_guild".to_string(), None, None) // TODO: Get Servant ID
            .with_resource_action(action, target_resource)
            .with_result(true, None, 0, Some(outcome)); // TODO: Add duration

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
            // For high-risk actions, require consensus approval
            println!("[SAFETY] HIGH RISK: {} on {}", action, target_resource);
            println!("[SAFETY] Requesting consensus approval...");

            // TODO: Trigger consensus vote
            // For now, block high-risk actions in production mode
            #[cfg(not(debug_assertions))]
            {
                return Ok(false); // Deny in production
            }

            // In debug mode, approve but warn
            println!("[SAFETY] DEBUG MODE: Approving high-risk action");
            Ok(true)
        } else {
            // Low-risk actions are auto-approved
            println!("[SAFETY] LOW RISK: {} on {} - Approved", action, target_resource);
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
