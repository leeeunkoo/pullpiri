#!/bin/bash

# Check environment argument
ENV="${1:-TODO}"

# Set environment variables
ROCKSDB_VERSION="v11.18.0"
ROCKSDB_IMAGE="ghcr.io/mco-piccolo/pullpiri-rocksdb:${ROCKSDB_VERSION}"

VERSION="latest"
if [ "$ENV" = "prod" ]; then
    CONTAINER_IMAGE="ghcr.io/eclipse-pullpiri/pullpiri:${VERSION}"
elif [ "$ENV" = "dev" ]; then
    CONTAINER_IMAGE="localhost/pullpiri:latest"
else
    echo "Error: Invalid environment '${ENV}'. Must be 'prod' or 'dev'."
    exit 1
fi
echo "Running server in ${ENV} mode with image: ${CONTAINER_IMAGE}"

HOST_IP=$(hostname -I | awk '{print $1}')
# Create a pod with host networking
podman pod create \
  --name piccolo-server \
  --network host \
  --pid host

# Run rocksdbservice container
podman run -d \
  --pod piccolo-server \
  --name piccolo-rocksdbservice \
  --user 0:0 \
  -e RUST_LOG="info" \
  -v /etc/piccolo/pullpiri_shared_rocksdb:/data:Z \
  ${ROCKSDB_IMAGE} \
  rocksdbservice --path /data --addr 0.0.0.0 --port 47007

# Run apiserver container
podman run -d \
  --pod piccolo-server \
  --name piccolo-apiserver \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/apiserver

# Run policymanager container
podman run -d \
  --pod piccolo-server \
  --name piccolo-policymanager \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  ${CONTAINER_IMAGE} \
  /piccolo/policymanager

# Run monitoringserver container
podman run -d \
  --pod piccolo-server \
  --name piccolo-monitoringserver \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/monitoringserver

# Run settingsservice container
podman run -d \
  --pod piccolo-server \
  --name piccolo-settingsservice \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/settingsservice --bind-address=${HOST_IP} --bind-port=8080 --log-level=debug
