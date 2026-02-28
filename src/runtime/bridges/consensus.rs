use crate::runtime::state::HostState;
use crate::runtime::bindings::zeroclaw::host::consensus::Host;
use crate::consensus::{DecisionType, Vote};
use wasmtime::Result;

#[async_trait::async_trait]
impl Host for HostState {
    /// Create a new proposal for collective decision
    async fn propose(&mut self, title: String, description: String) -> Result<String, String> {
        if let Some(ref consensus_engine) = self.consensus_engine {
            use crate::consensus::Proposal;
            
            // Create a proposal with default decision type (Normal)
            let proposal = consensus_engine
                .create_proposal(
                    title.clone(),
                    description.clone(),
                    "unknown_servant".to_string(), // TODO: Get actual servant ID from context
                    DecisionType::SystemUpdate,
                    None,
                )
                .map_err(|e| format!("Failed to create proposal: {}", e))?;
            
            Ok(proposal.id.clone())
        } else {
            Err("Consensus engine not initialized".to_string())
        }
    }

    /// Cast a vote on an existing proposal
    async fn vote(&mut self, proposal_id: String, approve: bool, reason: String) -> Result<(), String> {
        if let Some(ref consensus_engine) = self.consensus_engine {
            let vote_type = if approve {
                Vote::Approve
            } else {
                Vote::Reject
            };
            
            consensus_engine
                .cast_vote(
                    &proposal_id,
                    "unknown_servant".to_string(), // TODO: Get actual servant ID from context
                    vote_type,
                    reason,
                )
                .map_err(|e| format!("Failed to cast vote: {}", e))
        } else {
            Err("Consensus engine not initialized".to_string())
        }
    }
}
