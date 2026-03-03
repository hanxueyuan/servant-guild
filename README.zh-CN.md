<p align="center">
  <img src="servant-guild.png" alt="ServantGuild" width="200" />
</p>

<h1 align="center">使魔团 (ServantGuild) 🦀（简体中文）</h1>

<p align="center">
  <strong>自治、共识、进化 —— 多智能体协作的终极形态</strong>
</p>

<p align="center">
  <a href="LICENSE-APACHE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache%202.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
  <a href="NOTICE"><img src="https://img.shields.io/badge/contributors-27+-green.svg" alt="Contributors" /></a>
</p>

<p align="center">
  🌐 语言：<a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="docs/design/servant_guild_whitepaper_v1.1.md">白皮书</a> |
  <a href="docs/architecture/servant_guild_architecture_v1.0.md">架构设计</a> |
  <a href="docs/README.md">文档总览</a>
</p>

> 本文是对 `README.md` 的人工对齐翻译（强调可读性与准确性，不做逐字直译）。
> 
> 技术标识（命令、配置键、API 路径、Trait 名称）保持英文，避免语义漂移。
> 
> 最后对齐时间：**2026-02-27**。

## 项目简介

**使魔团 (ServantGuild)** 是一个 **Rust 优先、Wasm 驱动、高度自治** 的多智能体协作系统。它由 5 个核心常驻使魔和临时弹性使魔组成，通过共识引擎进行集体决策，利用 GitHub 作为基因库实现自我进化。

### 核心哲学

1. **自治 (Autonomy)**: 智能体（使魔）具备自我检测、自我决策、自我更新的能力。
2. **集体决策 (Consensus)**: 重大决策（如更新、发布、扩容）必须通过全团投票，拒绝单点独裁。
3. **安全隔离 (Isolation)**: 每个使魔运行在独立的 Wasm 沙盒中，互不干扰，热替换无重启。
4. **进化 (Evolution)**: 通过 GitHub 仓库作为基因库，使魔团能够编写、测试、发布自己的新版本，实现自我迭代。

## 五大核心使魔

系统由 5 个固定角色的使魔组成，负责维持系统的生存与演进：

| 使魔 | 角色 | 职责 |
|------|------|------|
| **Coordinator** (枢机团长) | 协调者 | 主人沟通接口，任务分发，状态汇报 |
| **Worker** (执行使魔) | 执行者 | 核心干活角色，代码编写、工具调用 |
| **Warden** (监工使魔) | 监守者 | 安全审计，性能监控，新版本集体验证 |
| **Speaker** (议长使魔) | 发言人 | 组织团议，收集投票，统计共识 |
| **Contractor** (契约使魔) | 契约者 | 使魔创建、销毁、配置管理、版本发布 |

## 自我更新流程 (The Evolution Loop)

```
感知 → 决策 → 开发 → 测试 → 发布 → 热更 → 验证 → 确认
  │      │      │      │      │      │      │      │
  │      │      │      │      │      │      │      └─▶ 验证通过 → 全量上线
  │      │      │      │      │      │      │          验证失败 → 集体回滚
  │      │      │      │      │      │      └─▶ 交叉验证（灰度发布）
  │      │      │      │      │      └─▶ 拉取新 Wasm，哈希校验
  │      │      │      │      └─▶ 发布 GitHub Release
  │      │      │      └─▶ 运行测试用例
  │      │      └─▶ 拉取代码，修改代码
  │      └─▶ 全团投票通过更新决议
  └─▶ 监工发现 Bug 或议长发提议
```

## 技术栈

| 层级 | 技术 |
|------|------|
| **语言** | Rust 1.87+ |
| **运行时** | Wasmtime (Wasm 沙盒) |
| **异步运行时** | Tokio |
| **Web 框架** | Axum |
| **数据库** | PostgreSQL + Redis |
| **向量数据库** | pgvector |
| **嵌入式存储** | Sled |
| **LLM 提供商** | 豆包 / DeepSeek / OpenAI / Anthropic |

## 快速开始

### 环境要求

