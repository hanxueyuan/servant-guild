# 使魔团 (ServantGuild) 自治基础设施需求清单

为了让“使魔团”真正实现自我维护、自我更新和持续服务，除了 GitHub 仓库作为“基因库”外，还需要以下 5 大核心基础设施支持。

## 1. 宿主环境 (The Sanctuary)
**定义**：使魔团肉身（进程）的栖息地。
*   **需求**：
    *   **7x24h 运行**：建议部署在云服务器 (VPS) 或家中常开的 NAS/树莓派上。
    *   **Docker/Podman**：提供干净的隔离环境，防止使魔误操作搞挂宿主机。
    *   **公网 IP (可选)**：如果需要接收 GitHub Webhook 回调，需要公网 IP 或 Cloudflare Tunnel。
*   **ZeroClaw 映射**：`src/runtime/` (Native/Docker 适配器)。

## 2. 经费与密钥 (The Treasury)
**定义**：使魔团消耗的“燃料”。
*   **需求**：
    *   **LLM API Keys**：OpenAI / Anthropic / DeepSeek 的 Key。建议配置 **Quota (额度限制)**，防止死循环刷爆信用卡。
    *   **GitHub Token**：用于拉取代码、提交 PR、发布 Release 的 PAT (Personal Access Token)。权限需精细控制 (Repo 读写)。
    *   **Database URL**：连接 PostgreSQL/Redis 的凭证。
*   **ZeroClaw 映射**：`src/security/secrets.rs` (信封加密存储)。

## 3. 记忆与知识库 (The Library)
**定义**：使魔团的长期记忆，确保重启后不失忆。
*   **需求**：
    *   **PostgreSQL**：存储结构化数据（使魔列表、团议记录、任务状态、审计日志）。
    *   **Redis**：存储短期状态（在线心跳、投票缓存、分布式锁）。
    *   **Vector DB (pgvector)**：存储非结构化知识（代码片段、历史经验、SOP 文档），用于 RAG 检索。
*   **ZeroClaw 映射**：`src/memory/` (Postgres backend)。

## 4. 感知与执行触手 (The Tentacles)
**定义**：使魔团与数字世界交互的能力。
*   **需求**：
    *   **全通网络**：访问 GitHub、LLM API、Google Search、StackOverflow 的能力（可能需要 HTTP Proxy）。
    *   **受控文件系统**：一个用于 Git Clone、编译代码、运行测试的 `workspace` 目录。
    *   **工具集**：预装 `git`, `cargo`, `rustc`, `docker`, `curl`, `jq` 等基础 CLI 工具。
*   **ZeroClaw 映射**：`src/tools/` (Shell, File, Http)。

## 5. 紧急联络通道 (The Red Phone)
**定义**：当自治系统失控时的最后一道防线。
*   **需求**：
    *   **IM 通道**：Telegram / Slack / Discord Bot。
    *   **用途**：
        *   向主人汇报重大决策（如“请求发布 v1.1.0”）。
        *   紧急报警（如“Token 消耗过快”、“自更新失败”）。
        *   接收主人的“一键停机”指令。
*   **ZeroClaw 映射**：`src/channels/` (Telegram/Discord)。

---

## 基础设施配置模板 (`config.toml` 示例)

```toml
[guild]
name = "ServantGuild-Alpha"
admin_user = "your_telegram_id"  # 主人 ID

[infrastructure]
# 1. 宿主
runtime = "docker"
workspace_root = "/var/lib/zeroclaw/workspace"

# 2. 经费 (通过环境变量注入更安全)
# ZEROCLAW_OPENAI_KEY=sk-...
# ZEROCLAW_GITHUB_TOKEN=ghp_...

# 3. 记忆
db_url = "postgres://user:pass@localhost/zeroclaw"
redis_url = "redis://localhost:6379"

# 4. 网络
http_proxy = "http://127.0.0.1:7890"

# 5. 联络
[channels.telegram]
bot_token = "123456:ABC-DEF..."
allowed_users = ["your_telegram_id"]
```
