//! Consensus Engine - Core Voting Logic
//!
//! The engine manages proposals, collects votes, and determines outcomes.

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::consensus::{
    ConsensusConfig, ConsensusResult, Constitution, DecisionType, GovernanceRule, Proposal,
    ProposalStatus, QuorumType, Vote, VoteCounts, VoteRecord, VoteTally,
};

/// The consensus engine manages the voting process
pub struct ConsensusEngine {
    /// Configuration
    config: ConsensusConfig,
    /// Constitution (governance rules)
    constitution: Constitution,
    /// Active proposals
    proposals: RwLock<HashMap<String, Proposal>>,
    /// Servant registry (who can vote)
    servants: RwLock<Vec<String>>,
    /// Owner ID (has veto power)
    owner_id: RwLock<Option<String>>,
}

impl ConsensusEngine {
    /// Create a new consensus engine with default configuration
    pub fn new() -> Self {
        Self::with_config(ConsensusConfig::default(), Constitution::default())
    }

    /// Create a consensus engine with custom configuration
    pub fn with_config(config: ConsensusConfig, constitution: Constitution) -> Self {
        Self {
            config,
            constitution,
            proposals: RwLock::new(HashMap::new()),
            servants: RwLock::new(Vec::new()),
            owner_id: RwLock::new(None),
        }
    }

    /// Register a servant (gives them voting rights)
    pub fn register_servant(&self, servant_id: String) {
        let mut servants = self.servants.write();
        if !servants.contains(&servant_id) {
            servants.push(servant_id);
        }
    }

    /// Unregister a servant
    pub fn unregister_servant(&self, servant_id: &str) {
        let mut servants = self.servants.write();
        servants.retain(|s| s != servant_id);
    }

    /// Get all registered servants
    pub fn get_servants(&self) -> Vec<String> {
        self.servants.read().clone()
    }

    /// Set the owner ID (has veto power)
    pub fn set_owner(&self, owner_id: String) {
        *self.owner_id.write() = Some(owner_id);
    }

    /// Get the owner ID
    pub fn get_owner(&self) -> Option<String> {
        self.owner_id.read().clone()
    }

    /// Create a new proposal
    pub fn create_proposal(
        &self,
        title: String,
        description: String,
        proposer: String,
        decision_type: DecisionType,
        payload: Option<serde_json::Value>,
    ) -> Result<Proposal> {
        // Check if proposer is a registered servant
        {
            let servants = self.servants.read();
            if !servants.contains(&proposer) {
                bail!("Proposer '{}' is not a registered servant", proposer);
            }
        }

        // Check if this decision type requires voting
        if !self.constitution.requires_vote(&decision_type) {
            bail!("Decision type {:?} does not require voting", decision_type);
        }

        let id = format!("prop-{}", Uuid::new_v4());

        let mut proposal = Proposal::new(id.clone(), title, description, proposer, decision_type);

        if let Some(p) = payload {
            proposal = proposal.with_payload(p);
        }

        // Set expiry if configured
        if self.config.voting_timeout_secs > 0 {
            proposal = proposal.with_expiry(chrono::Duration::seconds(
                self.config.voting_timeout_secs as i64,
            ));
        }

        self.proposals.write().insert(id.clone(), proposal.clone());

        Ok(proposal)
    }

    /// Cast a vote on a proposal
    pub fn cast_vote(
        &self,
        proposal_id: &str,
        voter: String,
        vote: Vote,
        reason: String,
    ) -> Result<()> {
        // Check if voter is registered
        {
            let servants = self.servants.read();
            if !servants.contains(&voter) {
                bail!("Voter '{}' is not a registered servant", voter);
            }
        }

        let mut proposals = self.proposals.write();

        let proposal = proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal '{}' not found", proposal_id))?;

        let record = VoteRecord::new(voter, vote, reason);
        proposal.cast_vote(record)?;

        Ok(())
    }

    /// Owner veto a proposal
    pub fn veto_proposal(&self, proposal_id: &str, owner_id: &str) -> Result<()> {
        // Verify owner
        let stored_owner = self.owner_id.read();
        if stored_owner.as_deref() != Some(owner_id) {
            bail!("Only the owner can veto proposals");
        }

        let mut proposals = self.proposals.write();
        let proposal = proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal '{}' not found", proposal_id))?;

        proposal.mark_vetoed();

        Ok(())
    }

