# ZeroClaw 架构升级技术方案设计文档 v1.4

## 1. 当前架构全景梳理 (Current Architecture Landscape)

### 1.1 技术栈 (Tech Stack)
*   **Core Runtime**: Rust (Tokio async)
*   **Configuration**: TOML (`config.toml`)
*   **Memory**: In-memory vector store (optional SQLite persistence)
*   **Communication**: In-process channel (`tokio::sync::mpsc`)
*   **Deployment**: Single binary (`zeroclaw`) running on bare-metal OS (Windows/Linux) with **Root/Admin privileges**.

### 1.2 模块边界与数据流 (Module Boundaries & Data Flow)
*   **Agent Loop**: `src/agent/` - 负责 `turn` 循环，调用 `Provider` 获取 LLM 响应，解析并执行 `Tools`。
*   **SOP Engine**: `src/sop/` - 静态定义的 SOP 流程，缺乏动态修改能力。
*   **Tools**: `src/tools/` - 编译期确定的工具集，无法动态加载。
*   **Data Flow**: User Input -> Agent -> Provider -> Tool -> Agent -> User Output。数据流主要在内存中流转，缺乏持久化审计日志。

### 1.3 痛点分析 (Pain Points)
*   **可维护性**: 所有 Agent 配置硬编码在 `config.toml`，修改需重启服务。
*   **扩展性**: 新增技能需重新编译 Rust 代码，无法运行时动态加载。
*   **隔离性**: 所有 Agent 共享同一进程内存，单点故障影响全局。
*   **状态管理**: 缺乏统一的持久化状态存储，重启后任务上下文丢失。

## 2. 需求映射与差距分析 (Gap Analysis)

| 需求 ID | 需求描述 | 当前现状 | 差距 (Gap) | 影响维度 |
| :--- | :--- | :--- | :--- | :--- |
| **SR-001** | 角色注册表 (DB-backed) | `config.toml` 静态配置 | 缺乏 CRUD API 与持久化存储 | 功能, 扩展性 |
| **SR-002** | 技能注册表 (Dynamic) | 编译期 Tool Trait | 不支持运行时加载 Script/WASM | 功能, 扩展性 |
| **SR-003** | 任务调度器 (Persistent) | 内存 `SopRun` | 缺乏持久化队列与优先级调度 | 性能, 可靠性 |
| **SR-007** | WASM 沙箱 | 无 | 所有代码在 Host 权限下运行 | 安全 |
| **NFR-006** | 审计日志 (Audit Trail) | 仅控制台日志 | 缺乏结构化、防篡改的审计记录 | 合规 |
| **NFR-005** | 数据加密 | 明文存储 | 敏感字段未加密，无分布式锁 | 安全, 可靠性 |

## 3. 架构演进原则 (Evolution Principles)

*   **内核特权 (Privileged Kernel)**: Rust 核心运行时拥有 OS 最高权限，负责执行需要 Root 权限的系统级操作（如驱动安装、系统配置修改）。
*   **审慎代理 (Prudent Agency)**: 鉴于系统拥有最高权限，所有高危操作（High-Risk Operations）必须通过 **Audit & Recovery** 中间层。
    *   **Audit**: 操作前记录完整意图和参数。
    *   **Recovery**: 每次变更前自动创建系统快照（Snapshot）或备份关键文件，确保可回滚。
*   **动态扩展 (Dynamic Extension)**: 业务逻辑全面数据化，下沉至数据库或脚本层。
*   **一步到位 (Direct Transition)**: 跳过兼容性过渡阶段，直接采用目标架构。
*   **深度防御 (Defense in Depth)**: 数据库字段级加密，多实例并发控制。

## 4. 目标架构蓝图 (Target Architecture Blueprint)

### 4.1 分层视图 (Layered View)

