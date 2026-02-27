use crate::runtime::state::HostState;
use crate::runtime::bindings::zeroclaw::host::tools::{Host, ToolDesc};
use wasmtime::Result;

impl Host for HostState {
    async fn list(&mut self) -> Result<Result<Vec<ToolDesc>, String>> {
        // TODO: Return actual tool list from src/tools
        Ok(Ok(vec![
            ToolDesc {
                name: "echo".to_string(),
                description: "Returns the input".to_string(),
                parameters: "{}".to_string(),
            }
        ]))
    }

    async fn execute(&mut self, name: String, args: String) -> Result<Result<String, String>> {
        // TODO: Map to src/tools via src/safety wrapper
        if name == "echo" {
            Ok(Ok(format!("Echo: {}", args)))
        } else {
            Ok(Err(format!("Tool not found: {}", name)))
        }
    }
}
