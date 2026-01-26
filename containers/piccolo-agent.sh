#!/bin/bash

# Initialize variables
MASTER_IP="127.0.0.1" # MUST FIX - Piccolo master IP address

NODE_IP=$(hostname -I | awk '{print $1}')   # This Node IP address
NODE_NAME=$(hostname)  # Always use system hostname
NODE_ROLE="nodeagent"  # Default node role (master, nodeagent, bluechi)
NODE_TYPE="vehicle"  # Default node type (vehicle, cloud)

# Check environment argument
ENV="${1:-TODO}"

# Get the root path (script is in containers/, so go up one level)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_PATH="$(dirname "$SCRIPT_DIR")"

# Set architecture
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    SUFFIX="amd64"
elif [ "$ARCH" = "aarch64" ]; then
    SUFFIX="arm64"
else
    echo "Error: Unsupported architecture '${ARCH}'."
    exit 1
fi

# Set Binary PATH and download/prepare binary
if [ "$ENV" = "prod" ]; then
    BINARY_URL="https://github.com/eclipse-pullpiri/pullpiri/releases/latest/download/nodeagent-linux-${SUFFIX}"
    echo "Downloading binary from ${BINARY_URL}..."
    curl -L -o nodeagent "${BINARY_URL}"
    if [ $? -ne 0 ]; then
        echo "Error: Failed to download binary from ${BINARY_URL}"
        exit 1
    fi
    chmod +x nodeagent
    BINARY_PATH="./nodeagent"
elif [ "$ENV" = "dev" ]; then
    BINARY_PATH="${ROOT_PATH}/src/agent/nodeagent/target/debug/nodeagent"
    if [ ! -f "${BINARY_PATH}" ]; then
        echo "Error: Binary not found at ${BINARY_PATH}"
        exit 1
    fi
    # Copy to current directory for consistent handling
    cp "${BINARY_PATH}" ./nodeagent
    BINARY_PATH="./nodeagent"
else
    echo "Error: Invalid environment '${ENV}'. Must be 'prod' or 'dev'."
    exit 1
fi
echo "Running agent in ${ENV} mode with binary: ${BINARY_PATH}"

# Install binary to /opt/piccolo
echo "Installing nodeagent to /opt/piccolo..."
sudo mkdir -p /opt/piccolo
sudo mv nodeagent /opt/piccolo/nodeagent
sudo chmod +x /opt/piccolo/nodeagent
echo "Binary installed to /opt/piccolo/nodeagent"

# Create configuration file
echo "Creating configuration file..."
sudo mkdir -p /etc/piccolo
cat > /etc/piccolo/nodeagent.yaml << EOF
nodeagent:
  node_name: "${NODE_NAME}"
  node_type: "${NODE_TYPE}"
  node_role: "${NODE_ROLE}"
  master_ip: "${MASTER_IP}"
  node_ip: "${NODE_IP}"
  grpc_port: 47004
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  system:
    hostname: "${NODE_NAME}"
    platform: "$(uname -s)"
    architecture: "${ARCH}"
EOF

# Create systemd service file
echo "Creating systemd service file..."
cat > /etc/systemd/system/nodeagent.service << EOF
[Unit]
Description=PICCOLO NodeAgent Service
After=network-online.target
Wants=podman.socket

[Service]
Type=simple
ExecStart=/opt/piccolo/nodeagent --config /etc/piccolo/nodeagent.yaml
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=info
Environment=MASTER_NODE_IP=${MASTER_IP}
Environment=NODE_IP=${NODE_IP}
Environment=GRPC_PORT=47004

# Security hardening settings
ProtectSystem=full
ProtectHome=true
NoNewPrivileges=true

ReadWritePaths=/etc/piccolo
ReadWritePaths=/etc/containers/systemd

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and enable service
echo "Enabling NodeAgent service..."
sudo systemctl daemon-reload
sudo systemctl enable nodeagent.service || {
    echo "Error: Failed to enable NodeAgent service."
    exit 1
}

# Start service
echo "Starting NodeAgent service..."
sudo systemctl start nodeagent.service || {
    echo "Warning: Failed to start NodeAgent service."
    exit 1
}