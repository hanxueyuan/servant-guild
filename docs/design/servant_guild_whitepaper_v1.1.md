# 使魔团 (ServantGuild) 架构白皮书

**版本**: v1.1
**日期**: 2026-02-27
**来源**: ZeroClaw + pi-mono 理念融合

## 1. 核心理念

**使魔团 (ServantGuild)** 是 ZeroClaw 项目的下一代架构形态，旨在构建一个 **Rust 优先、Wasm 驱动、高度自治** 的多智能体协作系统。

核心哲学：
1.  **自治 (Autonomy)**: 智能体（使魔）具备自我检测、自我决策、自我更新的能力。
2.  **集体决策 (Consensus)**: 重大决策（如更新、发布、扩容）必须通过全团投票，拒绝单点独裁。
3.  **安全隔离 (Isolation)**: 每个使魔运行在独立的 Wasm 沙盒中，互不干扰，热替换无重启。
4.  **进化 (Evolution)**: 通过 GitHub 仓库作为基因库，使魔团能够编写、测试、发布自己的新版本，实现自我迭代。

## 2. 技术选型 (基于 ZeroClaw 现状)

*   **主语言**: Rust (1.87+) —— 继承 ZeroClaw 的高性能、安全特性。
*   **运行时**: Wasmtime —— 提供轻量级沙盒与模块热替换能力（ZeroClaw `src/runtime/wasm.rs` 已有基础）。
*   **基础库**: ZeroClaw Core —— 复用 `zeroclaw` crate 中的 `tools`, `providers`, `channels` 模块。
*   **通信**: 内存通道 (Tokio mpsc) / HTTP (Axum) / 分布式 (NATS/MQTT 可选)。
*   **存储**: Sled (嵌入式 KV) / PostgreSQL (持久化记忆，ZeroClaw 已支持)。

## 3. 架构设计

### 3.1 角色分工体系

使魔团由 **核心常驻使魔** 和 **临时弹性使魔** 组成，总数保持奇数以避免投票死锁。

#### 核心常驻使魔 (Core Servants)
*数量*: 5 (固定)
*职责*: 负责生存、决策、安全、核心产出。

1.  **枢机团长 (Coordinator)**: 
    *   职责：主人沟通接口，任务分发，状态汇报。
    *   对应原 ZeroClaw: `Tony` (协调者)。
2.  **契约使魔 (Contractor)**: 
    *   职责：负责使魔的创建、销毁、配置管理、版本发布。
    *   对应原 ZeroClaw: `System/Admin` 职能。
3.  **议长使魔 (Speaker)**: 
    *   职责：组织团议，收集投票，统计共识。
    *   新增角色，负责 `Consensus Engine`。
4.  **监工使魔 (Warden)**: 
    *   职责：安全审计，性能监控，新版本集体验证。
    *   对应原 ZeroClaw: `src/security/audit.rs` 职能实体化。
5.  **执行使魔 (Worker)**: 
    *   职责：核心干活角色，代码编写、工具调用。
    *   对应原 ZeroClaw: `Ben` (逻辑) / `Lei` (研究) 的集合体。

#### 临时弹性使魔 (Ephemeral Servants)
*数量*: 0 ~ 15 (动态)
*职责*: 应对突发负载，处理非核心杂活（如批量测试、文档生成）。
*生命周期*: 按需创建 -> 任务完成/空闲 -> 投票销毁。

### 3.2 交互模式

**主人 (Owner) <-> 团长 (Master) <-> 使魔 (Servant)**

*   **主人 (Owner)**: 最高权限者。
    *   通过 CLI / API 下达指令（创建、扩容、否决）。
    *   查看团态面板。
    *   拥有最终一票否决权。
*   **团长 (Master)**: 系统的宿主进程/守护者 (ZeroClaw Daemon)。
    *   不决策，只执行。
    *   管理 Wasm 运行时容器。
    *   转发消息，维护使魔列表。
*   **使魔 (Servant)**: 实际的智能体实例。
    *   运行在 Wasm 沙盒内。
    *   拥有独立 LLM 配置与记忆。
    *   参与团议投票。

### 3.3 自治与进化机制

#### 自我更新流程 (The Evolution Loop)
1.  **感知**: 监工使魔发现能力缺口或 Bug，或议长发起定期更新提议。
2.  **决策**: 全团投票通过更新决议。
3.  **开发**: 契约使魔拉取 GitHub 代码，执行使魔修改代码。
4.  **测试**: 监工使魔运行测试用例。
5.  **发布**: 契约使魔通过 Bot 账号发布 GitHub Release (生成新 Wasm)。
6.  **热更**: 所有使魔拉取新 Wasm，进行哈希校验。
7.  **验证**: 未更新的使魔对已更新的使魔进行交叉验证（灰度发布）。
8.  **确认**: 验证通过 -> 全量上线；失败 -> 集体回滚。

## 4. 基础设施需求

为了实现完全自治，使魔团需要以下外部支持：

1.  **基因库 (GitHub)**:
    *   专用仓库存储代码。
    *   Bot 账号 (Limited Token) 用于提交代码与发布 Release。
2.  **宿主环境 (Sanctuary)**:
    *   7x24h 服务器 (VPS/NAS)。
    *   Docker 环境隔离。
3.  **经费与认证 (Treasury)**:
    *   LLM API Keys (OpenAI/Anthropic/DeepSeek)。
    *   GitHub PAT。
4.  **记忆存储 (Library)**:
    *   PostgreSQL (结构化数据)。
    *   Redis (状态缓存)。
5.  **联络通道 (Red Phone)**:
    *   Telegram/Discord Bot 用于紧急汇报与人工干预。

## 5. 实施路线图

1.  **Phase 1: 原型 (Genesis)**
    *   在 ZeroClaw 中集成 Wasmtime 宿主。
    *   将 `src/tools` 封装为 Wasm 可调用的 Host Functions。
    *   实现 CLI 交互界面适配使魔团指令。
2.  **Phase 2: 团队 (Assembly)**
    *   实现 5 大核心使魔的角色逻辑 (Wasm 模块)。
    *   实现团议投票机制 (Consensus Engine)。
3.  **Phase 3: 进化 (Evolution)**
    *   接入 GitHub API，实现代码拉取、编译、发布流程。
    *   实现 Wasm 热替换与回滚机制。
4.  **Phase 4: 完全自治 (Autonomy)**
    *   部署至长期运行环境。
    *   接入监控与报警。
    *   移交维护权给使魔团。
