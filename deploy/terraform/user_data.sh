#!/bin/bash
# ServantGuild Host Initialization Script
# This script is executed when the EC2 instance first boots

set -euo pipefail

# Configuration from Terraform
ENVIRONMENT="${environment}"
DB_ENDPOINT="${db_endpoint}"
REDIS_ENDPOINT="${redis_endpoint}"
LOG_GROUP_NAME="${log_group_name}"
GITHUB_SECRET_ARN="${github_secret_arn}"
LLM_SECRET_ARN="${llm_secret_arn}"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

log "Starting ServantGuild host initialization..."

# Update system packages
log "Updating system packages..."
yum update -y

# Install Docker
log "Installing Docker..."
yum install -y docker
systemctl start docker
systemctl enable docker
usermod -aG docker ec2-user

# Install AWS CLI (should be pre-installed, but ensure)
log "Installing AWS CLI..."
if ! command -v aws &> /dev/null; then
    yum install -y awscli
fi

# Install CloudWatch Agent
log "Installing CloudWatch Agent..."
yum install -y amazon-cloudwatch-agent

# Configure CloudWatch Agent
log "Configuring CloudWatch Agent..."
cat > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json <<EOF
{
    "logs": {
        "logs_collected": {
            "files": {
                "collect_list": [
                    {
                        "file_path": "/var/log/servant-guild/*.log",
                        "log_group_name": "${LOG_GROUP_NAME}",
                        "log_stream_name": "{instance_id}/servant-guild"
                    },
                    {
                        "file_path": "/var/log/docker",
                        "log_group_name": "${LOG_GROUP_NAME}",
                        "log_stream_name": "{instance_id}/docker"
                    },
                    {
                        "file_path": "/var/log/messages",
                        "log_group_name": "${LOG_GROUP_NAME}",
                        "log_stream_name": "{instance_id}/system"
                    }
                ]
            }
        }
    },
    "metrics": {
        "namespace": "ServantGuild/${ENVIRONMENT}",
        "metrics_collected": {
            "cpu": {
                "measurement": ["cpu_usage_active"],
                "metrics_collection_interval": 60
            },
            "mem": {
                "measurement": ["mem_used_percent"],
                "metrics_collection_interval": 60
            },
            "disk": {
                "measurement": ["disk_used_percent"],
                "metrics_collection_interval": 60,
                "resources": ["/"]
            },
            "net": {
                "measurement": ["net_bytes_recv", "net_bytes_sent"],
                "metrics_collection_interval": 60,
                "resources": ["eth0"]
            }
        }
    }
}
EOF

/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl -a fetch-config -m ec2 -s -c file:/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json

# Create application directory
log "Creating application directories..."
mkdir -p /opt/servant-guild/{config,data,logs,wasm}
chown -R ec2-user:ec2-user /opt/servant-guild

# Create configuration file
log "Creating configuration..."
cat > /opt/servant-guild/config/config.toml <<EOF
[server]
host = "0.0.0.0"
port = 8080
metrics_port = 9090

[database]
url = "postgres://servant_admin:CHANGE_ME@${DB_ENDPOINT}/servant_guild"
pool_size = 10

[redis]
url = "redis://${REDIS_ENDPOINT}:6379"

[logging]
level = "info"
format = "json"
path = "/opt/servant-guild/logs"

[runtime]
wasm_memory_limit_mb = 512
wasm_time_limit_secs = 60
sandbox_enabled = true

[consensus]
voting_timeout_secs = 3600
normal_quorum = 3
critical_quorum = 5

[evolution]
auto_evolve = false
require_human_approval = true
github_repo = "https://github.com/hanxueyuan/servant-guild.git"
EOF

# Set up secrets retrieval
log "Setting up secrets retrieval..."
mkdir -p /opt/servant-guild/secrets
chmod 700 /opt/servant-guild/secrets

# Create secrets fetch script
cat > /opt/servant-guild/fetch-secrets.sh <<'SECRETS_EOF'
#!/bin/bash
set -euo pipefail

