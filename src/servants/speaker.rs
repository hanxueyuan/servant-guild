//! Speaker Servant - Communication and Consensus Building
//!
//! The Speaker is the "voice" of the guild, responsible for:
//! - Managing proposals and voting
//! - Facilitating communication between servants
//! - Building consensus on decisions
//! - Maintaining the guild's "voice" (LLM interactions)
//! - Coordinating discussions

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{
    Servant, ServantId, ServantRole, ServantStatus, ServantTask, ServantResult, ServantError,
};
use crate::consensus::{
    ConsensusEngine, ConsensusResult, DecisionType, Proposal, ProposalStatus, Vote, VoteTally,
};

/// A message in the guild communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMessage {
    /// Message ID
    pub id: String,
    /// Sender servant ID
    pub sender: String,
    /// Message content
    pub content: String,
    /// Message type
    pub message_type: MessageType,
    /// When sent
    pub timestamp: DateTime<Utc>,
    /// Whether important (should be logged)
    pub important: bool,
}

/// Types of guild messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Normal communication
    Normal,
    /// Proposal announcement
    Proposal,
    /// Vote announcement
    Vote,
    /// Result announcement
    Result,
    /// Alert/warning
    Alert,
    /// System message
    System,
}

/// The Speaker servant
pub struct Speaker {
    /// Unique ID
    id: ServantId,
    /// Current status
    status: RwLock<ServantStatus>,
    /// Consensus engine reference
    consensus: Arc<ConsensusEngine>,
    /// Message history
    messages: RwLock<Vec<GuildMessage>>,
    /// Active discussions
    discussions: RwLock<HashMap<String, Discussion>>,
}

/// A discussion thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discussion {
    /// Discussion ID
    pub id: String,
    /// Topic
    pub topic: String,
    /// Related proposal (if any)
    pub proposal_id: Option<String>,
    /// Messages in the discussion
    pub messages: Vec<GuildMessage>,
    /// Participants
    pub participants: Vec<String>,
    /// When started
    pub started_at: DateTime<Utc>,
    /// When ended (if ended)
    pub ended_at: Option<DateTime<Utc>>,
    /// Resolution summary (if resolved)
    pub resolution: Option<String>,
}

impl Speaker {
    /// Create a new Speaker
    pub fn new(consensus: Arc<ConsensusEngine>) -> Self {
        Self {
            id: ServantId::new(ServantRole::Speaker.default_id()),
            status: RwLock::new(ServantStatus::Starting),
            consensus,
            messages: RwLock::new(Vec::new()),
            discussions: RwLock::new(HashMap::new()),
        }
    }
    
    /// Create and announce a new proposal
    pub async fn propose(
        &self,
        title: String,
        description: String,
        proposer: String,
        decision_type: DecisionType,
        payload: Option<serde_json::Value>,
    ) -> Result<Proposal, ServantError> {
        // Create the proposal
        let proposal = self.consensus.create_proposal(
            title,
            description,
            proposer.clone(),
            decision_type,
            payload,
        ).map_err(|e| ServantError::Internal(e.to_string()))?;
        
        // Announce the proposal
        self.broadcast(GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.id.as_str().to_string(),
            content: format!(
                "📢 New Proposal: {} (by {})\n{}\nType: {:?}",
                proposal.title, proposer, proposal.description, proposal.decision_type
            ),
            message_type: MessageType::Proposal,
            timestamp: Utc::now(),
            important: true,
        }).await;
        
