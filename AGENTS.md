# AGENTS.md — ServantGuild 智能体工程协议

本文档定义了本仓库中编码智能体的默认工作协议。
范围：整个仓库。https://github.com/hanxueyuan/servant-guild.git

## 1) 项目快照（必读）

ServantGuild 是一个 Rust 优先的自主智能体运行时，实现了 **“使魔团” (ServantGuild)** 架构，核心哲学为：

- **自治 (Autonomy)**：具备自我检测、自我决策、自我更新的能力。
- **集体决策 (Consensus)**：重大变更（如代码更新、成员扩缩）需通过全团投票。
- **安全隔离 (Isolation)**：基于 **Wasmtime Component Model** 驱动，实现模块热替换且无重启。
- **进化 (Evolution)**：以 GitHub 为基因库，实现基于代码的自我迭代。
- **高性能 (Performance)**：保持 Rust 原生的高效与确定性。

核心架构是基于 Trait 驱动、Wasm 宿主和模块化的。大多数扩展工作应通过实现 Trait 并在工厂模块中注册来完成。

关键扩展点：

- `wit/host.wit` (Wasm Interface Type) — **核心契约定义**
- `src/providers/traits.rs` (`Provider`)
- `src/channels/traits.rs` (`Channel`)
- `src/tools/traits.rs` (`Tool`)
- `src/memory/traits.rs` (`Memory`)
- `src/runtime/traits.rs` (`RuntimeAdapter`) — Wasmtime 集成点
- `src/peripherals/traits.rs` (`Peripheral`) — 硬件板卡 (STM32, RPi GPIO)

## 2) 深度架构观察（协议存在的理由）

以下代码库现状应驱动每一个设计决策：

1.  **使魔团 (ServantGuild) 是下一代形态**
    - 系统不再是单体 Agent，而是由核心常驻使魔（Coordinator, Contractor, Speaker, Warden, Worker）组成的自治团队。
    - 任何重大决策必须经过 `Consensus Engine` 投票，拒绝单点独裁。
2.  **Wasm 组件模型是互操作性的基石**
    - 所有的跨语言/跨模块交互必须通过 WIT (Wasm Interface Type) 定义。
    - 禁止绕过 WIT 直接进行内存操作或非标准 FFI 调用。
3.  **Trait + 工厂架构是宿主稳定性的基石**
    - 扩展点被设计为显式且可插拔的。
    - 大多数功能应通过 Trait 实现 + 工厂注册来添加，而不是进行横切式的重写。
4.  **安全关键面是一等公民且与互联网相邻**
    - `src/gateway/`, `src/security/`, `src/tools/`, `src/runtime/` 具有高爆炸半径。
    - 默认设置已经倾向于“默认安全”（配对、绑定安全、限制、机密处理）；请保持这种状态。
5.  **配置和运行时契约是面向用户的 API**
    - `src/config/schema.rs` 和 CLI 命令实际上是公共接口。
    - 向后兼容性和显式迁移非常重要。

## 3) 工程原则（规范性）

默认情况下，这些原则是强制性的。它们不是口号；它们是实现约束。

### 3.1 KISS (Keep It Simple, Stupid) - 保持简单

**原因：** 运行时 + 安全行为必须在压力下保持可审计性。

要求：

- 优先选择直观的控制流，而不是巧妙的元编程。
- 优先选择显式的匹配分支和类型化的结构体，而不是隐藏的动态行为。
- 保持错误路径明显且局部化。

### 3.2 YAGNI (You Aren't Gonna Need It) - 你不需要它

**原因：** 过早的功能会增加攻击面和维护负担。

要求：

- 在没有具体已接受的用例之前，不要添加新的配置键、Trait 方法、功能标志或工作流分支。
- 在没有至少一个当前调用者的情况下，不要引入投机性的“面向未来”的抽象。
- 保持不支持的路径显式化（报错），而不是添加部分的虚假支持。

### 3.3 DRY + 三次原则 (Rule of Three)

**原因：** 幼稚的 DRY 会在提供者/通道/工具之间创建脆弱的共享抽象。

要求：

- 当重复小的、局部的逻辑能保留清晰度时，请重复它。
- 仅在重复、稳定的模式出现后（三次原则）才提取共享工具。
- 提取时，保留模块边界并避免隐藏的耦合。

### 3.4 SRP + ISP (单一职责 + 接口隔离)

**原因：** Trait 驱动的架构已经编码了子系统边界。

要求：

- 保持每个模块专注于一个关注点。
- 尽可能通过实现现有的狭窄 Trait 来扩展行为。
- 避免混合策略 + 传输 + 存储的庞大接口和“上帝模块”。

