set -ex

function template() (
	helm template . --debug -f values.yaml -f values.private.yaml > helm-output.yaml
)

RELEASE_NAME=davis
function install() (
	helm install ${RELEASE_NAME} . --debug -f values.yaml -f values.private.yaml
)

function upgrade() (
	helm upgrade ${RELEASE_NAME} . --debug -f values.yaml -f values.private.yaml
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

if [ -z $@ ]; then
	template
else
	$@
fi