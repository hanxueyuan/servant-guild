# ServantGuild Commands Reference

This reference is derived from the current CLI surface.

**Last verified**: February 27, 2026
**Architecture**: [Whitepaper v1.1](./design/servant_guild_whitepaper_v1.1.md)

## Top-Level Commands

| Command | Purpose |
|---|---|
| `init` | Initialize workspace/config for ServantGuild |
| `daemon` | Start the Master Daemon (Wasm runtime + all servants) |
| `status` | Print current guild status and servant summary |
| `task` | Submit and manage tasks for the guild |
| `proposal` | Create and manage proposals for consensus |
| `vote` | Cast votes on active proposals |
| `evolve` | Trigger self-evolution workflow |
| `servant` | Manage servant instances |
| `estop` | Emergency stop (The Red Phone) |
| `doctor` | Run diagnostics and health checks |
| `config` | Export machine-readable config schema |
| `completions` | Generate shell completion scripts |

## Guild Management Commands

### `init`

Initialize ServantGuild workspace:

```bash
servant-guild init
servant-guild init --interactive
servant-guild init --admin-user <TELEGRAM_ID>
```

### `daemon`

Start the Master Daemon:

```bash
servant-guild daemon
servant-guild daemon --port 5000
servant-guild daemon --config /path/to/config.toml
```

The daemon manages:
- Wasmtime runtime for all servants
- Consensus Engine
- Message routing between servants
- External gateway (API/CLI)

### `status`

Show guild status:

```bash
servant-guild status
```

Output includes:
- Active servants and their status
- Active proposals and votes
- Resource usage (memory, CPU, tokens)
- Last heartbeat timestamps

## Task Commands

### `task`

Submit and manage tasks:

```bash
# Submit a new task
servant-guild task submit --type build --payload '{"module": "coordinator"}'

# List active tasks
servant-guild task list

# Get task status
servant-guild task status <task-id>

# Cancel a task
servant-guild task cancel <task-id>
```

## Consensus Commands

### `proposal`

Create and manage proposals:

```bash
# Create a proposal (triggers voting)
servant-guild proposal create --title "Update Worker to v2.0" \
    --type critical \
    --description "Performance improvements for Worker"

# List proposals
servant-guild proposal list

# Get proposal details
servant-guild proposal show <proposal-id>
```

### `vote`

Cast votes:

```bash
# Vote on a proposal
servant-guild vote <proposal-id> --approve --reason "LGTM"

# Vote against
servant-guild vote <proposal-id> --reject --reason "Need more testing"

# Abstain
servant-guild vote <proposal-id> --abstain
```

## Evolution Commands

### `evolve`

Trigger self-evolution workflow:

```bash
# Check for updates
servant-guild evolve check

# Trigger evolution manually
servant-guild evolve trigger --reason "Performance improvement"

# View evolution history
servant-guild evolve history
```

From the Whitepaper:
> **进化 (Evolution)**: 通过 GitHub 仓库作为基因库，使魔团能够编写、测试、发布自己的新版本。

## Servant Commands

### `servant`

Manage servant instances:

```bash
# List all servants
servant-guild servant list

# Get servant status
servant-guild servant status <servant-id>

# Create ephemeral servant (requires consensus)
servant-guild servant create --role worker --ttl 3600

# Destroy servant (requires consensus)
servant-guild servant destroy <servant-id>
```

## Emergency Commands

### `estop` (The Red Phone)

Emergency stop commands:

```bash
# Engage emergency stop (kills all servants)
servant-guild estop

# Stop specific servant
servant-guild estop --servant <servant-id>

# Resume from estop
servant-guild estop resume
```

From the [Infrastructure doc](./design/servant_guild_infrastructure.md):
> **紧急联络通道 (The Red Phone)**: 当自治系统失控时的最后一道防线。

---

## Legacy Commands (Backward Compatibility)

### `onboard`

- `zeroclaw onboard`
- `zeroclaw onboard --interactive`
- `zeroclaw onboard --channels-only`
- `zeroclaw onboard --force`
- `zeroclaw onboard --api-key <KEY> --provider <ID> --memory <sqlite|lucid|markdown|none>`
- `zeroclaw onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none>`
- `zeroclaw onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none> --force`

`onboard` safety behavior:

- If `config.toml` already exists and you run `--interactive`, onboarding now offers two modes:
  - Full onboarding (overwrite `config.toml`)
  - Provider-only update (update provider/model/API key while preserving existing channels, tunnel, memory, hooks, and other settings)
- In non-interactive environments, existing `config.toml` causes a safe refusal unless `--force` is passed.
- Use `zeroclaw onboard --channels-only` when you only need to rotate channel tokens/allowlists.

### `agent`

- `zeroclaw agent`
- `zeroclaw agent -m "Hello"`
- `zeroclaw agent --provider <ID> --model <MODEL> --temperature <0.0-2.0>`
- `zeroclaw agent --peripheral <board:path>`

Tip:

- In interactive chat, you can ask for route changes in natural language (for example “conversation uses kimi, coding uses gpt-5.3-codex”); the assistant can persist this via tool `model_routing_config`.

### `gateway` / `daemon`

- `zeroclaw gateway [--host <HOST>] [--port <PORT>]`
- `zeroclaw daemon [--host <HOST>] [--port <PORT>]`

### `estop`

