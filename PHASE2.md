# ServantGuild - Phase 2: Assembly (Cognition)

**Status:** ✅ **Completed**
**Reference:** [Whitepaper v1.1](./docs/design/servant_guild_whitepaper_v1.1.md)

## Overview

Phase 2 implements the **Assembly** capabilities of ServantGuild - establishing the Consensus Engine for collective decision-making, LLM integration for intelligent reasoning, and inter-servant communication protocols.

## Core Philosophy Alignment

From the ServantGuild Whitepaper v1.1:

> **Phase 2: 团队 (Assembly)**
> - 实现 5 大核心使魔的角色逻辑 (Wasm 模块)。
> - 实现团议投票机制 (Consensus Engine)。

Phase 2 delivers on this promise by implementing:

1. **Consensus Engine** - The democratic heart of the Guild
2. **LLM Integration** - Intelligent reasoning capabilities
3. **Context Management** - Memory and conversation handling
4. **Inter-Servant Protocol** - Standardized communication

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Consensus Engine                                 │
│              (提案 → 投票 → 决策 → 执行)                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
         ┌──────────────────────────┼──────────────────────────┐
         │                          │                          │
         ▼                          ▼                          ▼
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│     Speaker     │       │   Coordinator   │       │    Warden       │
│  (Propose Vote) │       │  (Dispatch Ops) │       │ (Audit Decisions)│
└────────┬────────┘       └────────┬────────┘       └────────┬────────┘
         │                         │                         │
         └─────────────────────────┼─────────────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────┐
                    │      LLM Provider       │
                    │   (豆包/DeepSeek/...)    │
                    └─────────────────────────┘
```

## Core Components

### 1. Consensus Engine (共识引擎)

The democratic heart of ServantGuild - all major decisions must pass through collective voting.

From the Whitepaper:
> **共识驱动 (Consensus-Driven)**: 所有关键决策需经多数同意，保障集体生存利益。

```rust
// src/consensus/mod.rs
pub struct ConsensusEngine {
    constitution: Constitution,      // The rules
    active_servants: HashSet<ServantId>,
    proposals: HashMap<String, Proposal>,
    votes: HashMap<String, Vec<Vote>>,
    config: ConsensusConfig,
}

impl ConsensusEngine {
    /// Create a new proposal
    pub fn create_proposal(
        &mut self,
        title: String,
        description: String,
        proposer: ServantId,
        decision_type: DecisionType,
    ) -> Result<Proposal>;
    
    /// Cast a vote
    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter: ServantId,
        vote: Vote,
        reason: String,
    ) -> Result<()>;
    
    /// Evaluate and tally votes
    pub fn evaluate(&self, proposal_id: &str) -> Result<VoteTally>;
    
    /// Owner veto
    pub fn veto(&mut self, proposal_id: &str, owner_id: &str) -> Result<()>;
}
```

#### Decision Types

```rust
pub enum DecisionType {
    /// Normal decisions require 3/5 approval
    Normal,
    /// Critical decisions require 5/5 unanimous approval
    Critical,
}

pub enum Vote {
    Approve,
    Reject,
    Abstain,
}
```

#### Quorum Rules

| Decision Type | Required Votes | Timeout |
|---------------|----------------|---------|
| Normal | 3/5 (60%) | 1 hour |
| Critical | 5/5 (100%) | 24 hours |

**Critical Decisions include:**
- Code updates and deployments
- Member addition/removal
- Configuration changes
- Budget allocation

### 2. Constitution (宪法)

The rule system that governs the Guild:

```rust
// src/consensus/constitution.rs
pub struct Constitution {
    /// Minimum quorum for normal decisions
    pub normal_quorum: usize,
    
    /// Minimum quorum for critical decisions
    pub critical_quorum: usize,
    
    /// Voting timeout in seconds
    pub voting_timeout_secs: u64,
    
    /// Owner veto enabled
    pub owner_veto_enabled: bool,
    
    /// Rules for categorizing decisions
    pub decision_rules: Vec<DecisionRule>,
}

pub struct DecisionRule {
    pub pattern: String,          // Match pattern for proposal content
    pub decision_type: DecisionType,
    pub auto_approve: bool,       // Skip voting if conditions met
    pub requires_owner: bool,     // Require owner awareness
}
```

### 3. LLM Integration (智能推理)

Integration with language models for intelligent reasoning:

```rust
// src/providers/mod.rs
pub struct LLMManager {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    default_provider: String,
    config: LLMConfig,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;
    
    /// Complete a prompt
    async fn complete(&self, request: CompleteRequest) -> Result<CompleteResponse>;
    
    /// Stream completion (for long responses)
    async fn complete_stream(&self, request: CompleteRequest) -> Result<CompletionStream>;
    
    /// Count tokens
    fn count_tokens(&self, text: &str) -> usize;
    