```mermaid
graph TD
    subgraph "OS Layer (Root/Admin)"
        OS[Operating System]
        FS[FileSystem]
        SysConfig[System Config]
    end

    subgraph "Infrastructure Layer"
        DB[(PostgreSQL/SQLite)]
        Redis[Redis (Distributed Lock/Cache)]
        Backup[Backup/Snapshot Storage]
    end

    subgraph "Core Kernel (Rust - Privileged)"
        Scheduler[Task Scheduler]
        Registry[Dynamic Registry]
        Router[LLM Router]
        Crypto[Encryption Service]
        
        subgraph "Safety & Recovery Module"
            Auditor[Operation Auditor]
            SnapshotMgr[Snapshot Manager]
            RollbackEng[Rollback Engine]
        end
    end

    subgraph "Dynamic Layer (Data/WASM)"
        SOPs[SOP Definitions]
        Skills[WASM/Py Skills]
        Agents[Agent Profiles]
    end

    subgraph "Interface Layer"
        API[REST/gRPC API]
        CLI[Command Line]
    end

    CLI --> API
    API --> Scheduler
    Scheduler --> Registry
    Registry --> Skills
    Skills --> Auditor
    Auditor --> SnapshotMgr
    SnapshotMgr --> Backup
    Auditor --> OS
    RollbackEng --> Backup
    RollbackEng --> OS
    Registry --> DB
    Crypto --> DB
```

### 4.2 关键变更
*   **权限模型**: 系统作为 Root/Admin 运行，不设限，但引入 **Safety & Recovery Module**。
*   **操作审计 (Auditor)**: 拦截所有对 OS 的写操作（File Write, Command Exec），记录日志。
*   **快照与回滚 (Snapshot & Rollback)**:
    *   在执行高危 SOP（如安装驱动）前，自动调用 `SnapshotManager` 备份相关文件或创建系统还原点。
    *   若任务失败或检测到系统异常，`RollbackEngine` 自动恢复状态。
*   **数据模型**: 引入 `roles`, `skills`, `tasks`, `audit_logs`, `snapshots` 表结构。

## 5. 迭代路线图 (Roadmap)

### Phase 1: 核心架构与安全基座 (Core & Safety)
*   **目标**: 构建 Kernel-Dynamic 架构，并实现“审慎操作”机制。
*   **里程碑**:
    1.  **DB Schema**: 完成表设计，含 `snapshots` 表。
    2.  **Core Modules**: 完成 Registry, CryptoService, TaskScheduler。
    3.  **Safety Module**: 实现 `Auditor` (日志记录) 和 `SnapshotManager` (文件备份/系统还原点)。
*   **交付物**: 具备自动备份与回滚能力的 Rust 内核。
*   **风险**: 系统还原点创建耗时可能影响任务响应速度。

### Phase 2: 智能化与自演进 (Self-Evolution)
*   **目标**: 实现架构自我分析、自动重构建议。
*   **交付物**: Self-Analysis Agent，Refactoring SOP，Audit Dashboard。

## 6. 实施任务拆解 (Implementation Tasks)

### Phase 1 任务列表 (High Priority)
1.  **[P0] 数据库与安全**: 引入 `sqlx`，实现 Schema 及 AES-256-GCM 加密。 (8 SP)
2.  **[P0] 动态注册表**: 实现 `AgentRegistry`，直接对接 DB。 (8 SP)
3.  **[P0] 安全恢复模块**: 实现 `SnapshotManager`，支持文件级备份和 OS 级还原点调用。 (13 SP)
4.  **[P0] 操作审计器**: 实现 `Auditor` 中间件，拦截 Tool 执行。 (5 SP)
5.  **[P0] 任务调度器**: 实现持久化任务队列。 (8 SP)

### 质量门禁 (Quality Gates)
*   **单元测试**: 覆盖 Registry 和 Crypto 逻辑。
*   **集成测试**: 模拟“修改系统文件 -> 失败 -> 自动回滚”的全流程。
*   **代码审查**: 核心团队审批。

## 7. 交付清单
*   [x] 架构设计文档 (`architecture_upgrade_design.md`)
*   [ ] 数据库 ER 图 (PlantUML/Mermaid)
*   [ ] API 接口契约 (OpenAPI YAML)
*   [ ] 迭代甘特图 (Mermaid Gantt)
