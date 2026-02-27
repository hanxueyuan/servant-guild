struct Contractor;

impl servant_sdk::Guest for Contractor {
    fn handle_task(task_id: String, input: String) -> Result<String, String> {
        Ok(format!("Contractor received task {}: {}", task_id, input))
    }
}

servant_sdk::export!(Contractor);
