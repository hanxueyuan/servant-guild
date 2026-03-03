# ServantGuild 部署指南

## 概述

本指南提供 ServantGuild 系统在生产环境中的部署步骤和最佳实践。

基于 [ServantGuild 基础设施需求](./design/servant_guild_infrastructure.md)，部署需要满足五大核心基础设施：

1. **宿主环境 (The Sanctuary)** - 7x24h 运行环境
2. **经费与密钥 (The Treasury)** - API Keys 和认证凭据
3. **记忆与知识库 (The Library)** - 数据存储
4. **感知与执行触手 (The Tentacles)** - 网络和工具
5. **紧急联络通道 (The Red Phone)** - 告警和干预通道

## 平台支持

| 平台 | 架构 | 部署方式 | 推荐场景 |
|------|------|----------|----------|
| **Linux** | x86_64, ARM64, ARMv7 | Docker, Kubernetes, Systemd | 生产环境、云服务器 |
| **Windows** | x86_64 | Docker, Windows Service | 开发测试、企业环境 |
| **macOS** | x86_64, ARM64 | Docker, Launchd | 开发测试、个人使用 |

## 前置条件

### 通用前置条件

- Docker 和 Docker Compose（可选但推荐）
- Rust 1.87+（从源码构建时需要）
- PostgreSQL 15+
- Redis 7+

### Linux 前置条件

```bash
# Debian/Ubuntu
sudo apt update
sudo apt install -y build-essential pkg-config libgit2-dev docker.io docker-compose

# Fedora/RHEL
sudo dnf install -y build-essential pkg-config libgit2-devel docker docker-compose

# 启动 Docker
sudo systemctl enable --now docker
```

### Windows 前置条件

```powershell
# 安装 Docker Desktop
winget install Docker.DockerDesktop

# 安装 Visual Studio Build Tools（用于源码构建）
winget install Microsoft.VisualStudio.2022.BuildTools
# 选择 "Desktop development with C++" 工作负载

# 安装 Rust（用于源码构建）
winget install Rustlang.Rustup
```

### macOS 前置条件

```bash
# 安装 Docker Desktop
brew install --cask docker

# 安装构建依赖
brew install libgit2

# 安装 Rust（用于源码构建）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## 部署选项

### 选项 A: Docker 部署（推荐）

适用于所有平台（Linux, Windows, macOS）。

### 选项 B: Kubernetes 部署

适用于生产环境，支持 Linux 节点。

### 选项 C: 系统服务部署

- **Linux**: Systemd 服务
- **Windows**: Windows Service
- **macOS**: Launchd

---

## 1. Docker 部署（跨平台）

### 1.1 使用 Docker Compose

```bash
# Linux/macOS
cd deploy/docker
docker-compose up -d

# Windows PowerShell
cd deploy\docker
docker-compose up -d
```

这将启动:
- PostgreSQL (端口 5432)
- Redis (端口 6379)
- Prometheus (端口 9090)
- Grafana (端口 3000)
- Loki (端口 3100)
- ServantGuild (端口 5000)

### 1.2 单容器部署

```bash
# 构建镜像
docker build -t servant-guild:latest -f deploy/docker/Dockerfile .

# 运行容器
docker run -d \
  --name servant-guild \
  -p 5000:5000 \
  -e DATABASE_URL=postgres://user:pass@host:5432/servant_guild \
  -e REDIS_URL=redis://host:6379 \
  -v servant-guild-data:/data \
  servant-guild:latest
```

### 1.3 Windows 容器部署

```powershell
# 构建镜像
docker build -t servant-guild:latest -f deploy\docker\Dockerfile .

# 运行容器
docker run -d `
  --name servant-guild `
  -p 5000:5000 `
  -e DATABASE_URL="postgres://user:pass@host:5432/servant_guild" `
  -e REDIS_URL="redis://host:6379" `
  -v servant-guild-data:/data `
  servant-guild:latest
```

---

## 2. 系统服务部署

### 2.1 Linux (Systemd)

创建 systemd 服务文件：

```bash
sudo tee /etc/systemd/system/servant-guild.service << EOF
[Unit]
Description=ServantGuild Daemon
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=servant-guild
Group=servant-guild
WorkingDirectory=/opt/servant-guild
ExecStart=/opt/servant-guild/bin/servant-guild daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# 启用并启动服务
sudo systemctl daemon-reload
sudo systemctl enable servant-guild
sudo systemctl start servant-guild
```

### 2.2 Windows (Windows Service)

使用 WinSW 或 NSSM 将 ServantGuild 注册为 Windows 服务：

```powershell
# 使用 NSSM 安装服务
nssm install ServantGuild "C:\Program Files\ServantGuild\servant-guild.exe" "daemon"
nssm start ServantGuild

