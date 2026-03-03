#!/bin/bash
#
# ServantGuild Uninstallation Script for Linux
#
# Usage: sudo ./uninstall.sh

set -e

# Configuration
INSTALL_DIR="/opt/servant-guild"
CONFIG_DIR="/etc/servant-guild"
LOG_DIR="/var/log/servant-guild"
SERVICE_NAME="servant-guild"
SERVICE_USER="servant-guild"
SERVICE_GROUP="servant-guild"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

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

# Confirmation
read -p "This will uninstall ServantGuild. Continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log_info "Uninstall cancelled."
    exit 0
fi

# Stop service
log_info "Stopping service..."
systemctl stop $SERVICE_NAME 2>/dev/null || true
systemctl disable $SERVICE_NAME 2>/dev/null || true

# Remove service file
log_info "Removing service file..."
rm -f /etc/systemd/system/$SERVICE_NAME.service
systemctl daemon-reload

# Ask about data removal
read -p "Remove data and config directories? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    log_info "Removing directories..."
    rm -rf "$INSTALL_DIR"
    rm -rf "$CONFIG_DIR"
    rm -rf "$LOG_DIR"
else
    log_info "Keeping data directories:"
    log_info "  - $INSTALL_DIR"
    log_info "  - $CONFIG_DIR"
    log_info "  - $LOG_DIR"
fi

# Remove user
read -p "Remove user '$SERVICE_USER'? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    log_info "Removing user..."
    userdel "$SERVICE_USER" 2>/dev/null || true
fi

log_info "Uninstallation complete."
