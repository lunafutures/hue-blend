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
	docker tag rust-hue:arm-${VERSION} registry.blahblahblah.xyz/rust-hue:arm-${VERSION}
	docker push registry.blahblahblah.xyz/rust-hue:arm-${VERSION}

	docker tag rust-hue:arm-${VERSION} registry.blahblahblah.xyz/rust-hue:arm-latest
	docker push registry.blahblahblah.xyz/rust-hue:arm-latest
)

function all() (
	cargo clippy
	cargo test
	buildx
	publish
)

$*