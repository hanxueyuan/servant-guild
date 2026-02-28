//! Proposal - Requests for Collective Decision
//!
//! A proposal represents a request for the guild to make a collective
//! decision. It includes metadata, status tracking, and vote records.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::consensus::{DecisionType, Vote};

/// A proposal for collective decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier
    pub id: String,
    /// Title of the proposal
    pub title: String,
    /// Detailed description
    pub description: String,
    /// ID of the servant who proposed this
    pub proposer: String,
    /// Type of decision being proposed
    pub decision_type: DecisionType,
    /// Optional payload (e.g., code changes, config values)
    pub payload: Option<serde_json::Value>,
    /// When the proposal was created
    pub created_at: DateTime<Utc>,
    /// When the proposal expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,
    /// Current status of the proposal
    pub status: ProposalStatus,
    /// Votes cast on this proposal
    pub votes: HashMap<String, VoteRecord>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Priority level (higher = more urgent)
    pub priority: u8,
}

impl Proposal {
    /// Create a new proposal
    pub fn new(
        id: String,
        title: String,
        description: String,
        proposer: String,
        decision_type: DecisionType,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(1); // Default 1 hour expiry
        
        Self {
            id,
            title,
            description,
            proposer,
            decision_type,
            payload: None,
            created_at: now,
            expires_at: Some(expires_at),
            status: ProposalStatus::Active,
            votes: HashMap::new(),
            tags: Vec::new(),
            priority: 5, // Normal priority
        }
    }
    
    /// Create a proposal with payload
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }
    
    /// Set expiration time
    pub fn with_expiry(mut self, duration: chrono::Duration) -> Self {
        self.expires_at = Some(Utc::now() + duration);
        self
    }
    
    /// Set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10); // Max priority is 10
        self
    }
    
    /// Cast a vote on this proposal
    pub fn cast_vote(&mut self, record: VoteRecord) -> Result<(), VoteError> {
        if self.status != ProposalStatus::Active {
            return Err(VoteError::ProposalNotActive {
                status: self.status.clone(),
            });
        }
        
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                self.status = ProposalStatus::Expired;
                return Err(VoteError::ProposalExpired);
            }
        }
        
        self.votes.insert(record.voter.clone(), record);
        Ok(())
    }
    
    /// Check if a servant has already voted
    pub fn has_voted(&self, servant_id: &str) -> bool {
        self.votes.contains_key(servant_id)
    }
    
    /// Get vote counts
    pub fn count_votes(&self) -> VoteCounts {
        let mut counts = VoteCounts::default();
        
        for record in self.votes.values() {
            match record.vote {
                Vote::Yes => counts.yes += 1,
                Vote::No => counts.no += 1,
                Vote::Abstain => counts.abstain += 1,
            }
        }
        
        counts.total = self.votes.len();
        counts
    }
    
    /// Check if the proposal is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }
    
    /// Mark the proposal as passed
    pub fn mark_passed(&mut self) {
        self.status = ProposalStatus::Passed;
    }
    
    /// Mark the proposal as rejected
    pub fn mark_rejected(&mut self) {
        self.status = ProposalStatus::Rejected;
    }
    
    /// Mark the proposal as vetoed by owner
    pub fn mark_vetoed(&mut self) {
        self.status = ProposalStatus::Vetoed;
    }
    
    /// Mark the proposal as expired
    pub fn mark_expired(&mut self) {
        self.status = ProposalStatus::Expired;
    }
}

/// Status of a proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Actively accepting votes
    Active,
    /// Passed and ready for execution
    Passed,
    /// Rejected by voting or veto
    Rejected,
    /// Expired without reaching quorum
    Expired,
    /// Vetoed by owner
    Vetoed,
    /// Currently being executed
    Executing,
    /// Successfully executed
    Executed,
    /// Execution failed
    Failed,
}

/// A vote record from a servant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRecord {
    /// ID of the voting servant
    pub voter: String,
    /// The vote cast
    pub vote: Vote,
    /// Reason for the vote (optional)
    pub reason: String,
    /// When the vote was cast
    pub timestamp: DateTime<Utc>,
    /// Confidence level (0-100)
    pub confidence: u8,
}

impl VoteRecord {
    /// Create a new vote record
    pub fn new(voter: String, vote: Vote, reason: String) -> Self {
        Self {
            voter,
            vote,
            reason,
            timestamp: Utc::now(),
            confidence: 100, // Default full confidence
        }
    }
    
    /// Set confidence level
    pub fn with_confidence(mut self, confidence: u8) -> Self {
        self.confidence = confidence.min(100);
        self
    }
}

/// Vote counts for a proposal
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoteCounts {
    pub total: usize,
    pub yes: usize,
    pub no: usize,
    pub abstain: usize,
}

/// Errors when voting
#[derive(Debug, Clone, thiserror::Error)]
pub enum VoteError {
    #[error("Proposal is not active (status: {status:?})")]
    ProposalNotActive { status: ProposalStatus },
    #[error("Proposal has expired")]
    ProposalExpired,
    #[error("Servant has already voted")]
    AlreadyVoted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_creation() {
        let proposal = Proposal::new(
            "prop-001".to_string(),
            "Test Proposal".to_string(),
            "This is a test".to_string(),
            "coordinator".to_string(),
            DecisionType::CodeChange,
        );
        
        assert_eq!(proposal.id, "prop-001");
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert!(proposal.votes.is_empty());
    }
    
    #[test]
    fn test_vote_casting() {
        let mut proposal = Proposal::new(
            "prop-001".to_string(),
            "Test Proposal".to_string(),
            "This is a test".to_string(),
            "coordinator".to_string(),
            DecisionType::CodeChange,
        );
        
        let vote = VoteRecord::new(
            "worker".to_string(),
            Vote::Yes,
            "I approve".to_string(),
        );
        
        assert!(proposal.cast_vote(vote).is_ok());
        assert!(proposal.has_voted("worker"));
        
        let counts = proposal.count_votes();
        assert_eq!(counts.total, 1);
        assert_eq!(counts.yes, 1);
    }
    
    #[test]
    fn test_vote_on_inactive_proposal() {
        let mut proposal = Proposal::new(
            "prop-001".to_string(),
            "Test Proposal".to_string(),
            "This is a test".to_string(),
            "coordinator".to_string(),
            DecisionType::CodeChange,
        );
        
        proposal.status = ProposalStatus::Passed;
        
        let vote = VoteRecord::new(
            "worker".to_string(),
            Vote::Yes,
            "I approve".to_string(),
        );
        
        assert!(proposal.cast_vote(vote).is_err());
    }
}
