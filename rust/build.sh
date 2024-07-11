#! /bin/bash

# Read the version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | sed -E 's/version = "(.*)"/\1/')
if [ -z ${VERSION} ]; then
	echo "\${VERSION} cannot be empty."
	exit 1
else
	echo VERSION=${VERSION}
fi

set -ex

function debug() (
	docker-compose up --build
)

function buildx() (
	docker buildx build -t rust-hue:arm-${VERSION} --platform linux/arm64 --output=type=docker .
)

function publish() (
	source .env
	if [ -z "${REGISTRY_URL}" ]; then
		echo "Env var \${REGISTRY_URL} not found."
		exit 1
	fi

	docker tag rust-hue:arm-${VERSION} ${REGISTRY_URL}/rust-hue:arm-${VERSION}
	docker push ${REGISTRY_URL}/rust-hue:arm-${VERSION}

	docker tag rust-hue:arm-${VERSION} ${REGISTRY_URL}/rust-hue:arm-latest
	docker push ${REGISTRY_URL}/rust-hue:arm-latest
)

function all() (
	cargo clippy
	cargo test
	buildx
	publish
)

$*