# Speaker API Reference

## Overview

The Speaker is the "voice" of the guild, responsible for:
- Managing proposals and voting
- Facilitating communication between servants
- Building consensus on decisions
- Maintaining the guild's "voice" (LLM interactions)
- Coordinating discussions
- Broadcasting notifications through multiple channels

The Speaker implements a multi-channel notification system supporting Console, Logs, External Webhooks, and Servant-to-Servant messaging.

## Table of Contents

- [Core Types](#core-types)
- [Initialization](#initialization)
- [Message Management](#message-management)
- [Proposal Management](#proposal-management)
- [Voting System](#voting-system)
- [Notifications](#notifications)
- [Event Subscriptions](#event-subscriptions)
- [Discussions](#discussions)
- [Webhook Integration](#webhook-integration)
- [Error Handling](#error-handling)
- [Usage Examples](#usage-examples)

---

## Core Types

### Speaker

The main Speaker servant structure.

```rust
pub struct Speaker {
    id: ServantId,
    status: RwLock<ServantStatus>,
    consensus: Arc<ConsensusEngine>,
    messages: RwLock<Vec<GuildMessage>>,
    discussions: RwLock<HashMap<String, Discussion>>,
    notification_config: RwLock<NotificationConfig>,
    subscriptions: RwLock<HashMap<String, EventSubscription>>,
    webhook_urls: RwLock<Vec<String>>,
}
```

**Fields:**
- `id`: Unique identifier for the speaker
- `status`: Current operational status
- `consensus`: Reference to consensus engine
- `messages`: Message history
- `discussions`: Active discussion threads
- `notification_config`: Notification settings
- `subscriptions`: Event subscriptions
- `webhook_urls`: External webhook endpoints

### GuildMessage

A message in the guild communication system.

```rust
pub struct GuildMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: DateTime<Utc>,
    pub important: bool,
    pub channels: Vec<NotificationChannel>,
    pub recipients: Option<Vec<String>>,
}
```

**Fields:**
- `id`: Unique message identifier
- `sender`: Servant ID that sent the message
- `content`: Message content
- `message_type`: Type of message
- `timestamp`: When the message was sent
- `important`: Whether message is important (triggers alerts)
- `channels`: Notification channels to use
- `recipients`: Specific recipients (None = broadcast)

### MessageType

Types of guild messages.

```rust
pub enum MessageType {
    Normal,
    Proposal,
    Vote,
    Result,
    Alert,
    System,
    TaskAssignment,
    TaskCompletion,
    SecurityEvent,
}
```

**Variants:**
- `Normal`: General communication
- `Proposal`: Proposal announcement
- `Vote`: Vote announcement
- `Result`: Consensus result
- `Alert`: Warning or alert
- `System`: System message
- `TaskAssignment`: Task assigned
- `TaskCompletion`: Task completed
- `SecurityEvent`: Security-related event

### NotificationChannel

Notification channels for distributing messages.

```rust
pub enum NotificationChannel {
    Console,
    Logs,
    External(String),
    Servant(String),
    All,
}
```

**Variants:**
- `Console`: Print to console
- `Logs`: Write to system logs
- `External(url)`: Send to external webhook
- `Servant(id)`: Send to specific servant
- `All`: Send to all channels

### NotificationConfig

Notification configuration settings.

```rust
pub struct NotificationConfig {
    pub channels: Vec<NotificationChannel>,
    pub log_notifications: bool,
    pub send_alerts: bool,
}
```

**Fields:**
- `channels`: Enabled notification channels
- `log_notifications`: Whether to log all notifications
- `send_alerts`: Whether to send alerts for important messages

### EventSubscription

Event subscription for selective notification.

```rust
pub struct EventSubscription {
    pub id: String,
    pub servant_id: String,
    pub event_types: Vec<MessageType>,
    pub active: bool,
}
```

**Fields:**
- `id`: Unique subscription ID
- `servant_id`: Servant subscribing to events
- `event_types`: Event types to subscribe to
- `active`: Whether subscription is active

### Discussion

A discussion thread for proposals.

```rust
pub struct Discussion {
    pub id: String,
    pub topic: String,
    pub proposal_id: Option<String>,
    // ... additional fields
}
```

---

## Initialization

### new()

Creates a new Speaker instance.

```rust
pub fn new() -> Self
```

**Returns:** A new Speaker instance

**Example:**
```rust
let speaker = Speaker::new();
```

### with_consensus()

Sets the consensus engine for the speaker.

```rust
pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self
```

**Parameters:**
- `consensus`: Shared reference to consensus engine

**Returns:** Self for builder pattern chaining

**Example:**
```rust
let speaker = Speaker::new()
    .with_consensus(consensus_engine);
```

---

## Message Management

### send_message()

Sends a message through the guild communication system.

```rust
pub fn send_message(
    &self,
    sender: String,
    content: String,
    message_type: MessageType
) -> Result<String, ServantError>
```

**Parameters:**
- `sender`: Sender servant ID
- `content`: Message content
- `message_type`: Type of message

**Returns:**
- `Ok(String)`: Message ID
- `Err(ServantError)`: Error if sending failed

**Example:**
```rust
let msg_id = speaker.send_message(
    "coordinator".to_string(),
    "New task assigned".to_string(),
    MessageType::TaskAssignment
)?;
```

### broadcast()

Broadcasts a message to all channels.

```rust
pub fn broadcast(
    &self,
    message: GuildMessage
) -> Result<(), ServantError>
```

**Parameters:**
- `message`: Message to broadcast

**Returns:**
- `Ok(())`: Broadcast successful
- `Err(ServantError)`: Error if broadcast failed

**Example:**
```rust
let message = GuildMessage {
    id: uuid::Uuid::new_v4().to_string(),
    sender: "speaker".to_string(),
    content: "System maintenance scheduled".to_string(),
    message_type: MessageType::Alert,
    timestamp: Utc::now(),
    important: true,
    channels: vec![NotificationChannel::All],
    recipients: None,
};

speaker.broadcast(message)?;
```

### get_message_history()

Gets message history.

```rust
pub fn get_message_history(&self) -> Vec<GuildMessage>
```

**Returns:** List of historical messages

**Example:**
```rust
let history = speaker.get_message_history();
for msg in history.iter().take(10) {
    println!("{}: {}", msg.sender, msg.content);
}
```

---

## Proposal Management

### announce_proposal()

Announces a new proposal to the guild.

```rust
pub async fn announce_proposal(&self, proposal: Proposal) -> Result<(), ServantError>
```

**Parameters:**
- `proposal`: Proposal to announce

**Returns:**
- `Ok(())`: Announcement successful
- `Err(ServantError)`: Error if announcement failed

**Example:**
```rust
let proposal = Proposal {
    id: uuid::Uuid::new_v4().to_string(),
    title: "Update system configuration".to_string(),
    description: "Update max_auto_risk_level to 3".to_string(),
    // ... other fields
};

speaker.announce_proposal(proposal).await?;
```

### announce_result()

Announces the result of a proposal vote.

```rust
pub async fn announce_result(&self, proposal_id: &str, result: ConsensusResult) -> Result<(), ServantError>
```

**Parameters:**
- `proposal_id`: ID of the proposal
- `result`: Consensus result

**Returns:**
- `Ok(())`: Announcement successful
- `Err(ServantError)`: Error if announcement failed

**Example:**
```rust
let result = ConsensusResult {
    proposal_id: "proposal-123".to_string(),
    passed: true,
    vote_counts: /* ... */,
    timestamp: Utc::now(),
};

speaker.announce_result("proposal-123", result).await?;
```

---

## Voting System

### collect_votes()

Collects votes for a proposal.

```rust
pub async fn collect_votes(&self, proposal_id: &str) -> Result<VoteTally, ServantError>
```

**Parameters:**
- `proposal_id`: ID of the proposal

**Returns:**
- `Ok(VoteTally)`: Vote tally results
- `Err(ServantError)`: Error if collection failed

**Example:**
```rust
let tally = speaker.collect_votes("proposal-123").await?;
println!("Votes for: {}", tally.votes_for);
println!("Votes against: {}", tally.votes_against);
```

### tally_votes()

Tallies votes and determines if proposal passes.

```rust
pub async fn tally_votes(&self, proposal_id: &str) -> Result<bool, ServantError>
```

**Parameters:**
- `proposal_id`: ID of the proposal

**Returns:**
- `Ok(bool)`: True if proposal passes, false otherwise
- `Err(ServantError)`: Error if tally failed

**Example:**
```rust
let passed = speaker.tally_votes("proposal-123").await?;
if passed {
    println!("Proposal passed!");
} else {
    println!("Proposal rejected");
}
```

---

## Notifications

### send_notification()

Sends a notification through specified channels.

```rust
pub fn send_notification(
    &self,
    message: GuildMessage,
    channels: Vec<NotificationChannel>
) -> Result<(), ServantError>
```

**Parameters:**
- `message`: Message to send
- `channels`: Channels to send through

**Returns:**
- `Ok(())`: Notification sent successfully
- `Err(ServantError)`: Error if sending failed

**Example:**
```rust
speaker.send_notification(
    message,
    vec![
        NotificationChannel::Console,
        NotificationChannel::Logs,
    ]
)?;
```

### set_notification_config()

Sets the notification configuration.

```rust
pub fn set_notification_config(&self, config: NotificationConfig)
```

**Parameters:**
- `config`: New notification configuration

**Example:**
```rust
let config = NotificationConfig {
    channels: vec![
        NotificationChannel::Console,
        NotificationChannel::Logs,
        NotificationChannel::External("https://hooks.slack.com/xxx".to_string()),
    ],
    log_notifications: true,
    send_alerts: true,
};

speaker.set_notification_config(config);
```

---

## Event Subscriptions

### subscribe()

Subscribes a servant to specific event types.

```rust
pub fn subscribe(
    &self,
    servant_id: &str,
    event_types: Vec<MessageType>
) -> Result<String, ServantError>
```

**Parameters:**
- `servant_id`: Servant ID to subscribe
- `event_types`: Event types to subscribe to

**Returns:**
- `Ok(String)`: Subscription ID
- `Err(ServantError)`: Error if subscription failed

**Example:**
```rust
let sub_id = speaker.subscribe(
    "worker",
    vec![
        MessageType::TaskAssignment,
        MessageType::Proposal,
        MessageType::Alert,
    ]
)?;
```

### unsubscribe()

Unsubscribes from event types.

```rust
pub fn unsubscribe(&self, subscription_id: &str) -> Result<(), ServantError>
```

**Parameters:**
- `subscription_id`: ID of subscription to cancel

**Returns:**
- `Ok(())`: Unsubscribe successful
- `Err(ServantError)`: Error if unsubscribe failed

**Example:**
```rust
speaker.unsubscribe("sub-123")?;
```

### get_subscriptions()

Gets all active subscriptions.

```rust
pub fn get_subscriptions(&self) -> Vec<EventSubscription>
```

**Returns:** List of active subscriptions

**Example:**
```rust
let subs = speaker.get_subscriptions();
for sub in subs {
    println!("{} subscribed to {:?}", sub.servant_id, sub.event_types);
}
```

---

## Discussions

### create_discussion()

Creates a new discussion thread.

```rust
pub fn create_discussion(
    &self,
    topic: String,
    proposal_id: Option<String>
) -> Result<String, ServantError>
```

**Parameters:**
- `topic`: Discussion topic
- `proposal_id`: Optional related proposal ID

**Returns:**
- `Ok(String)`: Discussion ID
- `Err(ServantError)`: Error if creation failed

**Example:**
```rust
let discussion_id = speaker.create_discussion(
    "System configuration updates".to_string(),
    Some("proposal-123".to_string())
)?;
```

### add_message_to_discussion()

Adds a message to a discussion.

```rust
pub fn add_message_to_discussion(
    &self,
    discussion_id: &str,
    message: GuildMessage
) -> Result<(), ServantError>
```

**Parameters:**
- `discussion_id`: ID of the discussion
- `message`: Message to add

**Returns:**
- `Ok(())`: Message added successfully
- `Err(ServantError)`: Error if addition failed

**Example:**
```rust
speaker.add_message_to_discussion("discussion-456", message)?;
```

---

## Webhook Integration

### add_webhook()

Adds an external webhook URL.

```rust
pub fn add_webhook(&self, url: String)
```

**Parameters:**
- `url`: Webhook URL

**Example:**
```rust
speaker.add_webhook("https://hooks.slack.com/services/XXX/YYY/ZZZ".to_string());
```

### remove_webhook()

Removes a webhook URL.

```rust
pub fn remove_webhook(&self, url: &str)
```

**Parameters:**
- `url`: Webhook URL to remove

**Example:**
```rust
speaker.remove_webhook("https://hooks.slack.com/services/XXX/YYY/ZZZ");
```

---

## Error Handling

### ServantError

Error types for speaker operations.

```rust
pub enum ServantError {
    MessageSendFailed(String),
    NotificationFailed(String),
    ProposalNotFound(String),
    VoteCollectionFailed(String),
    SubscriptionError(String),
    WebhookError(String),
}
```

**Error Handling Example:**
```rust
match speaker.send_message(sender, content, message_type) {
    Ok(msg_id) => println!("Message sent: {}", msg_id),
    Err(ServantError::MessageSendFailed(msg)) => {
        eprintln!("Failed to send message: {}", msg);
    },
    Err(e) => {
        eprintln!("Speaker error: {:?}", e);
    }
}
```

---

## Usage Examples

### Example 1: Basic Message Sending

```rust
use servant_guild::servants::{Speaker, MessageType};

fn main() {
    let speaker = Speaker::new();

    let msg_id = speaker.send_message(
        "coordinator".to_string(),
        "New task assigned to worker".to_string(),
        MessageType::TaskAssignment
    ).expect("Failed to send message");

    println!("Message sent: {}", msg_id);
}
```

### Example 2: Multi-Channel Notifications

```rust
use servant_guild::servants::{Speaker, NotificationConfig, NotificationChannel, MessageType};
use std::sync::Arc;
use servant_guild::consensus::ConsensusEngine;

fn main() {
    let consensus = ConsensusEngine::new();
    let speaker = Speaker::new()
        .with_consensus(Arc::new(consensus));

    // Configure notifications
    let config = NotificationConfig {
        channels: vec![
            NotificationChannel::Console,
            NotificationChannel::Logs,
            NotificationChannel::External("https://hooks.slack.com/xxx".to_string()),
        ],
        log_notifications: true,
        send_alerts: true,
    };

    speaker.set_notification_config(config);

    // Send important alert
    let msg_id = speaker.send_message(
        "warden".to_string(),
        "Security violation detected".to_string(),
        MessageType::Alert
    ).expect("Failed to send message");

    println!("Alert sent: {}", msg_id);
}
```

### Example 3: Event Subscriptions

```rust
use servant_guild::servants::{Speaker, MessageType};

fn main() {
    let speaker = Speaker::new();

    // Worker subscribes to task assignments and alerts
    let sub_id = speaker.subscribe(
        "worker",
        vec![
            MessageType::TaskAssignment,
            MessageType::Alert,
            MessageType::SecurityEvent,
        ]
    ).expect("Failed to subscribe");

    println!("Worker subscribed: {}", sub_id);

    // Warden subscribes to all security events
    let warden_sub = speaker.subscribe(
        "warden",
        vec![MessageType::SecurityEvent]
    ).expect("Failed to subscribe");

    // View all subscriptions
    let subs = speaker.get_subscriptions();
    println!("Total subscriptions: {}", subs.len());
}
```

### Example 4: Proposal Announcements

```rust
use servant_guild::servants::Speaker;
use servant_guild::consensus::{Proposal, DecisionType, ProposalStatus};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let speaker = Speaker::new();

    let proposal = Proposal {
        id: uuid::Uuid::new_v4().to_string(),
        title: "Update security policy".to_string(),
        description: "Reduce max_auto_risk_level to 3".to_string(),
        decision_type: DecisionType::Majority,
        status: ProposalStatus::Active,
        proposed_by: "warden".to_string(),
        created_at: chrono::Utc::now(),
        expires_at: None,
        quorum_required: 3,
    };

    speaker.announce_proposal(proposal).await?;
    println!("Proposal announced to guild");

    Ok(())
}
```

### Example 5: Webhook Integration

```rust
use servant_guild::servants::Speaker;

fn main() {
    let speaker = Speaker::new();

    // Add Slack webhook
    speaker.add_webhook(
        "https://hooks.slack.com/services/T000/B000/XXXX"
            .to_string()
    );

    // Add Discord webhook
    speaker.add_webhook(
        "https://discord.com/api/webhooks/XXX/YYY"
            .to_string()
    );

    // Send notification to all webhooks
    let message = create_guild_message(
        "System maintenance in 1 hour".to_string(),
        MessageType::Alert
    );

    speaker.broadcast(message).expect("Failed to broadcast");
}

fn create_guild_message(content: String, msg_type: MessageType) -> GuildMessage {
    GuildMessage {
        id: uuid::Uuid::new_v4().to_string(),
        sender: "speaker".to_string(),
        content,
        message_type: msg_type,
        timestamp: chrono::Utc::now(),
        important: true,
        channels: vec![NotificationChannel::All],
        recipients: None,
    }
}
```

---

## Best Practices

### 1. Use Appropriate Message Types
```rust
// Use specific types for better filtering
speaker.send_message(
    "coordinator".to_string(),
    "Task completed".to_string(),
    MessageType::TaskCompletion  // Specific type
).expect("Failed");
```

### 2. Set Important Flag for Critical Messages
```rust
let mut message = GuildMessage { /* ... */ };
message.important = true;  // Triggers alerts
speaker.broadcast(message).expect("Failed");
```

### 3. Use Selective Subscriptions
```rust
// Subscribe only to relevant events
speaker.subscribe(
    "worker",
    vec![
        MessageType::TaskAssignment,  // Relevant
        MessageType::Alert,           // Important
        // Don't subscribe to all events
    ]
).expect("Failed");
```

### 4. Configure Webhooks Securely
```rust
// Store webhook URLs securely
let webhook_url = std::env::var("SLACK_WEBHOOK_URL")
    .expect("SLACK_WEBHOOK_URL not set");

speaker.add_webhook(webhook_url);
```

### 5. Log Notifications for Audit
```rust
let config = NotificationConfig {
    log_notifications: true,  // Enable logging
    // ...
};
speaker.set_notification_config(config);
```

---

## Performance Considerations

- **Message History**: Store only recent messages to avoid memory bloat
- **Webhook Timeouts**: Set reasonable timeouts for webhook calls
- **Subscription Lookup**: Use efficient data structures for fast lookup
- **Notification Batching**: Batch notifications for external services

---

## Limitations

- Webhook calls are synchronous (may block if endpoint is slow)
- Message history is in-memory only (not persistent)
- No message delivery confirmation for external channels
- No retry logic for failed webhook calls
- Discussion threads are not persisted

---

## Future Enhancements

- **Persistent Message Store**: Save messages to database
- **Webhook Retry Logic**: Automatic retry for failed deliveries
- **Message Prioritization**: Queue system for important messages
- **Encryption**: End-to-end encryption for sensitive messages
- **Rich Content**: Support for attachments and formatting
- **Delivery Receipts**: Confirm message delivery
- **Rate Limiting**: Prevent notification spam

---

## See Also

- [Consensus Engine](../../consensus/README.md)
- [Worker API Reference](worker_api_reference.md)
- [Coordinator API Reference](coordinator_api_reference.md)
- [Architecture Overview](../../architecture/servant_guild_architecture_v1.0.md)
