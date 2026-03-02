//! Speaker Servant - Communication and Consensus Building
//!
//! The Speaker is the "voice" of the guild, responsible for:
//! - Managing proposals and voting
//! - Facilitating communication between servants
//! - Building consensus on decisions
//! - Maintaining the guild's "voice" (LLM interactions)
//! - Coordinating discussions
//! - Broadcasting notifications through multiple channels

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    Servant, ServantError, ServantId, ServantResult, ServantRole, ServantStatus, ServantTask,
};
use crate::consensus::{
    ConsensusEngine, ConsensusResult, DecisionType, Proposal, ProposalStatus, Vote, VoteTally,
};

/// Notification channel for distributing messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationChannel {
    /// Console output
    Console,
    /// System logs
    Logs,
    /// External API/Webhook
    External(String),
    /// Servant direct message
    Servant(String),
    /// All channels
    All,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enabled channels
    pub channels: Vec<NotificationChannel>,
    /// Whether to log all notifications
    pub log_notifications: bool,
    /// Whether to send alerts for important messages
    pub send_alerts: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            channels: vec![NotificationChannel::Console, NotificationChannel::Logs],
            log_notifications: true,
            send_alerts: true,
        }
    }
}

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
    /// Whether important (should be logged and alerted)
    pub important: bool,
    /// Notification channels to use
    pub channels: Vec<NotificationChannel>,
    /// Recipients (None means broadcast to all)
    pub recipients: Option<Vec<String>>,
}

/// Types of guild messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Task assignment
    TaskAssignment,
    /// Task completion
    TaskCompletion,
    /// Security event
    SecurityEvent,
}