### 3.5 快速失败 + 显式错误

**原因：** 智能体运行时中的静默回退可能会导致不安全或昂贵的行为。

要求：

- 对于不支持或不安全的状态，优先使用显式的 `bail!` / 错误。
- 永远不要静默地扩大权限/能力。
- 当回退是有意且安全的时候，记录回退行为。

### 3.6 默认安全 + 最小权限

**原因：** 网关/工具/运行时可以执行具有真实副作用的操作。

要求：

- 对访问和暴露边界实行“默认拒绝”。
- 永远不要记录机密、原始令牌或敏感载荷。
- 除非有明确理由，否则保持网络/文件系统/Shell 范围尽可能窄。

### 3.7 确定性 + 可复现性

**原因：** 可靠的 CI 和低延迟分类依赖于确定性行为。

要求：

- 在 CI 敏感路径中优先选择可复现的命令和锁定的依赖行为。
- 保持测试确定性（没有无保护措施的不稳定时间/网络依赖）。
- 确保本地验证命令映射到 CI 预期。

### 3.8 可逆性 + 回滚优先思维

**原因：** 在高 PR 数量下，快速恢复是强制性的。

要求：

- 保持变更易于恢复（小范围，清晰的爆炸半径）。
- 对于风险变更，在合并前定义回滚路径。
- 避免阻碍安全回滚的混合巨型补丁。

### 3.9 审慎代理 (Prudent Agency)

**原因：** 智能体拥有 Root/Admin 权限；安全保障来自于可恢复性而非限制。

要求：

- **审计优先 (Audit First)**：在执行任何 OS 变更操作（写文件、执行命令）前，必须先记录结构化审计日志。
- **快照常备 (Snapshot Always)**：在变更状态前，确保有可恢复的快照（文件级或系统级）。
- **回滚就绪 (Rollback Ready)**：每个变更操作必须有确定的回滚路径，由安全模块在失败时执行。
- **禁止裸调 (Prohibition)**：禁止直接使用 `std::fs` 或 `std::process::Command` 实现业务逻辑；必须通过 `src/safety/` 包装器。

### 3.10 共识驱动 (Consensus Driven)

**原因：** 自治系统需要防止单点故障或流氓智能体破坏系统。

要求：

- **重大变更需投票**：代码更新、成员增减、配置变更必须触发 `Consensus Engine` 投票。
- **奇数节点**：核心使魔数量保持奇数以避免死锁。
- **一票否决**：主人 (Owner) 对任何提案拥有一票否决权。

### 3.11 跨平台兼容性 (Cross-Platform Compatibility)

**原因：** 系统需要在 Linux、Windows、macOS 多平台上运行，确保开发者体验一致性。

要求：

- **路径处理**：禁止硬编码路径分隔符，使用 `std::path::PathBuf` 或 `path.join()` 构建跨平台路径。
- **Shell 命令**：禁止直接使用平台特定的 Shell 命令（如 `rm -rf`、`del`），必须通过 `src/tools/shell.rs` 的跨平台适配层。
- **文件权限**：Linux/macOS 的文件权限模型与 Windows 不同，权限设置代码需要条件编译 (`#[cfg(unix)]` / `#[cfg(windows)]`)。
- **环境变量**：不同平台的环境变量命名约定不同（如 `HOME` vs `USERPROFILE`），使用 `dirs` crate 获取用户目录。
- **进程管理**：使用 `tokio::process` 替代 `std::process::Command`，并处理平台差异。
- **服务管理**：
    - Linux: Systemd (`src/safety/service/linux.rs`)
    - Windows: Windows Service (`src/safety/service/windows.rs`)
    - macOS: Launchd (`src/safety/service/macos.rs`)
- **测试覆盖**：所有跨平台代码必须在 CI 中通过 Linux 和 Windows 的测试。

## 4) 仓库地图（高层级）

- `src/main.rs` — CLI 入口点和命令路由
- `src/lib.rs` — 模块导出和共享命令枚举
- `wit/` — **Wasm 接口定义 (WIT) 文件**
- `src/config/` — 模式 + 配置加载/合并
- `src/registry/` — 动态角色/技能/契约注册表 (DB-backed)
- `src/scheduler/` — 持久化任务队列与分发
- `src/safety/` — 审慎代理核心 (审计, 快照, 回滚)
- `src/safety/service/` — **跨平台服务管理** (systemd, windows-service, launchd)
- `src/consensus/` — 共识引擎与投票机制
- `src/gateway/` — Webhook/网关服务器
- `src/security/` — 策略、配对、机密存储 (crypto)
- `src/memory/` — Markdown/SQLite 记忆后端 + 嵌入/向量合并
- `src/providers/` — 模型提供者和弹性包装器
- `src/channels/` — Telegram/Discord/Slack/etc 通道
- `src/tools/` — 工具执行面（Shell, 文件, 内存, 浏览器）
- `src/peripherals/` — 硬件外设 (STM32, RPi GPIO)
- `src/runtime/` — 运行时适配器（Wasmtime Host 环境）
- `src/servants/` — 核心使魔的 Rust 实现 (Guest Modules)
- `docs/` — 面向任务的文档系统

