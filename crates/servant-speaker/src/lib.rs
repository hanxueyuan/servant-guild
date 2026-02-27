struct Speaker;

impl servant_sdk::Guest for Speaker {
    fn handle_task(task_id: String, input: String) -> Result<String, String> {
        // Prototype: Speaker creates a proposal based on the input
        let title = format!("Proposal from task {}", task_id);
        let description = input;

        match servant_sdk::zeroclaw::host::consensus::propose(&title, &description) {
            Ok(proposal_id) => Ok(format!("Speaker created proposal: {}", proposal_id)),
            Err(e) => Err(format!("Speaker failed to propose: {}", e)),
        }
    }
}

servant_sdk::export!(Speaker);
