<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# Test Procedure: Dashboard Integration with Monitoring Server and Settings Service

This guide describes how to set up and validate the integration between the NodeAgent, Monitoring Server, and Settings Service for dashboard development.

---

> **Note:**  
> NUC1 and NUC2 must be on the same subnet network to ensure proper communication between services.

---

## Prerequisites

- **Hardware:** 2 NUCs (NUC1 and NUC2)
- **Software:**  
  - Rust toolchain installed (`rustup`, `cargo`)
  - Podman installed for container management
  - ETCD v3.5.11 container image
  - Pullpiri source code checked out on both NUCs

---

## Step 1: Start ETCD on NUC1

Run the following command on **NUC1** to start ETCD as a container named `piccolo-etcd`:

```bash
podman run -it -d --net=host --name=piccolo-etcd \
  gcr.io/etcd-development/etcd:v3.5.11 "/usr/local/bin/etcd"
```

Verify ETCD is running:

```bash
podman ps | grep piccolo-etcd
```

---

## Step 2: Prepare NodeAgent Configuration on NUC2

Create the configuration file at `/etc/piccolo/nodeagent.yaml` on **NUC2**:

```yaml
nodeagent:
  node_name: "acrn-NUC11TNHi5"         # Hostname of NUC2
  node_type: "vehicle"
  node_role: "bluechi"
  master_ip: "<NUC1_IP_ADDRESS>"       # IP address of NUC1
  node_ip: "<NUC2_IP_ADDRESS>"         # IP address of NUC2
  grpc_port: 47004
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  system:
    hostname: "acrn-NUC11TNHi5"        # Hostname of NUC2
    platform: "Linux"                  # Modify according to NUC2 system
    architecture: "x86_64"
yaml_storage: "/etc/piccolo/yaml"
```

> **Note:** Replace `<NUC1_IP_ADDRESS>` and `<NUC2_IP_ADDRESS>` with the actual IP addresses.

---

## Step 3: Start Monitoring Server on NUC1

On **NUC1**, run the monitoring server:

```bash
cd ~/new_ak/new/pullpiri/src/server/monitoringserver
cargo run
```

---

## Step 4: Start Settings Service on NUC1

On **NUC1**, run the settings service:

```bash
cd ~/new_ak/new/pullpiri/src/server/settingsservice
cargo run
```

---

## Step 5: Start NodeAgent on NUC2

On **NUC2**, run the nodeagent with the prepared configuration:

```bash
cd ~/new_ak/new/pullpiri/src/agent/nodeagent
cargo run
```

---

## Step 6: Validate Data Flow

- **On NUC1 (Monitoring Server):**  
  Check the logs for messages indicating receipt of SOC/Board/Node/Container info from NUC2's NodeAgent.

- **On NUC1 (Settings Service):**  
  Use the REST API endpoints to query real-time info about nodes, boards, SOCs, and containers.

  Example (replace port and endpoint as needed):Or Follow /src/server/settingsservice/testing_procedure.md

  ```bash
  curl http://localhost:8080/api/v1/settings
  curl http://localhost:8080/api/v1/metrics
  curl http://localhost:8080/api/v1/history
  curl http://localhost:8080/api/v1/system/health
  ```

---

## Expected Results

- Monitoring server on NUC1 logs incoming data from NUC2 NodeAgent.
- Settings service REST APIs on NUC1 provide real-time information about node, board, SOC, and container status.

---

## Troubleshooting

- Ensure ETCD is running and accessible from both NUCs.
- Verify network connectivity between NUC1 and NUC2.
- Check configuration files for correct IP addresses and hostnames.
- Review logs for errors and resolve any issues with service startup.

---