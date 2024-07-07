set -ex

function template() (
	helm template . -f values.yaml -f values.private.yaml > helm-output.yaml
)

if [ -z $@ ]; then
	template
else
	$@
fi