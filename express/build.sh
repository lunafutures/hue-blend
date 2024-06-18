#! /bin/bash
set -ex
source .env
webpack
docker buildx build -t hue-express:arm-1.0 --platform linux/arm64 --output=type=docker .
docker tag "hue-express:arm-1.0" "${REGISTRY_URL}/hue-express:arm-1.0"
docker push "${REGISTRY_URL}/hue-express:arm-1.0"