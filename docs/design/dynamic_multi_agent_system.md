# Dynamic Multi-Agent System Design

## 1. Overview
This document outlines the technical design for the "Smart Development Team" on top of the ZeroClaw architecture. The system introduces dynamic agent recruitment, ephemeral skill acquisition, and a structured collaboration protocol.

## 2. Architecture Extensions

### 2.1 Dynamic Agent Registry
*   **Module**: `src/agent/registry.rs`
*   **Purpose**: Manage the lifecycle of all agents (Core + Dynamic).
*   **Data Structure**:
    ```rust
    struct AgentRegistry {
        core_agents: HashMap<String, AgentConfig>,
        dynamic_agents: HashMap<String, DynamicAgent>,
    }

    struct DynamicAgent {
        config: AgentConfig,
        created_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
        project_id: Option<String>,
    }
    ```
*   **Functionality**:
    *   `register_agent(name, config, ttl)`: Dynamically recruit an agent.
    *   `get_agent(name)`: Retrieve agent config (checks expiration).
    *   `cleanup_expired()`: Remove agents whose TTL has passed.

### 2.2 Ephemeral Skill Management
*   **Module**: `src/skills/manager.rs` (enhancement)
*   **Purpose**: Allow agents to "learn" skills temporarily.
*   **Mechanism**:
    *   Skills are currently loaded from `skills/` directory.
    *   We will add a `DynamicSkill` struct that can store skill definitions in memory (or a temporary DB) with an expiration time.
    *   `bind_skill(agent_name, skill_name)`: Updates the agent's `allowed_tools` or prompts to include the skill.

### 2.3 Collaboration Protocol (SOP-based)
We will leverage the existing **SOP Engine** to enforce the structured dialogue protocol.

*   **SOP Name**: `collaborative_problem_solving`
*   **Steps**:
    1.  **Proposal (Tony)**: Tony analyzes the input and delegates to relevant specialists for initial ideas.
    2.  **Challenge (Lisa)**: Lisa reviews the proposals and generates counter-arguments or alternative ideas.
    3.  **Verification (Ben/Lei)**:
        *   Ben checks logic/code.
        *   Lei checks facts/citations.
    4.  **Integration (Tony)**: Tony synthesizes all inputs, resolves conflicts, and produces the final output.

### 2.4 Coordination Bus Enhancements
*   The `InMemoryMessageBus` in `src/coordination/` is sufficient for message passing.
*   We need to define standard message schemas for the protocol stages (e.g., `ProposalMessage`, `ChallengeMessage`, `VerificationReport`).

## 3. Core Agent Specifications

### 3.1 Tony (Coordinator)
*   **System Prompt**: Emphasize honesty, conflict resolution, and synthesis.
*   **Tools**: `delegate` (to other agents), `read_memory`, `write_memory`.

### 3.2 Lei (Research)
*   **System Prompt**: Focus on fact-checking and sourcing.
*   **Tools**: `web_search`, `web_fetch`, `memory_search` (knowledge base).

### 3.3 Ben (Logic)
*   **System Prompt**: Rigorous logic, mathematical verification.
*   **Tools**: `python_repl` (for math/logic checks), `code_analysis`.

### 3.4 Lisa (Creative)
*   **System Prompt**: Devil's advocate, lateral thinking.
*   **Tools**: `brainstorming_templates` (skill), `bias_detector` (skill).

## 4. Implementation Plan

1.  **Phase 1: Registry & Lifecycle**: Implement `AgentRegistry` and dynamic agent/skill creation.
2.  **Phase 2: Core Agents**: Define the `DelegateAgentConfig` for the 4 core roles in `config.toml`.
3.  **Phase 3: Collaboration SOP**: Create the `collaborative_problem_solving.yaml` SOP definition.
4.  **Phase 4: Integration**: Update the main loop to use the Registry and Trigger the SOP for complex tasks.
