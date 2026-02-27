# ServantGuild V1.0 架构评审报告

**日期**: 2026-02-27
**评审人**: ISTJ Architecture Review Board
**文档**: `docs/architecture/servant_guild_architecture_v1.0.md`

---

## 1. 评审结论 (Conclusion)

- **状态**: ⚠ 条件通过 (Conditional Pass)
- **概要**: 总体架构方向与《使魔团白皮书 v1.1》一致，但在 **Wasm 宿主接口 (WIT)**、**共识阻塞流程** 和 **数据库实体关系** 方面存在设计细节缺失，需补充完善后方可进入开发阶段。

---

## 2. 关键发现 (Critical Findings)

### 2.1 Wasm 宿主接口定义模糊 (Interface Ambiguity)
- **问题**: 文档提到了 "Host Functions"，但未明确宿主暴露给 Wasm 的具体能力边界（例如：文件读写是否受限？网络请求是否需白名单？）。这直接影响 `src/runtime/` 的开发。
- **风险**: 高。若不明确，可能导致 Wasm 模块开发时无法调取必要能力，或过度暴露宿主权限导致安全漏洞。
- **建议**: 增加 "6.3 Wasm Interface Type (WIT) 契约" 章节，明确定义 `wasi:http`, `wasi:filesystem`, `zeroclaw:llm`, `zeroclaw:tool` 等接口。

### 2.2 共识机制的阻塞流程未闭环 (Consensus Blocking)
- **问题**: 文档描述了 "提案 -> 投票" 流程，但未说明在投票期间，发起提案的使魔（如 Contractor）是处于挂起状态还是异步轮询状态？
- **风险**: 中。可能导致任务死锁或状态不一致。
- **建议**: 在 "5.2 交互模式" 中补充异步提案的状态机流转图。

### 2.3 数据库实体关系缺失 (Data Model Gap)
- **问题**: "5.1 数据模型" 仅列出了对象，未给出实体关系图 (ERD)。
- **风险**: 中。可能导致数据冗余或查询效率低下。
- **建议**: 补充 Mermaid ER 图，明确 `Agent`, `Task`, `Proposal`, `AuditLog` 之间的外键关系。

---

## 3. 优化建议 (Recommendations)

### 3.1 错误处理规范化
- **建议**: 在 `openapi.yaml` 中增加统一的 `ErrorResponse` Schema，包含 `code`, `message`, `trace_id`。

### 3.2 安全沙盒策略细化
- **建议**: 在 "3.5 安全架构" 中明确 Wasmtime 的配置参数（内存限制、CPU 时间片限制、文件系统预打开目录）。

### 3.3 增加 "Prudent Agency" 时序图
- **建议**: 补充一个时序图，展示 `Worker -> Host (Audit) -> Host (Snapshot) -> Host (Execute) -> Host (Result)` 的完整调用链。

---

## 4. 行动计划 (Action Items)

1.  **更新主文档**: 补充 WIT 接口定义、ER 图、安全沙盒策略。
2.  **更新 OpenAPI**: 增加标准错误模型。
3.  **新增时序图**: 补充 `prudent_agency_flow.puml`。

---

[ISTJ审计: 结论先行=是 | 基于事实=是 | 情绪偏差=无]