    /// Evaluate a proposal and return the result
    pub fn evaluate_proposal(&self, proposal_id: &str) -> Result<VoteTally> {
        let mut proposals = self.proposals.write();

        let proposal = proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal '{}' not found", proposal_id))?;

        // Check for veto first
        if proposal.status == ProposalStatus::Vetoed {
            return Ok(VoteTally {
                proposal_id: proposal_id.to_string(),
                total_votes: proposal.votes.len(),
                yes_votes: 0,
                no_votes: 0,
                abstain_votes: 0,
                required_quorum: 0,
                result: ConsensusResult::Vetoed,
            });
        }

        // Check for expiry
        if proposal.is_expired() && proposal.status == ProposalStatus::Active {
            proposal.mark_expired();
        }

        if proposal.status == ProposalStatus::Expired {
            return Ok(VoteTally {
                proposal_id: proposal_id.to_string(),
                total_votes: proposal.votes.len(),
                yes_votes: 0,
                no_votes: 0,
                abstain_votes: 0,
                required_quorum: 0,
                result: ConsensusResult::Expired,
            });
        }

        // Count votes
        let counts = proposal.count_votes();
        let quorum_type = self.constitution.get_quorum_type(&proposal.decision_type);
        let required_quorum = self.get_required_quorum(&quorum_type);

        let result = if counts.total >= required_quorum {
            // We have enough votes to make a decision
            self.determine_outcome(&counts, &quorum_type, required_quorum)
        } else {
            ConsensusResult::Pending
        };

        // Update proposal status
        match result {
            ConsensusResult::Passed => proposal.mark_passed(),
            ConsensusResult::Rejected => proposal.mark_rejected(),
            ConsensusResult::Expired => proposal.mark_expired(),
            _ => {}
        }

        Ok(VoteTally {
            proposal_id: proposal_id.to_string(),
            total_votes: counts.total,
            yes_votes: counts.yes,
            no_votes: counts.no,
            abstain_votes: counts.abstain,
            required_quorum,
            result,
        })
    }

    /// Determine the outcome based on votes and quorum type
    fn determine_outcome(
        &self,
        counts: &VoteCounts,
        quorum_type: &QuorumType,
        required_quorum: usize,
    ) -> ConsensusResult {
        match quorum_type {
            QuorumType::Normal => {
                // Simple majority of YES votes among all cast votes
                if counts.yes > counts.no {
                    ConsensusResult::Passed
                } else {
                    ConsensusResult::Rejected
                }
            }
            QuorumType::Critical => {
                // Need unanimous YES (ignoring abstentions)
                let total_non_abstain = counts.yes + counts.no;
                if counts.yes == required_quorum && counts.no == 0 {
                    ConsensusResult::Passed
                } else if counts.no > 0 {
                    ConsensusResult::Rejected
                } else {
                    ConsensusResult::Pending
                }
            }
            QuorumType::Custom(threshold) => {
                // Custom percentage threshold
                let total = counts.yes + counts.no;
                if total == 0 {
                    return ConsensusResult::Pending;
                }

                let percentage = (counts.yes as f32 / total as f32 * 100.0) as u8;
                if percentage >= *threshold {
                    ConsensusResult::Passed
                } else {
                    ConsensusResult::Rejected
                }
            }
            QuorumType::Auto => {
                // Should not reach here
                ConsensusResult::Passed
            }
        }
    }

    /// Get the required quorum for a quorum type
    fn get_required_quorum(&self, quorum_type: &QuorumType) -> usize {
        match quorum_type {
            QuorumType::Normal => self.config.normal_quorum,
            QuorumType::Critical => self.config.critical_quorum,
            QuorumType::Custom(_) => self.config.normal_quorum,
            QuorumType::Auto => 0,
        }
    }

    /// Get a proposal by ID
    pub fn get_proposal(&self, proposal_id: &str) -> Option<Proposal> {
        self.proposals.read().get(proposal_id).cloned()
    }

    /// Get all active proposals
    pub fn get_active_proposals(&self) -> Vec<Proposal> {
        self.proposals
            .read()
            .values()
            .filter(|p| p.status == ProposalStatus::Active)
            .cloned()
            .collect()
    }

    /// Get all proposals
    pub fn get_all_proposals(&self) -> Vec<Proposal> {
        self.proposals.read().values().cloned().collect()
    }

    /// Check if a decision type requires voting
    pub fn requires_vote(&self, decision_type: &DecisionType) -> bool {
        self.constitution.requires_vote(decision_type)
    }

    /// Get the constitution
    pub fn get_constitution(&self) -> &Constitution {
        &self.constitution
    }

    /// Process expired proposals
    pub fn process_expired(&self) -> Vec<String> {
        let mut expired = Vec::new();
        let mut proposals = self.proposals.write();

        for (id, proposal) in proposals.iter_mut() {
            if proposal.status == ProposalStatus::Active && proposal.is_expired() {
                proposal.mark_expired();
                expired.push(id.clone());
            }
        }

        expired
    }

