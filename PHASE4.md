# ServantGuild - Phase 4: Autonomy (Production Deployment)

## Overview

Phase 4 implements production-grade deployment infrastructure, observability systems, economic model optimization, security hardening, and CI/CD automation to make ServantGuild ready for production deployment.

## Completed Features

### 1. Infrastructure as Code (Terraform)

Complete AWS infrastructure configuration including:

- **VPC Module**: Multi-AZ VPC with public/private subnets, NAT gateways, and VPC flow logs
- **EC2 Instances**: Application servers with auto-scaling support
- **RDS PostgreSQL**: Managed PostgreSQL with Multi-AZ deployment
- **ElastiCache Redis**: Managed Redis cluster for caching
- **S3 Storage**: Artifact and backup storage
- **CloudWatch**: Comprehensive monitoring and alerting
- **SNS Notifications**: Real-time alerts for critical events

**Files:**
- `deploy/terraform/main.tf` - Main infrastructure configuration
- `deploy/terraform/modules/vpc/main.tf` - VPC module
- `deploy/terraform/user_data.sh` - EC2 initialization script

### 2. Containerization (Docker)

Production-ready Docker configuration:

- **Multi-stage Dockerfile**: Optimized for minimal image size
- **Wasm Build Stage**: Compiles Wasm modules during build
- **Security Hardening**: Non-root user, minimal attack surface
- **Health Checks**: Built-in health monitoring

**Files:**
- `deploy/docker/Dockerfile` - Production Dockerfile
- `deploy/docker/docker-compose.yml` - Local development compose file

### 3. Kubernetes Deployment

Complete Kubernetes manifests for production deployment:

- **Deployment**: Rolling updates with zero downtime
- **Service**: ClusterIP with metrics port
- **HorizontalPodAutoscaler**: Auto-scaling based on CPU/memory
- **PodDisruptionBudget**: High availability during maintenance
- **Ingress**: TLS termination with cert-manager
- **Network Policies**: Network segmentation

**Files:**
- `deploy/kubernetes/servant-guild.yaml` - Complete Kubernetes manifests
- `deploy/helm/Chart.yaml` - Helm chart definition
- `deploy/helm/values.yaml` - Configurable values

### 4. Observability Stack

#### 4.1 Logging (Loki)

- Structured JSON logging
- Log retention (30 days)
- Log aggregation from multiple sources
- Kubernetes pod log collection

**Files:**
- `deploy/observability/loki/loki-config.yaml` - Loki configuration
- `deploy/observability/promtail/config.yml` - Log shipping configuration

#### 4.2 Metrics (Prometheus)

- Application metrics exposition
- System resource monitoring
- Database and cache metrics
- Custom alerting rules

**Files:**
- `deploy/observability/prometheus/prometheus.yml` - Prometheus configuration
- `deploy/observability/prometheus/rules/alerts.yml` - Alerting rules

#### 4.3 Tracing (OpenTelemetry)

- Distributed tracing collection
- Jaeger backend integration
- OTLP protocol support
- Tail sampling for cost optimization

**Files:**
- `deploy/observability/opentelemetry/otel-config.yaml` - OpenTelemetry configuration

### 5. Economic Model

Token optimization and cost management system:

#### 5.1 Budget Management

- Daily/hourly budget limits
- Per-agent budget allocation
- Auto-throttling when limits exceeded
- Budget status tracking

#### 5.2 Token Tracking

- Real-time token usage tracking
- Provider-level statistics
- Agent-level statistics
- Cache hit rate monitoring

#### 5.3 Pricing Engine

- Multi-provider pricing comparison
- Automatic cost calculation
- Cheapest provider selection
- Quality/cost trade-off analysis

#### 5.4 Token Optimization

- Prompt compression
- Response caching
- Provider auto-selection
- Optimization recommendations

**Files:**
- `src/economic/mod.rs` - Main economic model module
- `src/economic/budget.rs` - Budget management
- `src/economic/tracker.rs` - Token tracking
- `src/economic/pricing.rs` - Pricing engine
- `src/economic/optimizer.rs` - Token optimization
- `src/economic/metrics.rs` - Economic metrics
- `src/economic/provider.rs` - Provider selection
- `src/economic/cache.rs` - Token cache

### 6. Security Hardening

Comprehensive security implementation:

#### 6.1 Audit Logging

- Complete audit trail
- Multiple operation types tracked
- Compliance-ready exports (JSON/CSV)
- Retention management

#### 6.2 Secrets Management

- Encrypted secret storage
- Secret rotation policies
- Access tracking
- Expiration management

#### 6.3 Encryption

- AES-256-GCM encryption
- Key management
- Password hashing
- Secure random generation

#### 6.4 Network Isolation

- Network policy management
- Zone-based segmentation
- Connection tracking
- Default-deny policies

#### 6.5 Input Validation

- SQL injection detection
- XSS attack prevention
- Command injection blocking
- Path traversal prevention

**Files:**
- `src/security/mod.rs` - Main security module
- `src/security/audit.rs` - Audit logging
- `src/security/secrets.rs` - Secrets management
- `src/security/encryption.rs` - Encryption utilities
- `src/security/network.rs` - Network isolation
- `src/security/validation.rs` - Input validation

### 7. CI/CD Pipeline

Automated build, test, and deployment pipeline:

#### 7.1 Code Quality

- Rust formatting checks (rustfmt)
- Linting (clippy)
- Security audit (cargo-audit)
- Vulnerability scanning (Trivy)

#### 7.2 Build & Test

- Release build
- Unit tests
- Integration tests
- Coverage reporting