        Ok(proposal)
    }
    
    /// Cast a vote on a proposal
    pub async fn vote(
        &self,
        proposal_id: &str,
        voter: String,
        vote: Vote,
        reason: String,
    ) -> Result<(), ServantError> {
        // Cast the vote
        self.consensus.cast_vote(proposal_id, voter.clone(), vote, reason.clone())
            .map_err(|e| ServantError::Internal(e.to_string()))?;
        
        // Announce the vote
        self.broadcast(GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.id.as_str().to_string(),
            content: format!(
                "🗳️ {} voted {:?} on proposal {}\nReason: {}",
                voter, vote, proposal_id, reason
            ),
            message_type: MessageType::Vote,
            timestamp: Utc::now(),
            important: true,
        }).await;
        
        Ok(())
    }
    
    /// Evaluate and announce the result of a proposal
    pub async fn evaluate(&self, proposal_id: &str) -> Result<VoteTally, ServantError> {
        let tally = self.consensus.evaluate_proposal(proposal_id)
            .map_err(|e| ServantError::Internal(e.to_string()))?;
        
        // Announce the result
        let result_emoji = match tally.result {
            ConsensusResult::Passed => "✅",
            ConsensusResult::Rejected => "❌",
            ConsensusResult::Vetoed => "🚫",
            ConsensusResult::Expired => "⏰",
            ConsensusResult::Pending => "⏳",
        };
        
        self.broadcast(GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.id.as_str().to_string(),
            content: format!(
                "{} Proposal {} Result: {:?}\nVotes: {}/{} (Yes: {}, No: {}, Abstain: {})",
                result_emoji, proposal_id, tally.result,
                tally.total_votes, tally.required_quorum,
                tally.yes_votes, tally.no_votes, tally.abstain_votes
            ),
            message_type: MessageType::Result,
            timestamp: Utc::now(),
            important: true,
        }).await;
        
        Ok(tally)
    }
    
    /// Start a discussion
    pub async fn start_discussion(
        &self,
        topic: String,
        participants: Vec<String>,
    ) -> String {
        let discussion_id = uuid::Uuid::new_v4().to_string();
        
        let discussion = Discussion {
            id: discussion_id.clone(),
            topic,
            proposal_id: None,
            messages: Vec::new(),
            participants,
            started_at: Utc::now(),
            ended_at: None,
            resolution: None,
        };
        
        self.discussions.write().insert(discussion_id.clone(), discussion);
        
        discussion_id
    }
    
    /// Add a message to a discussion
    pub async fn discuss(
        &self,
        discussion_id: &str,
        sender: String,
        content: String,
    ) -> Result<(), ServantError> {
        let mut discussions = self.discussions.write();
        let discussion = discussions
            .get_mut(discussion_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Discussion {} not found", discussion_id)))?;
        
        let message = GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender,
            content,
            message_type: MessageType::Normal,
            timestamp: Utc::now(),
            important: false,
        };
        
        discussion.messages.push(message.clone());
        self.messages.write().push(message);
        
        Ok(())
    }
    
    /// End a discussion with a resolution
    pub async fn resolve_discussion(
        &self,
        discussion_id: &str,
        resolution: String,
    ) -> Result<(), ServantError> {
        let mut discussions = self.discussions.write();
        let discussion = discussions
            .get_mut(discussion_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Discussion {} not found", discussion_id)))?;
        
        discussion.ended_at = Some(Utc::now());
        discussion.resolution = Some(resolution);
        
        Ok(())
    }
    
    /// Broadcast a message to all servants
    async fn broadcast(&self, message: GuildMessage) {
        self.messages.write().push(message);
    }
    
    /// Get message history
    pub fn get_messages(&self) -> Vec<GuildMessage> {
        self.messages.read().clone()
    }
    
    /// Get active discussions
    pub fn get_discussions(&self) -> Vec<Discussion> {
        self.discussions.read().values().cloned().collect()
    }
    
    /// Get active proposals
    pub fn get_active_proposals(&self) -> Vec<Proposal> {
        self.consensus.get_active_proposals()
    }
    
    /// Check if a decision type requires voting
    pub fn requires_vote(&self, decision_type: &DecisionType) -> bool {
        self.consensus.requires_vote(decision_type)
    }
    
    /// Facilitate consensus building for a decision
    pub async fn facilitate_consensus(
        &self,
        decision_type: DecisionType,
        description: String,
        initiator: String,
    ) -> Result<ConsensusResult, ServantError> {
        // Check if voting is required
        if !self.consensus.requires_vote(&decision_type) {
            return Ok(ConsensusResult::Passed);
        }
        
        // Create a proposal
        let proposal = self.propose(
            format!("{:?} Request", decision_type),
            description,
            initiator,
            decision_type,
            None,
        ).await?;
        
        // Wait for votes (in a real implementation, this would be async)
        // For now, return Pending
        Ok(ConsensusResult::Pending)
    }
}

