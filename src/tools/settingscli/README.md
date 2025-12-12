<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# SettingsCLI

A command-line interface tool for interacting with the Pullpiri SettingsService via REST APIs.

## Overview

SettingsCLI provides developers with a convenient way to access all REST APIs provided by the SettingsService. Since SettingsService runs as a container, this separated CLI app can be used anywhere on the system where developers need to interact with the service.

## Features

- **Metrics Operations**: Get system metrics and board information
- **Board Management**: List and inspect board configurations
- **Node Management**: Monitor and manage individual nodes
- **SoC Management**: Handle System-on-Chip resource management
- **Container Management**: List and inspect container configurations and stats
- **YAML Artifact Management**: Apply and withdraw YAML artifacts to/from the system
- **Health Checks**: Verify SettingsService connectivity
- **Flexible Output**: Support for both formatted and raw JSON output
- **Comprehensive Help**: Built-in help system for all commands

## Installation

Build the CLI tool as part of the Pullpiri project:

```bash
cd src/tools/settingscli
cargo build --release
```

Or build all tools together:

```bash
cd src/tools
cargo build --release
```

## Usage

### Basic Syntax

```bash
settingscli [OPTIONS] <COMMAND>
```

### Global Options

- `-u, --url <URL>`: SettingsService URL (default: http://localhost:8080)
- `-t, --timeout <SECONDS>`: Request timeout in seconds (default: 30)
- `-v, --verbose`: Enable verbose output
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Commands

#### Health Check

Test connection to SettingsService:

```bash
settingscli health
```

#### Metrics

Get system metrics:

```bash
# Get formatted metrics
settingscli metrics get

# Get raw JSON metrics
settingscli metrics raw
```

#### Board Operations

```bash
# List all boards
settingscli board list

# Get specific board information
settingscli board get <BOARD_ID>

# Get raw board data
settingscli board raw [BOARD_ID]
```

#### Node Operations

```bash
# List all nodes
settingscli node list

# Get specific node information
settingscli node get <NODE_ID>

# Get raw node data
settingscli node raw [NODE_ID]
```

#### SoC Operations

```bash
# List all SoCs
settingscli soc list

# Get specific SoC information
settingscli soc get <SOC_ID>

# Get raw SoC data
settingscli soc raw [SOC_ID]
```

#### Container Operations

```bash
# List all containers
settingscli container list

# Get specific container information
settingscli container get <CONTAINER_ID>

# Get raw container data
settingscli container raw
```

#### YAML Artifact Management

```bash
# Apply YAML artifact from file
settingscli yaml apply <FILE_PATH>

# Apply YAML artifact from stdin
settingscli yaml apply -

# Withdraw YAML artifact from file
settingscli yaml withdraw <FILE_PATH>

# Withdraw YAML artifact from stdin
settingscli yaml withdraw -
```

### Examples

```bash
# Check if SettingsService is running
settingscli health

# Get system metrics with custom URL and timeout
settingscli -u http://192.168.1.100:8080 -t 60 metrics get

# List all boards with verbose output
settingscli -v board list

# Get specific node details
settingscli node get lg-OptiPlex-3070

# Get raw JSON output for a specific SoC
settingscli soc raw 10.221.40.190

# List all containers
settingscli container list

# Get specific container details by ID
settingscli container get 2a465a2ea2d8ce9d35ab5eaae729067267ec09377edf89d02daa6c78d3787d2e

# Get raw container data
settingscli container raw

# Apply YAML artifact
settingscli yaml apply /path/to/artifact.yaml

# Apply YAML from stdin
cat artifact.yaml | settingscli yaml apply -

# Withdraw YAML artifact
settingscli yaml withdraw /path/to/artifact.yaml
```

### YAML Artifact Format

When using YAML operations, the artifact should be a multi-document YAML containing:

```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: example-scenario
spec:
  condition: null
  action: update
  target: example-target
---
apiVersion: v1
kind: Package
metadata:
  name: example-package
spec:
  pattern:
    - type: plain
  models:
    - name: example-model
      node: target-node
---
apiVersion: v1
kind: Model
metadata:
  name: example-model
  annotations:
    io.piccolo.annotations.package-type: example
  labels:
    app: example
spec:
  hostNetwork: true
  containers:
    - name: example-container
      image: alpine:latest
  restartPolicy: Always
```

## Testing

### Running Tests

```bash
# Run unit tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_client_creation
```

### Integration Testing

With all services running, test the CLI:

```bash
# Test basic connectivity
./target/release/settingscli health

# Test metrics endpoint
./target/release/settingscli metrics get

# Test all resource endpoints
./target/release/settingscli board list
./target/release/settingscli node list
./target/release/settingscli soc list
./target/release/settingscli container list

# Test YAML operations (requires valid YAML file)
./target/release/settingscli yaml apply examples/helloworld.yaml
./target/release/settingscli yaml withdraw examples/helloworld.yaml
```

## Error Handling

The CLI provides clear error messages for common issues:

- **Connection errors**: When SettingsService is unreachable
- **Timeout errors**: When requests take too long
- **JSON parsing errors**: When response format is unexpected
- **HTTP errors**: When API endpoints return error status codes
- **File errors**: When YAML files cannot be read or parsed
- **YAML validation errors**: When YAML artifacts are missing required kinds

### Common Error Scenarios

```bash
# Service unreachable
$ settingscli health
✗ Failed to connect to SettingsService: Connection refused

# Invalid container ID
$ settingscli container get invalid-id
✗ Failed to fetch container invalid-id: Request failed with status: 404 Not Found

# Missing YAML file
$ settingscli yaml apply nonexistent.yaml
✗ Failed to apply YAML artifact: File not found: nonexistent.yaml

# Server error during container retrieval
$ settingscli container get <container-id>
✗ Failed to fetch container <container-id>: Request failed with status: 500 Internal Server Error
```

## Development

### Project Structure

```
src/tools/settingscli/
├── Cargo.toml          # Dependencies and metadata
├── src/
│   ├── main.rs         # CLI entry point
│   ├── lib.rs          # Library exports
│   ├── client.rs       # HTTP client implementation
│   ├── error.rs        # Error handling
│   └── commands/       # Command implementations
│       ├── mod.rs      # Command utilities
│       ├── metrics.rs  # Metrics operations
│       ├── board.rs    # Board operations
│       ├── node.rs     # Node operations
│       ├── soc.rs      # SoC operations
│       ├── container.rs # Container operations
│       └── yaml.rs     # YAML artifact management
└── tests/              # Integration tests
    ├── integration_test.rs
    └── cli_test.rs
```

### Adding New Commands

1. Create a new file in `src/commands/`
2. Implement the command logic with appropriate error handling
3. Add the command to the main CLI enum in `main.rs`
4. Update the command dispatcher
5. Add tests for the new functionality

### Code Validation

Before committing changes, always run:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
scripts/fmt_check.sh
scripts/clippy_check.sh
make build
```

**NEVER CANCEL** these validation steps.

## API Endpoints

The CLI interacts with the following SettingsService REST API endpoints:

### Resource Management APIs

| Command | HTTP Method | Endpoint | Description |
|---------|-------------|----------|-------------|
| `metrics get` | GET | `/api/v1/metrics` | Get all system metrics |
| `metrics raw` | GET | `/api/v1/metrics` | Get raw metrics data |
| `board list` | GET | `/api/v1/boards` | List all boards |
| `board get <id>` | GET | `/api/v1/boards/{id}` | Get specific board |
| `board raw` | GET | `/api/v1/boards` | Get raw board data |
| `node list` | GET | `/api/v1/nodes` | List all nodes |
| `node get <name>` | GET | `/api/v1/nodes/{name}` | Get specific node |
| `node raw` | GET | `/api/v1/nodes` | Get raw node data |
| `soc list` | GET | `/api/v1/socs` | List all SoCs |
| `soc get <id>` | GET | `/api/v1/socs/{id}` | Get specific SoC |
| `soc raw` | GET | `/api/v1/socs` | Get raw SoC data |
| `container list` | GET | `/api/v1/containers` | List all containers |
| `container get <id>` | GET | `/api/v1/containers/{id}` | Get specific container |
| `container raw` | GET | `/api/v1/containers` | Get raw container data |

### YAML Artifact APIs

| Command | HTTP Method | Endpoint | Description |
|---------|-------------|----------|-------------|
| `yaml apply <file>` | POST | `/api/v1/yaml` | Apply YAML artifact |
| `yaml withdraw <file>` | DELETE | `/api/v1/yaml` | Withdraw YAML artifact |

### System APIs

| Command | HTTP Method | Endpoint | Description |
|---------|-------------|----------|-------------|
| `health` | GET | `/api/v1/system/health` | Health check |

### Response Formats

**Metrics Response:**
```json
[
  {
    "component": "node|container|soc|board",
    "id": "resource-id",
    "metric_type": "NodeInfo|ContainerInfo|SocInfo|BoardInfo",
    "timestamp": "2025-10-23T08:47:40.207254214Z",
    "value": {
      "type": "NodeInfo",
      "value": { "node_name": "...", "cpu_usage": 7.34, ... }
    }
  }
]
```

**Container Response:**
```json
[
  {
    "id": "container-id",
    "names": ["container-name"],
    "image": "docker.io/library/alpine:latest",
    "state": {
      "Status": "running",
      "Running": "true",
      "Pid": "1234"
    },
    "config": {
      "Hostname": "hostname",
      "User": "root"
    },
    "stats": {
      "CpuTotalUsage": "68328000",
      "MemoryUsage": "1548288"
    }
  }
]
```

**YAML Apply/Withdraw Response:**
```json
{
  "message": "YAML artifact applied successfully",
  "applied": [
    {"kind": "Scenario", "name": "example"},
    {"kind": "Package", "name": "example"},
    {"kind": "Model", "name": "example"}
  ]
}
```

## Troubleshooting

### Common Issues and Solutions

#### 1. Connection Issues
```bash
# Problem: Cannot connect to SettingsService
$ settingscli health
✗ Failed to connect to SettingsService: Connection refused

# Solution: Check if SettingsService is running and accessible
# Verify the correct URL and port (default: localhost:8080)
settingscli -u http://localhost:8080 health
```

#### 2. Container API 500 Errors
```bash
# Problem: Container GET returns 500 Internal Server Error
$ settingscli container get <container-id>
✗ Failed to fetch container: Request failed with status: 500 Internal Server Error

# Solution: This is a known server-side issue. Use container list or raw instead:
settingscli container list
settingscli container raw
```

#### 3. Empty Results
```bash
# Problem: Commands return empty results
$ settingscli board list
No boards found.

# Solution: Ensure all required services are running:
# 1. podman.socket service
sudo systemctl start podman.socket

# 2. Pullpiri services (in order)
cargo run --manifest-path=src/player/statemanager/Cargo.toml &
cargo run --manifest-path=src/server/monitoringserver/Cargo.toml &
cargo run --manifest-path=src/server/settingsservice/Cargo.toml &
cargo run --manifest-path=src/agent/nodeagent/Cargo.toml &
```

#### 4. YAML Validation Warnings
```bash
# Problem: YAML apply shows missing kinds warning
⚠ Warning: Missing recommended kinds: Package, Model
The API Server expects Scenario, Package, and Model kinds for proper operation.

# Solution: Ensure your YAML contains all required document types:
# - Scenario (defines the operation)
# - Package (defines the package structure)
# - Model (defines the container/pod specification)
```

#### 5. Timeout Issues
```bash
# Problem: Requests timing out
$ settingscli metrics get
✗ Request timed out

# Solution: Increase timeout for slow responses
settingscli -t 60 metrics get
```

### Service Dependencies

The CLI requires these services to be running for full functionality:

1. **podman.socket** - Container management
2. **statemanager** - Application state management
3. **monitoringserver** - System metrics collection
4. **settingsservice** - REST API server
5. **nodeagent** - Node resource monitoring

### Debug Mode

Enable verbose output for troubleshooting:

```bash
# Enable verbose output
settingscli -v <command>

# Example with connection details
settingscli -v -u http://192.168.1.100:8080 health
```

## Complete Command Reference

### Global Commands

```bash
# Show help for all commands
settingscli --help

# Show version information
settingscli --version

# Test connectivity to SettingsService
settingscli health
settingscli -u http://custom-host:8080 health
```

### Metrics Commands

```bash
# Get formatted system metrics
settingscli metrics get

# Get raw JSON metrics data
settingscli metrics raw

# With custom settings
settingscli -u http://remote-host:8080 -t 30 metrics get
```

### Board Commands

```bash
# List all boards (formatted)
settingscli board list

# Get specific board details
settingscli board get 10.221.40.100

# Get raw board data (all boards)
settingscli board raw

# Get raw data for specific board
settingscli board raw 10.221.40.100
```

### Node Commands

```bash
# List all nodes (formatted)
settingscli node list

# Get specific node details
settingscli node get lg-OptiPlex-3070

# Get raw node data (all nodes)
settingscli node raw

# Get raw data for specific node
settingscli node raw lg-OptiPlex-3070
```

### SoC Commands

```bash
# List all SoCs (formatted)
settingscli soc list

# Get specific SoC details
settingscli soc get 10.221.40.190

# Get raw SoC data (all SoCs)
settingscli soc raw

# Get raw data for specific SoC
settingscli soc raw 10.221.40.190
```

### Container Commands

```bash
# List all containers (formatted)
settingscli container list

# Get specific container details (may return 500 error - known issue)
settingscli container get 2a465a2ea2d8ce9d35ab5eaae729067267ec09377edf89d02daa6c78d3787d2e

# Get raw container data (recommended for detailed info)
settingscli container raw
```

### YAML Artifact Commands

```bash
# Apply YAML artifact from file
settingscli yaml apply /path/to/artifact.yaml
settingscli yaml apply ./examples/helloworld.yaml

# Apply YAML from stdin
cat artifact.yaml | settingscli yaml apply -
echo "apiVersion: v1..." | settingscli yaml apply -

# Withdraw YAML artifact from file
settingscli yaml withdraw /path/to/artifact.yaml

# Withdraw YAML from stdin
cat artifact.yaml | settingscli yaml withdraw -
```

### Advanced Usage Examples

```bash
# Chain commands with jq for JSON processing
settingscli metrics raw | jq '.[] | select(.component == "node")'

# Save raw output to file
settingscli container raw > containers.json

# Check specific container by name pattern
settingscli container raw | jq '.[] | select(.names[0] | contains("alpine"))'

# Apply YAML with error handling
settingscli yaml apply deployment.yaml && echo "Success" || echo "Failed"

# Use custom timeout for slow networks
settingscli -t 120 -u http://remote-vehicle:8080 board list

# Verbose mode with custom settings
settingscli -v -u http://192.168.1.100:8080 -t 60 metrics get
```

## Related Components

- **SettingsService**: The REST API server this CLI communicates with (port 8080)
- **Pullpiri Core**: The main vehicle service orchestrator framework
- **Monitoring Server**: Provides system metrics and monitoring data (port 47001+)
- **State Manager**: Manages application state and configuration
- **Node Agent**: Reports node resource utilization and container status
- **API Server**: Processes YAML artifacts for vehicle service orchestration (port 47099)

### Service Architecture

```
┌─────────────────┐    HTTP/REST    ┌─────────────────┐
│   SettingsCLI   │◄──────────────► │ SettingsService │
│                 │                 │   (port 8080)   │
└─────────────────┘                 └─────────────────┘
                                            │
                                            │ ETCD
                                            ▼
                                    ┌─────────────────┐
                                    │      ETCD       │
                                    │  (2379, 2380)   │
                                    └─────────────────┘
                                            ▲
                    ┌───────────────────────┼───────────────────────┐
                    │                       │                       │
            ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
            │ MonitoringServer│    │   StateManager  │    │    NodeAgent    │
            │   (gRPC/REST)   │    │   (gRPC/REST)   │    │   (gRPC/REST)   │
            └─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Port Allocation

Following Pullpiri networking conventions:
- **SettingsService**: `8080` (configurable within 47001-47099 range)
- **SettingsCLI**: Connects to SettingsService (no listening port)
- **API Server**: `47099` (receives YAML artifacts from SettingsService)
- **Other Pullpiri Services**: `47001-47099` (gRPC: 47001+, REST: up to 47099)
- **ETCD**: `2379, 2380` (standard ETCD ports)

## License

This project is licensed under the Apache-2.0 license.