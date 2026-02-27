# Phase 1: Genesis - ServantGuild Foundation (Updated)

**Status:** In Progress
**Focus:** Runtime Infrastructure, Safety Core, and Basic Servant Prototypes
**Reference:** `docs/design/servant_guild_whitepaper_v1.1.md`

## 1. Runtime Infrastructure (Wasmtime Host)

The foundation of the ServantGuild is a secure, efficient Wasm runtime based on the Wasmtime Component Model.

- [x] **Dependency Migration**
    - [x] Add `wasmtime`, `wasi-common`, `wit-bindgen` to `Cargo.toml`.
    - [x] Remove `wasmi` (or deprecate usage).
- [ ] **WIT Interface Definition (`wit/host.wit`)**
    - [ ] Define `llm` world (Text generation, Embedding).
    - [ ] Define `tools` world (File I/O via Safety wrapper, Network, Shell).
    - [ ] Define `memory` world (Vector store access, Short-term memory).
    - [ ] Define `consensus` world (Voting, Proposal).
- [ ] **Host Implementation (`src/runtime/`)**
    - [x] **Core Engine Setup**: Config, Fuel metering, Linker initialization.
    - [ ] **LLM Trait Bridge**: Expose `src/providers` to Wasm guests via WIT.
    - [ ] **Tool Trait Bridge**: Expose `src/tools` to Wasm guests via WIT.
    - [ ] **Safety Layer Integration**: Ensure all Guest operations pass through `src/safety`.
- [ ] **Guest SDK (`crates/zeroclaw-sdk`)**
    - [ ] Create a Rust crate for convenient Wasm guest development.
    - [ ] Wrap generated WIT bindings in idiomatic Rust APIs.

## 2. Safety & Security Core (Prudent Agency)

Before any agent can act, the safety mechanisms must be in place.

- [x] **Module Structure**
    - [x] Migrate `src/security/audit.rs` to `src/safety/audit.rs`.
    - [x] Create `src/safety/snapshot.rs`.
    - [x] Create `src/safety/rollback.rs` (Placeholder/Basic impl).
- [ ] **Audit System**
    - [ ] Implement structured logging for all side-effects.
    - [ ] Add tamper-evident hashing (optional for Phase 1).
- [ ] **Snapshot Manager**
    - [x] Implement file-level backup.
    - [ ] Implement system-level state snapshot (DB/Memory).
- [ ] **Rollback Mechanism**
    - [ ] Implement atomic rollback for file operations.
    - [ ] Define recovery policies for failed agent actions.

## 3. Core Servants (Prototypes)

Develop the initial Wasm modules for the core roles.

- [ ] **Coordinator (The Brain)**
    - [ ] Implement basic task dispatch logic.
    - [ ] Connect to Host LLM interface.
- [ ] **Worker (The Hands)**
    - [ ] Implement tool execution logic.
    - [ ] Connect to Host Tools interface.
- [ ] **Warden (The Guard)**
    - [ ] Implement basic audit log verification.
    - [ ] Define simple safety policies.
- [ ] **Speaker (The Voice)**
    - [ ] Implement basic voting interface.

## 4. Consensus Engine (The Soul)

The mechanism for collective decision making.

- [ ] **Vote Manager (`src/consensus/`)**
    - [ ] Define `Proposal` and `Vote` structs.
    - [ ] Implement tallying logic.
    - [ ] Integrate with `Speaker` servant.

## 5. Integration & Verification

- [ ] **End-to-End Test**
    - [ ] Host loads `coordinator.wasm`.
    - [ ] Coordinator receives a user instruction.
    - [ ] Coordinator delegates to `worker.wasm` (mocked or real).
    - [ ] Worker executes a safe tool (e.g., read file).
    - [ ] Warden verifies the action.
- [ ] **Documentation**
    - [ ] Update `AGENTS.md` (Completed).
    - [ ] Create `docs/guides/wasm_servant_development.md`.

## 6. Milestones

1.  **M1: Runtime Boot** - Host can load a simple Wasm component and call a "Hello World" function. (Target: Immediate)
2.  **M2: Safe Tools** - Guest can execute a file read via the Host's Safety layer.
3.  **M3: LLM Loop** - Guest can call the Host's LLM provider to generate text.
4.  **M4: The Guild** - Multiple agents running and communicating (basic).

## Notes
- **Strict Adherence**: All code must follow the `AGENTS.md` protocols.
- **Wasm First**: Avoid implementing logic in the Host if it can belong in a Guest.
- **Test Driven**: Write tests for WIT interfaces before implementing.
