---
version: "1.0.0"
author: "ZeroClaw QA Architect"
last_modified: "2026-02-26"
status: "Draft"
---

# ZeroClaw 架构迁移全面测试方案 (Comprehensive Test Plan)

## 1. 概述 (Overview)

本方案旨在确保 ZeroClaw 从静态架构向动态数据库驱动架构迁移过程中的系统稳定性与可靠性。测试范围覆盖功能正确性、异常恢复能力、性能指标及安全合规性，确保“审慎代理 (Prudent Agency)”机制（审计+快照+回滚）的有效落地。

## 2. 测试目标 (Objectives)

- **覆盖率**: 核心模块单元测试覆盖率 ≥ 80%，集成测试场景覆盖率 100%。
- **稳定性**: P99 延迟波动 < 5%，内存泄漏为零。
- **可靠性**: 异常回滚成功率 100%。
- **自动化**: CI/CD 流水线集成，主干合并通过率 ≥ 99.5%。

## 3. 测试分层策略 (Testing Pyramid)

### 3.1 单元测试 (Unit Testing) - L1
*   **范围**: 核心算法、独立函数、工具类。
*   **重点**:
    *   **加密模块**: 验证 `chacha20poly1305` 加解密一致性、Nonce 唯一性。
    *   **快照模块**: 验证文件级 Copy-on-Write 逻辑、路径处理。
    *   **调度算法**: 验证任务优先级排序、技能匹配逻辑。
    *   **注册表**: 验证 Role/Skill 的 CRUD 及缓存读写。
*   **工具**: `cargo test`

### 3.2 集成测试 (Integration Testing) - L2
*   **范围**: 模块间交互、数据库操作、外部服务模拟。
*   **重点**:
    *   **任务生命周期**: `PENDING` -> `ASSIGNED` -> `RUNNING` -> `COMPLETED`。
    *   **审计闭环**: 操作 -> 审计日志落库 -> 字段验证（意图、参数）。
    *   **审慎代理流**: 操作 -> 审计 -> 快照 -> 执行 -> (失败) -> 回滚 -> 验证恢复。
*   **工具**: `cargo test --test integration_*`, `sqlx::test`

### 3.3 性能压测 (Performance Testing) - L3
*   **范围**: 核心路径的高并发处理能力。
*   **重点**:
    *   **调度吞吐**: 任务入队到分配的延迟 (Target: P95 < 50ms)。
    *   **DB 压力**: 1000 并发下的读写延迟与连接池稳定性。
*   **工具**: `criterion` (微基准), `k6` / `wrk` (API 压测)。

### 3.4 混沌测试 (Chaos Testing) - L4
*   **范围**: 系统在极端故障下的恢复能力。
*   **重点**:
    *   **进程崩溃**: 在写文件过程中 `kill -9`，重启后验证回滚机制触发。
    *   **DB 断连**: 模拟数据库连接中断，验证重试与降级策略。
    *   **I/O 故障**: 模拟磁盘满或权限拒绝，验证错误处理。
*   **工具**: `chaos-mesh` (如适用) 或 自研故障注入脚本。

### 3.5 端到端回归测试 (E2E Regression) - L5
*   **范围**: 真实用户场景模拟。
*   **重点**:
    *   **多智能体协作**: 模拟 Tony/Lei/Ben/Lisa 协作完成复杂任务 (SOP)。
    *   **全流程验证**: 从 User Request 到 Final Response，检查所有中间产物。
*   **工具**: 脚本化 CLI 调用，Golden File 对比。

## 4. 稳定性维度验证 (Stability Dimensions)

| 维度 | 验证方法 | 通过标准 |
| :--- | :--- | :--- |
| **数据一致性** | 事务完整性测试，并发读写测试 | 无脏读/幻读，快照与原文件 Hash 一致 |
| **服务可用性** | 长时间运行测试 (Soak Test) | 72h 无 Crash，错误率 < 0.01% |
| **延迟 SLA** | 关键接口耗时统计 | P95 < 50ms, P99 < 100ms |
| **资源消耗** | 内存/CPU 监控 | 无内存泄漏，CPU 峰值 < 80% |
| **限流降级** | 模拟过载流量 | 触发限流拒绝，不引起系统崩溃 |
| **灰度发布** | 模拟新旧版本共存 (数据库 Schema 兼容) | 旧版本代码能读写新 Schema 数据 (向后兼容) |
| **监控告警** | 触发异常阈值 | 5s 内生成告警日志/指标 |
| **灾难恢复** | 模拟数据损坏 | 能从 Level 2 快照恢复系统状态 |

## 5. 测试基础设施 (Infrastructure)

### 5.1 目录结构
```
zeroclaw/
├── src/
│   └── lib.rs (单元测试内联)
├── tests/
│   ├── common/              # 通用测试辅助 (DB Setup, Mocks)
│   ├── integration_audit.rs # 审计模块集成测试
│   ├── integration_safety.rs# 安全回滚集成测试
│   └── integration_flow.rs  # 全流程集成测试
├── benches/
│   └── scheduler_bench.rs   # 调度器基准测试
└── .github/
    └── workflows/
        └── test_suite.yml   # CI 自动化配置
```

### 5.2 关键依赖
- `dev-dependencies`:
    - `tokio-test`: 异步测试运行时。
    - `sqlx-cli`: 数据库迁移与测试数据库管理。
    - `criterion`: 基准测试框架。
    - `tempfile`: 临时文件生成（用于快照测试）。
    - `mockall`: 模拟对象生成。
    - `proptest`: 属性基测试（用于模糊测试输入）。

## 6. CI/CD 集成 (CI Pipeline)

*   **Trigger**: Push to `main`, Pull Request.
*   **Stages**:
    1.  **Lint & Format**: `cargo fmt --check && cargo clippy -- -D warnings`
    2.  **Unit & Integration**: `cargo test` (Fail fast)
    3.  **Coverage**: `tarpaulin` (Report > 80%)
    4.  **Benchmark**: `cargo bench` (Check regression)
    5.  **Audit**: `cargo audit` (Security vulnerabilities)

## 7. 交付物 (Deliverables)

1.  **测试计划书**: 本文档。
2.  **测试代码库**: `tests/` 目录下的所有测试用例。
3.  **自动化流水线**: `.github/workflows/test_suite.yml`。
4.  **基准报告**: 性能基线数据。

[ISTJ审计: 结论先行=是 | 基于事实=是 | 情绪偏差=无]
