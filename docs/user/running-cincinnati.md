# Running Cincinnati

## Prerequisite

APP-SRE publishes the Cincinnati container images at [Quay registry](https://quay.io/repository/app-sre/cincinnati).

We need an image tag from the [Quay registry](https://quay.io/repository/app-sre/cincinnati) to use with the deployment config.

The Cincinnati deployment is available [here](../../dist/openshift/cincinnati-deployment.yaml).

### Steps

#### Create a new namespace for Cincinnati

```shell
oc new-project cincinnati
```

#### Create secret for container registry

If Cincinnati is configured to use services which require authentication, you need to create a secret with name `cincinnati-credentials`. Make sure the auth for the container registry is present in the config.json file to access a container registry with release payloads. See the [container auth format specification][container-auth-format-spec] for more information.

```shell
oc create secret generic cincinnati-credentials --from-file=.dockerconfigjson=/home/lmohanty/.docker/config.json --type=kubernetes.io/dockerconfigjson
```

An example config.json:

```
$ cat ~/.docker/config.json
{
        "auths": {
                "quay.io": {
                        "auth": "bar=="
                }
        }
}
```

If you are using a secure external container registry to hold mirrored OpenShift
release images, Cincinnati will need access to this registry in order to build
an upgrade graph.  Here's how you can inject your CA Cert into the Cincinnati
pod.

OpenShift has an external registry API, located at `image.config.openshift.io`,
that we'll use to store the external registry CA Cert.  You can read more about
this API in the [OpenShift documentation](https://docs.openshift.com/container-platform/4.3/registry/configuring-registry-operator.html#images-configuration-cas_configuring-registry-operator).

For more information around use of the container registry, see the section on [configuring a container registry](#configure-a-container-registry-to-scrape-release-payload-information).

#### Create Cincinnati deployment

```shell
export IMAGE_TAG=<TAG>
oc new-app -f cincinnati-deployment \
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
oc new-app -f dist/openshift/cincinnati-deployment.yaml \
  -p GB_PLUGIN_SETTINGS="$(cat <<-EOF
      [[plugin_settings]]
      name = "release-scrape-dockerv2"
      registry = "registry-1.docker.io"
      repository = "ocprepository/openshift-release-dev_ocp-release"
EOF
)"
```

[registry-api-v2]: https://docs.docker.com/registry/spec/api
[container-auth-format-spec]: https://github.com/containers/image/blob/v5.5.2/docs/containers-auth.json.5.md