- `zeroclaw estop` (engage `kill-all`)
- `zeroclaw estop --level network-kill`
- `zeroclaw estop --level domain-block --domain "*.chase.com" [--domain "*.paypal.com"]`
- `zeroclaw estop --level tool-freeze --tool shell [--tool browser]`
- `zeroclaw estop status`
- `zeroclaw estop resume`
- `zeroclaw estop resume --network`
- `zeroclaw estop resume --domain "*.chase.com"`
- `zeroclaw estop resume --tool shell`
- `zeroclaw estop resume --otp <123456>`

Notes:

- `estop` commands require `[security.estop].enabled = true`.
- When `[security.estop].require_otp_to_resume = true`, `resume` requires OTP validation.
- OTP prompt appears automatically if `--otp` is omitted.

### `service`

- `zeroclaw service install`
- `zeroclaw service start`
- `zeroclaw service stop`
- `zeroclaw service restart`
- `zeroclaw service status`
- `zeroclaw service uninstall`

### `cron`

- `zeroclaw cron list`
- `zeroclaw cron add <expr> [--tz <IANA_TZ>] <command>`
- `zeroclaw cron add-at <rfc3339_timestamp> <command>`
- `zeroclaw cron add-every <every_ms> <command>`
- `zeroclaw cron once <delay> <command>`
- `zeroclaw cron remove <id>`
- `zeroclaw cron pause <id>`
- `zeroclaw cron resume <id>`

Notes:

- Mutating schedule/cron actions require `cron.enabled = true`.
- Shell command payloads for schedule creation (`create` / `add` / `once`) are validated by security command policy before job persistence.

### `models`

- `zeroclaw models refresh`
- `zeroclaw models refresh --provider <ID>`
- `zeroclaw models refresh --force`

`models refresh` currently supports live catalog refresh for provider IDs: `openrouter`, `openai`, `anthropic`, `groq`, `mistral`, `deepseek`, `xai`, `together-ai`, `gemini`, `ollama`, `llamacpp`, `sglang`, `vllm`, `astrai`, `venice`, `fireworks`, `cohere`, `moonshot`, `glm`, `zai`, `qwen`, and `nvidia`.

### `doctor`

- `zeroclaw doctor`
- `zeroclaw doctor models [--provider <ID>] [--use-cache]`
- `zeroclaw doctor traces [--limit <N>] [--event <TYPE>] [--contains <TEXT>]`
- `zeroclaw doctor traces --id <TRACE_ID>`

`doctor traces` reads runtime tool/model diagnostics from `observability.runtime_trace_path`.

### `channel`

- `zeroclaw channel list`
- `zeroclaw channel start`
- `zeroclaw channel doctor`
- `zeroclaw channel bind-telegram <IDENTITY>`
- `zeroclaw channel add <type> <json>`
- `zeroclaw channel remove <name>`

Runtime in-chat commands (Telegram/Discord while channel server is running):

- `/models`
- `/models <provider>`
- `/model`
- `/model <model-id>`
- `/new`

Channel runtime also watches `config.toml` and hot-applies updates to:
- `default_provider`
- `default_model`
- `default_temperature`
- `api_key` / `api_url` (for the default provider)
- `reliability.*` provider retry settings

`add/remove` currently route you back to managed setup/manual config paths (not full declarative mutators yet).

### `integrations`

- `zeroclaw integrations info <name>`

### `skills`

- `zeroclaw skills list`
- `zeroclaw skills audit <source_or_name>`
- `zeroclaw skills install <source>`
- `zeroclaw skills remove <name>`

`<source>` accepts git remotes (`https://...`, `http://...`, `ssh://...`, and `git@host:owner/repo.git`) or a local filesystem path.

`skills install` always runs a built-in static security audit before the skill is accepted. The audit blocks:
- symlinks inside the skill package
- script-like files (`.sh`, `.bash`, `.zsh`, `.ps1`, `.bat`, `.cmd`)
- high-risk command snippets (for example pipe-to-shell payloads)
- markdown links that escape the skill root, point to remote markdown, or target script files

Use `skills audit` to manually validate a candidate skill directory (or an installed skill by name) before sharing it.

Skill manifests (`SKILL.toml`) support `prompts` and `[[tools]]`; both are injected into the agent system prompt at runtime, so the model can follow skill instructions without manually reading skill files.

### `migrate`

- `zeroclaw migrate openclaw [--source <path>] [--dry-run]`

### `config`

- `zeroclaw config schema`

`config schema` prints a JSON Schema (draft 2020-12) for the full `config.toml` contract to stdout.

### `completions`

- `zeroclaw completions bash`
- `zeroclaw completions fish`
- `zeroclaw completions zsh`
- `zeroclaw completions powershell`
- `zeroclaw completions elvish`

`completions` is stdout-only by design so scripts can be sourced directly without log/warning contamination.

### `hardware`

- `zeroclaw hardware discover`
- `zeroclaw hardware introspect <path>`
- `zeroclaw hardware info [--chip <chip_name>]`

### `peripheral`

- `zeroclaw peripheral list`
- `zeroclaw peripheral add <board> <path>`
- `zeroclaw peripheral flash [--port <serial_port>]`
- `zeroclaw peripheral setup-uno-q [--host <ip_or_host>]`
- `zeroclaw peripheral flash-nucleo`

## Validation Tip

To verify docs against your current binary quickly:

```bash
zeroclaw --help
zeroclaw <command> --help
```
