# Wasm Servant Development Guide

This guide explains how to create, build, and deploy a Wasm Servant for the ServantGuild runtime.

## Overview

From the [Whitepaper v1.1](../design/servant_guild_whitepaper_v1.1.md):

> **安全隔离 (Isolation)**: 每个使魔运行在独立的 Wasm 沙盒中，互不干扰，热替换无重启。

Each servant runs in its own Wasm sandbox with:
- **Memory limit**: 512MB per instance
- **CPU fuel**: 5s max per call
- **File system**: Pre-opened directories only
- **Network**: Whitelisted domains only

## 1. Prerequisites

- **Rust**: Latest stable toolchain.
- **Wasm Target**: `rustup target add wasm32-wasi`
- **WIT Bindgen**: `cargo install wit-bindgen-cli` (Optional, for inspection)

## 2. Creating a New Servant

We use the `servant-sdk` to simplify development.

1.  **Create a new library crate**:
    ```bash
    cargo new --lib crates/servant-my-role
    ```

2.  **Update `Cargo.toml`**:
    ```toml
    [package]
    name = "servant-my-role"
    version = "0.1.0"
    edition = "2021"

    [lib]
    crate-type = ["cdylib"]

    [dependencies]
    servant-sdk = { path = "../servant-sdk" }
    wit-bindgen = "0.21.0"
    ```

3.  **Implement the Guest Trait**:
    In `src/lib.rs`:
    ```rust
    struct MyRole;

    impl servant_sdk::Guest for MyRole {
        fn handle_task(task_id: String, input: String) -> Result<String, String> {
            // Your logic here
            // Call host capabilities:
            // servant_sdk::servant_guild::host::llm::chat(...)
            // servant_sdk::servant_guild::host::tools::execute(...)
            
            Ok(format!("MyRole completed task {}", task_id))
        }
    }

    // Export the implementation to the Wasm runtime
    servant_sdk::export!(MyRole);
    ```

## 3. Building

Build the Wasm component:

```bash
cargo build -p servant-my-role --target wasm32-wasi --release
```

The resulting binary will be at:
`target/wasm32-wasi/release/servant_my_role.wasm`

## 4. Deploying

1.  Copy the `.wasm` file to the `tools/` directory (or configured `wasm_runtime.tools_dir`).
2.  The Host will load it by name (e.g., `my-role`).

## 5. Host Capabilities

The `servant-sdk` exposes the following host capabilities defined in `wit/host.wit`:

- **LLM**: `servant_guild::host::llm`
    - `chat(req)`: Generate text/tool calls.
    - `embed(req)`: Generate embeddings.
- **Tools**: `servant_guild::host::tools`
    - `execute(name, args)`: Run a host tool (safe).
    - `list()`: List available tools.
- **Safety**: `servant_guild::host::safety`
    - `audit_log(action, resource, ...)`: Log security events.
- **Consensus**: `servant_guild::host::consensus`
    - `propose(...)`: Create a proposal.
    - `vote(...)`: Vote on a proposal.
- **Memory**: `servant_guild::host::memory`
    - `get/set/delete`: KV store.
    - `search`: Vector store.

## 6. Testing

You can unit test your logic by extracting it into functions that don't depend on the host bindings, or by mocking the host bindings (advanced).

For integration testing, use the `servant_guild` CLI (Host) to load your servant.