## 5) 使魔团架构 (ServantGuild Architecture)

本部分定义了系统的核心角色与交互模型。

### 5.1 核心常驻使魔 (Core Servants)

系统由 5 个固定角色的使魔组成，负责维持系统的生存与演进：

1.  **枢机团长 (Coordinator)**
    - *职责*：主人沟通接口，任务分发，状态汇报。
    - *对应模块*：`src/servants/coordinator` (Wasm Guest)。
2.  **契约使魔 (Contractor)**
    - *职责*：负责使魔的创建、销毁、配置管理、版本发布。
    - *对应模块*：`src/servants/contractor` (Wasm Guest)。
3.  **议长使魔 (Speaker)**
    - *职责*：组织团议，收集投票，统计共识。
    - *对应模块*：`src/servants/speaker` (Wasm Guest)。
4.  **监工使魔 (Warden)**
    - *职责*：安全审计，性能监控，新版本集体验证。
    - *对应模块*：`src/servants/warden` (Wasm Guest)。
5.  **执行使魔 (Worker)**
    - *职责*：核心干活角色，代码编写、工具调用。
    - *对应模块*：`src/servants/worker` (Wasm Guest)。

### 5.2 交互模式

- **主人 (Owner)**：最高权限者，拥有最终否决权，通过 CLI/API 下达指令。
- **团长 (Master)**：ZeroClaw Daemon 宿主进程 (Rust Host)，不决策，只执行 Wasm 容器管理与消息转发。
- **使魔 (Servant)**：运行在 Wasmtime 沙盒内的智能体实例，通过 WIT 接口与宿主交互。

### 5.3 进化循环 (Evolution Loop)

1.  **感知**：监工发现问题或议长发起更新。
2.  **决策**：全团投票通过。
3.  **开发**：执行使魔修改代码。
4.  **发布**：契约使魔发布 Release (Wasm Component)。
5.  **热更**：全团热加载新 Wasm。
6.  **验证**：交叉验证与灰度发布。

## 6) 风险等级与使魔分工 (Review Depth Contract)

在决定验证深度和审查严格程度时使用这些等级。

- **低风险**：仅文档/杂务/测试变更
    - *负责角色*：临时弹性使魔 (Ephemeral Servants) / 执行使魔 (Worker)
- **中风险**：大多数 `src/**` 行为变更，无边界/安全影响
    - *负责角色*：执行使魔 (Worker) + 监工使魔 (Warden) 审计
- **高风险**：`src/security/**`, `src/runtime/**`, `wit/**`, `src/gateway/**`, `src/tools/**`, `.github/workflows/**`, 访问控制边界
    - *负责角色*：监工使魔 (Warden) 审计 + 议长使魔 (Speaker) 投票确认 + 主人 (Owner) 最终批准

如果不确定，归类为更高风险。

## 7) 智能体工作流（必需）

1. **写前读**
    - 在编辑之前检查现有模块、工厂连线和相邻测试。
2. **定义范围边界**
    - 每个 PR 一个关注点；避免混合功能+重构+基础设施补丁。
3. **实现最小补丁**
    - 显式应用 KISS/YAGNI/DRY 三次原则。
4. **按风险等级验证**
    - 仅文档：轻量级检查。
    - 代码/风险变更：完整的相关检查和重点场景。
5. **记录影响**
    - 更新文档/PR 说明，说明行为、风险、副作用和回滚。
    - 如果 CLI/配置/提供者/通道行为发生变化，更新相应的运行时契约参考。
    - **如果 WIT 接口发生变化，必须同步更新 Host 实现和 Guest SDK。**
6. **尊重队列卫生**
    - 如果是堆叠 PR：声明 `Depends on #...`。
    - 如果替换旧 PR：声明 `Supersedes #...`。

（保留后续关于分支、PR 处置、工作树、代码命名、架构边界契约的内容...）

## 8) 变更剧本

### 8.1 添加提供者 (Provider)

- 在 `src/providers/` 中实现 `Provider`。
- 在 `src/providers/mod.rs` 工厂中注册。
- 为工厂连线和错误路径添加重点测试。
- 避免提供者特定的行为泄漏到共享编排代码中。

### 8.2 添加通道 (Channel)

