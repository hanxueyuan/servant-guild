# Phase 2: Assembly - The Guild Formed

**Status:** Pending
**Focus:** Core Servants Logic, Consensus Engine, and Multi-Agent Collaboration
**Reference:** `docs/design/servant_guild_whitepaper_v1.1.md`, `docs/architecture/servant_guild_architecture_v1.0.md`

## 1. Core Servants Implementation (The Team)

Transform the prototypes from Phase 1 into fully functional agents with distinct roles and responsibilities.

- [ ] **Coordinator (The Brain)**
    - [ ] Implement advanced task decomposition (planning) using LLM.
    - [ ] Implement delegation logic: Assign subtasks to Worker or Contractor.
    - [ ] Implement status aggregation: Collect results from other agents and report to Owner.
    - [ ] **Deliverable**: `servant-coordinator.wasm` capable of handling complex multi-step instructions.

- [ ] **Worker (The Hands)**
    - [ ] Implement robust tool execution loop (ReAct pattern or similar).
    - [ ] Integrate full suite of Host Tools: File System, Network (HTTP), Shell.
    - [ ] Implement error handling and retry logic for tool failures.
    - [ ] **Deliverable**: `servant-worker.wasm` capable of executing code modification and system operations safely.

- [ ] **Warden (The Guard)**
    - [ ] Implement "Prudent Agency" audit logic: Review pending actions from Worker.
    - [ ] Implement security policy enforcement (e.g., prevent access to `.env`, restrict network domains).
    - [ ] Implement performance monitoring (optional for Phase 2).
    - [ ] **Deliverable**: `servant-warden.wasm` acting as a mandatory middleware for high-risk actions.

- [ ] **Speaker (The Voice)**
    - [ ] Implement proposal creation interface.
    - [ ] Implement vote collection and tallying logic.
    - [ ] Implement announcement of consensus results.
    - [ ] **Deliverable**: `servant-speaker.wasm` managing the governance lifecycle.

- [ ] **Contractor (The Builder)**
    - [ ] Implement configuration management: Read/Write agent configs.
    - [ ] Implement lifecycle hooks: Initialize new agents (mock for Phase 2).
    - [ ] **Deliverable**: `servant-contractor.wasm` managing agent metadata.

## 2. Consensus Engine (The Soul)

Establish the mechanism for collective decision-making.

- [ ] **Host Consensus Bridge (`src/runtime/bridges/consensus.rs`)**
    - [ ] Implement `propose` with persistence (SQLite/Sled).
    - [ ] Implement `vote` with cryptographic signing (simulated or real).
    - [ ] Implement `tally` logic to determine pass/fail based on quorum (3/5 or 5/5).
- [ ] **Governance Flow**
    - [ ] Define "Constitution": Rules for what requires a vote (e.g., Code Push, Config Change).
    - [ ] Implement voting workflow: Proposal -> Discussion (Chat) -> Vote -> Execution.

## 3. Memory & Knowledge (The Library)

Enable agents to remember past interactions and access project knowledge.

- [ ] **Host Memory Bridge (`src/runtime/bridges/memory.rs`)**
    - [ ] Connect to `src/memory/` backend (PostgreSQL/Vector DB).
    - [ ] Implement `get`/`set` for short-term context.
    - [ ] Implement `search` for long-term semantic retrieval.
- [ ] **Agent Integration**
    - [ ] Update `servant-sdk` to expose Memory APIs.
    - [ ] Enable Coordinator to retrieve past task history.

## 4. Integration & Verification

- [ ] **Multi-Agent Test Scenario**
    - [ ] **Scenario**: "Update the README to include a new feature."
    - [ ] **Flow**:
        1. Owner -> Coordinator: "Update README."
        2. Coordinator -> Worker: "Draft the change."
        3. Worker -> Warden: "Request permission to write README.md."
        4. Warden -> Host: "Audit Log: Write Allowed."
        5. Worker -> Host: "Write File."
        6. Coordinator -> Owner: "Done."
- [ ] **Consensus Test Scenario**
    - [ ] **Scenario**: "Change the system prompt of the Worker."
    - [ ] **Flow**:
        1. Contractor -> Speaker: "Propose config change."
        2. Speaker -> All: "Vote required."
        3. Agents -> Speaker: "Vote YES."
        4. Speaker -> Host: "Execute change."

## 5. Documentation

- [ ] Create `docs/guides/servant_roles_deep_dive.md`.
- [ ] Update `docs/architecture/c4_component.puml` with detailed interactions.
