# Phase 3: Evolution - The Self-Improvement Loop

**Status:** Pending
**Focus:** GitHub Integration, Build Automation, and Hot-Swap Mechanism
**Reference:** `docs/design/servant_guild_whitepaper_v1.1.md`, `docs/architecture/servant_guild_architecture_v1.0.md`

## 1. GitHub Integration (The Genome)

Enable the ServantGuild to access and modify its own source code.

- [ ] **Host GitHub Bridge (`src/runtime/bridges/github.rs`)**
    - [ ] Implement `repo` interface: `clone`, `pull`, `commit`, `push`.
    - [ ] Implement `pr` interface: `create`, `list`, `comment`.
    - [ ] Implement `release` interface: `create_release`, `upload_asset`.
    - [ ] **Security**: Ensure GitHub PAT is stored securely and accessed only by authorized agents.
- [ ] **Agent Integration**
    - [ ] Update `servant-sdk` with GitHub APIs.
    - [ ] Enable Contractor to browse and manage the codebase.

## 2. Build Automation (The Forge)

Allow agents to compile Rust code into Wasm binaries.

- [ ] **Host Build Tools (`src/tools/build.rs`)**
    - [ ] Implement `cargo build` wrapper: Support target `wasm32-wasip1`.
    - [ ] Implement `wit-bindgen` integration: Ensure bindings are up-to-date.
    - [ ] **Safety**: Sandbox the build process (Docker or restricted environment).
- [ ] **Contractor Capabilities**
    - [ ] Implement build pipeline logic: "Pull -> Edit -> Build -> Test -> Commit".
    - [ ] Implement error analysis: Parse compiler errors and attempt fixes.

## 3. Hot-Swap Mechanism (The Metamorphosis)

Enable the runtime to update Wasm modules without restarting the Host.

- [ ] **Runtime Manager (`src/runtime/manager.rs`)**
    - [ ] Implement module versioning: `servant-v1.wasm`, `servant-v2.wasm`.
    - [ ] Implement atomic swap: Replace running instance with new version.
    - [ ] Implement state migration: Export/Import memory or use external storage during swap.
- [ ] **Consensus Integration**
    - [ ] Define "Update Proposal": A special proposal type that triggers a hot-swap.
    - [ ] Implement verification: Only signed/hashed Wasm binaries are accepted.

## 4. Rollback & Recovery (The Safety Net)

Ensure the system can recover from bad updates.

- [ ] **Snapshot Manager**
    - [ ] Implement full system snapshot before update.
    - [ ] Implement `rollback` command: Revert to previous Wasm version and state.
- [ ] **Warden Logic**
    - [ ] Implement "Canary Test": Run new version in isolation before full deployment.
    - [ ] Implement automatic rollback trigger on critical failure.

## 5. Integration & Verification

- [ ] **Self-Evolution Scenario**
    - [ ] **Scenario**: "Fix a typo in the `servant-worker` log message."
    - [ ] **Flow**:
        1. Owner -> Coordinator: "Fix the typo."
        2. Coordinator -> Contractor: "Pull code, find file, edit."
        3. Contractor -> Worker: "Edit `src/lib.rs`."
        4. Contractor -> Host: "Build Wasm."
        5. Contractor -> Warden: "Run tests."
        6. Contractor -> Speaker: "Propose Update."
        7. Speaker -> All: "Vote YES."
        8. Speaker -> Host: "Deploy new Wasm."
        9. Host: Hot-swap.
        10. Verification: Check logs for fixed typo.

## 6. Documentation

- [ ] Create `docs/guides/agent_development_lifecycle.md`.
- [ ] Document the update protocol in `AGENTS.md`.
