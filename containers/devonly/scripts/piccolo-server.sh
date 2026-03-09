#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

if [ -n "${1:-}" ]; then
	MASTER_IP="$1"
else
	MASTER_IP="$(hostname -I | awk '{print $1}')"
fi

# Set environment variables
ROCKSDB_VERSION="v11.18.0"
ROCKSDB_IMAGE="ghcr.io/mco-piccolo/pullpiri-rocksdb:${ROCKSDB_VERSION}"

CONTAINER_IMAGE="localhost/pullpiri:latest"
echo "Running server with image: ${CONTAINER_IMAGE}"

# Create a pod with host networking
podman pod create \
  --name piccolo-server 

# Run rocksdbservice container
podman run -d \
  --pod piccolo-server \
  --network host \
  --name piccolo-rocksdbservice \
  --user 0:0 \
  -e RUST_LOG="info" \
  -v /etc/piccolo/pullpiri_shared_rocksdb:/data:Z \
  ${ROCKSDB_IMAGE} \
  rocksdbservice --path /data --addr 0.0.0.0 --port 47007

# Run apiserver container
podman run -d \
  --pod piccolo-server \
  --network host \
  --name piccolo-apiserver \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -e ZENOH_CONFIG="/etc/piccolo/zenoh-config.json5" \
  -e UPROTOCOL_TOPIC="up://pullpiri-api/D200/2/D200" \
  -e VEHICLE_ID="pullpiri-vehicle-001" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /etc/piccolo/zenoh-config.json5:/etc/piccolo/zenoh-config.json5:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/apiserver

# Run policymanager container
podman run -d \
  --pod piccolo-server \
  --network host \
  --name piccolo-policymanager \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/policymanager

# Run monitoringserver container
podman run -d \
  --pod piccolo-server \
  --network host \
  --name piccolo-monitoringserver \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/monitoringserver

# Run logservice container
podman run -d \
  --pod piccolo-server \
  --network host \
  --name piccolo-logservice \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/logservice

# Run settingsservice container
podman run -d \
  --pod piccolo-server \
  --network host \
  --name piccolo-settingsservice \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -e ZENOH_CONFIG="/etc/piccolo/zenoh-config.json5" \
  -e UPROTOCOL_TOPIC="up://pullpiri-settings/D200/1/D200" \
  -e VEHICLE_ID="pullpiri-vehicle-001" \
  -e STATUS_INTERVAL_SECS="5" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /etc/piccolo/zenoh-config.json5:/etc/piccolo/zenoh-config.json5:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/settingsservice --bind-address=${MASTER_IP} --bind-port=8888 --log-level=debug