# 或使用 sc 命令
sc create ServantGuild binPath= "C:\Program Files\ServantGuild\servant-guild.exe daemon" start= auto
sc start ServantGuild
```

### 2.3 macOS (Launchd)

创建 LaunchAgent 配置：

```bash
# 创建 plist 文件
cat > ~/Library/LaunchAgents/com.servantguild.daemon.plist << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.servantguild.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/servant-guild</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
EOF

# 加载服务
launchctl load ~/Library/LaunchAgents/com.servantguild.daemon.plist
```

---

## 3. Kubernetes 部署（生产环境）

### 3.1 前置条件

- Kubernetes 集群（Linux 节点）
- kubectl 和 Helm
- 持久化存储支持

### 3.2 使用 kubectl 部署

### 1.1 启动依赖服务

```bash
cd deploy/docker
docker-compose up -d
```

这将启动:
- PostgreSQL (端口 5432)
- Redis (端口 6379)
- Prometheus (端口 9090)
- Grafana (端口 3000)
- Loki (端口 3100)

### 1.2 构建和运行

```bash
# 构建项目
cargo build --release

# 运行服务
cargo run --release
```

## 2. 基础设施部署 (Terraform)

### 2.1 初始化 Terraform

```bash
cd deploy/terraform

# 创建 terraform.tfvars 文件
cat > terraform.tfvars << EOF
aws_region = "us-east-1"
environment = "production"
db_password = "YOUR_SECURE_PASSWORD"
redis_password = "YOUR_REDIS_PASSWORD"
EOF

# 初始化
terraform init

# 查看计划
terraform plan

# 应用
terraform apply
```

### 2.2 资源清单

部署完成后将创建:
- VPC (10.0.0.0/16)
- 公有子网 x 2 (多可用区)
- 私有子网 x 2 (多可用区)
- EC2 实例 (t3.large)
- RDS PostgreSQL (db.t3.medium)
- ElastiCache Redis (cache.t3.micro)
- S3 存储桶 (快照和日志)
- CloudWatch 日志组

## 3. 容器构建和推送

### 3.1 构建镜像

```bash
cd deploy/docker

# 构建
docker build -t servant-guild:latest .

# 标记
docker tag servant-guild:latest YOUR_REGISTRY/servant-guild:v1.0.0

# 推送
docker push YOUR_REGISTRY/servant-guild:v1.0.0
```

## 4. Kubernetes 部署

### 4.1 使用 kubectl

```bash
cd deploy/kubernetes

# 创建命名空间
kubectl create namespace servant-guild

# 创建密钥
kubectl create secret generic servant-guild-secrets \
  --from-literal=database-url='postgresql://user:pass@host:5432/servant_guild' \
  --from-literal=redis-url='redis://host:6379' \
  -n servant-guild

# 部署
kubectl apply -f servant-guild.yaml
```

### 4.2 使用 Helm

```bash
cd deploy/helm

# 安装
helm install servant-guild . \
  --namespace servant-guild \
  --set image.repository=YOUR_REGISTRY/servant-guild \
  --set image.tag=v1.0.0 \
  --set replicaCount=3
```

### 4.3 部署配置

| 参数 | 默认值 | 描述 |
|------|--------|------|
| `replicaCount` | 3 | 副本数量 |
| `resources.requests.memory` | 512Mi | 内存请求 |
| `resources.requests.cpu` | 250m | CPU 请求 |
| `resources.limits.memory` | 2Gi | 内存限制 |
| `resources.limits.cpu` | 1000m | CPU 限制 |
| `autoscaling.enabled` | true | 启用自动伸缩 |
| `autoscaling.minReplicas` | 2 | 最小副本数 |
| `autoscaling.maxReplicas` | 10 | 最大副本数 |

## 5. 可观测性配置

### 5.1 Prometheus 指标

服务暴露 `/metrics` 端点，包含:
- Wasm 运行时指标 (内存、CPU、Fuel)
- 业务指标 (任务完成数、令牌使用量)
- 系统指标 (错误率、延迟)

### 5.2 日志收集

日志以 JSON 格式输出到 stdout/stderr，由 Promtail 收集并发送到 Loki。

查询示例:
```logql
# 查看所有错误
{app="servant-guild"} |= "error"

