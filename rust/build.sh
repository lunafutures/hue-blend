#! /bin/bash

# Read the version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | sed -E 's/version = "(.*)"/\1/')
if [ -z ${VERSION} ]; then
	echo "\${VERSION} cannot be empty."
	exit 1
else
	echo VERSION=${VERSION}
fi


function debug() (
	set -ex
	docker-compose --build up
)

function buildx() (
	set -ex
	docker buildx build -t rust-hue:arm-${VERSION} --platform linux/arm64 --output=type=docker .
)

function publish() (
	set -ex
	docker tag rust-hue:arm-${VERSION} registry.blahblahblah.xyz/rust-hue:arm-${VERSION}
	docker push registry.blahblahblah.xyz/rust-hue:arm-${VERSION}

	docker tag rust-hue:arm-${VERSION} registry.blahblahblah.xyz/rust-hue:arm-latest
	docker push registry.blahblahblah.xyz/rust-hue:arm-latest
)

function all() (
	set -ex
	cargo test
	buildx
	publish
)

$*