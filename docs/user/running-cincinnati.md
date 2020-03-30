# Running Cincinnati

## Prerequisite

APP-SRE publishes the Cincinnati container images at [Quay registry](https://quay.io/repository/app-sre/cincinnati).

We need an image tag from the [Quay registry](https://quay.io/repository/app-sre/cincinnati) to use with the deployment config.

The Cincinnati deploymentconfig is available [here](../../dist/openshift/cincinnati.yaml).

### Steps

#### Create a new namespace for Cincinnati

```shell
oc new-project cincinnati
```

#### Create Cincinnati deployment

```shell
export IMAGE_TAG=<TAG>
oc new-app -f cincinnati.yaml \
  -p IMAGE_TAG=${IMAGE_TAG}
```

#### Wait for the deployment config to rollout

```shell
oc wait --for=condition=available --timeout=5m deploymentconfig/cincinnati
```

#### Create an ingress route to the Cincinnati policy engine

Example of a yaml for an ingress route to the policy engine

```yaml
kind: Route
apiVersion: route.openshift.io/v1
metadata:
  name: cincinnati
  namespace: cincinnati
  labels:
    app: cincinnati-policy-engine
  annotations:
    openshift.io/host.generated: 'true'
spec:
  host: cincinnati-cincinnati.apps.ci-ln-13sjybk-d5d6b.origin-ci-int-aws.dev.rhcloud.com
  to:
    kind: Service
    name: cincinnati-policy-engine
    weight: 100
  port:
    targetPort: policy-engine
  tls:
    termination: edge
  wildcardPolicy: None
status:
  ingress:
  - conditions:
    host: cincinnati-cincinnati.apps.ci-ln-13sjybk-d5d6b.origin-ci-int-aws.dev.rhcloud.com
    routerCanonicalHostname: apps.ci-ln-13sjybk-d5d6b.origin-ci-int-aws.dev.rhcloud.com
    routerName: default
    wildcardPolicy: None
```

#### Example

Here is an example [bash script](../../hack/deploy_cincinnati.sh) to depoly Cincinnati on OpenShift.

## Configure a container registry to scrape release payload information

Cincinnati can fetch the release payload information (primary metadata) from any container registry compatible with [Docker registry API v2][registry-api-v2].

You can change the default registry in Cincinnati deployment config when you start a deployment.

Example:

```shell
oc new-app -f dist/openshift/cincinnati.yaml \
  -p GB_PLUGIN_SETTINGS="$(cat <<-EOF
      [[plugin_settings]]
      name = "release-scrape-dockerv2"
      registry = "registry-1.docker.io"
      repository = "ocprepository/openshift-release-dev_ocp-release"
EOF
)"
```

[registry-api-v2]: https://docs.docker.com/registry/spec/api