# 按使魔类型过滤
{app="servant-guild"} | json | servant_type = "coordinator"
```

### 5.3 分布式追踪

使用 OpenTelemetry 进行分布式追踪，支持:
- Jaeger 导出
- 自动采样配置
- 跨服务追踪

## 6. 安全配置

### 6.1 密钥管理

使用 Kubernetes Secrets 或 AWS Secrets Manager:

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: servant-guild-secrets
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secretsmanager
  target:
    name: servant-guild-secrets
  data:
    - secretKey: database-url
      remoteRef:
        key: servant-guild/database
        property: url
```

### 6.2 网络策略

Kubernetes NetworkPolicy 限制:
- 只允许内部服务通信
- 限制外部访问仅通过 Ingress
- 数据库仅允许应用命名空间访问

## 7. 运维操作

### 7.1 滚动更新

```bash
# 更新镜像
kubectl set image deployment/servant-guild \
  servant-guild=YOUR_REGISTRY/servant-guild:v1.1.0 \
  -n servant-guild

# 查看更新状态
kubectl rollout status deployment/servant-guild -n servant-guild
```

### 7.2 回滚

```bash
# 查看历史版本
kubectl rollout history deployment/servant-guild -n servant-guild

# 回滚到上一版本
kubectl rollout undo deployment/servant-guild -n servant-guild

# 回滚到指定版本
kubectl rollout undo deployment/servant-guild --to-revision=2 -n servant-guild
```

### 7.3 扩容

```bash
# 手动扩容
kubectl scale deployment/servant-guild --replicas=5 -n servant-guild

# 自动扩容 (已通过 HPA 配置)
# 自动根据 CPU/内存使用率扩容
```

### 7.4 备份

快照备份:
```bash
# 创建快照
curl -X POST http://servant-guild:5000/api/snapshot

# 恢复快照
curl -X POST http://servant-guild:5000/api/snapshot/restore \
  -H "Content-Type: application/json" \
  -d '{"snapshot_id": "snap_xxx"}'
```

## 8. 监控和告警

### 8.1 关键指标

| 指标 | 告警阈值 |
|------|---------|
| 错误率 | > 5% 持续 5 分钟 |
| P95 延迟 | > 2s 持续 5 分钟 |
| 内存使用 | > 80% 持续 5 分钟 |
| CPU 使用 | > 90% 持续 5 分钟 |
| 任务失败率 | > 10% 持续 5 分钟 |

### 8.2 Grafana 仪表盘

导入预配置仪表盘:
1. 打开 Grafana (http://localhost:3000)
2. 导航到 Dashboards → Import
3. 上传 `deploy/observability/grafana/dashboards/servant-guild.json`

## 9. 故障排查

### 9.1 常见问题

**问题**: 服务无法连接数据库
```bash
# 检查密钥配置
kubectl get secret servant-guild-secrets -n servant-guild -o yaml

# 检查网络策略
kubectl get networkpolicy -n servant-guild
```

**问题**: 高内存使用
```bash
# 检查 Wasm 内存使用
curl http://servant-guild:5000/metrics | grep wasm_memory

# 检查泄漏
kubectl logs -l app=servant-guild -n servant-guild | grep "memory warning"
```

**问题**: 性能下降
```bash
# 检查追踪
# 访问 Jaeger UI 查看慢请求

# 检查指标
curl http://servant-guild:5000/metrics | grep duration
```

## 10. 升级指南

### 10.1 数据库迁移

```bash
# 检查迁移状态
cargo sqlx migrate info

# 执行迁移
cargo sqlx migrate run
```

### 10.2 版本兼容性

- v1.0.x -> v1.1.x: 直接升级
- v1.x.x -> v2.x.x: 需要数据迁移

## 参考文档

- [Terraform AWS Provider](https://registry.terraform.io/providers/hashicorp/aws/)
- [Kubernetes 官方文档](https://kubernetes.io/docs/)
- [Prometheus 查询指南](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Loki 日志查询](https://grafana.com/docs/loki/latest/logql/)