# Fetch GitHub token
aws secretsmanager get-secret-value --secret-id "${GITHUB_SECRET_ARN}" --query SecretString --output text > /opt/servant-guild/secrets/github_token

# Fetch LLM API keys
aws secretsmanager get-secret-value --secret-id "${LLM_SECRET_ARN}" --query SecretString --output text > /opt/servant-guild/secrets/llm_keys.json

chmod 600 /opt/servant-guild/secrets/*
SECRETS_EOF

chmod +x /opt/servant-guild/fetch-secrets.sh

# Run secrets fetch
/opt/servant-guild/fetch-secrets.sh || log "Warning: Could not fetch secrets (may not exist yet)"

# Pull Docker image (if available)
log "Pulling ServantGuild Docker image..."
docker pull hanxueyuan/servant-guild:latest || log "Docker image not yet available"

# Create systemd service for ServantGuild
log "Creating systemd service..."
cat > /etc/systemd/system/servant-guild.service <<EOF
[Unit]
Description=ServantGuild Host Service
After=docker.service
Requires=docker.service

[Service]
Type=simple
User=ec2-user
Group=ec2-user
WorkingDirectory=/opt/servant-guild
ExecStartPre=/opt/servant-guild/fetch-secrets.sh
ExecStart=/usr/bin/docker run --rm \\
    --name servant-guild \\
    -p 8080:8080 \\
    -p 9090:9090 \\
    -v /opt/servant-guild/config:/app/config:ro \\
    -v /opt/servant-guild/data:/app/data \\
    -v /opt/servant-guild/logs:/app/logs \\
    -v /opt/servant-guild/wasm:/app/wasm \\
    -v /opt/servant-guild/secrets:/app/secrets:ro \\
    -e RUST_LOG=info \\
    hanxueyuan/servant-guild:latest
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable servant-guild

# Install Prometheus Node Exporter (for metrics)
log "Installing Node Exporter..."
curl -LO https://github.com/prometheus/node_exporter/releases/download/v1.6.1/node_exporter-1.6.1.linux-amd64.tar.gz
tar xzf node_exporter-1.6.1.linux-amd64.tar.gz
mv node_exporter-1.6.1.linux-amd64/node_exporter /usr/local/bin/
rm -rf node_exporter-1.6.1.linux-amd64*

# Create node exporter service
cat > /etc/systemd/system/node-exporter.service <<EOF
[Unit]
Description=Prometheus Node Exporter
After=network.target

[Service]
Type=simple
User=nobody
Group=nobody
ExecStart=/usr/local/bin/node_exporter \\
    --web.listen-address=:9100 \\
    --collector.systemd

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable node-exporter
systemctl start node-exporter

# Configure firewall (using iptables)
log "Configuring firewall..."
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
iptables -A INPUT -p tcp --dport 22 -j ACCEPT
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
iptables -A INPUT -p tcp --dport 9090 -s 10.0.0.0/8 -j ACCEPT  # Internal only
iptables -A INPUT -p tcp --dport 9100 -s 10.0.0.0/8 -j ACCEPT  # Internal only
iptables -A INPUT -j DROP

# Save iptables rules
service iptables save || iptables-save > /etc/sysconfig/iptables

# Set up automatic updates
log "Configuring automatic updates..."
yum install -y yum-cron
sed -i 's/apply_updates = no/apply_updates = yes/' /etc/yum/yum-cron.conf
systemctl enable yum-cron
systemctl start yum-cron

# Final status
log "ServantGuild host initialization complete!"
log "Configuration files: /opt/servant-guild/config/"
log "Logs directory: /opt/servant-guild/logs/"
log "Data directory: /opt/servant-guild/data/"

# Start the service
log "Starting ServantGuild service..."
systemctl start servant-guild || log "Service start deferred (image not available)"

# Print status
systemctl status servant-guild --no-pager || true
