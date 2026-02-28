# Phase 1: Genesis - ServantGuild Foundation (Updated)

**Status:** Completed ✅
**Focus:** Runtime Infrastructure, Safety Core, and Basic Servant Prototypes
**Reference:** `docs/design/servant_guild_whitepaper_v1.1.md`

## 1. Runtime Infrastructure (Wasmtime Host)

The foundation of the ServantGuild is a secure, efficient Wasm runtime based on the Wasmtime Component Model.

- [x] **Dependency Migration**
    - [x] Add `wasmtime`, `wasi-common`, `wit-bindgen` to `Cargo.toml`.
    - [x] Remove `wasmi` (or deprecate usage).
- [x] **WIT Interface Definition (`wit/host.wit`)**
    - [x] Define `llm` world (Text generation, Embedding).
    - [x] Define `tools` world (File I/O via Safety wrapper, Network, Shell).
    - [x] Define `memory` world (Vector store access, Short-term memory).
    - [x] Define `consensus` world (Voting, Proposal).
- [x] **Host Implementation (`src/runtime/`)**
    - [x] **Core Engine Setup**: Config, Fuel metering, Linker initialization.
    - [x] **LLM Trait Bridge**: Expose `src/providers` to Wasm guests via WIT.
    - [x] **Tool Trait Bridge**: Expose `src/tools` to Wasm guests via WIT.
    - [x] **Safety Layer Integration**: Ensure all Guest operations pass through `src/safety`.
- [x] **Guest SDK (`crates/servant-sdk`)**
    - [x] Create a Rust crate for convenient Wasm guest development.
    - [x] Wrap generated WIT bindings in idiomatic Rust APIs.

## 2. Safety & Security Core (Prudent Agency)

Before any agent can act, the safety mechanisms must be in place.

- [x] **Module Structure**
    - [x] Migrate `src/security/audit.rs` to `src/safety/audit.rs`.
    - [x] Create `src/safety/snapshot.rs`.
    - [x] Create `src/safety/rollback.rs` (Full implementation).
- [x] **Audit System**
    - [x] Implement structured logging for all side-effects.
    - [x] Add tamper-evident hashing (hash chain verification).
- [x] **Snapshot Manager**
    - [x] Implement file-level backup.
    - [x] Implement database-level snapshot (SQLite).
    - [x] Implement memory state snapshot.
    - [x] Implement system-level state snapshot (multi-component).
- [x] **Rollback Mechanism**
    - [x] Implement atomic rollback for file operations.
    - [x] Implement transaction manager with prepare/execute/commit lifecycle.
    - [x] Define recovery policies for failed agent actions.

## 3. Core Servants (Prototypes)

Develop the initial Wasm modules for the core roles.

- [x] **Coordinator (The Brain)**
    - [x] Implement basic task dispatch logic (Prototype created).
    - [x] Connect to Host LLM interface (via SDK).
- [x] **Worker (The Hands)**
    - [x] Implement tool execution logic (Prototype created).
    - [x] Connect to Host Tools interface (via SDK).
- [x] **Warden (The Guard)**
    - [x] Implement basic audit log verification (Prototype created).
    - [x] Define simple safety policies (Prototype created).
- [x] **Speaker (The Voice)**
    - [x] Implement basic voting interface (Prototype created).

## 4. Consensus Engine (The Soul)

The mechanism for collective decision making.

- [x] **Vote Manager (`src/consensus/`)**
    - [x] Define `Proposal` and `Vote` structs.
    - [x] Implement tallying logic (Basic Quorum).
    - [x] Integrate with `Speaker` servant (via SDK).

## 5. Integration & Verification

- [x] **End-to-End Test**
    - [x] Test audit system with hash chain integrity.
    - [x] Test snapshot manager (file, database, memory, system).
    - [x] Test rollback mechanism with transactions.
    - [x] Test full safety flow (audit → snapshot → execute → verify/rollback).
- [x] **Documentation**
    - [x] Update `AGENTS.md` (Completed).
    - [x] Create `docs/guides/wasm_servant_development.md` (Completed).

## 6. Milestones

1.  **M1: Runtime Boot** - Host can load a simple Wasm component and call a "Hello World" function. ✅
2.  **M2: Safe Tools** - Guest can execute a file read via the Host's Safety layer. ✅
3.  **M3: LLM Loop** - Guest can call the Host's LLM provider to generate text. ✅
4.  **M4: The Guild** - Multiple agents running and communicating (basic). ✅

## Notes
- **Strict Adherence**: All code must follow the `AGENTS.md` protocols.
- **Wasm First**: Avoid implementing logic in the Host if it can belong in a Guest.
- **Test Driven**: Write tests for WIT interfaces before implementing.