#### 7.3 Docker Build

- Multi-architecture builds
- Image scanning
- Registry push (GitHub Container Registry)

#### 7.4 Deployment

- Staging deployment (develop branch)
- Production deployment (main branch)
- Canary deployments
- Automatic rollback on failure

**Files:**
- `.github/workflows/ci-cd.yml` - Complete CI/CD pipeline

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Production Environment                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                        AWS Infrastructure (Terraform)                │  │
│  │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                │  │
│  │  │     VPC     │   │     EC2     │   │     RDS     │                │  │
│  │  │  (Multi-AZ) │──▶│ (Auto-Scale)│──▶│ PostgreSQL  │                │  │
│  │  └─────────────┘   └─────────────┘   └─────────────┘                │  │
│  │                           │                                          │  │
│  │  ┌─────────────┐   ┌─────┴─────┐   ┌─────────────┐                  │  │
│  │  │ ElastiCache │   │     S3    │   │ CloudWatch  │                  │  │
│  │  │    Redis    │◀──│  Storage  │──▶│  Monitoring │                  │  │
│  │  └─────────────┘   └───────────┘   └─────────────┘                  │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                           │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                      Kubernetes Cluster (EKS)                        │  │
│  │                                                                      │  │
│  │  ┌─────────────────────────────────────────────────────────────┐    │  │
│  │  │                    ServantGuild Deployment                    │    │  │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │    │  │
│  │  │  │Coordinator  │  │   Worker    │  │   Warden    │           │    │  │
│  │  │  │   (Wasm)    │  │   (Wasm)    │  │   (Wasm)    │           │    │  │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘           │    │  │
│  │  │  ┌─────────────┐  ┌─────────────┐                             │    │  │
│  │  │  │   Speaker   │  │ Contractor  │                             │    │  │
│  │  │  │   (Wasm)    │  │   (Wasm)    │                             │    │  │
│  │  │  └─────────────┘  └─────────────┘                             │    │  │
│  │  └─────────────────────────────────────────────────────────────┘    │  │
│  │                                                                      │  │
│  │  ┌─────────────────────────────────────────────────────────────┐    │  │
│  │  │                     Observability Stack                       │    │  │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐      │    │  │
│  │  │  │Prometheus│  │   Loki   │  │  Jaeger  │  │ Grafana  │      │    │  │
│  │  │  │ (Metrics)│  │  (Logs)  │  │ (Traces) │  │ (Visual) │      │    │  │
│  │  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘      │    │  │
│  │  └─────────────────────────────────────────────────────────────┘    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

## Deployment Guide

### Prerequisites

- AWS Account with appropriate permissions
- kubectl configured
- Helm 3.x installed
- Terraform 1.5+ installed
- Docker installed

### Quick Start

1. **Initialize Infrastructure**
   ```bash
   cd deploy/terraform
   terraform init
   terraform plan
   terraform apply
   ```

2. **Configure kubectl**
   ```bash
   aws eks update-kubeconfig --name servant-guild-production --region us-east-1
   ```

3. **Deploy with Helm**
   ```bash
   helm upgrade --install servant-guild ./deploy/helm \
     --namespace servant-guild \
     --create-namespace \
     --values ./deploy/helm/values-production.yaml
   ```

4. **Verify Deployment**
   ```bash
   kubectl get pods -n servant-guild
   kubectl port-forward svc/servant-guild 8080:8080 -n servant-guild
   curl http://localhost:8080/health
   ```

### Local Development

```bash
# Start all services
cd deploy/docker
docker-compose --profile observability up -d

# View logs
docker-compose logs -f servant-guild

# Access services
# API: http://localhost:8080
# Grafana: http://localhost:3000
# Prometheus: http://localhost:9091
```

## Monitoring

### Key Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `servant_guild_tokens_total` | Total tokens used | Daily budget limit |
| `servant_guild_cost_usd_total` | Total cost in USD | $50/day |
| `servant_guild_cache_hit_rate` | Cache hit percentage | < 30% |
| `servant_guild_avg_latency_ms` | Average request latency | > 1000ms |

### Dashboards

Access Grafana at `http://grafana.servantguild.dev` (default: admin/admin)

Pre-built dashboards:
- Application Overview
- Economic Metrics
- Security Events
- Infrastructure Health

### Alerting

Alerts are configured in Prometheus and sent via:
- SNS notifications
- Slack integration
- Email alerts

## Security

### Network Policies

Default policies enforce:
- Internal agent communication (port 8080, 9090)
- Database access from agents only (port 5432)
- Redis access from agents only (port 6379)
- External API calls via HTTPS only (port 443)

### Secrets Management

Secrets are:
- Encrypted at rest (AES-256-GCM)
- Rotated on schedule
- Access-audited
- Never logged or exposed

## Cost Optimization

### Economic Model Features

1. **Provider Selection**: Automatically selects cheapest provider for quality requirements
2. **Token Caching**: Reuses responses for identical prompts
3. **Prompt Compression**: Reduces token count by 15-20%
4. **Budget Alerts**: Notifies at 70% and 90% of budget

### Expected Savings

- Provider optimization: ~60% cost reduction (DeepSeek vs OpenAI)
- Caching: ~20% reduction in API calls
- Prompt compression: ~15% token reduction

## Next Steps

1. **Phase 5: Advanced Autonomy**
   - Multi-region deployment
   - Advanced ML models
   - Self-healing infrastructure
   - Predictive scaling

2. **Continuous Improvement**
   - Performance optimization
   - Cost reduction initiatives
   - Security hardening
   - Documentation updates

## License

MIT License - See [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
