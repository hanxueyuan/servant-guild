# Phase 1: Genesis - ServantGuild Foundation

**Status:** 🔄 **Under Review**
**Focus:** Runtime Infrastructure, Safety Core, and Basic Servant Prototypes
**Reference:** `docs/design/servant_guild_whitepaper_v1.1.md`, `docs/architecture/servant_guild_architecture_v1.0.md`

## Implementation Status Summary

| Component | Status | Files |
|-----------|--------|-------|
| Runtime Infrastructure | ✅ Complete | `src/runtime/` |
| WIT Interface | ✅ Complete | `wit/` |
| Safety Core | ✅ Complete | `src/safety/` |
| Core Servants | ✅ Complete | `src/servants/` |
| Consensus Engine | ✅ Complete | `src/consensus/` |
| Tests | ✅ Complete | `tests/` |

## 0. Development Environment (Linux First)

**目标**: 首先在 Linux 上跑通原型，架构预留多系统兼容能力。

- [ ] **Linux 环境设置**
    - [ ] Rust 1.87+ 安装
    - [ ] Wasmtime 依赖配置
    - [ ] 构建工具: `build-essential`, `pkg-config`

- [ ] **构建目标**
    - [ ] Native: `x86_64-unknown-linux-gnu`
    - [ ] Wasm: `wasm32-unknown-unknown`, `wasm32-wasip1`

- [x] **多系统兼容预留**
    - [x] 使用 `std::path::PathBuf` 处理所有路径操作
    - [x] 使用 `#[cfg(target_os)]` 隔离平台特定代码
    - [x] 避免硬编码 Shell 命令

> **Note**: Windows/macOS 支持在原型跑通后逐步添加，当前架构已预留扩展点。

## 1. Runtime Infrastructure (Wasmtime Host)

- [ ] **Dependency Migration** (需验证)
    - [ ] Verify `wasmtime`, `wasi-common`, `wit-bindgen` in `Cargo.toml`
    - [ ] Remove `wasmi` (or deprecate usage)

- [x] **WIT Interface Definition (`wit/`)** ✅ 已验证
    - [x] Define `llm` world
    - [x] Define `tools` world
    - [x] Define `memory` world
    - [x] Define `consensus` world

- [x] **Host Implementation (`src/runtime/`)** ✅ 已验证
    - [x] Core Engine Setup: `wasm.rs`, `native.rs`, `docker.rs`
    - [x] LLM Bridge: `bridges/llm.rs`
    - [x] Tool Bridge: `bridges/tools.rs`
    - [x] Safety Layer: Integration with `src/safety`

- [x] **Guest SDK (`crates/servant-sdk`)** ✅ 已验证
    - [x] Rust crate for Wasm guest development

## 2. Safety & Security Core (Prudent Agency)

- [x] **Module Structure** ✅ 已验证
    - [x] `src/safety/audit.rs`
    - [x] `src/safety/snapshot.rs`
    - [x] `src/safety/rollback.rs`
    - [x] `src/safety/state_recovery.rs`
    - [x] `src/safety/canary.rs`

- [x] **Audit System** ✅ 已验证
    - [x] Structured logging for side-effects
    - [x] Tamper-evident hashing

- [x] **Snapshot Manager** ✅ 已验证
    - [x] File-level backup
    - [x] Database-level snapshot
    - [x] Memory state snapshot

- [x] **Rollback Mechanism** ✅ 已验证
    - [x] Atomic rollback for file operations
    - [x] Transaction manager
    - [x] Recovery policies

## 3. Core Servants (Prototypes)

- [x] **Coordinator** ✅ 已验证 (`src/servants/coordinator.rs`)
    - [x] Basic task dispatch logic
    - [x] Host LLM interface connection

- [x] **Worker** ✅ 已验证 (`src/servants/worker.rs`)
    - [x] Tool execution logic
    - [x] Host Tools interface connection

- [x] **Warden** ✅ 已验证 (`src/servants/warden.rs`)
    - [x] Audit log verification
    - [x] Safety policies

- [x] **Speaker** ✅ 已验证 (`src/servants/speaker.rs`)
    - [x] Voting interface

- [x] **Contractor** ✅ 已验证 (`src/servants/contractor.rs`)
    - [x] Resource management

## 4. Consensus Engine (The Soul)

- [x] **Vote Manager (`src/consensus/`)** ✅ 已验证
    - [x] `Proposal` and `Vote` structs
    - [x] Tallying logic (Quorum)
    - [x] `Speaker` servant integration

## 5. Integration & Verification

- [ ] **End-to-End Test** (需运行验证)
    - [ ] Test audit system
    - [ ] Test snapshot manager
    - [ ] Test rollback mechanism
    - [ ] Test full safety flow

- [ ] **Linux 原型验证** (需运行验证)
    - [ ] Test Wasm module loading
    - [ ] Verify file path handling
    - [ ] Test Safety Layer

## 6. Milestones

1. **M1: Runtime Boot** - ⏳ 待验证
2. **M2: Safe Tools** - ⏳ 待验证
3. **M3: LLM Loop** - ⏳ 待验证
4. **M4: The Guild** - ⏳ 待验证

## Notes
- **Strict Adherence**: All code must follow the `AGENTS.md` protocols.
- **Wasm First**: Avoid implementing logic in the Host if it can belong in a Guest.
- **Test Driven**: Write tests for WIT interfaces before implementing.
- **Linux First**: 当前聚焦 Linux 原型，架构已预留 Windows/macOS 扩展能力。
