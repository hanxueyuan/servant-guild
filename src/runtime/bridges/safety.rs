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
        // TODO: Implement actual permission request (e.g., pause for human approval)
        // For now, always approve in dev mode
        println!("[SAFETY] Requesting permission for: {} on {}", action, target_resource);
        Ok(true)
    }
}
