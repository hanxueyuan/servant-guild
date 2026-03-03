# Getting Started Docs

For first-time setup and quick orientation.

## Platform Support

ServantGuild runs on all major operating systems:

| Platform | Architectures | Install Methods |
|----------|--------------|-----------------|
| **Linux** | x86_64, ARM64, ARMv7 | Docker, Binary, Source |
| **Windows** | x86_64 | Docker, Binary, Source |
| **macOS** | x86_64, ARM64 (M1/M2) | Docker, Binary, Homebrew, Source |

## Quick Start by Platform

### Linux

```bash
# Option 1: One-line install (recommended)
curl -fsSL https://raw.githubusercontent.com/hanxueyuan/servant-guild/main/scripts/install.sh | bash

# Option 2: Docker
docker run -d -p 5000:5000 servant-guild:latest

# Option 3: From source
git clone https://github.com/hanxueyuan/servant-guild.git
cd servant-guild && cargo build --release
```

### Windows

```powershell
# Option 1: Download binary from releases
# https://github.com/hanxueyuan/servant-guild/releases

# Option 2: Docker
docker run -d -p 5000:5000 servant-guild:latest

# Option 3: From source (requires Visual Studio Build Tools)
git clone https://github.com/hanxueyuan/servant-guild.git
cd servant-guild
cargo build --release
```

### macOS

```bash
# Option 1: Homebrew (recommended)
brew tap hanxueyuan/servant-guild
brew install servant-guild

# Option 2: Docker
docker run -d -p 5000:5000 servant-guild:latest

# Option 3: From source
git clone https://github.com/hanxueyuan/servant-guild.git
cd servant-guild && cargo build --release
```

## Start Path

1. Main overview and quick start: [../../README.md](../../README.md)
2. Platform-specific setup: See sections above
3. Deployment options: [../deployment_guide.md](../deployment_guide.md)
4. Find commands by tasks: [../commands-reference.md](../commands-reference.md)

## Choose Your Path

| Scenario | Command |
|----------|---------|
| I have an API key, want fastest setup | `servant-guild init --api-key sk-... --provider openrouter` |
| I want guided prompts | `servant-guild init --interactive` |
| Config exists, just fix channels | `servant-guild init --channels-only` |
| Using subscription auth | See Subscription Auth in main README |

## Onboarding and Validation

- Quick onboarding: `servant-guild init --api-key "sk-..." --provider openrouter`
- Interactive onboarding: `servant-guild init --interactive`
- Existing config protection: reruns require explicit confirmation (or `--force` in non-interactive flows)
- Validate environment: `servant-guild status` + `servant-guild doctor`

## Platform-Specific Notes

### Linux

- Install `libgit2-dev` for GitHub integration features
- Use `systemd` for production deployment
- SELinux may require additional configuration

### Windows

- Requires Visual Studio Build Tools with C++ workload
- PowerShell 7+ recommended for best experience
- Windows Defender may flag binaries (add exclusion if needed)

### macOS

- Install Xcode Command Line Tools: `xcode-select --install`
- Apple Silicon (M1/M2) fully supported
- Use `launchd` for production deployment

## Next

- Runtime operations: [../operations/README.md](../operations/README.md)
- Reference catalogs: [../reference/README.md](../reference/README.md)
- Deployment guide: [../deployment_guide.md](../deployment_guide.md)
