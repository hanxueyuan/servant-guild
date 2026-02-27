# 架构升级技术方案可行性复核报告 (Architecture Upgrade Feasibility Review Report)

**评审对象**: `docs/design/architecture_upgrade_design.md` (v1.0)  
**评审日期**: 2026-02-26  
**评审结论**: **有条件通过 (Conditional Pass)** - 建议补充高可用设计与回滚细节后推进。

---

## 1. 技术可行性评估 (Technical Feasibility Assessment)

### 1.1 技术选型合理性
*   **Core Runtime (Rust)**: **合理**。维持 Rust 作为内核能保证高性能与内存安全，符合 ZeroClaw 的原有定位。
*   **Database (SQLite/PostgreSQL)**: **合理**。
    *   SQLite 适合单机/边缘部署，维护成本低。
    *   PostgreSQL 适合企业级集群部署，支持复杂查询与 JSONB（存储动态配置）。
    *   *建议*: 使用 `sqlx` 或 `sea-orm` 实现数据库无关性，确保一套代码支持两种后端。
*   **Sandbox (Wasmtime)**: **合理但有挑战**。WASM 是实现插件化与安全隔离的最佳实践，但 Rust -> WASM 的宿主绑定（Host Functions）开发成本较高，且调试相对复杂。

### 1.2 兼容性评估
*   **配置迁移**: 方案提到了 "配置迁移工具"，这是关键。从 TOML 到 DB 的迁移是破坏性的，必须提供 `zeroclaw migrate --from config.toml` 命令。
*   **API 兼容性**: 引入 Registry 后，原有直接读取 `config.struct` 的代码需全部重构为 `registry.get_agent().await` 的异步调用。这对现有代码库（`src/agent/`, `src/sop/`）侵入性极强，需分步重构。

### 1.3 高可用与容灾 (HA/DR)
*   **现状**: 设计文档主要关注“单体数据化”，对分布式部署的描述较少。
*   **缺失**:
    *   **多实例协同**: 若部署多个 ZeroClaw 实例（连接同一 PG），缺乏分布式锁（Distributed Lock）或选主机制来防止任务被重复调度。
    *   **数据库容灾**: 未明确 DB 连接断开时的降级策略（是 Panic 还是降级为只读/缓存模式？）。

## 2. 风险评估与缺陷识别 (Risk & Defect Analysis)

| 风险点 | 概率 | 影响 | 严重级 | 说明 |
| :--- | :--- | :--- | :--- | :--- |
| **单点故障 (SPOF)** | 中 | 高 | **High** | 当前设计偏向单体应用连接 DB。若 DB 不可用，整个 Agent 系统将瘫痪。需考虑本地缓存 (Local Cache) 策略。 |
| **WASM 性能损耗** | 高 | 中 | Medium | 频繁跨越 Host/WASM 边界（内存拷贝）可能导致 Tool 执行延迟增加。需优化数据传递机制。 |
| **迁移数据丢失** | 中 | 极高 | **Critical** | `config.toml` 结构复杂，若迁移脚本覆盖不全，会导致 Agent 丢失 Prompt 或配置。 |
| **热更新一致性** | 低 | 高 | High | 在任务执行中途更新 Agent 定义或 SOP，可能导致状态不一致（"SopRun" 指向了旧版本的 Step）。 |
| **开发复杂度激增** | 高 | 中 | High | 引入 DB 和 WASM 后，CI/CD 需增加数据库容器和 WASM 编译步骤，开发环境搭建门槛提高。 |

## 3. 资源与成本评估 (Resource & Cost Estimation)

基于 Phase 1 (基础架构数据化) 的工作量估算：

### 3.1 开发工作 Token 消耗预估
*   **Database Schema & Migrations**: ~10k tokens (设计与 SQL 编写)
*   **Registry Module (Rust)**: ~25k tokens (CRUD 逻辑与缓存层)
*   **Admin API**: ~20k tokens (REST 接口定义与实现)
*   **Migration Tool**: ~15k tokens (TOML 解析与入库)
*   **Refactoring Existing Code**: ~50k tokens (替换配置读取逻辑)
*   **Total Dev**: **~120k tokens**

### 3.2 测试工作 Token 消耗预估
*   **Unit Tests**: ~40k tokens (覆盖率 80%)
*   **Integration Tests**: ~30k tokens (Docker-compose 环境下的全流程测试)
*   **Total Test**: **~70k tokens**

*注：此预估仅包含 LLM 生成代码的消耗，不含调试与人工 Review 开销。*

## 4. 合规与标准检查 (Compliance Check)

*   **监控与告警**:
    *   *现状*: 提到 "Audit Dashboard"。
    *   *缺失*: 缺乏 Prometheus Metrics 定义（如 `db_query_latency`, `active_agents_count`）。
*   **数据安全**:
    *   *现状*: 提到 "Audit Logs"。
    *   *缺失*: 数据库连接加密（TLS）、敏感字段（API Key）在数据库中的加密存储方案（At-rest encryption）。
*   **日志规范**:
    *   需强制实施结构化日志（Structured Logging, e.g., JSON），以便于 ELK/Loki 收集。

## 5. 问题清单与改进建议 (Problem List & Recommendations)

| ID | 问题描述 | 严重级 | 改进建议 |
| :--- | :--- | :--- | :--- |
| **P-01** | **缺乏分布式并发控制** | **High** | 在 `tasks` 表引入 `version/lock` 字段，或使用 Redis 实现分布式锁，防止多实例抢占任务。 |
| **P-02** | **数据库敏感信息明文存储** | **Critical** | 在 `roles` 表存储 `api_key` 等敏感字段前，必须使用 AES-256-GCM 加密，密钥由环境变量注入。 |
| **P-03** | **缺乏版本回滚机制** | **High** | 为 `roles` 和 `skills` 表增加 `version` 字段和 `history` 表，支持 API 一键回滚到上一版本。 |
| **P-04** | **WASM 调试困难** | Medium | 在 Phase 2 初期，先支持 `Native Plugin` (动态加载 `.so/.dll`) 作为过渡，或提供完善的 WASM 调试宿主工具。 |
| **P-05** | **配置读取性能瓶颈** | Medium | 在 `Registry` 模块实现 **Read-Through Cache** (内存缓存)，避免每次 Agent `turn` 都查询数据库。 |

## 6. 最终结论 (Conclusion)

该架构升级方案在**技术方向上是正确的**，能够解决 ZeroClaw 目前静态配置带来的扩展性瓶颈。

**建议继续推进**，但需在进入开发前补充以下设计细节（作为 Phase 1 的前置条件）：
1.  **安全增强**：设计数据库字段级加密方案。
2.  **并发控制**：明确多实例部署下的任务调度互斥逻辑。
3.  **缓存策略**：设计 Registry 的内存缓存与失效机制。

**批准状态**: ✅ **APPROVED with COMMENTS**