- 在 `src/channels/` 中实现 `Channel`。
- 保持 `send`, `listen`, `health_check`, 类型语义一致。
- 用测试覆盖认证/允许列表/健康行为。

### 8.3 添加工具 (Tool)

- 在 `src/tools/` 中实现 `Tool`，具有严格的参数模式。
- 验证并清理所有输入。
- 返回结构化的 `ToolResult`；避免在运行时路径中恐慌 (panic)。
- **如果工具需要暴露给 Wasm 使魔，必须在 `wit/host.wit` 中定义接口并在 `src/runtime/` 中实现绑定。**

### 8.4 添加外设 (Peripheral)

- 在 `src/peripherals/` 中实现 `Peripheral`。
- 外设暴露 `tools()` — 每个工具委托给硬件（GPIO, 传感器等）。
- 如果需要，在配置模式中注册板卡类型。
- 见 `docs/hardware-peripherals-design.md` 获取协议和固件说明。

### 8.5 安全 / 运行时 / 网关变更

- 包含威胁/风险说明和回滚策略。
- 为故障模式和边界添加/更新测试或验证证据。
- 保持可观测性有用但非敏感。
- 对于 `.github/workflows/**` 变更，在 PR 说明中包含 Actions 允许列表影响，并在源变更时更新 `docs/actions-source-policy.md`。

### 8.6 文档系统 / README / IA 变更

- 将文档导航视为产品 UX：保留从 README -> 文档中心 -> SUMMARY -> 分类索引的清晰路径。
- 保持顶级导航简洁；避免相邻导航块之间的重复链接。
- 当运行时界面变更时，更新相关参考（`commands/providers/channels/config/runbook/troubleshooting`）。
- 当导航或关键措辞变更时，保持所有支持的语言环境（`en`, `zh-CN`, `ja`, `ru`, `fr`, `vi`, `el`）的入口点一致性。

### 8.7 修改 WIT 接口 (New)

- 修改 `wit/host.wit` 定义。
- 运行 `wit-bindgen` 或相关构建脚本更新生成的绑定代码。
- 更新 `src/runtime/` 中的 Host 实现以匹配新接口。
- 更新 `src/servants/` 或 `sdk/` 中的 Guest 调用代码。
- 增加针对新接口边界的集成测试。

### 8.8 跨平台代码变更 (New)

- **路径处理**：使用 `std::path::PathBuf` 而非字符串拼接路径。
- **条件编译**：平台特定代码必须使用 `#[cfg(target_os = "...")]` 隔离。
- **测试验证**：在 Linux 和 Windows 上运行 `cargo test` 确保兼容性。
- **文档更新**：如有平台差异，更新 `README.md` 中的平台说明。
- **CI 覆盖**：确保 `.github/workflows/` 中的 CI 在多平台矩阵中运行。

## 9) 验证矩阵

代码变更的默认本地检查：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

首选本地预 PR 验证路径（推荐，非必需）：

```bash
./dev/ci.sh all
```

注意：

- 当 Docker 可用时，强烈建议使用基于 Docker 的本地 CI。
- 如果本地 Docker CI 不可用，贡献者不会被阻止打开 PR；在这种情况下，运行最相关的原生检查并记录运行的内容。

按变更类型的额外期望：

- **文档/仅模板**：
    - 运行 Markdown Lint 和链接完整性检查
    - 如果涉及 README/docs-hub/SUMMARY/集合索引，验证 EN/ZH-CN/JA/RU/FR/VI/EL 导航一致性
    - 如果涉及引导文档/脚本，运行 `bash -n bootstrap.sh scripts/bootstrap.sh scripts/install.sh`
- **工作流变更**：验证 YAML 语法；如果可用，运行工作流 Lint/健全性检查。
- **安全/运行时/网关/工具**：包含至少一个边界/故障模式验证。
- **Wasm 接口变更**：必须包含 Host-Guest 交互测试。
- **跨平台相关变更**：
    - 在 Linux 和 Windows 上运行测试矩阵
    - 验证路径处理使用 `PathBuf` 而非字符串拼接
    - 检查条件编译 (`#[cfg(target_os)]`) 的正确性
    - 验证服务管理代码的平台特定分支

如果全面检查不切实际，运行最相关的子集并记录跳过的内容及原因。

## 10) 协作与 PR 纪律

（保留原有内容...）

## 11) 引用与参考

- `docs/design/servant_guild_whitepaper_v1.1.md` (架构蓝图)
- `docs/design/llm_architecture_migration_playbook_zh.md` (迁移手册)
- `CONTRIBUTING.md`
- `docs/README.md`
- `docs/SUMMARY.md`

（保留后续内容...）
