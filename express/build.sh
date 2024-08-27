#! /bin/bash
set -ex

version=$npm_package_version
if [ -z $version ]; then
	echo "\$version cannot be empty. Run this script from \`npm run\`."
	exit 1
else
	echo version=${version}
fi

npm run lint
npm run test

source .env
webpack

docker buildx build -t "hue-express:arm-${version}" --platform linux/arm64 --output=type=docker .
docker tag "hue-express:arm-${version}" "${REGISTRY_URL}/hue-express:arm-${version}"
docker push "${REGISTRY_URL}/hue-express:arm-${version}"

docker tag "hue-express:arm-${version}" "${REGISTRY_URL}/hue-express:arm-latest"
docker push "${REGISTRY_URL}/hue-express:arm-latest"