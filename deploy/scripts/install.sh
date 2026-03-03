#!/bin/bash
#
# ServantGuild Installation Script for Linux
# 
# Usage: sudo ./install.sh
#
# This script installs ServantGuild as a systemd service on Linux.

set -e

# Configuration
INSTALL_DIR="/opt/servant-guild"
CONFIG_DIR="/etc/servant-guild"
LOG_DIR="/var/log/servant-guild"
BIN_NAME="servant-guild"
SERVICE_NAME="servant-guild"
SERVICE_USER="servant-guild"
SERVICE_GROUP="servant-guild"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    log_error "Please run as root (use sudo)"
    exit 1
fi

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        BINARY_ARCH="x86_64"
        ;;
    aarch64)
        BINARY_ARCH="aarch64"
        ;;
    *)
        log_error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

log_info "Detected architecture: $ARCH"

# Check for required commands
check_requirements() {
    log_info "Checking requirements..."
    
    # Check for systemd
    if ! command -v systemctl &> /dev/null; then
        log_error "systemctl not found. This script requires systemd."
        exit 1
    fi
    
    log_info "Requirements satisfied."
}

# Create user and group
create_user() {
    if id "$SERVICE_USER" &>/dev/null; then
        log_info "User '$SERVICE_USER' already exists"
    else
        log_info "Creating user '$SERVICE_USER'..."
        useradd --system --no-create-home --shell /bin/false "$SERVICE_USER"
    fi
}

# Create directories
create_directories() {
    log_info "Creating directories..."
    
    mkdir -p "$INSTALL_DIR/bin"
    mkdir -p "$INSTALL_DIR/data"
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$LOG_DIR"
    
    # Set permissions
    chown -R "$SERVICE_USER:$SERVICE_GROUP" "$INSTALL_DIR"
    chown -R "$SERVICE_USER:$SERVICE_GROUP" "$LOG_DIR"
    chmod 750 "$INSTALL_DIR"
    chmod 750 "$LOG_DIR"
}

# Install binary
install_binary() {
    log_info "Installing binary..."
    
    # Check if binary exists in current directory or release directory
    if [ -f "./target/release/$BIN_NAME" ]; then
        cp "./target/release/$BIN_NAME" "$INSTALL_DIR/bin/"
    elif [ -f "./$BIN_NAME" ]; then
        cp "./$BIN_NAME" "$INSTALL_DIR/bin/"
    else
        log_warn "Binary not found in expected locations."
        log_info "Please build the project first: cargo build --release"
        log_info "Or place the binary in the current directory."
        exit 1
    fi
    
    chmod 755 "$INSTALL_DIR/bin/$BIN_NAME"
    chown "$SERVICE_USER:$SERVICE_GROUP" "$INSTALL_DIR/bin/$BIN_NAME"
}

# Create default config
create_config() {
    CONFIG_FILE="$CONFIG_DIR/config.toml"
    
    if [ -f "$CONFIG_FILE" ]; then
        log_info "Config file already exists, skipping..."
        return
    fi
    
    log_info "Creating default config..."
    
    cat > "$CONFIG_FILE" << 'EOF'
# ServantGuild Configuration
# See: https://github.com/hanxueyuan/servant-guild

[guild]
name = "ServantGuild-Alpha"

[daemon]
# Listen address for the daemon
listen = "127.0.0.1:8080"
# Enable debug mode
debug = false

[runtime]
# Runtime type: native, wasm
kind = "native"

[memory]
# Memory backend: sqlite, markdown, qdrant
backend = "sqlite"
# Embedding model for semantic search
embedding_model = "text-embedding-3-small"

[consensus]
# Number of core servants (must be odd)
core_servants_count = 5
# Voting timeout in seconds
voting_timeout_secs = 3600

[log]
# Log level: trace, debug, info, warn, error
level = "info"
# Log format: json, pretty
format = "json"
EOF
    
    chmod 640 "$CONFIG_FILE"
    chown "$SERVICE_USER:$SERVICE_GROUP" "$CONFIG_FILE"
}

# Install systemd service
install_service() {
    log_info "Installing systemd service..."
    
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    SERVICE_FILE="$SCRIPT_DIR/systemd/servant-guild.service"
    
    if [ ! -f "$SERVICE_FILE" ]; then
        # Create service file if not exists
        cat > /etc/systemd/system/$SERVICE_NAME.service << 'EOF'
[Unit]
Description=ServantGuild Daemon - Multi-Agent Autonomy System
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
User=servant-guild
Group=servant-guild
WorkingDirectory=/opt/servant-guild
ExecStart=/opt/servant-guild/bin/servant-guild daemon
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s
TimeoutStartSec=30s
TimeoutStopSec=30s
LimitNOFILE=65536
LimitNPROC=4096

NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/servant-guild /var/log/servant-guild
ReadOnlyPaths=/etc

Environment="RUST_LOG=info"
Environment="SERVANT_GUILD_CONFIG=/etc/servant-guild/config.toml"

[Install]
WantedBy=multi-user.target
EOF
    else
        cp "$SERVICE_FILE" /etc/systemd/system/
    fi
    
    # Reload systemd
    systemctl daemon-reload
    
    # Enable service
    systemctl enable $SERVICE_NAME
    
    log_info "Service installed successfully."
}

# Start service
start_service() {
    log_info "Starting service..."
    systemctl start $SERVICE_NAME
    
    sleep 2
    
    if systemctl is-active --quiet $SERVICE_NAME; then
        log_info "Service started successfully."
        systemctl status $SERVICE_NAME --no-pager
    else
        log_error "Service failed to start. Check logs with: journalctl -u $SERVICE_NAME -xe"
        exit 1
    fi
}

# Main installation
main() {
    log_info "==================================="
    log_info "ServantGuild Installation Script"
    log_info "==================================="
    
    check_requirements
    create_user
    create_directories
    install_binary
    create_config
    install_service
    
    read -p "Start the service now? [Y/n] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        start_service
    fi
    
    log_info "==================================="
    log_info "Installation Complete!"
    log_info "==================================="
    echo ""
    echo "Useful commands:"
    echo "  Status:  sudo systemctl status $SERVICE_NAME"
    echo "  Start:   sudo systemctl start $SERVICE_NAME"
    echo "  Stop:    sudo systemctl stop $SERVICE_NAME"
    echo "  Restart: sudo systemctl restart $SERVICE_NAME"
    echo "  Logs:    sudo journalctl -u $SERVICE_NAME -f"
    echo ""
    echo "Config file: $CONFIG_DIR/config.toml"
    echo "Data directory: $INSTALL_DIR/data"
    echo "Log directory: $LOG_DIR"
}

main "$@"
