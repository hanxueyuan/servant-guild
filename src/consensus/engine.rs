use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::{Result, bail};
use crate::consensus::proposal::{Proposal, ProposalStatus, VoteRecord};
use crate::consensus::vote::Vote;

pub struct ConsensusEngine {
    proposals: Arc<Mutex<HashMap<String, Proposal>>>,
    quorum_threshold: usize,
}

impl ConsensusEngine {
    pub fn new(quorum_threshold: usize) -> Self {
        Self {
            proposals: Arc::new(Mutex::new(HashMap::new())),
            quorum_threshold,
        }
    }

    pub fn create_proposal(&self, title: String, description: String, proposer: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let proposal = Proposal::new(id.clone(), title, description, proposer);
        
        let mut proposals = self.proposals.lock().unwrap();
        proposals.insert(id.clone(), proposal);
        
        id
    }

    pub fn cast_vote(&self, proposal_id: &str, voter: &str, vote: Vote, reason: String) -> Result<()> {
        let mut proposals = self.proposals.lock().unwrap();
        
        let proposal = match proposals.get_mut(proposal_id) {
            Some(p) => p,
            None => bail!("Proposal not found: {}", proposal_id),
        };
        // ... (validation checks) ...
        if proposal.status != ProposalStatus::Active {
            bail!("Proposal is not active");
        }

        if proposal.votes.contains_key(voter) {
            bail!("Voter {} has already voted on proposal {}", voter, proposal_id);
        }

        proposal.votes.insert(voter.to_string(), VoteRecord {
            voter: voter.to_string(),
            vote,
            reason,
            timestamp: chrono::Utc::now().timestamp(),
        });

        // Tally logic with configurable quorum
        if proposal.votes.len() >= self.quorum_threshold {
            let yes_votes = proposal.votes.values().filter(|v| v.vote == Vote::Yes).count();
            if yes_votes > proposal.votes.len() / 2 {
                proposal.status = ProposalStatus::Passed;
            } else {
                proposal.status = ProposalStatus::Rejected;
            }
        }

        Ok(())
    }
    
    pub fn get_proposal(&self, proposal_id: &str) -> Option<Proposal> {
        let proposals = self.proposals.lock().unwrap();
        proposals.get(proposal_id).cloned()
    }
}