- Rust 1.87+ (推荐使用 rustup)
- PostgreSQL 15+
- Redis 7+
- libgit2-dev (用于 GitHub 集成)

### 安装

```bash
# 克隆仓库
git clone https://github.com/hanxueyuan/servant-guild.git
cd servant-guild

# 构建
cargo build --release

# 安装
cargo install --path .
```

### 配置

创建 `config.toml`:

```toml
[guild]
name = "ServantGuild-Alpha"
admin_user = "your_telegram_id"  # 主人 ID

[consensus]
core_servants_count = 5
normal_quorum = 3    # 普通决策: 3/5 通过
critical_quorum = 5  # 关键决策: 5/5 全票

[llm]
default_provider = "doubao"

[channels.telegram]
bot_token = "${TELEGRAM_BOT_TOKEN}"
allowed_users = ["${ADMIN_TELEGRAM_ID}"]
```

### 运行

```bash
# 启动守护进程
servant-guild daemon

# 查看状态
servant-guild status

# 提交任务
servant-guild task submit --type build --payload '{"module": "coordinator"}'
```

## 架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Master Daemon (Rust)                            │
│                      ┌─────────────────────────────┐                     │
│                      │    Wasmtime Runtime Host    │                     │
│                      │    ┌───────────────────┐    │                     │
│                      │    │  Host Functions   │    │                     │
│                      │    │  (tools, network, │    │                     │
│                      │    │   memory, crypto) │    │                     │
│                      │    └─────────┬─────────┘    │                     │
│                      └──────────────┼──────────────┘                     │
└─────────────────────────────────────┼────────────────────────────────────┘
                                      │
           ┌──────────────────────────┼──────────────────────────┐
           │                          │                          │
           ▼                          ▼                          ▼
    ┌─────────────┐            ┌─────────────┐            ┌─────────────┐
    │ Coordinator │            │   Worker    │            │   Warden    │
    │   (Wasm)    │            │   (Wasm)    │            │   (Wasm)    │
    └─────────────┘            └─────────────┘            └─────────────┘
           │                          │                          │
           ▼                          ▼                          ▼
    ┌─────────────┐            ┌─────────────┐
    │   Speaker   │            │ Contractor  │
    │   (Wasm)    │            │   (Wasm)    │
    └─────────────┘            └─────────────┘
```

## 基础设施需求

详见 [基础设施需求文档](./docs/design/servant_guild_infrastructure.md)。

### 五大核心基础设施

1. **宿主环境 (The Sanctuary)** - 7x24h 运行的服务器环境
2. **经费与密钥 (The Treasury)** - LLM API Keys、GitHub Token 等
3. **记忆与知识库 (The Library)** - PostgreSQL、Redis、Vector DB
4. **感知与执行触手 (The Tentacles)** - 网络访问、文件系统、工具集
5. **紧急联络通道 (The Red Phone)** - Telegram/Slack Bot 用于紧急汇报

## 实施路线图

| 阶段 | 名称 | 状态 | 描述 |
|------|------|------|------|
| Phase 1 | Genesis | ✅ 完成 | Wasmtime 集成、核心 Trait、五大使魔角色 |
| Phase 2 | Assembly | ✅ 完成 | 共识引擎、LLM 集成、上下文管理 |
| Phase 3 | Evolution | ✅ 完成 | GitHub 集成、热替换、回滚恢复 |
| Phase 4 | Autonomy | ✅ 完成 | 生产部署、监控报警、安全加固 |

## 文档

- [白皮书 v1.1](./docs/design/servant_guild_whitepaper_v1.1.md)
- [架构设计 v1.0](./docs/architecture/servant_guild_architecture_v1.0.md)
- [基础设施需求](./docs/design/servant_guild_infrastructure.md)
- [API 参考](./docs/api_reference.md)
- [部署指南](./docs/deployment_guide.md)

## 贡献

参见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## 许可证

本项目采用 MIT OR Apache-2.0 双许可 - 详见 [LICENSE](./LICENSE)。

---

*"自我维护、自我进化、自我治理的使魔团。"*
