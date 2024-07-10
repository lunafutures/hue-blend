set -ex

function template() (
	helm template . --debug -f values.yaml -f values.private.yaml > helm-output.yaml
)

function build-deps() (
	helm dependency build
)

function lint() (
	helm lint .
)

function create-namespaces() {
	for namespace in registry; do
		kubectl create namespace ${namespace} --dry-run=client -o yaml | kubectl apply -f -
	done
}

RELEASE_NAME=blue
function install() (
	create-namespaces

	kubens default
	helm upgrade --install ${RELEASE_NAME} . \
		-f values.yaml \
		-f values.private.yaml
)

function uninstall() (
	helm uninstall ${RELEASE_NAME}
)

function uninstall-dry-run() (
	helm uninstall ${RELEASE_NAME} --dry-run
)

function show-installed() (
	helm list --all --all-namespaces
)

if [ -z "$@" ]; then
	template
else
	$@
fi