struct Worker;

wit_bindgen::generate!({
    path: "../../wit/host.wit",
    world: "servant",
});

impl Guest for Worker {
    fn handle_task(_task_id: String, input: String) -> Result<String, String> {
        // Prototype: Try to execute the 'echo' tool with the input as arguments
        let tool_name = "echo";
        let args = input;

        // Use generated bindings
        match zeroclaw::host::tools::execute(tool_name, &args) {
            Ok(result) => Ok(format!("Worker executed tool '{}': {}", tool_name, result)),
            Err(e) => Err(format!("Worker failed to execute tool: {}", e)),
        }
    }
}

export!(Worker);
