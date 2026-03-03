# ServantGuild Documentation

Welcome to the **ServantGuild** documentation. This directory contains all design documents, architecture specifications, and implementation guides.

## Overview

ServantGuild is a **Rust-first, Wasm-driven, highly autonomous multi-agent collaboration system**. It consists of 5 core permanent servants and temporary elastic servants, making collective decisions through a consensus engine and achieving self-evolution using GitHub as its gene pool.

## Documentation Structure

```
docs/
├── README.md                          # This file - Documentation entry point
├── design/                            # Design specifications
│   ├── servant_guild_whitepaper_v1.1.md  # Core whitepaper (核心理念)
│   └── servant_guild_infrastructure.md   # Infrastructure requirements (基础设施)
├── architecture/                      # Architecture specifications
│   └── servant_guild_architecture_v1.0.md # System architecture (系统架构)
├── api_reference.md                   # API reference
├── deployment_guide.md               # Deployment guide
└── task_planning.md                  # Task planning and roadmap
```

## Quick Navigation

### For Newcomers

1. Start with [Whitepaper v1.1](./design/servant_guild_whitepaper_v1.1.md) to understand the core philosophy
2. Read [Architecture v1.0](./architecture/servant_guild_architecture_v1.0.md) for system design
3. Check [Infrastructure Requirements](./design/servant_guild_infrastructure.md) for deployment prerequisites

### For Developers

1. [API Reference](./api_reference.md) - Complete API documentation
2. [Deployment Guide](./deployment_guide.md) - Production deployment instructions
3. [Task Planning](./task_planning.md) - Development roadmap and task breakdown

### For Operators

1. [Infrastructure Requirements](./design/servant_guild_infrastructure.md) - What you need to run ServantGuild
2. [Deployment Guide](./deployment_guide.md) - Step-by-step deployment
3. [Architecture v1.0](./architecture/servant_guild_architecture_v1.0.md) - Understanding the system for troubleshooting

## Core Documents

### [Whitepaper v1.1](./design/servant_guild_whitepaper_v1.1.md)

The foundational document describing ServantGuild's philosophy, roles, and evolution path.

**Key Sections:**
- Core Philosophy (集体生存、共识驱动、持续进化)
- The 5 Core Servant Roles (Coordinator, Worker, Warden, Speaker, Contractor)
- 4-Phase Implementation Roadmap
- Success Metrics

### [Architecture v1.0](./architecture/servant_guild_architecture_v1.0.md)

Detailed system architecture specification.

**Key Sections:**
- Layered Architecture (Core, Runtime, Bridge, Agent, Infrastructure)
- Module Structure
- Data Flow Diagrams
- Security Model
- Observability Architecture

### [Infrastructure Requirements](./design/servant_guild_infrastructure.md)

Complete infrastructure requirements for running ServantGuild in production.

**Key Sections:**
- The Sanctuary (宿主环境)
- The Treasury (经费与密钥)
- The Library (记忆与知识库)
- The Tentacles (感知与执行触手)
- The Red Phone (紧急联络通道)

## Implementation Phases

| Phase | Name | Status | Document |
|-------|------|--------|----------|
| 1 | Core | ✅ Completed | [PHASE1.md](../PHASE1.md) |
| 2 | Cognition | ✅ Completed | [PHASE2.md](../PHASE2.md) |
| 3 | Orchestration (Evolution) | ✅ Completed | [PHASE3.md](../PHASE3.md) |
| 4 | Autonomy | ✅ Completed | [PHASE4.md](../PHASE4.md) |

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines on contributing to ServantGuild.

## License

This project is licensed under the MIT License - see [LICENSE](../LICENSE) for details.

---

*"The Guild that maintains itself, evolves itself, and governs itself."*