    /// Clean up old proposals (for memory management)
    pub fn cleanup_old_proposals(&self, older_than: DateTime<Utc>) -> usize {
        let mut proposals = self.proposals.write();
        let initial_count = proposals.len();

        proposals.retain(|_, p| p.status == ProposalStatus::Active || p.created_at > older_than);

        initial_count - proposals.len()
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_engine() -> ConsensusEngine {
        let engine = ConsensusEngine::new();
        engine.register_servant("coordinator".to_string());
        engine.register_servant("worker".to_string());
        engine.register_servant("warden".to_string());
        engine.register_servant("speaker".to_string());
        engine.register_servant("contractor".to_string());
        engine.set_owner("coordinator".to_string());
        engine
    }

    #[test]
    fn test_create_proposal() {
        let engine = setup_engine();

        let proposal = engine.create_proposal(
            "Test Proposal".to_string(),
            "Description".to_string(),
            "coordinator".to_string(),
            DecisionType::CodeChange,
            None,
        );

        assert!(proposal.is_ok());
        let p = proposal.unwrap();
        assert_eq!(p.title, "Test Proposal");
        assert_eq!(p.status, ProposalStatus::Active);
    }

    #[test]
    fn test_unregistered_proposer() {
        let engine = ConsensusEngine::new();
        engine.register_servant("worker".to_string());

        let result = engine.create_proposal(
            "Test".to_string(),
            "Description".to_string(),
            "unknown".to_string(),
            DecisionType::CodeChange,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_voting_flow() {
        let engine = setup_engine();

        let proposal = engine
            .create_proposal(
                "Test Proposal".to_string(),
                "Description".to_string(),
                "coordinator".to_string(),
                DecisionType::CodeChange,
                None,
            )
            .unwrap();

        // Cast 3 YES votes
        engine
            .cast_vote(
                &proposal.id,
                "worker".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "warden".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "speaker".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();

        let tally = engine.evaluate_proposal(&proposal.id).unwrap();
        assert_eq!(tally.result, ConsensusResult::Passed);
        assert_eq!(tally.yes_votes, 3);
    }

    #[test]
    fn test_veto() {
        let engine = setup_engine();

        let proposal = engine
            .create_proposal(
                "Test Proposal".to_string(),
                "Description".to_string(),
                "coordinator".to_string(),
                DecisionType::CodeChange,
                None,
            )
            .unwrap();

        // Cast some YES votes
        engine
            .cast_vote(
                &proposal.id,
                "worker".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "warden".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();

        // Owner vetoes
        engine.veto_proposal(&proposal.id, "coordinator").unwrap();

        let tally = engine.evaluate_proposal(&proposal.id).unwrap();
        assert_eq!(tally.result, ConsensusResult::Vetoed);
    }

    #[test]
    fn test_rejection() {
        let engine = setup_engine();

        let proposal = engine
            .create_proposal(
                "Test Proposal".to_string(),
                "Description".to_string(),
                "coordinator".to_string(),
                DecisionType::CodeChange,
                None,
            )
            .unwrap();

        // Cast 3 NO votes
        engine
            .cast_vote(
                &proposal.id,
                "worker".to_string(),
                Vote::No,
                "Reject".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "warden".to_string(),
                Vote::No,
                "Reject".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "speaker".to_string(),
                Vote::No,
                "Reject".to_string(),
            )
            .unwrap();

        let tally = engine.evaluate_proposal(&proposal.id).unwrap();
        assert_eq!(tally.result, ConsensusResult::Rejected);
    }

    #[test]
    fn test_critical_quorum() {
        let engine = setup_engine();

        let proposal = engine
            .create_proposal(
                "Security Update".to_string(),
                "Description".to_string(),
                "coordinator".to_string(),
                DecisionType::SecurityChange,
                None,
            )
            .unwrap();

        // For critical decisions, need unanimous YES
        engine
            .cast_vote(
                &proposal.id,
                "worker".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "warden".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "speaker".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "contractor".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "coordinator".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();

        let tally = engine.evaluate_proposal(&proposal.id).unwrap();
        assert_eq!(tally.result, ConsensusResult::Passed);
    }

    #[test]
    fn test_critical_quorum_with_single_no() {
        let engine = setup_engine();

        let proposal = engine
            .create_proposal(
                "Security Update".to_string(),
                "Description".to_string(),
                "coordinator".to_string(),
                DecisionType::SecurityChange,
                None,
            )
            .unwrap();

        // Even one NO should reject a critical decision
        engine
            .cast_vote(
                &proposal.id,
                "worker".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "warden".to_string(),
                Vote::No,
                "Reject".to_string(),
            )
            .unwrap();
        engine
            .cast_vote(
                &proposal.id,
                "speaker".to_string(),
                Vote::Yes,
                "Approve".to_string(),
            )
            .unwrap();

        let tally = engine.evaluate_proposal(&proposal.id).unwrap();
        assert_eq!(tally.result, ConsensusResult::Rejected);
    }
}