#[async_trait]
impl Servant for Speaker {
    fn id(&self) -> &ServantId {
        &self.id
    }
    
    fn role(&self) -> ServantRole {
        ServantRole::Speaker
    }
    
    fn status(&self) -> ServantStatus {
        self.status.read().clone()
    }
    
    async fn start(&mut self) -> Result<(), ServantError> {
        *self.status.write() = ServantStatus::Ready;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<(), ServantError> {
        *self.status.write() = ServantStatus::Stopping;
        *self.status.write() = ServantStatus::Paused;
        Ok(())
    }
    
    fn capabilities(&self) -> Vec<String> {
        vec![
            "propose".to_string(),
            "vote".to_string(),
            "facilitate".to_string(),
            "broadcast".to_string(),
            "discuss".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_speaker() -> Speaker {
        let consensus = Arc::new(ConsensusEngine::new());
        consensus.register_servant("coordinator".to_string());
        consensus.register_servant("worker".to_string());
        consensus.register_servant("warden".to_string());
        consensus.register_servant("speaker".to_string());
        consensus.register_servant("contractor".to_string());
        consensus.set_owner("coordinator".to_string());
        
        Speaker::new(consensus)
    }

    #[tokio::test]
    async fn test_speaker_creation() {
        let speaker = setup_speaker();
        assert_eq!(speaker.role(), ServantRole::Speaker);
        assert_eq!(speaker.status(), ServantStatus::Starting);
    }
    
    #[tokio::test]
    async fn test_speaker_start_stop() {
        let mut speaker = setup_speaker();
        
        speaker.start().await.unwrap();
        assert_eq!(speaker.status(), ServantStatus::Ready);
        
        speaker.stop().await.unwrap();
        assert_eq!(speaker.status(), ServantStatus::Paused);
    }
    
    #[tokio::test]
    async fn test_propose() {
        let mut speaker = setup_speaker();
        speaker.start().await.unwrap();
        
        let proposal = speaker.propose(
            "Test Proposal".to_string(),
            "This is a test".to_string(),
            "coordinator".to_string(),
            DecisionType::CodeChange,
            None,
        ).await;
        
        assert!(proposal.is_ok());
        let p = proposal.unwrap();
        assert_eq!(p.title, "Test Proposal");
        
        // Should have broadcast a message
        let messages = speaker.get_messages();
        assert!(!messages.is_empty());
    }
    
    #[tokio::test]
    async fn test_vote() {
        let mut speaker = setup_speaker();
        speaker.start().await.unwrap();
        
        // Create a proposal first
        let proposal = speaker.propose(
            "Test".to_string(),
            "Test".to_string(),
            "coordinator".to_string(),
            DecisionType::CodeChange,
            None,
        ).await.unwrap();
        
        // Vote
        let result = speaker.vote(
            &proposal.id,
            "worker".to_string(),
            Vote::Yes,
            "I approve".to_string(),
        ).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_discussion() {
        let mut speaker = setup_speaker();
        speaker.start().await.unwrap();
        
        // Start a discussion
        let discussion_id = speaker.start_discussion(
            "How to improve performance?".to_string(),
            vec!["coordinator".to_string(), "worker".to_string()],
        ).await;
        
        // Add messages
        speaker.discuss(&discussion_id, "coordinator".to_string(), "Let's optimize the cache.".to_string()).await.unwrap();
        speaker.discuss(&discussion_id, "worker".to_string(), "Good idea!".to_string()).await.unwrap();
        
        // Resolve
        speaker.resolve_discussion(&discussion_id, "Implement cache optimization.".to_string()).await.unwrap();
        
        let discussions = speaker.get_discussions();
        assert_eq!(discussions.len(), 1);
        assert!(discussions[0].resolution.is_some());
    }
}
