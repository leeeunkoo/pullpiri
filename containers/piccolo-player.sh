#!/bin/bash

# Check environment argument
ENV="${1:-dev}"

# Set environment variables
VERSION="latest"
if [ "$ENV" = "prod" ]; then
    CONTAINER_IMAGE="ghcr.io/eclipse-pullpiri/pullpiri:${VERSION}"
else
    CONTAINER_IMAGE="localhost/pullpiri:latest"
fi
HOST_IP=$(hostname -I | awk '{print $1}')

echo "Running in ${ENV} mode with image: ${CONTAINER_IMAGE}"

# Create a pod with host networking
podman pod create \
  --name piccolo-player \
  --network host \
  --pid host

# Run filtergateway container
podman run -d \
  --pod piccolo-player \
  --name piccolo-filtergateway \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/yaml:/root/piccolo_yaml:Z \
  -v /etc/containers/systemd/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/filtergateway

# Run actioncontroller container
podman run -d \
  --pod piccolo-player \
  --name piccolo-actioncontroller \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/yaml:/root/piccolo_yaml:Z \
  -v /run/dbus:/run/dbus:Z \
  -v /etc/containers/systemd:/etc/containers/systemd:Z \
  -v /etc/containers/systemd/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/actioncontroller

# Run statemanager container
podman run -d \
  --pod piccolo-player \
  --name piccolo-statemanager \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/yaml:/root/piccolo_yaml:Z \
  -v /run/dbus:/run/dbus:Z \
  -v /etc/containers/systemd:/etc/containers/systemd:Z \
  -v /etc/containers/systemd/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/statemanager