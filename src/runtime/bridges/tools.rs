use crate::runtime::state::HostState;
use crate::runtime::bindings::zeroclaw::host::tools::{Host, ToolDesc};
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

            let started = Instant::now();
            match tool.execute(args_json).await {
                Ok(result) => {
                    let duration_ms = started.elapsed().as_millis() as u64;
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
                Err(e) => Err(format!("Tool execution failed: {}", e)),
            }
        } else {
            Err(format!("Tool not found: {}", name))
        }
    }
}
