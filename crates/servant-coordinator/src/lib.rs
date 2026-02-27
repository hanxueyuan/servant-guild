struct Coordinator;

impl servant_sdk::Guest for Coordinator {
    fn handle_task(task_id: String, input: String) -> Result<String, String> {
        Ok(format!("Coordinator received task {}: {}", task_id, input))
    }
}

servant_sdk::export!(Coordinator);