    /// Get pricing info
    fn pricing(&self) -> PricingInfo;
}
```

#### Supported Providers

| Provider | Model | Use Case |
|----------|-------|----------|
| 豆包 (Doubao) | LLM 2.0 Lite | Default, cost-effective |
| DeepSeek | DeepSeek Chat | Reasoning-heavy tasks |
| OpenAI | GPT-4o | Complex analysis |
| Anthropic | Claude 3.5 | Code generation |

### 4. Context Management (上下文管理)

Memory and conversation handling:

```rust
// src/memory/mod.rs
pub struct ContextManager {
    short_term: Arc<dyn Memory>,    // Redis
    long_term: Arc<dyn Memory>,     // PostgreSQL
    vector_store: Arc<VectorStore>, // pgvector
}

impl ContextManager {
    /// Add message to conversation
    pub async fn add_message(&self, conversation_id: &str, message: Message) -> Result<()>;
    
    /// Get conversation history
    pub async fn get_history(&self, conversation_id: &str) -> Result<Vec<Message>>;
    
    /// Search relevant context (RAG)
    pub async fn search_context(&self, query: &str, k: usize) -> Result<Vec<Context>>;
    
    /// Create snapshot
    pub async fn snapshot(&self, servant_id: &str) -> Result<SnapshotId>;
}
```

### 5. Inter-Servant Protocol (使魔通信协议)

Standardized communication between servants:

```rust
// src/protocol/mod.rs
pub enum GuildMessage {
    // Task management
    TaskAssign {
        target: ServantRole,
        task: Task,
        priority: Priority,
    },
    TaskResult {
        from: ServantId,
        task_id: String,
        result: TaskResult,
    },
    
    // Consensus
    ProposalCreate {
        proposal: Proposal,
    },
    VoteRequest {
        proposal_id: String,
        deadline: DateTime<Utc>,
    },
    VoteCast {
        voter: ServantId,
        proposal_id: String,
        vote: Vote,
        reason: String,
    },
    DecisionMade {
        proposal_id: String,
        outcome: DecisionOutcome,
    },
    
    // System
    Heartbeat {
        servant: ServantId,
        status: ServantStatus,
        timestamp: DateTime<Utc>,
    },
    Alert {
        severity: Severity,
        source: ServantId,
        message: String,
        requires_action: bool,
    },
}
```

## Communication Patterns

### 1. Task Dispatch Flow

```
Owner                 Coordinator              Worker
  │                        │                      │
  │  "Build the module"    │                      │
  │───────────────────────▶│                      │
  │                        │                      │
  │                        │  TaskAssign(Build)   │
  │                        │─────────────────────▶│
  │                        │                      │
  │                        │                      │ Execute
  │                        │                      │───────▶
  │                        │                      │
  │                        │  TaskResult(Success) │
  │                        │◀─────────────────────│
  │                        │                      │
  │  "Build completed"     │                      │
  │◀───────────────────────│                      │
```

### 2. Consensus Flow

```
Warden              Speaker              All Servants           Owner
  │                    │                      │                   │
  │  Bug detected      │                      │                   │
  │───────────────────▶│                      │                   │
  │                    │                      │                   │
  │                    │  ProposalCreate      │                   │
  │                    │─────────────────────▶│                   │
  │                    │                      │                   │
  │                    │  VoteRequest         │                   │
  │                    │─────────────────────▶│                   │
  │                    │                      │                   │
  │                    │                      │  VoteCast         │
  │                    │◀─────────────────────│                   │
  │                    │                      │                   │
  │                    │  DecisionMade        │                   │
  │                    │─────────────────────▶│                   │
  │                    │                      │                   │
  │                    │                      │  Notify (optional)│
  │                    │                      │──────────────────▶│
```

## Configuration

```toml
# servant-guild.toml

[consensus]
core_servants_count = 5
normal_quorum = 3
critical_quorum = 5
voting_timeout_secs = 3600
owner_veto_enabled = true

[consensus.rules]
# Auto-approve low-risk changes
auto_approve_docs = true
auto_approve_tests = true

# Always require voting for
require_vote_code = true
require_vote_config = true

[llm]
default_provider = "doubao"
fallback_provider = "deepseek"

[llm.providers.doubao]
model = "doubao-lite-2.0"
max_tokens = 4096
temperature = 0.7

[llm.providers.deepseek]
model = "deepseek-chat"
max_tokens = 8192
temperature = 0.5

[memory]
short_term_backend = "redis"
long_term_backend = "postgres"
vector_backend = "pgvector"

[memory.redis]
url = "redis://localhost:6379"
ttl_secs = 3600

[memory.postgres]
url = "postgres://localhost/servant_guild"
```

## Testing

```bash
# Run all tests
cargo test

# Run consensus tests
cargo test --test consensus_test

# Run LLM integration tests (requires API keys)
cargo test --test llm_integration_test -- --ignored

# Run full Phase 2 integration
cargo test --test phase2_integration_test
```

## Next Phase

Phase 3 (Evolution) builds on Phase 2 to deliver:
- GitHub integration (gene pool)
- Build automation
- Hot-swap mechanism
- Rollback & recovery
- Self-evolution engine

See [PHASE3.md](./PHASE3.md) for details.