/// Event subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    /// Subscription ID
    pub id: String,
    /// Servant subscribing
    pub servant_id: String,
    /// Event types to subscribe to
    pub event_types: Vec<MessageType>,
    /// Active status
    pub active: bool,
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
    /// Notification configuration
    notification_config: RwLock<NotificationConfig>,
    /// Event subscriptions
    subscriptions: RwLock<HashMap<String, EventSubscription>>,
    /// External webhook URLs
    webhook_urls: RwLock<Vec<String>>,
    inboxes: RwLock<HashMap<String, Vec<GuildMessage>>>,
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
            notification_config: RwLock::new(NotificationConfig::default()),
            subscriptions: RwLock::new(HashMap::new()),
            webhook_urls: RwLock::new(Vec::new()),
            inboxes: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new Speaker with custom notification config
    pub fn with_config(consensus: Arc<ConsensusEngine>, config: NotificationConfig) -> Self {
        Self {
            id: ServantId::new(ServantRole::Speaker.default_id()),
            status: RwLock::new(ServantStatus::Starting),
            consensus,
            messages: RwLock::new(Vec::new()),
            discussions: RwLock::new(HashMap::new()),
            notification_config: RwLock::new(config),
            subscriptions: RwLock::new(HashMap::new()),
            webhook_urls: RwLock::new(Vec::new()),
            inboxes: RwLock::new(HashMap::new()),
        }
    }

    /// Set notification configuration
    pub fn set_notification_config(&self, config: NotificationConfig) {
        *self.notification_config.write() = config;
    }

    /// Get notification configuration
    pub fn get_notification_config(&self) -> NotificationConfig {
        self.notification_config.read().clone()
    }

    /// Add a webhook URL for external notifications
    pub fn add_webhook(&self, url: String) {
        self.webhook_urls.write().push(url);
    }

    /// Remove a webhook URL
    pub fn remove_webhook(&self, url: &str) {
        let mut webhooks = self.webhook_urls.write();
        webhooks.retain(|w| w != url);
    }

    /// Subscribe to event types
    pub fn subscribe(
        &self,
        servant_id: String,
        event_types: Vec<MessageType>,
    ) -> Result<String, ServantError> {
        let subscription_id = uuid::Uuid::new_v4().to_string();

        let subscription = EventSubscription {
            id: subscription_id.clone(),
            servant_id,
            event_types,
            active: true,
        };

        self.subscriptions
            .write()
            .insert(subscription_id.clone(), subscription);

        Ok(subscription_id)
    }

    /// Unsubscribe from events
    pub fn unsubscribe(&self, subscription_id: &str) -> Result<(), ServantError> {
        if self.subscriptions.write().remove(subscription_id).is_some() {
            Ok(())
        } else {
            Err(ServantError::InvalidTask(format!(
                "Subscription {} not found",
                subscription_id
            )))
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
        let proposal = self
            .consensus
            .create_proposal(title, description, proposer.clone(), decision_type, payload)
            .map_err(|e| ServantError::Internal(e.to_string()))?;

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
            channels: Vec::new(),
            recipients: None,
        })
        .await;

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
        self.consensus
            .cast_vote(proposal_id, voter.clone(), vote, reason.clone())
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
            channels: Vec::new(),
            recipients: None,
        })
        .await;

        Ok(())
    }

    /// Evaluate and announce the result of a proposal
    pub async fn evaluate(&self, proposal_id: &str) -> Result<VoteTally, ServantError> {
        let tally = self
            .consensus
            .evaluate_proposal(proposal_id)
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
                result_emoji,
                proposal_id,
                tally.result,
                tally.total_votes,
                tally.required_quorum,
                tally.yes_votes,
                tally.no_votes,
                tally.abstain_votes
            ),
            message_type: MessageType::Result,
            timestamp: Utc::now(),
            important: true,
            channels: Vec::new(),
            recipients: None,
        })
        .await;

        Ok(tally)
    }

    /// Start a discussion
    pub async fn start_discussion(&self, topic: String, participants: Vec<String>) -> String {
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

        self.discussions
            .write()
            .insert(discussion_id.clone(), discussion);

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
        let discussion = discussions.get_mut(discussion_id).ok_or_else(|| {
            ServantError::InvalidTask(format!("Discussion {} not found", discussion_id))
        })?;

        let message = GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender,
            content,
            message_type: MessageType::Normal,
            timestamp: Utc::now(),
            important: false,
            channels: Vec::new(),
            recipients: None,
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
        let discussion = discussions.get_mut(discussion_id).ok_or_else(|| {
            ServantError::InvalidTask(format!("Discussion {} not found", discussion_id))
        })?;

        discussion.ended_at = Some(Utc::now());
        discussion.resolution = Some(resolution);

        Ok(())
    }

    /// Broadcast a message to all servants with multi-channel notification
    async fn broadcast(&self, mut message: GuildMessage) {
        // Default channels from config
        if message.channels.is_empty() {
            message.channels = self.notification_config.read().channels.clone();
        }

        // Always add to message history
        self.messages.write().push(message.clone());

        // Log notification if enabled
        if self.notification_config.read().log_notifications {
            self.log_notification(&message);
        }

        // Send to console channel
        if message.channels.contains(&NotificationChannel::Console)
            || message.channels.contains(&NotificationChannel::All)
        {
            self.send_to_console(&message);
        }

        // Send to logs channel
        if message.channels.contains(&NotificationChannel::Logs)
            || message.channels.contains(&NotificationChannel::All)
        {
            self.send_to_logs(&message);
        }

        // Send to external webhooks
        if message.channels.contains(&NotificationChannel::All) {
            self.send_to_webhooks(&message).await;
        }

        // Send to specific external channels
        for channel in &message.channels {
            if let NotificationChannel::External(url) = channel {
                self.send_to_webhook(url, &message).await;
            }
        }

        // Send to specific servant(s)
        if let Some(recipients) = &message.recipients {
            for recipient in recipients {
                if message
                    .channels
                    .contains(&NotificationChannel::Servant(recipient.clone()))
                {
                    self.send_to_servant(recipient, &message);
                }
            }
        }

        // Notify subscribers
        self.notify_subscribers(&message);
    }

    /// Log a notification
    fn log_notification(&self, message: &GuildMessage) {
        let importance = if message.important {
            "IMPORTANT"
        } else {
            "INFO"
        };
        println!(
            "[Speaker - {}] [{}] {} sent: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            importance,
            message.sender,
            message.content
        );
    }

    /// Send message to console
    fn send_to_console(&self, message: &GuildMessage) {
        if message.important {
            println!("📢 [Speaker] {}", message.content);
        } else {
            println!("[Speaker] {}", message.content);
        }
    }

    /// Send message to logs
    fn send_to_logs(&self, message: &GuildMessage) {
        let level = if message.important { "WARN" } else { "INFO" };
        println!(
            "[{}][Speaker] {} - {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            level,
            message.content
        );
    }

    /// Send message to webhooks
    async fn send_to_webhooks(&self, message: &GuildMessage) {
        let webhooks = self.webhook_urls.read().clone();
        for webhook in webhooks {
            self.send_to_webhook(&webhook, message).await;
        }
    }

    /// Send message to a specific webhook
    async fn send_to_webhook(&self, _url: &str, _message: &GuildMessage) {
        let client = crate::config::build_runtime_proxy_client_with_timeouts(
            "servant_speaker.webhook",
            15,
            10,
        );

        let payload = json!({
            "id": _message.id,
            "sender": _message.sender,
            "content": _message.content,
            "message_type": format!("{:?}", _message.message_type),
            "timestamp": _message.timestamp.to_rfc3339(),
            "important": _message.important,
        });

        let resp = client.post(_url).json(&payload).send().await;
        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => {
                println!("[Speaker] Webhook send failed: {}", e);
                return;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            println!("[Speaker] Webhook returned {}: {}", status, body);
        }
    }

    /// Send message to a specific servant
    fn send_to_servant(&self, _servant_id: &str, _message: &GuildMessage) {
        let mut inboxes = self.inboxes.write();
        inboxes
            .entry(_servant_id.to_string())
            .or_insert_with(Vec::new)
            .push(_message.clone());
    }

    /// Notify subscribers of events
    fn notify_subscribers(&self, message: &GuildMessage) {
        let subscriptions = self.subscriptions.read();

        for subscription in subscriptions.values() {
            if !subscription.active {
                continue;
            }

            // Check if subscription matches message type
            if subscription.event_types.contains(&message.message_type) {
                self.send_to_servant(&subscription.servant_id, message);
            }
        }
    }

    pub fn get_inbox(&self, servant_id: &str) -> Vec<GuildMessage> {
        self.inboxes
            .read()
            .get(servant_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Send an alert notification
    pub async fn send_alert(&self, content: String) {
        self.broadcast(GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.id.as_str().to_string(),
            content,
            message_type: MessageType::Alert,
            timestamp: Utc::now(),
            important: true,
            channels: vec![NotificationChannel::All],
            recipients: None,
        })
        .await;
    }

    /// Send a task assignment notification
    pub async fn notify_task_assignment(
        &self,
        task_id: String,
        servant_id: String,
        description: String,
    ) {
        let recipient_id = servant_id.clone();
        self.broadcast(GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.id.as_str().to_string(),
            content: format!(
                "📋 Task assigned to {}: {}\nTask ID: {}",
                servant_id, description, task_id
            ),
            message_type: MessageType::TaskAssignment,
            timestamp: Utc::now(),
            important: true,
            channels: vec![
                NotificationChannel::Console,
                NotificationChannel::Servant(recipient_id.clone()),
            ],
            recipients: Some(vec![recipient_id]),
        })
        .await;
    }

    /// Send a task completion notification
    pub async fn notify_task_completion(
        &self,
        task_id: String,
        servant_id: String,
        result: String,
    ) {
        self.broadcast(GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.id.as_str().to_string(),
            content: format!(
                "✅ Task completed by {}: {}\nTask ID: {}\nResult: {}",
                servant_id, task_id, task_id, result
            ),
            message_type: MessageType::TaskCompletion,
            timestamp: Utc::now(),
            important: true,
            channels: vec![NotificationChannel::All],
            recipients: None,
        })
        .await;
    }

    /// Send a security event notification
    pub async fn notify_security_event(
        &self,
        servant_id: String,
        event_type: String,
        details: String,
    ) {
        self.broadcast(GuildMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.id.as_str().to_string(),
            content: format!(
                "🔒 Security event from {}: {}\nDetails: {}",
                servant_id, event_type, details
            ),
            message_type: MessageType::SecurityEvent,
            timestamp: Utc::now(),
            important: true,
            channels: vec![NotificationChannel::Logs, NotificationChannel::Console],
            recipients: None,
        })
        .await;
    }

    pub fn get_messages(&self) -> Vec<GuildMessage> {
        self.messages.read().clone()
    }

    /// Get message history filtered by type
    pub fn get_messages_by_type(&self, message_type: MessageType) -> Vec<GuildMessage> {
        self.messages
            .read()
            .iter()
            .filter(|m| m.message_type == message_type)
            .cloned()
            .collect()
    }

    /// Get message history for a specific servant
    pub fn get_messages_for_servant(&self, servant_id: &str) -> Vec<GuildMessage> {
        self.messages
            .read()
            .iter()
            .filter(|m| {
                m.sender == servant_id
                    || m.recipients
                        .as_ref()
                        .map_or(false, |r| r.iter().any(|id| id == servant_id))
            })
            .cloned()
            .collect()
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
        let proposal = self
            .propose(
                format!("{:?} Request", decision_type),
                description,
                initiator,
                decision_type,
                None,
            )
            .await?;

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
            "notify".to_string(),
            "alert".to_string(),
            "subscribe".to_string(),
            "webhook".to_string(),
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

        let proposal = speaker
            .propose(
                "Test Proposal".to_string(),
                "This is a test".to_string(),
                "coordinator".to_string(),
                DecisionType::CodeChange,
                None,
            )
            .await;

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
        let proposal = speaker
            .propose(
                "Test".to_string(),
                "Test".to_string(),
                "coordinator".to_string(),
                DecisionType::CodeChange,
                None,
            )
            .await
            .unwrap();

        // Vote
        let result = speaker
            .vote(
                &proposal.id,
                "worker".to_string(),
                Vote::Yes,
                "I approve".to_string(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discussion() {
        let mut speaker = setup_speaker();
        speaker.start().await.unwrap();

        // Start a discussion
        let discussion_id = speaker
            .start_discussion(
                "How to improve performance?".to_string(),
                vec!["coordinator".to_string(), "worker".to_string()],
            )
            .await;

        // Add messages
        speaker
            .discuss(
                &discussion_id,
                "coordinator".to_string(),
                "Let's optimize the cache.".to_string(),
            )
            .await
            .unwrap();
        speaker
            .discuss(
                &discussion_id,
                "worker".to_string(),
                "Good idea!".to_string(),
            )
            .await
            .unwrap();

        // Resolve
        speaker
            .resolve_discussion(&discussion_id, "Implement cache optimization.".to_string())
            .await
            .unwrap();

        let discussions = speaker.get_discussions();
        assert_eq!(discussions.len(), 1);
        assert!(discussions[0].resolution.is_some());
    }
}
