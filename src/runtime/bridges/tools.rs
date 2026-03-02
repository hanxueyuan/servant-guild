use crate::runtime::state::HostState;
use crate::runtime::bindings::zeroclaw::host::tools::{Host, ToolDesc};
use crate::consensus::{ConsensusResult, DecisionType, Vote};
use crate::safety::audit::{AuditEvent, AuditEventType};
use wasmtime::Result;
use std::time::Instant;

#[async_trait::async_trait]
impl Host for HostState {
    async fn list_tools(&mut self) -> Result<Vec<ToolDesc>, String> {
        let mut tool_list = Vec::new();
        for tool in self.tools.values() {
            let spec = tool.spec();
            tool_list.push(ToolDesc {
                name: spec.name,
                description: spec.description,
                parameters: spec.parameters.to_string(),
            });
        }
        Ok(tool_list)
    }

    async fn execute(&mut self, name: String, args: String) -> Result<String, String> {
        if let Some(tool) = self.tools.get(&name) {
            let args_json: serde_json::Value = match serde_json::from_str(&args) {
                Ok(v) => v,
                Err(e) => return Err(format!("Invalid JSON arguments: {}", e)),
            };

            let high_risk = is_high_risk_tool(&name);

            if high_risk {
                let consensus = self
                    .consensus_engine
                    .as_ref()
                    .ok_or_else(|| "Consensus engine not initialized".to_string())?;

                let decision_type = if name.to_lowercase().contains("secret")
                    || name.to_lowercase().contains("credential")
                    || name.to_lowercase().contains("key")
                {
                    DecisionType::SecurityChange
                } else {
                    DecisionType::SystemUpdate
                };

                let proposal = consensus
                    .create_proposal(
                        format!("Approve tool: {name}"),
                        "WASM servant requests permission to execute a high-risk tool".to_string(),
                        self.servant_id.clone(),
                        decision_type,
                        Some(args_json.clone()),
                    )
                    .map_err(|e| format!("Failed to create proposal: {e}"))?;

                consensus
                    .cast_vote(
                        &proposal.id,
                        self.servant_id.clone(),
                        Vote::Yes,
                        "requester approval".to_string(),
                    )
                    .map_err(|e| format!("Failed to cast vote: {e}"))?;

                let tally = consensus
                    .evaluate_proposal(&proposal.id)
                    .map_err(|e| format!("Failed to evaluate proposal: {e}"))?;

                if !matches!(tally.result, ConsensusResult::Passed) {
                    return Err("Consensus approval required".to_string());
                }

                if let Some(manager) = &self.rollback_manager {
                    manager
                        .lock()
                        .begin()
                        .map_err(|e| format!("Failed to begin rollback transaction: {e}"))?;
                }
            }

            let started = Instant::now();
            match tool.execute(args_json).await {
                Ok(result) => {
                    let duration_ms = started.elapsed().as_millis() as u64;
                    if high_risk {
                        if let Some(manager) = &self.rollback_manager {
                            let _ = if result.success {
                                manager.lock().commit()
                            } else {
                                manager.lock().rollback()
                            };
                        }
                    }
                    let event = AuditEvent::new(AuditEventType::ServantAction)
                        .with_agent(self.servant_id.clone())
                        .with_action(
                            format!("tool:{}", name),
                            "medium".to_string(),
                            true,
                            result.success,
                        )
                        .with_result(
                            result.success,
                            None,
                            duration_ms,
                            result.error.clone(),
                        )
                        .finalize();
                    let _ = self.audit_logger.log(&event);
                    if result.success {
                        Ok(result.output)
                    } else {
                        Err(result.error.unwrap_or_else(|| "Unknown tool error".to_string()))
                    }
                }
                Err(e) => {
                    if high_risk {
                        if let Some(manager) = &self.rollback_manager {
                            let _ = manager.lock().rollback();
                        }
                    }
                    Err(format!("Tool execution failed: {}", e))
                }
            }
        } else {
            Err(format!("Tool not found: {}", name))
        }
    }
}

fn is_high_risk_tool(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("delete")
        || lower.contains("remove")
        || lower.contains("write")
        || lower.contains("run")
        || lower.contains("exec")
        || lower.contains("shell")
        || lower.contains("http")
        || lower.contains("network")
        || lower.contains("chmod")
        || lower.contains("chown")
}

#[cfg(test)]
mod tests {
    use super::is_high_risk_tool;

    #[test]
    fn tool_risk_classification() {
        assert!(is_high_risk_tool("delete_file"));
        assert!(is_high_risk_tool("run_command"));
        assert!(is_high_risk_tool("http_request"));
        assert!(!is_high_risk_tool("read_file"));
        assert!(!is_high_risk_tool("analyze_code"));
    }
}
