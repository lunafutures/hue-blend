#! /bin/bash
set -ex

source .env
webpack

version=$npm_package_version
docker buildx build -t "hue-express:arm-${version}" --platform linux/arm64 --output=type=docker .
docker tag "hue-express:arm-${version}" "${REGISTRY_URL}/hue-express:arm-${version}"
docker push "${REGISTRY_URL}/hue-express:arm-${version}"