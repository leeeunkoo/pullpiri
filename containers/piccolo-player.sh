#!/bin/bash

# Check environment argument
ENV="${1:-TODO}"

# Set environment variables
VERSION="latest"
if [ "$ENV" = "prod" ]; then
    CONTAINER_IMAGE="ghcr.io/eclipse-pullpiri/pullpiri:${VERSION}"
elif [ "$ENV" = "dev" ]; then
    CONTAINER_IMAGE="localhost/pullpiri:latest"
else
    echo "Error: Invalid environment '${ENV}'. Must be 'prod' or 'dev'."
    exit 1
fi
echo "Running player in ${ENV} mode with image: ${CONTAINER_IMAGE}"

HOST_IP=$(hostname -I | awk '{print $1}')
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
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/filtergateway

# Run actioncontroller container
podman run -d \
  --pod piccolo-player \
  --name piccolo-actioncontroller \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/actioncontroller

# Run statemanager container
podman run -d \
  --pod piccolo-player \
  --name piccolo-statemanager \
  -e ROCKSDB_SERVICE_URL="http://${HOST_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  ${CONTAINER_IMAGE} \
  /piccolo/statemanager